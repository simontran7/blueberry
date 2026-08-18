use std::collections::HashMap;

use crate::common::context::CompilerContext;
use crate::common::types::TypeId;
use crate::front_end::semantic_analysis::hir::{
    BindingId, BindingKind, DefinitionBindingId, DefinitionId, DefinitionKind, ExpressionId,
    ExpressionIdSpan, ExpressionKind, Hir, ParameterIdSpan, StatementId, StatementIdSpan,
    StatementKind,
};
use crate::front_end::syntactic_analysis::cst::operators::{BinOp, UnOp};
use crate::middle_end::cfg_builder::CfgBuilder;
use crate::middle_end::mir::{
    BlockId, Cfg, Function, FunctionReferenceId, Mir, Signature, ValueId,
};

pub(crate) struct MirLowerer<'a> {
    hir: &'a Hir,
    ctx: &'a CompilerContext,
    mir: Mir,
    cfg: CfgBuilder,
    loop_frames: Vec<LoopFrame>,
    callee_references: HashMap<DefinitionBindingId, FunctionReferenceId>,
    constant_definition_initializers: HashMap<DefinitionBindingId, ExpressionId>,
}

struct LoopFrame {
    header_id: BlockId,
    exit_id: BlockId,
    exit_is_branched_to: bool,
}

impl<'a> MirLowerer<'a> {
    pub(crate) fn new(hir: &'a Hir, ctx: &'a CompilerContext) -> Self {
        Self {
            hir,
            ctx,
            mir: Mir::new(),
            cfg: CfgBuilder::new(),
            loop_frames: Vec::new(),
            callee_references: HashMap::new(),
            constant_definition_initializers: hir
                .definitions
                .values()
                .filter_map(|definition| match definition.kind {
                    DefinitionKind::Constant {
                        definition_binding_id,
                        initializer_id,
                    } => Some((definition_binding_id, initializer_id)),
                    DefinitionKind::Function { .. } => None,
                })
                .collect(),
        }
    }

    pub(crate) fn lower(mut self) -> Mir {
        for function_id in self.hir.functions_ids() {
            let function = self.lower_function(function_id);
            self.mir.add_function(function);
        }
        self.mir
    }

    fn lower_function(&mut self, function_id: DefinitionId) -> Function {
        let DefinitionKind::Function {
            definition_binding_id,
            parameter_id_span,
            body_id,
        } = *self.hir.get_definition(function_id).kind()
        else {
            panic!("`MirLowerer::lower_function()` expects an HIR function handle")
        };
        let definition_binding_view = self.hir.get_definition_binding(definition_binding_id);

        let name = definition_binding_view.name();

        let (parameter_type_ids, return_type_id) = self
            .ctx
            .type_interner
            .as_func(definition_binding_view.ty())
            .unwrap();
        let signature = Signature {
            parameter_type_ids: parameter_type_ids
                .iter()
                .copied()
                .filter(|&ty| !self.ctx.type_interner.is_zero_sized(ty))
                .collect(),
            return_type_id: return_type_id,
        };

        self.loop_frames.clear();
        self.callee_references.clear();
        let body = self.lower_body(parameter_id_span, body_id);

        Function {
            definition_binding_id,
            name,
            signature,
            body,
        }
    }

    fn lower_body(&mut self, parameter_id_span: ParameterIdSpan, body_id: ExpressionId) -> Cfg {
        let entry_block_id = self.cfg.create_block();
        self.cfg.add_block(entry_block_id);
        self.cfg.seal_block(entry_block_id);

        for &parameter_binding_id in self.hir.get_parameter_binding_ids(parameter_id_span) {
            let parameter_binding_view = self.hir.get_local_binding(parameter_binding_id);
            if self
                .ctx
                .type_interner
                .is_zero_sized(parameter_binding_view.ty())
            {
                continue;
            }
            let value_id = self
                .cfg
                .append_block_parameter(entry_block_id, parameter_binding_view.ty());
            self.cfg
                .write_variable(parameter_binding_id, entry_block_id, value_id);
        }

        let tail_id = self.lower_expression(body_id);

        if !self.cfg.is_filled_here() {
            match tail_id {
                Some(value_id) => self.cfg.emit_return(&[value_id]),
                None => self.cfg.emit_return(&[]),
            };
        }

        self.cfg.finish()
    }

