use std::collections::HashMap;

use crate::core::common::handle_collections::Handle;
use crate::core::common::handle_collections::handle_list::{
    HandleArrayList, HandleArrayListAllocator,
};
use crate::core::common::handle_collections::handle_map::SideHandleMap;
use crate::core::mir_construction::mir::{
    BlockId, Cfg, Instruction, InstructionId, ValueId, ValueOrigin,
};
use crate::core::semantic_analysis::ast_lowering::nodes::{
    BinaryOperator, LocalBindingHandle, UnaryOperator,
};
use crate::core::semantic_analysis::keys::{ConstantKey, FunctionKey};
use crate::core::semantic_analysis::type_checking::types::Ty;

pub(in crate::core) struct CfgBuilder<'db> {
    cfg: Cfg<'db>,
    position: Position,
    predecessor_edge_suballocator: HandleArrayListAllocator<InstructionId>,
    current_defs: HashMap<(LocalBindingHandle, BlockId), ValueId>,
    incomplete_placeholders: HashMap<BlockId, Vec<(LocalBindingHandle, ValueId)>>,
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
    predecessors: HandleArrayList<InstructionId>,
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

impl<'db> CfgBuilder<'db> {
    pub(in crate::core) fn new() -> Self {
        Self {
            cfg: Cfg::new(),
            position: Position::Nowhere,
            predecessor_edge_suballocator: HandleArrayListAllocator::new(),
            current_defs: HashMap::new(),
            incomplete_placeholders: HashMap::new(),
            block_states: SideHandleMap::new(),
        }
    }

    pub(in crate::core) fn first_result(&self, instruction_id: InstructionId) -> Option<ValueId> {
        self.cfg.get_instruction(instruction_id).first_result()
    }

    pub(in crate::core) fn current_block(&self) -> Option<BlockId> {
        match self.position {
            Position::Nowhere => None,
            Position::At(instruction_id) => {
                self.cfg.get_instruction(instruction_id).containing_block()
            }
            Position::Before(block_id) | Position::After(block_id) => Some(block_id),
        }
    }

    pub(in crate::core) fn is_filled_here(&self) -> bool {
        self.current_block()
            .is_some_and(|block_id| self.block_states[block_id].status == BlockStatus::Filled)
    }

    pub(in crate::core) fn finish(&mut self) -> Cfg<'db> {
        let mut cfg = std::mem::replace(&mut self.cfg, Cfg::new());
        cfg.flush_aliases();

        self.position = Position::Nowhere;
        self.predecessor_edge_suballocator.reset();
        self.current_defs.clear();
        self.incomplete_placeholders.clear();
        self.block_states.clear();

