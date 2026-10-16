use crate::core::mir_construction::cfg_builder::CfgBuilder;
use crate::core::mir_construction::mir::{BlockId, Cfg, MirBody, Signature, ValueId};
use crate::core::semantic_analysis::hir::nodes::{
    BinaryOperator, DefinitionBody, Expression, ExpressionHandle, LocalBindingHandle, Statement,
    StatementHandle, UnaryOperator,
};
use crate::core::semantic_analysis::name_resolution::resolver::Resolution;
use crate::core::semantic_analysis::type_checking::type_checker::TypeCheckSink;
use crate::core::semantic_analysis::type_checking::types::{Ty, TyKind};

pub(in crate::core) struct MirLowerer<'a, 'db> {
    db: &'db dyn crate::Db,
    body: &'a DefinitionBody<'db>,
    types: &'a TypeCheckSink<'db>,
    cfg: CfgBuilder<'db>,
    loop_frames: Vec<LoopFrame>,
}

struct LoopFrame {
    header_id: BlockId,
    exit_id: BlockId,
    exit_is_branched_to: bool,
}

impl<'a, 'db> MirLowerer<'a, 'db> {
    pub(in crate::core) fn new(
        db: &'db dyn crate::Db,
        body: &'a DefinitionBody<'db>,
        types: &'a TypeCheckSink<'db>,
    ) -> Self {
        Self {
            db,
            body,
            types,
            cfg: CfgBuilder::new(),
            loop_frames: Vec::new(),
        }
    }

