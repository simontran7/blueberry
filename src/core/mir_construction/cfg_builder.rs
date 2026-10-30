use crate::core::common::handle_collections::Handle;
use crate::core::common::handle_collections::handle_map::SideHandleMap;
use crate::core::mir_construction::mir::{
    BlockHandle, Cfg, Instruction, InstructionHandle, ValueHandle,
};
use crate::core::mir_construction::ssa_variables::SsaVariables;
use crate::core::semantic_analysis::ast_lowering::nodes::{self, LocalBindingHandle};
use crate::core::semantic_analysis::keys::{ConstantKey, FunctionKey};
use crate::core::semantic_analysis::type_checking::types::Ty;

pub(crate) struct CfgBuilder<'db> {
    cfg: Cfg<'db>,
    /// the block new instructions are appended to
    current_block: Option<BlockHandle>,
    block_statuses: SideHandleMap<BlockHandle, BlockStatus>,
    variables: SsaVariables,
}

// Clone: required by `SideHandleMap::add` for resize padding
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum BlockStatus {
    #[default]
    Empty,
    Partial,
    Filled,
}

impl<'db> CfgBuilder<'db> {
    pub(crate) fn new() -> Self {
        Self {
            cfg: Cfg::new(),
            current_block: None,
            block_statuses: SideHandleMap::new(),
            variables: SsaVariables::new(),
        }
    }

    pub(crate) fn first_result(
        &self,
        instruction_handle: InstructionHandle,
    ) -> Option<ValueHandle> {
        self.cfg.instruction(instruction_handle).first_result()
    }

    pub(crate) fn current_block(&self) -> Option<BlockHandle> {
        self.current_block
    }

    pub(crate) fn is_filled_here(&self) -> bool {
        self.current_block
            .is_some_and(|block_handle| self.block_statuses[block_handle] == BlockStatus::Filled)
    }