        cfg
    }

    pub(in crate::core) fn append_block_parameter(
        &mut self,
        block_id: BlockId,
        ty: Ty<'db>,
    ) -> ValueId {
        self.cfg.get_block_mut(block_id).append_parameter(ty)
    }

    pub(in crate::core) fn create_block(&mut self) -> BlockId {
        let block_id = self.cfg.allocate_block();
        assert_eq!(
            block_id.index(),
            self.block_states.count(),
            "blocks must be tracked in the order they're allocated"
        );
        self.block_states.add(block_id, BlockState::default());
        block_id
    }

    pub(in crate::core) fn add_block(&mut self, block_id: BlockId) {
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

    pub(in crate::core) fn seal_block(&mut self, block_id: BlockId) {
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

    pub(in crate::core) fn write_variable(
        &mut self,
        variable_id: LocalBindingHandle,
        block_id: BlockId,
        value_id: ValueId,
    ) {
        self.current_defs.insert((variable_id, block_id), value_id);
    }

    pub(in crate::core) fn read_variable(
        &mut self,
        variable: LocalBindingHandle,
        ty: Ty<'db>,
        block: BlockId,
    ) -> ValueId {
        if let Some(&value_id) = self.current_defs.get(&(variable, block)) {
            return value_id;
        }
        self.read_variable_recursive(variable, ty, block)
    }

    pub(in crate::core) fn emit_binary(
        &mut self,
        operator: BinaryOperator,
        lhs_id: ValueId,
        rhs_id: ValueId,
        ty: Ty<'db>,
    ) -> ValueId {
        let instruction = self.cfg.allocate_binary(operator, [lhs_id, rhs_id]);
        let instruction_id = self.add_instruction(instruction, &[ty]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_unary(
        &mut self,
        operator: UnaryOperator,
        operand_id: ValueId,
        ty: Ty<'db>,
    ) -> ValueId {
        let instruction = self.cfg.allocate_unary(operator, operand_id);
        let instruction_id = self.add_instruction(instruction, &[ty]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_integer_literal(&mut self, ty: Ty<'db>, value: u128) -> ValueId {
        let instruction = self.cfg.allocate_integer_literal(value);
        let instruction_id = self.add_instruction(instruction, &[ty]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_boolean_literal(&mut self, value: bool, ty: Ty<'db>) -> ValueId {
        let instruction = self.cfg.allocate_boolean_literal(value);
        let instruction_id = self.add_instruction(instruction, &[ty]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_tuple(&mut self, element_ids: &[ValueId], ty: Ty<'db>) -> ValueId {
        let instruction = self.cfg.allocate_tuple(element_ids);
        let instruction_id = self.add_instruction(instruction, &[ty]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_extract_field(
        &mut self,
        operand_id: ValueId,
        index: u32,
        ty: Ty<'db>,
    ) -> ValueId {
        let instruction = self.cfg.allocate_extract_field(operand_id, index);
        let instruction_id = self.add_instruction(instruction, &[ty]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_insert_field(
        &mut self,
        tuple_id: ValueId,
        value_id: ValueId,
        index: u32,
        ty: Ty<'db>,
    ) -> ValueId {
        let instruction = self.cfg.allocate_insert_field(tuple_id, value_id, index);
        let instruction_id = self.add_instruction(instruction, &[ty]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_constant(
        &mut self,
        constant: ConstantKey<'db>,
        ty: Ty<'db>,
    ) -> ValueId {
        let instruction = self.cfg.allocate_constant(constant);
        let instruction_id = self.add_instruction(instruction, &[ty]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_function_address(
        &mut self,
        function: FunctionKey<'db>,
        ty: Ty<'db>,
    ) -> ValueId {
        let instruction = self.cfg.allocate_function_address(function);
        let instruction_id = self.add_instruction(instruction, &[ty]);
        self.cfg
            .get_instruction(instruction_id)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_call(
        &mut self,
        function: FunctionKey<'db>,
        argument_ids: &[ValueId],
        result_types: &[Ty<'db>],
    ) -> InstructionId {
        let instruction = self.cfg.allocate_call(function, argument_ids);
        self.add_instruction(instruction, result_types)
    }

    pub(in crate::core) fn emit_call_indirect(
        &mut self,
        callee_and_argument_ids: &[ValueId],
        result_types: &[Ty<'db>],
    ) -> InstructionId {
        let instruction = self.cfg.allocate_call_indirect(callee_and_argument_ids);
        self.add_instruction(instruction, result_types)
    }

    pub(in crate::core) fn emit_jump(
        &mut self,
        destination_id: BlockId,
        block_argument_ids: &[ValueId],
    ) -> InstructionId {
        let instruction = self.cfg.allocate_jump(destination_id, block_argument_ids);
        let instruction_id = self.add_instruction(instruction, &[]);
        self.track_predecessor(destination_id, instruction_id);
        instruction_id
    }

    pub(in crate::core) fn emit_conditional_branch(
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

    pub(in crate::core) fn emit_return(&mut self, output_ids: &[ValueId]) -> InstructionId {
        let instruction = self.cfg.allocate_return(output_ids);
        self.add_instruction(instruction, &[])
    }

    pub(in crate::core) fn emit_unreachable(&mut self) -> InstructionId {
        let instruction = self.cfg.allocate_unreachable();
        self.add_instruction(instruction, &[])
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

    fn read_variable_recursive(
        &mut self,
        variable_id: LocalBindingHandle,
        ty: Ty<'db>,
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
        variable: LocalBindingHandle,
        ty: Ty<'db>,
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
        ty: Ty<'db>,
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
        instruction: Instruction<'db>,
        result_types: &[Ty<'db>],
    ) -> InstructionId {
        match self.position {
            Position::At(instruction_id) => {
                self.cfg
                    .add_instruction_before(instruction, result_types, instruction_id)
            }
            Position::After(block_id) => {
                assert!(
                    self.block_states[block_id].status != BlockStatus::Filled,
                    "cannot append to a block that already ends in a terminator"
                );
                let instruction_id =
                    self.cfg
                        .append_instruction(block_id, instruction, result_types);
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
}
