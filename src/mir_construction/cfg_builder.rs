use std::collections::HashMap;

use soup::handle_map::{Handle, SideHandleMap};

use crate::common::string_interner::Symbol;
use crate::common::types::TypeId;
use crate::front_end::semantic_analysis::hir::{DefinitionBindingId, LocalBindingId};
use crate::front_end::syntactic_analysis::cst::operators::{BinOp, UnOp};
use crate::middle_end::handle_list::{HandleList, HandleListSubAllocator};
use crate::middle_end::mir::{
    BlockId, Cfg, FunctionReferenceId, Instruction, InstructionId, Signature, SignatureId, ValueId,
    ValueOrigin,
};

pub(crate) struct CfgBuilder {
    cfg: Cfg,
    position: Position,
    predecessor_edge_suballocator: HandleListSubAllocator<InstructionId>,
    current_defs: HashMap<(LocalBindingId, BlockId), ValueId>,
    incomplete_placeholders: HashMap<BlockId, Vec<(LocalBindingId, ValueId)>>,
    block_states: SideHandleMap<BlockId, BlockState>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Position {
    Nowhere,
    At(InstructionId),
    Before(BlockId),
    After(BlockId),
}

// Clone: required by `SideHandleMap::add` for resize padding
#[derive(Clone, Default)]
struct BlockState {
    predecessors: HandleList<InstructionId>,
    sealed: bool,
    status: BlockStatus,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum BlockStatus {
    #[default]
    Empty,
    Partial,
    Filled,
}

impl CfgBuilder {
    pub(crate) fn new() -> Self {
        Self {
            cfg: Cfg::new(),
            position: Position::Nowhere,
            predecessor_edge_suballocator: HandleListSubAllocator::new(),
            current_defs: HashMap::new(),
            incomplete_placeholders: HashMap::new(),
            block_states: SideHandleMap::new(),
        }
    }

    pub(crate) fn finish(&mut self) -> Cfg {
        let mut cfg = std::mem::replace(&mut self.cfg, Cfg::new());
        cfg.flush_aliases();

        self.position = Position::Nowhere;
        self.predecessor_edge_suballocator.reset();
        self.current_defs.clear();
        self.incomplete_placeholders.clear();
        self.block_states.clear();

        cfg
    }

    pub(crate) fn append_block_parameter(&mut self, block_id: BlockId, type_id: TypeId) -> ValueId {
        self.cfg.get_block_mut(block_id).append_parameter(type_id)
    }

    pub(crate) fn first_result(&self, instruction_id: InstructionId) -> Option<ValueId> {
        self.cfg.get_instruction(instruction_id).first_result()
    }

    pub(crate) fn add_signature(&mut self, signature: Signature) -> SignatureId {
        self.cfg.add_signature(signature)
    }

    pub(crate) fn add_function_reference(
        &mut self,
        definition_binding_id: DefinitionBindingId,
        name: Symbol,
        signature_id: SignatureId,
    ) -> FunctionReferenceId {
        self.cfg
            .add_function_reference(definition_binding_id, name, signature_id)
    }

    pub(crate) fn create_block(&mut self) -> BlockId {
        let block_id = self.cfg.allocate_block();
        assert_eq!(
            block_id.index(),
            self.block_states.count(),
            "blocks must be tracked in the order they're allocated"
        );
        self.block_states.add(block_id, BlockState::default());
        block_id
    }

    pub(crate) fn add_block(&mut self, block_id: BlockId) {
        match self.position {
            Position::At(before) => {
                self.cfg.split_block(block_id, before);
                return;
            }
            Position::Nowhere => self.cfg.append_block(block_id),
            Position::Before(before) => self.cfg.add_block_before(block_id, before),
            Position::After(after) => self.cfg.add_block_after(block_id, after),
        }
        self.position = Position::After(block_id);
    }