    pub(in crate::core) fn lower_function(mut self, signature_type: Ty<'db>) -> MirBody<'db> {
        let TyKind::Function {
            parameters,
            r#return,
        } = signature_type.kind(self.db)
        else {
            unreachable!("a function's signature type is always a function type");
        };
        let signature = Signature {
            parameter_types: parameters
                .iter()
                .copied()
                .filter(|&ty| !self.is_zero_sized(ty))
                .collect(),
            return_type: *r#return,
        };
        let cfg = self.lower_body();
        MirBody { signature, cfg }
    }

    pub(in crate::core) fn lower_constant(mut self, declared_type: Ty<'db>) -> MirBody<'db> {
        let signature = Signature {
            parameter_types: Box::new([]),
            return_type: declared_type,
        };
        let cfg = self.lower_body();
        MirBody { signature, cfg }
    }

    fn lower_body(&mut self) -> Cfg<'db> {
        let body = self.body;
        let entry_block_id = self.cfg.create_block();
        self.cfg.add_block(entry_block_id);
        self.cfg.seal_block(entry_block_id);

        for &binding_handle in &body.parameter_handles {
            let ty = self.types.local_binding_type(binding_handle);
            if self.is_zero_sized(ty) {
                continue;
            }
            let value_id = self.cfg.append_block_parameter(entry_block_id, ty);
            self.cfg
                .write_variable(binding_handle, entry_block_id, value_id);
        }

        let tail_id = self.lower_expression(body.root_handle);

        if !self.cfg.is_filled_here() {
            match tail_id {
                Some(value_id) => self.cfg.emit_return(&[value_id]),
                None => self.cfg.emit_return(&[]),
            };
        }

        self.cfg.finish()
    }

    fn lower_statement(&mut self, statement_handle: StatementHandle) {
        match &self.body.store[statement_handle] {
            Statement::Expression {
                expression_handle, ..
            } => {
                self.lower_expression(*expression_handle);
            }
            Statement::Let {
                name_handle,
                initializer_handle,
                ..
            } => {
                let value_id = self.lower_expression(*initializer_handle);
                self.write_local(*name_handle, value_id);
            }
            Statement::Definition => {
                // nested definitions are lowered as separate MIR bodies
            }
        }
    }

    fn lower_expression(&mut self, expression_handle: ExpressionHandle) -> Option<ValueId> {
        let body = self.body;
        let ty = self.types.expression_type(expression_handle);
        match &body.store[expression_handle] {
            Expression::Error => {
                unreachable!("MIR is only built for bodies without errors")
            }
            Expression::Tuple { element_run } => self.lower_tuple(&body.store[*element_run], ty),
            Expression::Field {
                receiver_handle,
                name,
            } => self.lower_field(
                *receiver_handle,
                name.unwrap().as_tuple_index(self.db).unwrap(),
                ty,
            ),
            Expression::Boolean(value) => Some(self.cfg.emit_boolean_literal(*value, ty)),
            Expression::Integer(value) => Some(self.cfg.emit_integer_literal(ty, value.unwrap())),
            Expression::UnaryOperation {
                operator,
                operand_handle,
            } => self.lower_unary_operation(*operator, *operand_handle, ty),
            Expression::BinaryOperation {
                lhs_handle,
                operator,
                rhs_handle,
            } => {
                let operator = operator.unwrap();
                self.lower_binary_operation(operator, *lhs_handle, *rhs_handle, ty)
            }
            Expression::Path(_) => self.lower_path(expression_handle, ty),
            Expression::Assignment {
                target_handle,
                value_handle,
            } => self.lower_assign(*target_handle, *value_handle),
            Expression::Return { value_handle } => self.lower_return(*value_handle),
            Expression::Call {
                callee_handle,
                argument_run,
            } => self.lower_call(*callee_handle, &body.store[*argument_run], ty),
            Expression::Block {
                statement_run,
                tail_handle,
                ..
            } => self.lower_block(&body.store[*statement_run], *tail_handle),
            Expression::If {
                condition_handle,
                then_branch_handle,
                else_branch_handle,
            } => self.lower_if_expression(
                *condition_handle,
                *then_branch_handle,
                *else_branch_handle,
                ty,
            ),
            Expression::Loop { body_handle, .. } => self.lower_loop(*body_handle, ty),
            Expression::Break { value_handle } => self.lower_break(*value_handle),
            Expression::Continue => self.lower_continue(),
        }
    }

    fn lower_tuple(
        &mut self,
        element_handles: &[ExpressionHandle],
        ty: Ty<'db>,
    ) -> Option<ValueId> {
        let mut element_ids = Vec::with_capacity(element_handles.len());
        for &element_handle in element_handles {
            let element_id = self.lower_expression(element_handle);
            if self.cfg.is_filled_here() {
                return None;
            }
            element_ids.extend(element_id);
        }
        if self.is_zero_sized(ty) {
            return None;
        }
        Some(self.cfg.emit_tuple(&element_ids, ty))
    }

    fn lower_field(
        &mut self,
        receiver_handle: ExpressionHandle,
        index: u32,
        ty: Ty<'db>,
    ) -> Option<ValueId> {
        let receiver_id = self.lower_expression(receiver_handle);
        if self.cfg.is_filled_here() || self.is_zero_sized(ty) {
            return None;
        }
        Some(self.cfg.emit_extract_field(receiver_id.unwrap(), index, ty))
    }

    fn lower_unary_operation(
        &mut self,
        operator: UnaryOperator,
        operand_handle: ExpressionHandle,
        ty: Ty<'db>,
    ) -> Option<ValueId> {
        let operand_id = self.lower_expression(operand_handle)?;
        Some(self.cfg.emit_unary(operator, operand_id, ty))
    }

    fn lower_binary_operation(
        &mut self,
        operator: BinaryOperator,
        lhs_handle: ExpressionHandle,
        rhs_handle: ExpressionHandle,
        ty: Ty<'db>,
    ) -> Option<ValueId> {
        match operator {
            BinaryOperator::And | BinaryOperator::Or => {
                self.lower_short_circuiting_expression(operator, lhs_handle, rhs_handle, ty)
            }
            _ => {
                let lhs_id = self.lower_expression(lhs_handle);
                if self.cfg.is_filled_here() {
                    return None;
                }
                let rhs_id = self.lower_expression(rhs_handle);
                if self.cfg.is_filled_here() {
                    return None;
                }
                match (lhs_id, rhs_id) {
                    (Some(lhs_id), Some(rhs_id)) => {
                        Some(self.cfg.emit_binary(operator, lhs_id, rhs_id, ty))
                    }
                    _ => {
                        let value = matches!(
                            operator,
                            BinaryOperator::Eq | BinaryOperator::Le | BinaryOperator::Ge
                        );
                        Some(self.cfg.emit_boolean_literal(value, ty))
                    }
                }
            }
        }
    }

    fn lower_path(&mut self, expression_handle: ExpressionHandle, ty: Ty<'db>) -> Option<ValueId> {
        match self.types.path_resolution(expression_handle).unwrap() {
            Resolution::Local(binding_handle) => {
                if self.is_zero_sized(ty) {
                    return None;
                }
                let block = self.current_block();
                Some(self.cfg.read_variable(binding_handle, ty, block))
            }
            Resolution::Constant(constant) => {
                if self.is_zero_sized(ty) {
                    return None;
                }
                Some(self.cfg.emit_constant(constant, ty))
            }
            Resolution::Function(function) => Some(self.cfg.emit_function_address(function, ty)),
            Resolution::Module(_) => {
                unreachable!("semantic analysis already rejects modules used as values")
            }
            Resolution::Error => {
                unreachable!("semantic analysis already rejects paths through a failed import")
            }
        }
    }

    fn lower_assign(
        &mut self,
        target_handle: ExpressionHandle,
        value_handle: ExpressionHandle,
    ) -> Option<ValueId> {
        let mut indices = Vec::new();
        let mut base_handle = target_handle;
        while let Expression::Field {
            receiver_handle,
            name,
        } = &self.body.store[base_handle]
        {
            indices.push(name.unwrap().as_tuple_index(self.db).unwrap());
            base_handle = *receiver_handle;
        }
        indices.reverse();

        let Some(Resolution::Local(binding_handle)) = self.types.path_resolution(base_handle)
        else {
            unreachable!(
                "semantic analysis already rejects assignments whose target isn't a local binding"
            );
        };
        let value_id = self.lower_expression(value_handle);
        if self.cfg.is_filled_here() {
            return None;
        }
        let Some(value_id) = value_id else {
            return None;
        };

        let base_ty = self.types.expression_type(base_handle);
        let block = self.current_block();
        let base_id = self.cfg.read_variable(binding_handle, base_ty, block);
        let new_base_id = self.insert_field(base_id, base_ty, &indices, value_id);
        self.write_local(binding_handle, Some(new_base_id));
        None
    }

    fn insert_field(
        &mut self,
        tuple_id: ValueId,
        tuple_ty: Ty<'db>,
        indices: &[u32],
        value_id: ValueId,
    ) -> ValueId {
        let Some((&index, rest)) = indices.split_first() else {
            return value_id;
        };
        let TyKind::Tuple(elements) = tuple_ty.kind(self.db) else {
            unreachable!("semantic analysis already rejects fields of non-tuples");
        };
        let element_ty = elements[index as usize];
        let element_id = if rest.is_empty() {
            value_id
        } else {
            let element_id = self.cfg.emit_extract_field(tuple_id, index, element_ty);
            self.insert_field(element_id, element_ty, rest, value_id)
        };
        self.cfg
            .emit_insert_field(tuple_id, element_id, index, tuple_ty)
    }

    fn lower_return(&mut self, value_handle: Option<ExpressionHandle>) -> Option<ValueId> {
        let value_id = value_handle.and_then(|value_handle| self.lower_expression(value_handle));
        if !self.cfg.is_filled_here() {
            match value_id {
                Some(value_id) => self.cfg.emit_return(&[value_id]),
                None => self.cfg.emit_return(&[]),
            };
        }
        None
    }

    fn lower_call(
        &mut self,
        callee_handle: ExpressionHandle,
        argument_handles: &[ExpressionHandle],
        ty: Ty<'db>,
    ) -> Option<ValueId> {
        let function = match &self.body.store[callee_handle] {
            Expression::Path(_) => match self.types.path_resolution(callee_handle).unwrap() {
                Resolution::Function(function) => Some(function),
                Resolution::Local(_)
                | Resolution::Constant(_)
                | Resolution::Module(_)
                | Resolution::Error => None,
            },
            _ => None,
        };

        let mut operand_ids = Vec::new();
        if function.is_none() {
            let callee_id = self.lower_expression(callee_handle);
            if self.cfg.is_filled_here() {
                return None;
            }
            operand_ids.push(callee_id.unwrap());
        }
        for &argument_handle in argument_handles {
            let argument_id = self.lower_expression(argument_handle);
            if self.cfg.is_filled_here() {
                return None;
            }
            operand_ids.extend(argument_id);
        }

        let result_types = if self.is_zero_sized(ty) || self.is_bottom(ty) {
            Vec::new()
        } else {
            vec![ty]
        };
        let call_id = match function {
            Some(function) => self.cfg.emit_call(function, &operand_ids, &result_types),
            None => self.cfg.emit_call_indirect(&operand_ids, &result_types),
        };

        if self.is_bottom(ty) {
            self.cfg.emit_unreachable();
            return None;
        }

        self.cfg.first_result(call_id)
    }

    fn lower_block(
        &mut self,
        statement_handles: &[StatementHandle],
        tail_handle: Option<ExpressionHandle>,
    ) -> Option<ValueId> {
        for &statement_handle in statement_handles {
            self.lower_statement(statement_handle);
            if self.cfg.is_filled_here() {
                return None;
            }
        }
        tail_handle.and_then(|tail_handle| self.lower_expression(tail_handle))
    }

    // If with else:
    //             [condition block]
    //     cond. branch /        \
    //                 /          \
    //         [then block]   [else block]
    //             \               /
    //         jump \             / jump
    //             [merge block]
    //
    // If without else:
    //                 [condition block]
    //        cond. branch /        \
    //                    /          \
    //           [then block]        /  cond. branch
    //                \             /
    //            jump \           /
    //                 [merge block]
    fn lower_if_expression(
        &mut self,
        condition_handle: ExpressionHandle,
        then_branch_handle: ExpressionHandle,
        else_branch_handle: Option<ExpressionHandle>,
        ty: Ty<'db>,
    ) -> Option<ValueId> {
        let condition_id = self.lower_expression(condition_handle)?;
        if self.cfg.is_filled_here() {
            return None;
        }

        let produces_value = !self.is_zero_sized(ty) && !self.is_bottom(ty);

        let then_block_id = self.cfg.create_block();
        let else_block_id = else_branch_handle.map(|_| self.cfg.create_block());
        let merge_block_id = self.cfg.create_block();

        let block_parameter_id =
            produces_value.then(|| self.cfg.append_block_parameter(merge_block_id, ty));

        self.cfg.emit_conditional_branch(
            condition_id,
            then_block_id,
            &[],
            else_block_id.unwrap_or(merge_block_id),
            &[],
        );

        self.cfg.add_block(then_block_id);
        self.cfg.seal_block(then_block_id);

        let then_value_id = self.lower_expression(then_branch_handle);

        let mut merge_has_predecessor = false;
        if !self.cfg.is_filled_here() {
            let block_argument_ids = match then_value_id {
                Some(then_value_id) if produces_value => vec![then_value_id],
                _ => vec![],
            };
            self.cfg.emit_jump(merge_block_id, &block_argument_ids);
            merge_has_predecessor = true;
        }

        if let (Some(else_block_id), Some(else_branch_handle)) = (else_block_id, else_branch_handle)
        {
            self.cfg.add_block(else_block_id);
            self.cfg.seal_block(else_block_id);

            let else_value_id = self.lower_expression(else_branch_handle);

            if !self.cfg.is_filled_here() {
                match else_value_id {
                    Some(else_value_id) if produces_value => {
                        self.cfg.emit_jump(merge_block_id, &[else_value_id])
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

    //             [lhs block]
    // cond. branch /        \
    //             /          \
    //      [rhs block]       /  cond. branch
    //           \           /
    //       jump \         /
    //           [merge block]
    fn lower_short_circuiting_expression(
        &mut self,
        operator: BinaryOperator,
        lhs_handle: ExpressionHandle,
        rhs_handle: ExpressionHandle,
        ty: Ty<'db>,
    ) -> Option<ValueId> {
        let lhs_id = self.lower_expression(lhs_handle)?;
        if self.cfg.is_filled_here() {
            return None;
        }

        let rhs_block_id = self.cfg.create_block();
        let merge_block_id = self.cfg.create_block();

        let block_parameter_id = self.cfg.append_block_parameter(merge_block_id, ty);

        match operator {
            // `lhs and rhs` desugars to `if lhs { rhs } else { lhs }`
            BinaryOperator::And => self.cfg.emit_conditional_branch(
                lhs_id,
                rhs_block_id,
                &[],
                merge_block_id,
                &[lhs_id],
            ),
            // `lhs or rhs` desugars to `if lhs { lhs } else { rhs }`
            BinaryOperator::Or => self.cfg.emit_conditional_branch(
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

        let rhs_id = self.lower_expression(rhs_handle);

        if !self.cfg.is_filled_here() {
            let rhs_id = rhs_id.unwrap();
            self.cfg.emit_jump(merge_block_id, &[rhs_id]);
        }

        self.cfg.add_block(merge_block_id);
        self.cfg.seal_block(merge_block_id);

        Some(block_parameter_id)
    }

    // [pre-loop block]
    //                |
    //              jump
    //                v
    //         [body block] <---------+
    //           |          \         |
    //      (break; N times)  \    back-edge
    //           |              \     |
    //           v                +---+
    //      [exit block]
    fn lower_loop(&mut self, body_handle: ExpressionHandle, ty: Ty<'db>) -> Option<ValueId> {
        let body_block_id = self.cfg.create_block();
        let exit_block_id = self.cfg.create_block();

        // jump into the body from the preceding block, then switch into the body itself
        // must happen in this order, since emitting the jump moves nothing, so emitting it
        // after switching would append it as body_block_id's own first instruction (a
        // self-jump) instead of the preceding block's terminator
        self.cfg.emit_jump(body_block_id, &[]);
        self.cfg.add_block(body_block_id);

        // do NOT seal body_block_id here, as the back-edge hasn't been emitted yet

        let produces_value = !self.is_zero_sized(ty) && !self.is_bottom(ty);

        let result_parameter_id =
            produces_value.then(|| self.cfg.append_block_parameter(exit_block_id, ty));

        self.loop_frames.push(LoopFrame {
            header_id: body_block_id,
            exit_id: exit_block_id,
            exit_is_branched_to: false, // only a `break` can make exit reachable
        });

        self.lower_expression(body_handle);

        // back-edge (only if the body's end isn't already filled by e.g. a `return`)
        if !self.cfg.is_filled_here() {
            self.cfg.emit_jump(body_block_id, &[]);
        }

        self.cfg.seal_block(body_block_id);

        let frame = self.loop_frames.pop().unwrap();

        if !frame.exit_is_branched_to {
            return None;
        }

        self.cfg.add_block(exit_block_id);
        self.cfg.seal_block(exit_block_id);

        result_parameter_id
    }

    fn lower_break(&mut self, value_handle: Option<ExpressionHandle>) -> Option<ValueId> {
        let value_id = value_handle.and_then(|value_handle| self.lower_expression(value_handle));
        if self.cfg.is_filled_here() {
            return None;
        }

        let frame = self.loop_frames.last_mut().unwrap();
        let exit_id = frame.exit_id;
        frame.exit_is_branched_to = true;

        let block_argument_ids: &[ValueId] = match &value_id {
            Some(value_id) => std::slice::from_ref(value_id),
            None => &[],
        };

        self.cfg.emit_jump(exit_id, block_argument_ids);

        None
    }

    fn lower_continue(&mut self) -> Option<ValueId> {
        if self.cfg.is_filled_here() {
            return None;
        }

        let header_id = self.loop_frames.last().unwrap().header_id;

        self.cfg.emit_jump(header_id, &[]);

        None
    }

    fn write_local(&mut self, binding_handle: LocalBindingHandle, value_id: Option<ValueId>) {
        if let Some(value_id) = value_id
            && !self.cfg.is_filled_here()
        {
            let block = self.current_block();
            self.cfg.write_variable(binding_handle, block, value_id);
        }
    }

    fn current_block(&self) -> BlockId {
        self.cfg.current_block().unwrap()
    }

    fn is_zero_sized(&self, ty: Ty<'db>) -> bool {
        match ty.kind(self.db) {
            TyKind::Tuple(elements) => elements.iter().all(|&element| self.is_zero_sized(element)),
            _ => false,
        }
    }

    fn is_bottom(&self, ty: Ty<'db>) -> bool {
        matches!(ty.kind(self.db), TyKind::Bottom)
    }
}