    fn lower_statement(&mut self, statement_id: StatementId) {
        match *self.hir.get_statement(statement_id).kind() {
            StatementKind::Expression { expression_id, .. } => {
                self.lower_expression(expression_id);
            }
            StatementKind::Let {
                pattern_id,
                value_id,
            } => {
                // No initializer: leave the binding unwritten in this block.
                // A later read resolves it through the ordinary SSA
                // machinery, which falls back to `add_undefined` in
                // `cfg_builder.rs` when no definition reaches that point.
                let Some(value_id) = value_id else {
                    return;
                };
                let ssa_value_id = self.lower_expression(value_id);
                if !self.cfg.is_filled_here()
                    && let Some(ssa_value_id) = ssa_value_id
                {
                    let block = self
                        .cfg
                        .current_block()
                        .expect("not positioned in any block");
                    self.cfg.write_variable(pattern_id, block, ssa_value_id);
                }
            }
            StatementKind::Definition { .. } => {
                // nested non-closure functions are lowered as separate MIR Functions
            }
        }
    }

    fn lower_expression(&mut self, expression_id: ExpressionId) -> Option<ValueId> {
        let expression_view = self.hir.get_expression(expression_id);
        match *expression_view.kind() {
            ExpressionKind::Missing => {
                unreachable!("driver stops before MIR lowering whenever the CST has Error nodes")
            }
            ExpressionKind::Unit => None,
            ExpressionKind::Boolean(value) => {
                Some(self.cfg.emit_boolean_literal(value, expression_view.ty()))
            }
            ExpressionKind::Integer(value) => {
                Some(self.cfg.emit_integer_literal(expression_view.ty(), value))
            }
            ExpressionKind::Unary {
                operator,
                operand_id,
            } => self.lower_unary_operation(operator, operand_id, expression_view.ty()),
            ExpressionKind::Binary {
                operator,
                lhs_id,
                rhs_id,
            } => self.lower_binary_operation(operator, lhs_id, rhs_id, expression_view.ty()),
            ExpressionKind::Variable(binding_id) => {
                self.lower_variable(binding_id, expression_view.ty())
            }
            ExpressionKind::Assign {
                target_id,
                value_id,
            } => self.lower_assign(target_id, value_id),
            ExpressionKind::Return { value_id } => self.lower_return(value_id),
            ExpressionKind::Call {
                callee_id,
                argument_id_span,
            } => self.lower_call(callee_id, argument_id_span, expression_view.ty()),
            ExpressionKind::Block {
                statement_id_span,
                tail_id,
            } => self.lower_block(statement_id_span, tail_id),
            ExpressionKind::If {
                condition_id,
                then_branch_id,
                else_branch_id,
            } => self.lower_if_expression(
                condition_id,
                then_branch_id,
                else_branch_id,
                expression_view.ty(),
            ),
            ExpressionKind::Loop { body_id, .. } => self.lower_loop(body_id, expression_view.ty()),
            ExpressionKind::Break { value_id } => self.lower_break(value_id),
            ExpressionKind::Continue => self.lower_continue(),
        }
    }

    fn lower_unary_operation(
        &mut self,
        operator: UnOp,
        operand_id: ExpressionId,
        type_id: TypeId,
    ) -> Option<ValueId> {
        let operand_id = self.lower_expression(operand_id)?;
        Some(self.cfg.emit_unary(operator, operand_id, type_id))
    }

