use std::collections::HashMap;

use crate::core::common::handle_collections::Handle;
use crate::core::common::handle_collections::handle_list::{
    HandleArrayList, HandleArrayListAllocator,
};
use crate::core::common::handle_collections::handle_map::SideHandleMap;
use crate::core::mir_construction::mir::{
    BlockHandle, Cfg, Instruction, InstructionHandle, ValueHandle, ValueOrigin,
};
use crate::core::semantic_analysis::ast_lowering::nodes::{
    BinaryOperator, LocalBindingHandle, UnaryOperator,
};
use crate::core::semantic_analysis::keys::{ConstantKey, FunctionKey};
use crate::core::semantic_analysis::type_checking::types::Ty;

pub(in crate::core) struct CfgBuilder<'db> {
    cfg: Cfg<'db>,
    position: Position,
    predecessor_edge_suballocator: HandleArrayListAllocator<InstructionHandle>,
    current_defs: HashMap<(LocalBindingHandle, BlockHandle), ValueHandle>,
    incomplete_placeholders: HashMap<BlockHandle, Vec<(LocalBindingHandle, ValueHandle)>>,
    block_states: SideHandleMap<BlockHandle, BlockState>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Position {
    Nowhere,
    #[expect(
        dead_code,
        reason = "positions for inserting blocks mid-CFG, which no lowering needs yet"
    )]
    At(InstructionHandle),
    #[expect(
        dead_code,
        reason = "positions for inserting blocks mid-CFG, which no lowering needs yet"
    )]
    Before(BlockHandle),
    After(BlockHandle),
}

// Clone: required by `SideHandleMap::add` for resize padding
#[derive(Clone, Default)]
struct BlockState {
    predecessors: HandleArrayList<InstructionHandle>,
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

    pub(in crate::core) fn first_result(
        &self,
        instruction_handle: InstructionHandle,
    ) -> Option<ValueHandle> {
        self.cfg.instruction(instruction_handle).first_result()
    }

    pub(in crate::core) fn current_block(&self) -> Option<BlockHandle> {
        match self.position {
            Position::Nowhere => None,
            Position::At(instruction_handle) => {
                self.cfg.instruction(instruction_handle).containing_block()
            }
            Position::Before(block_handle) | Position::After(block_handle) => Some(block_handle),
        }
    }

    pub(in crate::core) fn is_filled_here(&self) -> bool {
        self.current_block().is_some_and(|block_handle| {
            self.block_states[block_handle].status == BlockStatus::Filled
        })
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
        block_handle: BlockHandle,
        ty: Ty<'db>,
    ) -> ValueHandle {
        self.cfg.block_mut(block_handle).append_parameter(ty)
    }

    pub(in crate::core) fn create_block(&mut self) -> BlockHandle {
        let block_handle = self.cfg.allocate_block();
        assert_eq!(
            block_handle.index(),
            self.block_states.count(),
            "blocks must be tracked in the order they're allocated"
        );
        self.block_states.add(block_handle, BlockState::default());
        block_handle
    }

    pub(in crate::core) fn add_block(&mut self, block_handle: BlockHandle) {
        match self.position {
            Position::At(before) => {
                self.cfg.split_block(block_handle, before);
                return;
            }
            Position::Nowhere => self.cfg.append_block(block_handle),
            Position::Before(before) => self.cfg.add_block_before(block_handle, before),
            Position::After(after) => self.cfg.add_block_after(block_handle, after),
        }
        self.position = Position::After(block_handle);
    }

    pub(in crate::core) fn seal_block(&mut self, block_handle: BlockHandle) {
        assert!(
            !self.block_states[block_handle].sealed,
            "block {block_handle:?} is already sealed"
        );

        if let Some(placeholders) = self.incomplete_placeholders.remove(&block_handle) {
            for (variable_handle, block_parameter) in placeholders {
                let ty = self.cfg.value(block_parameter).ty();
                let block_args = self.collect_block_arguments(variable_handle, ty, block_handle);
                self.finalize_block_parameter(block_parameter, ty, block_handle, &block_args);
            }
        }

        self.block_states[block_handle].sealed = true;
    }

    pub(in crate::core) fn write_variable(
        &mut self,
        variable_handle: LocalBindingHandle,
        block_handle: BlockHandle,
        value_handle: ValueHandle,
    ) {
        self.current_defs
            .insert((variable_handle, block_handle), value_handle);
    }

    pub(in crate::core) fn read_variable(
        &mut self,
        variable: LocalBindingHandle,
        ty: Ty<'db>,
        block: BlockHandle,
    ) -> ValueHandle {
        if let Some(&value_handle) = self.current_defs.get(&(variable, block)) {
            return value_handle;
        }
        self.read_variable_recursive(variable, ty, block)
    }