    pub(crate) fn current_block(&self) -> Option<BlockId> {
        match self.position {
            Position::Nowhere => None,
            Position::At(instruction_id) => {
                self.cfg.get_instruction(instruction_id).containing_block()
            }
            Position::Before(block_id) | Position::After(block_id) => Some(block_id),
        }
    }

    pub(crate) fn is_filled_here(&self) -> bool {
        self.current_block()
            .is_some_and(|block_id| self.block_states[block_id].status == BlockStatus::Filled)
    }

    pub(crate) fn seal_block(&mut self, block_id: BlockId) {
        assert!(
            !self.block_states[block_id].sealed,
            "block {block_id:?} is already sealed"
        );

        if let Some(placeholders) = self.incomplete_placeholders.remove(&block_id) {
            for (variable_id, block_parameter) in placeholders {
                let ty = self.cfg.get_value(block_parameter).ty();
                let block_args = self.collect_block_arguments(variable_id, ty, block_id);
                self.finalize_block_parameter(block_parameter, ty, block_id, &block_args);
            }
        }

        self.block_states[block_id].sealed = true;
    }

    fn track_predecessor(&mut self, block_id: BlockId, instruction_id: InstructionId) {
        assert!(
            !self.block_states[block_id].sealed,
            "cannot add a predecessor to an already-sealed block"
        );
        self.block_states[block_id]
            .predecessors
            .add_last(&mut self.predecessor_edge_suballocator, instruction_id);
    }

    pub(crate) fn write_variable(
        &mut self,
        variable_id: LocalBindingId,
        block_id: BlockId,
        value_id: ValueId,
    ) {
        self.current_defs.insert((variable_id, block_id), value_id);
    }

    pub(crate) fn read_variable(
        &mut self,
        variable: LocalBindingId,
        ty: TypeId,
        block: BlockId,
    ) -> ValueId {
        if let Some(&value_id) = self.current_defs.get(&(variable, block)) {
            return value_id;
        }
        self.read_variable_recursive(variable, ty, block)
    }

    fn read_variable_recursive(
        &mut self,
        variable_id: LocalBindingId,
        ty: TypeId,
        block_id: BlockId,
    ) -> ValueId {
        let value_id = if !self.block_states[block_id].sealed {
            // case 1: incomplete CFG
            let block_parameter = self.cfg.get_block_mut(block_id).append_parameter(ty);
            self.incomplete_placeholders
                .entry(block_id)
                .or_default()
                .push((variable_id, block_parameter));
            block_parameter
        } else if self.block_states[block_id]
            .predecessors
            .count(&self.predecessor_edge_suballocator)
            == 1
        {
            // case 2: common case of one predecessor (no block parameter needed)
            let instruction_id = self.block_states[block_id]
                .predecessors
                .get(&self.predecessor_edge_suballocator, 0)
                .unwrap();
            let predecessor_id = self
                .cfg
                .get_instruction(instruction_id)
                .containing_block()
                .unwrap();
            self.read_variable(variable_id, ty, predecessor_id)
        } else {
            // case 3: general case with predecessors
            let block_parameter = self.cfg.get_block_mut(block_id).append_parameter(ty);
            self.write_variable(variable_id, block_id, block_parameter);
            let block_args = self.collect_block_arguments(variable_id, ty, block_id);
            self.finalize_block_parameter(block_parameter, ty, block_id, &block_args)
        };
        self.write_variable(variable_id, block_id, value_id);
        value_id
    }

    fn collect_block_arguments(
        &mut self,
        variable: LocalBindingId,
        ty: TypeId,
        block_id: BlockId,
    ) -> Vec<(InstructionId, ValueId)> {
        let mut block_args = Vec::new();
        let predecessor_edges = self.block_states[block_id]
            .predecessors
            .to_slice(&self.predecessor_edge_suballocator)
            .to_vec();

        for instruction_id in predecessor_edges {
            let predecessor = self
                .cfg
                .get_instruction(instruction_id)
                .containing_block()
                .unwrap();
            let value = self.read_variable(variable, ty, predecessor);
            block_args.push((instruction_id, value));
        }

        block_args
    }