    fn lower_binary_operation(
        &mut self,
        operator: BinOp,
        lhs_id: ExpressionId,
        rhs_id: ExpressionId,
        ty: TypeId,
    ) -> Option<ValueId> {
        match operator {
            BinOp::And | BinOp::Or => {
                self.lower_short_circuiting_expression(operator, lhs_id, rhs_id)
            }
            _ => {
                let lhs_ssa_value_id = self.lower_expression(lhs_id)?;
                let rhs_ssa_value_id = self.lower_expression(rhs_id)?;
                Some(
                    self.cfg
                        .emit_binary(operator, lhs_ssa_value_id, rhs_ssa_value_id, ty),
                )
            }
        }
    }

    fn lower_variable(&mut self, binding_id: BindingId, ty_id: TypeId) -> Option<ValueId> {
        match binding_id.kind() {
            BindingKind::Local => {
                if self.ctx.type_interner.is_zero_sized(ty_id) {
                    return None;
                }
                let local_binding_id = binding_id.as_local().unwrap();
                let block = self
                    .cfg
                    .current_block()
                    .expect("not positioned in any block");
                Some(self.cfg.read_variable(local_binding_id, ty_id, block))
            }
            BindingKind::Definition => {
                let definition_binding_id = binding_id.as_definition().unwrap();
                // No storage for constants yet (see TODO.md)
                let initializer_id = self
                    .constant_definition_initializers
                    .get(&definition_binding_id)
                    .expect("function references are not first-class values (yet)");
                self.lower_expression(*initializer_id)
            }
        }
    }

    fn lower_assign(&mut self, target_id: ExpressionId, value_id: ExpressionId) -> Option<ValueId> {
        let local_binding_id = match *self.hir.get_expression(target_id).kind() {
            ExpressionKind::Variable(binding_id) => binding_id.as_local().unwrap(),
            _ => unreachable!(
                "semantic analysis already rejects assignments whose target isn't a plain variable"
            ),
        };

        let ssa_value_id = self.lower_expression(value_id);

        if let Some(ssa_value_id) = ssa_value_id
            && !self.cfg.is_filled_here()
        {
            let block = self
                .cfg
                .current_block().unwrap();
            self.cfg
                .write_variable(local_binding_id, block, ssa_value_id);
        }
        None
    }

    fn lower_return(&mut self, output_id: Option<ExpressionId>) -> Option<ValueId> {
        let output_id = output_id.and_then(|output_id| self.lower_expression(output_id));
        if !self.cfg.is_filled_here() {
            match output_id {
                Some(output_id) => self.cfg.emit_return(&[output_id]),
                None => self.cfg.emit_return(&[]),
            };
        }
        None
    }

    fn lower_call(
        &mut self,
        callee_id: ExpressionId,
        argument_id_span: ExpressionIdSpan,
        type_id: TypeId,
    ) -> Option<ValueId> {
        let definition_binding_id = match *self.hir.get_expression(callee_id).kind() {
            ExpressionKind::Variable(binding_id) => binding_id.as_definition().unwrap(),
            _ => unreachable!("semantic analysis guarantees a callable callee"),
        };
        let callee_reference_id = self.get_callee_reference(definition_binding_id);

        let mut argument_ids = Vec::new();
        for &argument_id in self.hir.get_expression_ids(argument_id_span) {
            let argument_id = self.lower_expression(argument_id);
            if self.cfg.is_filled_here() {
                return None;
            }
            if let Some(argument_id) = argument_id {
                argument_ids.push(argument_id);
            }
        }

        let call_id = if self.ctx.type_interner.is_zero_sized_or_bottom(type_id) {
            self.cfg.emit_call(callee_reference_id, &argument_ids, &[])
        } else {
            self.cfg
                .emit_call(callee_reference_id, &argument_ids, &[type_id])
        };

        if type_id == self.ctx.type_interner.bottom_id {
            self.cfg.emit_unreachable();
            return None;
        }

        self.cfg.first_result(call_id)
    }