    pub(in crate::core) fn emit_binary(
        &mut self,
        operator: BinaryOperator,
        lhs_handle: ValueHandle,
        rhs_handle: ValueHandle,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_binary(operator, [lhs_handle, rhs_handle]);
        let instruction_handle = self.add_instruction(instruction, &[ty]);
        self.cfg
            .instruction(instruction_handle)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_unary(
        &mut self,
        operator: UnaryOperator,
        operand_handle: ValueHandle,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_unary(operator, operand_handle);
        let instruction_handle = self.add_instruction(instruction, &[ty]);
        self.cfg
            .instruction(instruction_handle)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_integer_literal(
        &mut self,
        ty: Ty<'db>,
        value: u128,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_integer_literal(value);
        let instruction_handle = self.add_instruction(instruction, &[ty]);
        self.cfg
            .instruction(instruction_handle)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_boolean_literal(
        &mut self,
        value: bool,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_boolean_literal(value);
        let instruction_handle = self.add_instruction(instruction, &[ty]);
        self.cfg
            .instruction(instruction_handle)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_tuple(
        &mut self,
        element_handles: &[ValueHandle],
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_tuple(element_handles);
        let instruction_handle = self.add_instruction(instruction, &[ty]);
        self.cfg
            .instruction(instruction_handle)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_extract_field(
        &mut self,
        operand_handle: ValueHandle,
        index: u32,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_extract_field(operand_handle, index);
        let instruction_handle = self.add_instruction(instruction, &[ty]);
        self.cfg
            .instruction(instruction_handle)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_insert_field(
        &mut self,
        tuple_handle: ValueHandle,
        value_handle: ValueHandle,
        index: u32,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self
            .cfg
            .allocate_insert_field(tuple_handle, value_handle, index);
        let instruction_handle = self.add_instruction(instruction, &[ty]);
        self.cfg
            .instruction(instruction_handle)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_constant(
        &mut self,
        constant: ConstantKey<'db>,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_constant(constant);
        let instruction_handle = self.add_instruction(instruction, &[ty]);
        self.cfg
            .instruction(instruction_handle)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_function_address(
        &mut self,
        function: FunctionKey<'db>,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_function_address(function);
        let instruction_handle = self.add_instruction(instruction, &[ty]);
        self.cfg
            .instruction(instruction_handle)
            .first_result()
            .unwrap()
    }

    pub(in crate::core) fn emit_call(
        &mut self,
        function: FunctionKey<'db>,
        argument_handles: &[ValueHandle],
        result_types: &[Ty<'db>],
    ) -> InstructionHandle {
        let instruction = self.cfg.allocate_call(function, argument_handles);
        self.add_instruction(instruction, result_types)
    }

    pub(in crate::core) fn emit_call_indirect(
        &mut self,
        callee_and_argument_handles: &[ValueHandle],
        result_types: &[Ty<'db>],
    ) -> InstructionHandle {
        let instruction = self.cfg.allocate_call_indirect(callee_and_argument_handles);
        self.add_instruction(instruction, result_types)
    }

    pub(in crate::core) fn emit_jump(
        &mut self,
        destination_handle: BlockHandle,
        block_argument_handles: &[ValueHandle],
    ) -> InstructionHandle {
        let instruction = self
            .cfg
            .allocate_jump(destination_handle, block_argument_handles);
        let instruction_handle = self.add_instruction(instruction, &[]);
        self.track_predecessor(destination_handle, instruction_handle);
        instruction_handle
    }

    pub(in crate::core) fn emit_conditional_branch(
        &mut self,
        operand_handle: ValueHandle,
        true_block_handle: BlockHandle,
        true_block_argument_handles: &[ValueHandle],
        false_block_handle: BlockHandle,
        false_block_argument_handles: &[ValueHandle],
    ) -> InstructionHandle {
        let instruction = self.cfg.allocate_conditional_branch(
            operand_handle,
            true_block_handle,
            true_block_argument_handles,
            false_block_handle,
            false_block_argument_handles,
        );
        let instruction_handle = self.add_instruction(instruction, &[]);
        self.track_predecessor(true_block_handle, instruction_handle);
        self.track_predecessor(false_block_handle, instruction_handle);
        instruction_handle
    }

    pub(in crate::core) fn emit_return(
        &mut self,
        output_handles: &[ValueHandle],
    ) -> InstructionHandle {
        let instruction = self.cfg.allocate_return(output_handles);
        self.add_instruction(instruction, &[])
    }

    pub(in crate::core) fn emit_unreachable(&mut self) -> InstructionHandle {
        let instruction = self.cfg.allocate_unreachable();
        self.add_instruction(instruction, &[])
    }

    fn track_predecessor(
        &mut self,
        block_handle: BlockHandle,
        instruction_handle: InstructionHandle,
    ) {
        assert!(
            !self.block_states[block_handle].sealed,
            "cannot add a predecessor to an already-sealed block"
        );
        self.block_states[block_handle]
            .predecessors
            .add_last(&mut self.predecessor_edge_suballocator, instruction_handle);
    }

    fn read_variable_recursive(
        &mut self,
        variable_handle: LocalBindingHandle,
        ty: Ty<'db>,
        block_handle: BlockHandle,
    ) -> ValueHandle {
        let value_handle = if !self.block_states[block_handle].sealed {
            // case 1: incomplete CFG
            let block_parameter = self.cfg.block_mut(block_handle).append_parameter(ty);
            self.incomplete_placeholders
                .entry(block_handle)
                .or_default()
                .push((variable_handle, block_parameter));
            block_parameter
        } else if self.block_states[block_handle]
            .predecessors
            .count(&self.predecessor_edge_suballocator)
            == 1
        {
            // case 2: common case of one predecessor (no block parameter needed)
            let instruction_handle = self.block_states[block_handle]
                .predecessors
                .get(&self.predecessor_edge_suballocator, 0)
                .unwrap();
            let predecessor_handle = self
                .cfg
                .instruction(instruction_handle)
                .containing_block()
                .unwrap();
            self.read_variable(variable_handle, ty, predecessor_handle)
        } else {
            // case 3: general case with predecessors
            let block_parameter = self.cfg.block_mut(block_handle).append_parameter(ty);
            self.write_variable(variable_handle, block_handle, block_parameter);
            let block_args = self.collect_block_arguments(variable_handle, ty, block_handle);
            self.finalize_block_parameter(block_parameter, ty, block_handle, &block_args)
        };
        self.write_variable(variable_handle, block_handle, value_handle);
        value_handle
    }

    fn collect_block_arguments(
        &mut self,
        variable: LocalBindingHandle,
        ty: Ty<'db>,
        block_handle: BlockHandle,
    ) -> Vec<(InstructionHandle, ValueHandle)> {
        let mut block_args = Vec::new();
        let predecessor_edges = self.block_states[block_handle]
            .predecessors
            .to_slice(&self.predecessor_edge_suballocator)
            .to_vec();

        for instruction_handle in predecessor_edges {
            let predecessor = self
                .cfg
                .instruction(instruction_handle)
                .containing_block()
                .unwrap();
            let value = self.read_variable(variable, ty, predecessor);
            block_args.push((instruction_handle, value));
        }

        block_args
    }

    fn finalize_block_parameter(
        &mut self,
        block_parameter: ValueHandle,
        ty: Ty<'db>,
        block_handle: BlockHandle,
        block_args: &[(InstructionHandle, ValueHandle)],
    ) -> ValueHandle {
        let mut common_value: Option<ValueHandle> = None;

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
                for &(instruction_handle, value) in block_args {
                    self.cfg
                        .instruction_mut(instruction_handle)
                        .append_block_argument(block_handle, value);
                }
                return block_parameter;
            }
        }

        // trivial: converges to a common value, or to a fresh undefined if there wasn't one
        let replacement_value = common_value.unwrap_or_else(|| self.cfg.add_undefined(ty));

        // remove the block parameter
        let index = match self.cfg.value(block_parameter).origin() {
            ValueOrigin::Parameter(_, index) => index as usize,
            ValueOrigin::InstructionResult(..) | ValueOrigin::Undefined(_) => {
                unreachable!("`block_parameter` must originate from a block parameter slot")
            }
        };
        self.cfg.block_mut(block_handle).remove_parameter(index);

        // alias to replacement_value; stale references are fixed up later by `flush_aliases`, not now
        self.cfg.mark_as_alias(block_parameter, replacement_value);

        replacement_value
    }

    fn add_instruction(
        &mut self,
        instruction: Instruction<'db>,
        result_types: &[Ty<'db>],
    ) -> InstructionHandle {
        match self.position {
            Position::At(instruction_handle) => {
                self.cfg
                    .add_instruction_before(instruction, result_types, instruction_handle)
            }
            Position::After(block_handle) => {
                assert!(
                    self.block_states[block_handle].status != BlockStatus::Filled,
                    "cannot append to a block that already ends in a terminator"
                );
                let instruction_handle =
                    self.cfg
                        .append_instruction(block_handle, instruction, result_types);
                self.block_states[block_handle].status =
                    if self.cfg.instruction(instruction_handle).is_terminator() {
                        BlockStatus::Filled
                    } else {
                        BlockStatus::Partial
                    };
                instruction_handle
            }
            Position::Nowhere | Position::Before(_) => {
                panic!("invalid builder position for add_instruction")
            }
        }
    }
}