    fn finalize_block_parameter(
        &mut self,
        block_parameter: ValueId,
        ty: TypeId,
        block_id: BlockId,
        block_args: &[(InstructionId, ValueId)],
    ) -> ValueId {
        let mut common_value: Option<ValueId> = None;

        for &(_, block_arg) in block_args {
            let resolved_block_arg = self.cfg.resolve_aliases(block_arg);

            // self-reference via the cycle-breaking placeholder
            if resolved_block_arg == block_parameter {
                continue;
            }

            if common_value.is_none() {
                common_value = Some(resolved_block_arg); // set to the first real value seen so far
                continue;
            }

            // not trivial: commit every argument and keep the block parameter
            if common_value != Some(resolved_block_arg) {
                for &(instruction_id, value) in block_args {
                    self.cfg
                        .get_instruction_mut(instruction_id)
                        .append_block_argument(block_id, value);
                }
                return block_parameter;
            }
        }

        // trivial: converges to a common value, or to a fresh undefined if there wasn't one
        let replacement_value = common_value.unwrap_or_else(|| self.cfg.add_undefined(ty));

        // remove the block parameter
        let index = match self.cfg.get_value(block_parameter).origin() {
            ValueOrigin::Parameter(_, index) => index as usize,
            ValueOrigin::InstructionResult(..) | ValueOrigin::Undefined(_) => {
                unreachable!("`block_parameter` must originate from a block parameter slot")
            }
        };
        self.cfg.get_block_mut(block_id).remove_parameter(index);

        // alias to replacement_value; stale references are fixed up later by `flush_aliases`, not now
        self.cfg.mark_as_alias(block_parameter, replacement_value);

        replacement_value
    }

    fn add_instruction(
        &mut self,
        instruction: Instruction,
        result_type_ids: &[TypeId],
    ) -> InstructionId {
        match self.position {
            Position::At(instruction_id) => {
                self.cfg
                    .add_instruction_before(instruction, result_type_ids, instruction_id)
            }
            Position::After(block_id) => {
                assert!(
                    self.block_states[block_id].status != BlockStatus::Filled,
                    "cannot append to a block that already ends in a terminator"
                );
                let instruction_id =
                    self.cfg
                        .append_instruction(block_id, instruction, result_type_ids);
                self.block_states[block_id].status =
                    if self.cfg.get_instruction(instruction_id).is_terminator() {
                        BlockStatus::Filled
                    } else {
                        BlockStatus::Partial
                    };
                instruction_id
            }
            Position::Nowhere | Position::Before(_) => {
                panic!("invalid builder position for add_instruction")
            }
        }
    }