    fn lower_block(
        &mut self,
        statement_id_span: StatementIdSpan,
        tail_id: Option<ExpressionId>,
    ) -> Option<ValueId> {
        for &statement_id in self.hir.get_statement_ids(statement_id_span) {
            self.lower_statement(statement_id);
            if self.cfg.is_filled_here() {
                return None;
            }
        }
        tail_id.and_then(|tail_id| self.lower_expression(tail_id))
    }

    fn lower_if_expression(
        &mut self,
        condition_id: ExpressionId,
        then_branch_id: ExpressionId,
        else_branch_id: Option<ExpressionId>,
        type_id: TypeId,
    ) -> Option<ValueId> {
        let condition_id = self.lower_expression(condition_id).unwrap();
        if self.cfg.is_filled_here() {
            return None;
        }

        let produces_value = !self.ctx.type_interner.is_zero_sized(type_id);

        let then_block_id = self.cfg.create_block();
        let else_block_id = else_branch_id.map(|_| self.cfg.create_block());
        let merge_block_id = self.cfg.create_block();

        let block_parameter_id =
            produces_value.then(|| self.cfg.append_block_parameter(merge_block_id, type_id));

        self.cfg.emit_conditional_branch(
            condition_id,
            then_block_id,
            &[],
            else_block_id.unwrap_or(merge_block_id),
            &[],
        );

        self.cfg.add_block(then_block_id);
        self.cfg.seal_block(then_block_id);

        let then_branch_id = self.lower_expression(then_branch_id);

        let mut merge_has_predecessor = false;
        if !self.cfg.is_filled_here() {
            let block_argument_ids = match then_branch_id {
                Some(then_id) if produces_value => vec![then_id],
                _ => vec![],
            };
            self.cfg.emit_jump(merge_block_id, &block_argument_ids);
            merge_has_predecessor = true;
        }

        if let (Some(else_block_id), Some(else_branch_id)) = (else_block_id, else_branch_id) {
            self.cfg.add_block(else_block_id);
            self.cfg.seal_block(else_block_id);

            let else_branch_id = self.lower_expression(else_branch_id);

            if !self.cfg.is_filled_here() {
                match else_branch_id {
                    Some(value_id) if produces_value => {
                        self.cfg.emit_jump(merge_block_id, &[value_id])
                    }
                    _ => self.cfg.emit_jump(merge_block_id, &[]),
                };
                merge_has_predecessor = true;
            }
        } else {
            merge_has_predecessor = true;
        }

        // if the `then` and `else` blocks contain an explicit `return`
        if !merge_has_predecessor {
            return None;
        }

        self.cfg.add_block(merge_block_id);
        self.cfg.seal_block(merge_block_id);

        block_parameter_id
    }

    fn lower_short_circuiting_expression(
        &mut self,
        operator: BinOp,
        lhs_id: ExpressionId,
        rhs_id: ExpressionId,
    ) -> Option<ValueId> {
        let lhs_id = self.lower_expression(lhs_id)?;
        if self.cfg.is_filled_here() {
            return None;
        }

        let rhs_block_id = self.cfg.create_block();
        let merge_block_id = self.cfg.create_block();

        let block_parameter_id = self
            .cfg
            .append_block_parameter(merge_block_id, self.ctx.type_interner.bool_id);

        match operator {
            // `lhs and rhs` desugars to `if lhs { rhs } else { lhs }`, but
            BinOp::And => self.cfg.emit_conditional_branch(
                lhs_id,
                rhs_block_id,
                &[],
                merge_block_id,
                &[lhs_id],
            ),
            // `lhs or rhs` desugars to `if lhs { lhs } else { rhs }`
            BinOp::Or => self.cfg.emit_conditional_branch(
                lhs_id,
                merge_block_id,
                &[lhs_id],
                rhs_block_id,
                &[],
            ),
            _ => unreachable!("short-circuit expressions can only contain `and` or `or` operators"),
        };

        self.cfg.add_block(rhs_block_id);
        self.cfg.seal_block(rhs_block_id);

        let rhs_id = self.lower_expression(rhs_id).unwrap();

        if !self.cfg.is_filled_here() {
            self.cfg.emit_jump(merge_block_id, &[rhs_id]);
        }

        self.cfg.add_block(merge_block_id);
        self.cfg.seal_block(merge_block_id);

        Some(block_parameter_id)
    }