    pub(crate) fn finish(self) -> Cfg<'db> {
        let mut cfg = self.cfg;
        cfg.flush_aliases();
        cfg
    }

    pub(crate) fn append_block_parameter(
        &mut self,
        block_handle: BlockHandle,
        ty: Ty<'db>,
    ) -> ValueHandle {
        self.cfg.block_mut(block_handle).append_parameter(ty)
    }

    pub(crate) fn create_block(&mut self) -> BlockHandle {
        let block_handle = self.cfg.allocate_block();
        assert_eq!(
            block_handle.index(),
            self.block_statuses.count(),
            "blocks must be tracked in the order they're allocated"
        );
        self.block_statuses
            .add(block_handle, BlockStatus::default());
        self.variables.declare_block(block_handle);
        block_handle
    }

    pub(crate) fn add_block(&mut self, block_handle: BlockHandle) {
        match self.current_block {
            None => self.cfg.append_block(block_handle),
            Some(after) => self.cfg.add_block_after(block_handle, after),
        }
        self.current_block = Some(block_handle);
    }

    pub(crate) fn seal_block(&mut self, block_handle: BlockHandle) {
        self.variables.seal_block(&mut self.cfg, block_handle);
    }

    pub(crate) fn write_variable(
        &mut self,
        variable_handle: LocalBindingHandle,
        block_handle: BlockHandle,
        value_handle: ValueHandle,
    ) {
        self.variables
            .write_variable(variable_handle, block_handle, value_handle);
    }

    pub(crate) fn read_variable(
        &mut self,
        variable: LocalBindingHandle,
        ty: Ty<'db>,
        block: BlockHandle,
    ) -> ValueHandle {
        self.variables
            .read_variable(&mut self.cfg, variable, ty, block)
    }

    pub(crate) fn emit_binary(
        &mut self,
        operator: nodes::BinaryOperator,
        lhs_handle: ValueHandle,
        rhs_handle: ValueHandle,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_binary(operator, [lhs_handle, rhs_handle]);
        self.add_value_instruction(instruction, ty)
    }

    pub(crate) fn emit_unary(
        &mut self,
        operator: nodes::UnaryOperator,
        operand_handle: ValueHandle,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_unary(operator, operand_handle);
        self.add_value_instruction(instruction, ty)
    }

    pub(crate) fn emit_integer_literal(&mut self, ty: Ty<'db>, value: u128) -> ValueHandle {
        let instruction = self.cfg.allocate_integer_literal(value);
        self.add_value_instruction(instruction, ty)
    }

    pub(crate) fn emit_boolean_literal(&mut self, value: bool, ty: Ty<'db>) -> ValueHandle {
        let instruction = self.cfg.allocate_boolean_literal(value);
        self.add_value_instruction(instruction, ty)
    }

    pub(crate) fn emit_tuple(
        &mut self,
        element_handles: &[ValueHandle],
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_tuple(element_handles);
        self.add_value_instruction(instruction, ty)
    }

    pub(crate) fn emit_extract_field(
        &mut self,
        operand_handle: ValueHandle,
        index: u32,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_extract_field(operand_handle, index);
        self.add_value_instruction(instruction, ty)
    }

    pub(crate) fn emit_insert_field(
        &mut self,
        tuple_handle: ValueHandle,
        value_handle: ValueHandle,
        index: u32,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self
            .cfg
            .allocate_insert_field(tuple_handle, value_handle, index);
        self.add_value_instruction(instruction, ty)
    }

    pub(crate) fn emit_constant(&mut self, constant: ConstantKey<'db>, ty: Ty<'db>) -> ValueHandle {
        let instruction = self.cfg.allocate_constant(constant);
        self.add_value_instruction(instruction, ty)
    }

    pub(crate) fn emit_function_address(
        &mut self,
        function: FunctionKey<'db>,
        ty: Ty<'db>,
    ) -> ValueHandle {
        let instruction = self.cfg.allocate_function_address(function);
        self.add_value_instruction(instruction, ty)
    }

    pub(crate) fn emit_call(
        &mut self,
        function: FunctionKey<'db>,
        argument_handles: &[ValueHandle],
        result_types: &[Ty<'db>],
    ) -> InstructionHandle {
        let instruction = self.cfg.allocate_call(function, argument_handles);
        self.add_instruction(instruction, result_types)
    }

    pub(crate) fn emit_call_indirect(
        &mut self,
        callee_and_argument_handles: &[ValueHandle],
        result_types: &[Ty<'db>],
    ) -> InstructionHandle {
        let instruction = self.cfg.allocate_call_indirect(callee_and_argument_handles);
        self.add_instruction(instruction, result_types)
    }

    pub(crate) fn emit_jump(
        &mut self,
        destination_handle: BlockHandle,
        block_argument_handles: &[ValueHandle],
    ) -> InstructionHandle {
        let instruction = self
            .cfg
            .allocate_jump(destination_handle, block_argument_handles);
        let instruction_handle = self.add_instruction(instruction, &[]);
        self.variables
            .declare_predecessor(destination_handle, instruction_handle);
        instruction_handle
    }

    pub(crate) fn emit_conditional_branch(
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
        self.variables
            .declare_predecessor(true_block_handle, instruction_handle);
        self.variables
            .declare_predecessor(false_block_handle, instruction_handle);
        instruction_handle
    }

    pub(crate) fn emit_return(&mut self, output_handles: &[ValueHandle]) -> InstructionHandle {
        let instruction = self.cfg.allocate_return(output_handles);
        self.add_instruction(instruction, &[])
    }

    pub(crate) fn emit_unreachable(&mut self) -> InstructionHandle {
        let instruction = self.cfg.allocate_unreachable();
        self.add_instruction(instruction, &[])
    }

    fn add_instruction(
        &mut self,
        instruction: Instruction<'db>,
        result_types: &[Ty<'db>],
    ) -> InstructionHandle {
        let Some(block_handle) = self.current_block else {
            panic!("cannot add an instruction before adding a block");
        };
        assert!(
            self.block_statuses[block_handle] != BlockStatus::Filled,
            "cannot append to a block that already ends in a terminator"
        );
        let instruction_handle =
            self.cfg
                .append_instruction(block_handle, instruction, result_types);
        self.block_statuses[block_handle] =
            if self.cfg.instruction(instruction_handle).is_terminator() {
                BlockStatus::Filled
            } else {
                BlockStatus::Partial
            };
        instruction_handle
    }

    /// Adds an instruction that produces exactly one value, of type `ty`, and returns that value.
    fn add_value_instruction(&mut self, instruction: Instruction<'db>, ty: Ty<'db>) -> ValueHandle {
        let instruction_handle = self.add_instruction(instruction, &[ty]);
        self.cfg
            .instruction(instruction_handle)
            .first_result()
            .unwrap()
    }
}