    pub(crate) fn emit_binary(
        &mut self,
        operator: BinOp,
        lhs_id: ValueId,
        rhs_id: ValueId,
        type_id: TypeId,
    ) -> ValueId {
        let instruction = self.cfg.allocate_binary(operator, [lhs_id, rhs_id]);
        let instruction_id = self.add_instruction(instruction, &[type_id]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(crate) fn emit_unary(
        &mut self,
        operator: UnOp,
        operand_id: ValueId,
        type_id: TypeId,
    ) -> ValueId {
        let instruction = self.cfg.allocate_unary(operator, operand_id);
        let instruction_id = self.add_instruction(instruction, &[type_id]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(crate) fn emit_integer_literal(&mut self, ty: TypeId, value: u128) -> ValueId {
        let instruction = self.cfg.allocate_integer_literal(value);
        let instruction_id = self.add_instruction(instruction, &[ty]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(crate) fn emit_boolean_literal(&mut self, value: bool, type_id: TypeId) -> ValueId {
        let instruction = self.cfg.allocate_boolean_literal(value);
        let instruction_id = self.add_instruction(instruction, &[type_id]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(crate) fn emit_call(
        &mut self,
        callee_reference_id: FunctionReferenceId,
        argument_ids: &[ValueId],
        result_type_ids: &[TypeId],
    ) -> InstructionId {
        let instruction = self.cfg.allocate_call(callee_reference_id, argument_ids);
        self.add_instruction(instruction, result_type_ids)
    }

    pub(crate) fn emit_jump(
        &mut self,
        destination_id: BlockId,
        block_argument_ids: &[ValueId],
    ) -> InstructionId {
        let instruction = self.cfg.allocate_jump(destination_id, block_argument_ids);
        let instruction_id = self.add_instruction(instruction, &[]);
        self.track_predecessor(destination_id, instruction_id);
        instruction_id
    }

    pub(crate) fn emit_conditional_branch(
        &mut self,
        operand_id: ValueId,
        true_block_id: BlockId,
        true_block_argument_ids: &[ValueId],
        false_block_id: BlockId,
        false_block_argument_ids: &[ValueId],
    ) -> InstructionId {
        let instruction = self.cfg.allocate_conditional_branch(
            operand_id,
            true_block_id,
            true_block_argument_ids,
            false_block_id,
            false_block_argument_ids,
        );
        let instruction_id = self.add_instruction(instruction, &[]);
        self.track_predecessor(true_block_id, instruction_id);
        self.track_predecessor(false_block_id, instruction_id);
        instruction_id
    }

    pub(crate) fn emit_return(&mut self, output_ids: &[ValueId]) -> InstructionId {
        let instruction = self.cfg.allocate_return(output_ids);
        self.add_instruction(instruction, &[])
    }

    pub(crate) fn emit_unreachable(&mut self) -> InstructionId {
        let instruction = self.cfg.allocate_unreachable();
        self.add_instruction(instruction, &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_block() {
        let mut graph = CfgBuilder::new();
        let i32_ty = TypeId(0);
        let x_var = LocalBindingId::new(0);

        let block = graph.create_block();
        graph.add_block(block);

        let x1 = graph.emit_integer_literal(i32_ty, 1);
        graph.write_variable(x_var, block, x1);

        assert_eq!(graph.read_variable(x_var, i32_ty, block), x1);
    }

    #[test]
    fn sequence_of_blocks() {
        let mut graph = CfgBuilder::new();
        let i32_ty = TypeId(0);
        let x_var = LocalBindingId::new(0);

        let block0 = graph.create_block();
        graph.add_block(block0);
        let x1 = graph.emit_integer_literal(i32_ty, 1);
        graph.write_variable(x_var, block0, x1);

        let block1 = graph.create_block();
        graph.emit_jump(block1, &[]);
        graph.add_block(block1);
        graph.seal_block(block1);

        let block2 = graph.create_block();
        graph.emit_jump(block2, &[]);
        graph.add_block(block2);
        graph.seal_block(block2);

        // single-predecessor chains should just forward x1, no block parameters anywhere
        assert_eq!(graph.read_variable(x_var, i32_ty, block2), x1);
        assert!(graph.cfg.get_block(block1).parameters().is_empty());
        assert!(graph.cfg.get_block(block2).parameters().is_empty());
    }

    #[test]
    fn merge_of_equal_values_is_trivial() {
        let mut graph = CfgBuilder::new();
        let i32_ty = TypeId(0);
        let bool_ty = TypeId(1);
        let x_var = LocalBindingId::new(0);

        let entry = graph.create_block();
        graph.add_block(entry);
        let x1 = graph.emit_integer_literal(i32_ty, 1);
        graph.write_variable(x_var, entry, x1);
        let cond = graph.emit_boolean_literal(true, bool_ty);

        let then_block = graph.create_block();
        let else_block = graph.create_block();
        graph.emit_conditional_branch(cond, then_block, &[], else_block, &[]);

        graph.add_block(then_block);
        graph.seal_block(then_block);
        let merge = graph.create_block();
        graph.emit_jump(merge, &[]);

        graph.add_block(else_block);
        graph.seal_block(else_block);
        graph.emit_jump(merge, &[]);

        graph.add_block(merge);
        graph.seal_block(merge);

        // both branches forward the same x1 unchanged, so no real block parameter is needed
        let x_in_merge = graph.read_variable(x_var, i32_ty, merge);
        assert_eq!(graph.cfg.resolve_aliases(x_in_merge), x1);
        assert!(graph.cfg.get_block(merge).parameters().is_empty());
    }

    #[test]
    fn merge_of_different_values_creates_a_block_parameter() {
        let mut graph = CfgBuilder::new();
        let i32_ty = TypeId(0);
        let bool_ty = TypeId(1);
        let x_var = LocalBindingId::new(0);

        let entry = graph.create_block();
        graph.add_block(entry);
        let cond = graph.emit_boolean_literal(true, bool_ty);

        let then_block = graph.create_block();
        let else_block = graph.create_block();
        graph.emit_conditional_branch(cond, then_block, &[], else_block, &[]);

        graph.add_block(then_block);
        graph.seal_block(then_block);
        let x1 = graph.emit_integer_literal(i32_ty, 1);
        graph.write_variable(x_var, then_block, x1);
        let merge = graph.create_block();
        let then_jump = graph.emit_jump(merge, &[]);

        graph.add_block(else_block);
        graph.seal_block(else_block);
        let x2 = graph.emit_integer_literal(i32_ty, 2);
        graph.write_variable(x_var, else_block, x2);
        let else_jump = graph.emit_jump(merge, &[]);

        graph.add_block(merge);
        graph.seal_block(merge);

        let x_in_merge = graph.read_variable(x_var, i32_ty, merge);
        assert_eq!(graph.cfg.get_block(merge).parameters(), &[x_in_merge]);
        assert_eq!(graph.cfg.block_argument(x_in_merge, then_jump), x1);
        assert_eq!(graph.cfg.block_argument(x_in_merge, else_jump), x2);
    }

    #[test]
    fn program_with_loop() {
        let mut graph = CfgBuilder::new();
        let i32_ty = TypeId(0);
        let bool_ty = TypeId(1);
        let x_var = LocalBindingId::new(0);

        // entry: x = 1; jump header
        let entry = graph.create_block();
        graph.add_block(entry);
        let x1 = graph.emit_integer_literal(i32_ty, 1);
        graph.write_variable(x_var, entry, x1);

        let header = graph.create_block();
        let entry_jump = graph.emit_jump(header, &[]);

        // header: unsealed, its back-edge from the loop body doesn't exist yet
        graph.add_block(header);

        // reading x here, before sealing, forces the placeholder block-parameter path
        let x_in_header = graph.read_variable(x_var, i32_ty, header);

        let cond = graph.emit_boolean_literal(true, bool_ty);
        let body = graph.create_block();
        let exit = graph.create_block();
        graph.emit_conditional_branch(cond, body, &[], exit, &[]);

        // body: x = 2; jump header (the back-edge)
        graph.add_block(body);
        graph.seal_block(body);
        let x2 = graph.emit_integer_literal(i32_ty, 2);
        graph.write_variable(x_var, body, x2);
        let back_edge = graph.emit_jump(header, &[]);

        // header's predecessor set is finally complete: seal it
        graph.seal_block(header);

        // exit: read x after the loop
        graph.add_block(exit);
        graph.seal_block(exit);
        let x_in_exit = graph.read_variable(x_var, i32_ty, exit);

        // x1 != x2, so header's placeholder should have resolved to a real, kept parameter
        assert_eq!(graph.cfg.get_block(header).parameters(), &[x_in_header]);
        assert_eq!(graph.cfg.block_argument(x_in_header, entry_jump), x1);
        assert_eq!(graph.cfg.block_argument(x_in_header, back_edge), x2);
        // exit has a single predecessor (the branch out of header), so it should just
        // forward header's parameter directly, no new value created
        assert_eq!(x_in_exit, x_in_header);
    }
}