    fn lower_loop(&mut self, body_id: ExpressionId, type_id: TypeId) -> Option<ValueId> {
        let body_block_id = self.cfg.create_block();
        let exit_block_id = self.cfg.create_block();

        // jump into the body from the preceding block, then switch into the body itself
        // must happen in this order, since emitting the jump moves nothing, so emitting it
        // after switching would append it as body_block_id's own first instruction (a
        // self-jump) instead of the preceding block's terminator
        self.cfg.emit_jump(body_block_id, &[]);
        self.cfg.add_block(body_block_id);

        // do NOT seal body_block_id here, as the back-edge hasn't been emitted yet

        let produces_value = !self.ctx.type_interner.is_zero_sized(type_id)
            && type_id != self.ctx.type_interner.bottom_id;

        let result_param_id =
            produces_value.then(|| self.cfg.append_block_parameter(exit_block_id, type_id));

        self.loop_frames.push(LoopFrame {
            header_id: body_block_id,
            exit_id: exit_block_id,
            exit_is_branched_to: false, // only a `break` can make exit reachable
        });

        self.lower_expression(body_id);

        // back-edge (only if the body's end isn't already filled by e.g. a `return`)
        if !self.cfg.is_filled_here() {
            self.cfg.emit_jump(body_block_id, &[]);
        }

        self.cfg.seal_block(body_block_id);

        let frame = self
            .loop_frames
            .pop()
            .expect("just pushed this loop's own frame");

        if !frame.exit_is_branched_to {
            return None;
        }

        self.cfg.add_block(exit_block_id);
        self.cfg.seal_block(exit_block_id);

        result_param_id
    }

    fn lower_break(&mut self, value_id: Option<ExpressionId>) -> Option<ValueId> {
        let ssa_value_id = value_id.and_then(|value_id| self.lower_expression(value_id));
        if self.cfg.is_filled_here() {
            return None;
        }

        let frame = self
            .loop_frames
            .last_mut()
            .unwrap();
        let exit_id = frame.exit_id;
        frame.exit_is_branched_to = true;

        let block_argument_ids: &[ValueId] = if let Some(ssa_value_id) = &ssa_value_id {
            std::slice::from_ref(ssa_value_id)
        } else {
            &[]
        };

        self.cfg.emit_jump(exit_id, block_argument_ids);

        None
    }

    fn lower_continue(&mut self) -> Option<ValueId> {
        if self.cfg.is_filled_here() {
            return None;
        }

        let header_id = self
            .loop_frames
            .last_mut().unwrap()
            .header_id;

        self.cfg.emit_jump(header_id, &[]);

        None
    }

    fn get_callee_reference(&mut self, binding_id: DefinitionBindingId) -> FunctionReferenceId {
        if let Some(&function_reference_id) = self.callee_references.get(&binding_id) {
            return function_reference_id;
        }

        let binding_view = self.hir.get_definition_binding(binding_id);

        let (parameter_type_ids, return_type_id) =
            self.ctx.type_interner.as_func(binding_view.ty()).unwrap();
        let parameter_type_ids = parameter_type_ids
            .iter()
            .copied()
            .filter(|&ty| !self.ctx.type_interner.is_zero_sized(ty))
            .collect();
        let signature_id = self.cfg.add_signature(Signature {
            parameter_type_ids,
            return_type_id,
        });

        let function_reference_id =
            self.cfg
                .add_function_reference(binding_id, binding_view.name(), signature_id);

        self.callee_references
            .insert(binding_id, function_reference_id);

        function_reference_id
    }
}

