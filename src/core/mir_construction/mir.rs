use std::slice;

use crate::core::common::handle_collections::handle_impl;
use crate::core::common::handle_collections::handle_list::{
    HandleArrayList, HandleArrayListAllocator,
};
use crate::core::common::handle_collections::handle_map::{HandleMap, SideHandleMap};
use crate::core::semantic_analysis::ast_lowering::nodes;
use crate::core::semantic_analysis::keys::{ConstantKey, FunctionKey};
use crate::core::semantic_analysis::type_checking::types::Ty;

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct MirBody<'db> {
    pub(in crate::core::mir_construction) signature: Signature<'db>,
    pub(in crate::core::mir_construction) cfg: Cfg<'db>,
}

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct Cfg<'db> {
    dfg: DataFlowGraph<'db>,
    layout: Layout,
}

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct Value<'db> {
    ty: Ty<'db>,
    alias_handle: Option<ValueHandle>,
    origin: ValueOrigin<'db>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum ValueOrigin<'db> {
    InstructionResult(InstructionHandle, u16),
    Parameter(BlockHandle, u16),
    Undefined(Ty<'db>),
}

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct Block {
    parameter_handles: HandleArrayList<ValueHandle>,
}

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum InstructionKind<'db, T> {
    // Arithmetic
    Binary {
        operator: nodes::BinaryOperator,
        operand_handles: [ValueHandle; 2],
    },
    Unary {
        operator: nodes::UnaryOperator,
        operand_handle: ValueHandle,
    },

    // Literals
    IntegerLiteral {
        value: u128,
    },
    BooleanLiteral {
        value: bool,
    },

    // Aggregates
    Tuple {
        element_handles: T,
    },
    ExtractField {
        operand_handle: ValueHandle,
        index: u32,
    },
    InsertField {
        operand_handles: [ValueHandle; 2],
        index: u32,
    },

    // Definitions
    Constant {
        constant: ConstantKey<'db>,
    },
    FunctionAddress {
        function: FunctionKey<'db>,
    },

    // Calls
    Call {
        function: FunctionKey<'db>,
        argument_handles: T,
    },
    CallIndirect {
        callee_and_argument_handles: T,
    },

    // Terminators
    Jump {
        destination_handle: BlockHandle,
        block_argument_handles: T,
    },
    ConditionalBranch {
        operand_handle: ValueHandle,
        true_block_handle: BlockHandle,
        true_block_argument_handles: T,
        false_block_handle: BlockHandle,
        false_block_argument_handles: T,
    },
    Return {
        output_handles: T,
    },
    Unreachable,
}

pub(crate) type Instruction<'db> = InstructionKind<'db, HandleArrayList<ValueHandle>>;

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct Signature<'db> {
    pub(in crate::core::mir_construction) parameter_types: Box<[Ty<'db>]>,
    pub(in crate::core::mir_construction) return_type: Ty<'db>,
}

// Opaque, 4-byte handles into the tables above.
handle_impl!(pub(crate) BlockHandle);

handle_impl!(pub(crate) InstructionHandle);

handle_impl!(pub(crate) ValueHandle);

pub(crate) struct BlockView<'a, 'db> {
    block_handle: BlockHandle,
    cfg: &'a Cfg<'db>,
}

pub(crate) struct BlockViewMut<'a, 'db> {
    block_handle: BlockHandle,
    cfg: &'a mut Cfg<'db>,
}

pub(crate) struct InstructionView<'a, 'db> {
    instruction_handle: InstructionHandle,
    cfg: &'a Cfg<'db>,
}

pub(crate) type InstructionRef<'a, 'db> = InstructionKind<'db, &'a [ValueHandle]>;

pub(crate) struct InstructionViewMut<'a, 'db> {
    instruction_handle: InstructionHandle,
    cfg: &'a mut Cfg<'db>,
}

pub(crate) struct ValueView<'a, 'db> {
    value_handle: ValueHandle,
    cfg: &'a Cfg<'db>,
}

pub(crate) enum UsedValuesIter<'a> {
    Slice(slice::Iter<'a, ValueHandle>),
    Branch {
        operand_handle: Option<ValueHandle>,
        then_argument_handles: slice::Iter<'a, ValueHandle>,
        else_argument_handles: slice::Iter<'a, ValueHandle>,
    },
}

pub(crate) struct BlockIter<'a> {
    layout: &'a Layout,
    next_handle: Option<BlockHandle>,
}

pub(crate) struct InstructionIter<'a> {
    layout: &'a Layout,
    next_handle: Option<InstructionHandle>,
}

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
struct DataFlowGraph<'db> {
    values: HandleMap<ValueHandle, Value<'db>>,
    instructions: HandleMap<InstructionHandle, Instruction<'db>>,
    instruction_results: SideHandleMap<InstructionHandle, HandleArrayList<ValueHandle>>,
    blocks: HandleMap<BlockHandle, Block>,
    suballocator: HandleArrayListAllocator<ValueHandle>,
}

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
struct Layout {
    entry_handle: Option<BlockHandle>,
    exit_handle: Option<BlockHandle>,
    blocks: SideHandleMap<BlockHandle, BlockNode>,
    instructions: SideHandleMap<InstructionHandle, InstructionNode>,
}

// Clone: required by `SideHandleMap::add` for resize padding
#[derive(Clone, Default, Debug, PartialEq, Eq, salsa::SalsaValue)]
struct BlockNode {
    previous_handle: Option<BlockHandle>,
    first_handle: Option<InstructionHandle>,
    last_handle: Option<InstructionHandle>,
    next_handle: Option<BlockHandle>,
}

// Clone: required by `SideHandleMap::add` for resize padding
#[derive(Clone, Default, Debug, PartialEq, Eq, salsa::SalsaValue)]
struct InstructionNode {
    previous_handle: Option<InstructionHandle>,
    block_handle: Option<BlockHandle>,
    next_handle: Option<InstructionHandle>,
}

impl<'db> Cfg<'db> {
    pub(crate) fn new() -> Self {
        Self {
            dfg: DataFlowGraph::new(),
            layout: Layout::new(),
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "MIR API for codegen and MIR passes, which aren't written yet"
        )
    )]
    pub(crate) fn entry(&self) -> Option<BlockView<'_, 'db>> {
        self.layout.entry_handle.map(|block_handle| BlockView {
            block_handle,
            cfg: self,
        })
    }

    #[expect(
        dead_code,
        reason = "MIR API for codegen and MIR passes, which aren't written yet"
    )]
    pub(crate) fn exit(&self) -> Option<BlockView<'_, 'db>> {
        self.layout.exit_handle.map(|block_handle| BlockView {
            block_handle,
            cfg: self,
        })
    }

    pub(crate) fn block_handles(&self) -> BlockIter<'_> {
        BlockIter {
            layout: &self.layout,
            next_handle: self.layout.entry_handle,
        }
    }

    pub(crate) fn ssa_handles(&self) -> impl Iterator<Item = ValueHandle> + '_ {
        self.dfg.values.keys()
    }

    pub(crate) fn block(&self, block_handle: BlockHandle) -> BlockView<'_, 'db> {
        BlockView {
            block_handle,
            cfg: self,
        }
    }

    pub(crate) fn contains(&self, block_handle: BlockHandle) -> bool {
        Some(block_handle) == self.layout.entry_handle
            || self
                .layout
                .blocks
                .get(block_handle)
                .is_some_and(|node| node.previous_handle.is_some())
    }

    pub(crate) fn instruction(
        &self,
        instruction_handle: InstructionHandle,
    ) -> InstructionView<'_, 'db> {
        InstructionView {
            instruction_handle,
            cfg: self,
        }
    }

    pub(crate) fn allocate_binary(
        &self,
        operator: nodes::BinaryOperator,
        operand_handles: [ValueHandle; 2],
    ) -> Instruction<'db> {
        Instruction::Binary {
            operator,
            operand_handles,
        }
    }

    pub(crate) fn allocate_unary(
        &self,
        operator: nodes::UnaryOperator,
        operand_handle: ValueHandle,
    ) -> Instruction<'db> {
        Instruction::Unary {
            operator,
            operand_handle,
        }
    }

    pub(crate) fn allocate_integer_literal(&self, value: u128) -> Instruction<'db> {
        Instruction::IntegerLiteral { value }
    }

    pub(crate) fn allocate_boolean_literal(&self, value: bool) -> Instruction<'db> {
        Instruction::BooleanLiteral { value }
    }

    pub(crate) fn allocate_extract_field(
        &self,
        operand_handle: ValueHandle,
        index: u32,
    ) -> Instruction<'db> {
        Instruction::ExtractField {
            operand_handle,
            index,
        }
    }

    pub(crate) fn allocate_insert_field(
        &self,
        tuple_handle: ValueHandle,
        value_handle: ValueHandle,
        index: u32,
    ) -> Instruction<'db> {
        Instruction::InsertField {
            operand_handles: [tuple_handle, value_handle],
            index,
        }
    }

    pub(crate) fn allocate_constant(&self, constant: ConstantKey<'db>) -> Instruction<'db> {
        Instruction::Constant { constant }
    }

    pub(crate) fn allocate_function_address(&self, function: FunctionKey<'db>) -> Instruction<'db> {
        Instruction::FunctionAddress { function }
    }

    pub(crate) fn allocate_unreachable(&self) -> Instruction<'db> {
        Instruction::Unreachable
    }

    pub(crate) fn value(&self, value_handle: ValueHandle) -> ValueView<'_, 'db> {
        ValueView {
            value_handle,
            cfg: self,
        }
    }

    #[expect(
        dead_code,
        reason = "MIR API for codegen and MIR passes, which aren't written yet"
    )]
    pub(crate) fn block_argument(
        &self,
        block_parameter: ValueHandle,
        predecessor_edge: InstructionHandle,
    ) -> ValueHandle {
        let (block_handle, index) = match self.value(block_parameter).origin() {
            ValueOrigin::Parameter(block_handle, index) => (block_handle, index as usize),
            ValueOrigin::InstructionResult(..) | ValueOrigin::Undefined(_) => {
                panic!("value is not a block parameter")
            }
        };
        self.instruction(predecessor_edge)
            .block_argument(block_handle, index)
    }

    pub(crate) fn resolve_aliases(&self, value_handle: ValueHandle) -> ValueHandle {
        let mut current_value = value_handle;
        for _ in 0..=self.dfg.values.count() {
            match self.dfg.values[current_value].alias_handle {
                Some(original) => current_value = original,
                None => return current_value,
            }
        }
        panic!("value alias loop detected");
    }

    pub(crate) fn allocate_block(&mut self) -> BlockHandle {
        self.dfg.blocks.add(Block {
            parameter_handles: HandleArrayList::<ValueHandle>::new(),
        })
    }

    pub(crate) fn block_mut(&mut self, block_handle: BlockHandle) -> BlockViewMut<'_, 'db> {
        BlockViewMut {
            block_handle,
            cfg: self,
        }
    }

    pub(crate) fn append_block(&mut self, block_handle: BlockHandle) {
        assert!(
            !self.contains(block_handle),
            "cannot append a block that is already in the cfg"
        );
        let node = BlockNode {
            previous_handle: self.layout.exit_handle,
            next_handle: None,
            first_handle: None,
            last_handle: None,
        };
        self.layout.blocks.add(block_handle, node);
        if let Some(exit) = self.layout.exit_handle {
            self.layout.blocks[exit].next_handle = Some(block_handle);
        } else {
            self.layout.entry_handle = Some(block_handle);
        }
        self.layout.exit_handle = Some(block_handle);
    }

    pub(crate) fn add_block_before(
        &mut self,
        block_handle: BlockHandle,
        before_handle: BlockHandle,
    ) {
        assert!(
            !self.contains(block_handle),
            "cannot insert a block that is already in the cfg"
        );
        assert!(
            self.contains(before_handle),
            "block insertion point is not in the cfg"
        );
        let old_prev_handle = self.layout.blocks[before_handle].previous_handle;
        let node = BlockNode {
            previous_handle: old_prev_handle,
            next_handle: Some(before_handle),
            first_handle: None,
            last_handle: None,
        };
        self.layout.blocks.add(block_handle, node);
        self.layout.blocks[before_handle].previous_handle = Some(block_handle);
        match old_prev_handle {
            Some(old_prev_handle) => {
                self.layout.blocks[old_prev_handle].next_handle = Some(block_handle);
            }
            None => self.layout.entry_handle = Some(block_handle),
        }
    }

    pub(crate) fn add_block_after(&mut self, block_handle: BlockHandle, after_handle: BlockHandle) {
        assert!(
            !self.contains(block_handle),
            "cannot insert a block that is already in the cfg"
        );
        assert!(
            self.contains(after_handle),
            "block insertion point is not in the cfg"
        );
        let before_handle = self.layout.blocks[after_handle].next_handle;
        let node = BlockNode {
            previous_handle: Some(after_handle),
            next_handle: before_handle,
            first_handle: None,
            last_handle: None,
        };
        self.layout.blocks.add(block_handle, node);
        self.layout.blocks[after_handle].next_handle = Some(block_handle);
        match before_handle {
            None => self.layout.exit_handle = Some(block_handle),
            Some(before_handle) => {
                self.layout.blocks[before_handle].previous_handle = Some(block_handle);
            }
        }
    }

    #[expect(
        dead_code,
        reason = "MIR API for codegen and MIR passes, which aren't written yet"
    )]
    pub(crate) fn remove_block(&mut self, block_handle: BlockHandle) {
        assert!(
            self.contains(block_handle),
            "block pointed by `block_handle` is not in the cfg"
        );
        assert!(
            self.layout.blocks[block_handle].first_handle.is_none(),
            "cannot remove a block that still has instructions"
        );
        let prev = self.layout.blocks[block_handle].previous_handle;
        let next = self.layout.blocks[block_handle].next_handle;
        self.layout.blocks[block_handle].previous_handle = None;
        self.layout.blocks[block_handle].next_handle = None;
        match prev {
            None => self.layout.entry_handle = next,
            Some(p) => self.layout.blocks[p].next_handle = next,
        }
        match next {
            None => self.layout.exit_handle = prev,
            Some(n) => self.layout.blocks[n].previous_handle = prev,
        }
    }

    #[expect(
        dead_code,
        reason = "MIR API for codegen and MIR passes, which aren't written yet"
    )]
    pub(crate) fn clear_block(&mut self, block_handle: BlockHandle) {
        assert!(
            self.contains(block_handle),
            "block pointed by `block_handle` is not in the cfg"
        );
        while let Some(instruction_handle) = self.layout.blocks[block_handle].first_handle {
            self.remove_instruction(instruction_handle);
        }
    }

    pub(crate) fn instruction_mut(
        &mut self,
        instruction_handle: InstructionHandle,
    ) -> InstructionViewMut<'_, 'db> {
        InstructionViewMut {
            instruction_handle,
            cfg: self,
        }
    }

    pub(crate) fn add_instruction_before(
        &mut self,
        instruction: Instruction<'db>,
        result_types: &[Ty<'db>],
        before_handle: InstructionHandle,
    ) -> InstructionHandle {
        let instruction_handle = self.add_instruction(instruction, result_types);

        let block_handle = self.layout.instructions[before_handle]
            .block_handle
            .unwrap();

        let old_prev = self.layout.instructions[before_handle].previous_handle;

        self.layout.instructions.add(
            instruction_handle,
            InstructionNode {
                block_handle: Some(block_handle),
                previous_handle: old_prev,
                next_handle: Some(before_handle),
            },
        );

        self.layout.instructions[before_handle].previous_handle = Some(instruction_handle);

        match old_prev {
            None => self.layout.blocks[block_handle].first_handle = Some(instruction_handle),
            Some(before_prev_handle) => {
                self.layout.instructions[before_prev_handle].next_handle = Some(instruction_handle);
            }
        }

        instruction_handle
    }

    pub(crate) fn append_instruction(
        &mut self,
        block_handle: BlockHandle,
        instruction: Instruction<'db>,
        result_types: &[Ty<'db>],
    ) -> InstructionHandle {
        let instruction_handle = self.add_instruction(instruction, result_types);
        self.link_instruction_to_block(block_handle, instruction_handle);
        instruction_handle
    }

    #[expect(
        dead_code,
        reason = "MIR API for codegen and MIR passes, which aren't written yet"
    )]
    pub(crate) fn set_terminator(
        &mut self,
        block_handle: BlockHandle,
        terminator: Instruction<'db>,
    ) {
        let instruction_handle = self.add_instruction(terminator, &[]);
        self.link_instruction_to_block(block_handle, instruction_handle);
    }

    // NOTE: Its DFG data (and any values it defines) remains valid but is no longer
    // reachable via layout traversal.
    #[expect(
        dead_code,
        reason = "MIR API for codegen and MIR passes, which aren't written yet"
    )]
    pub(crate) fn remove_instruction(&mut self, instruction_handle: InstructionHandle) {
        let block_handle = self.layout.instructions[instruction_handle]
            .block_handle
            .unwrap();

        let old_prev = self.layout.instructions[instruction_handle].previous_handle;
        let old_next = self.layout.instructions[instruction_handle].next_handle;

        self.layout.instructions[instruction_handle].block_handle = None;
        self.layout.instructions[instruction_handle].previous_handle = None;
        self.layout.instructions[instruction_handle].next_handle = None;

        match old_prev {
            None => self.layout.blocks[block_handle].first_handle = old_next,
            Some(old_prev_handle) => {
                self.layout.instructions[old_prev_handle].next_handle = old_next;
            }
        }
        match old_next {
            None => self.layout.blocks[block_handle].last_handle = old_prev,
            Some(old_next_handle) => {
                self.layout.instructions[old_next_handle].previous_handle = old_prev;
            }
        }
    }

    pub(crate) fn split_block(
        &mut self,
        new_block_handle: BlockHandle,
        split_point_handle: InstructionHandle,
    ) {
        assert!(
            !self.contains(new_block_handle),
            "cannot split into a block that is already in the cfg"
        );

        let old_block_handle = self.layout.instructions[split_point_handle]
            .block_handle
            .unwrap();

        self.layout.blocks.add(
            new_block_handle,
            BlockNode {
                previous_handle: Some(old_block_handle),
                next_handle: self.layout.blocks[old_block_handle].next_handle,
                first_handle: Some(split_point_handle),
                last_handle: self.layout.blocks[old_block_handle].last_handle,
            },
        );

        let old_next = self.layout.blocks[old_block_handle].next_handle;
        self.layout.blocks[old_block_handle].next_handle = Some(new_block_handle);
        match old_next {
            None => self.layout.exit_handle = Some(new_block_handle),
            Some(old_next_handle) => {
                self.layout.blocks[old_next_handle].previous_handle = Some(new_block_handle);
            }
        }

        let before_split_handle = self.layout.instructions[split_point_handle].previous_handle;
        self.layout.instructions[split_point_handle].previous_handle = None;
        self.layout.blocks[old_block_handle].last_handle = before_split_handle;

        match before_split_handle {
            None => self.layout.blocks[old_block_handle].first_handle = None,
            Some(instruction_handle) => {
                self.layout.instructions[instruction_handle].next_handle = None;
            }
        }

        let mut current = Some(split_point_handle);
        while let Some(instruction_handle) = current {
            self.layout.instructions[instruction_handle].block_handle = Some(new_block_handle);
            current = self.layout.instructions[instruction_handle].next_handle;
        }
    }

    pub(crate) fn allocate_jump(
        &mut self,
        destination_handle: BlockHandle,
        block_argument_handles: &[ValueHandle],
    ) -> Instruction<'db> {
        Instruction::Jump {
            destination_handle,
            block_argument_handles: HandleArrayList::<ValueHandle>::from(
                &mut self.dfg.suballocator,
                block_argument_handles,
            ),
        }
    }

    pub(crate) fn allocate_return(&mut self, output_handles: &[ValueHandle]) -> Instruction<'db> {
        Instruction::Return {
            output_handles: HandleArrayList::<ValueHandle>::from(
                &mut self.dfg.suballocator,
                output_handles,
            ),
        }
    }

    pub(crate) fn allocate_tuple(&mut self, element_handles: &[ValueHandle]) -> Instruction<'db> {
        Instruction::Tuple {
            element_handles: HandleArrayList::<ValueHandle>::from(
                &mut self.dfg.suballocator,
                element_handles,
            ),
        }
    }

    pub(crate) fn allocate_call(
        &mut self,
        function: FunctionKey<'db>,
        argument_handles: &[ValueHandle],
    ) -> Instruction<'db> {
        Instruction::Call {
            function,
            argument_handles: HandleArrayList::<ValueHandle>::from(
                &mut self.dfg.suballocator,
                argument_handles,
            ),
        }
    }

    pub(crate) fn allocate_call_indirect(
        &mut self,
        callee_and_argument_handles: &[ValueHandle],
    ) -> Instruction<'db> {
        Instruction::CallIndirect {
            callee_and_argument_handles: HandleArrayList::<ValueHandle>::from(
                &mut self.dfg.suballocator,
                callee_and_argument_handles,
            ),
        }
    }

    pub(crate) fn allocate_conditional_branch(
        &mut self,
        operand_handle: ValueHandle,
        true_block_handle: BlockHandle,
        true_block_argument_handles: &[ValueHandle],
        false_block_handle: BlockHandle,
        false_block_argument_handles: &[ValueHandle],
    ) -> Instruction<'db> {
        Instruction::ConditionalBranch {
            operand_handle,
            true_block_handle,
            true_block_argument_handles: HandleArrayList::<ValueHandle>::from(
                &mut self.dfg.suballocator,
                true_block_argument_handles,
            ),
            false_block_handle,
            false_block_argument_handles: HandleArrayList::<ValueHandle>::from(
                &mut self.dfg.suballocator,
                false_block_argument_handles,
            ),
        }
    }

    pub(crate) fn add_undefined(&mut self, ty: Ty<'db>) -> ValueHandle {
        self.dfg.values.add(Value {
            ty,
            alias_handle: None,
            origin: ValueOrigin::Undefined(ty),
        })
    }

    pub(crate) fn mark_as_alias(
        &mut self,
        destination_handle: ValueHandle,
        source_handle: ValueHandle,
    ) {
        assert!(
            !self.value_is_attached(destination_handle),
            "cannot alias a value that is still attached to an instruction or block"
        );

        let original = self.resolve_aliases(source_handle);

        assert!(
            destination_handle != original,
            "aliasing a value to itself would create a loop"
        );

        assert!(
            self.dfg.values[destination_handle].ty == self.dfg.values[original].ty,
            "aliasing values of different types"
        );

        self.dfg.values[destination_handle].alias_handle = Some(original);
    }

    pub(crate) fn flush_aliases(&mut self) {
        // step 1: compresses every alias chain so each aliased value points directly at its final target
        let value_handles: Vec<ValueHandle> = self.dfg.values.keys().collect();
        for mut value_handle in value_handles {
            if let Some(original_pointee_handle) = self.dfg.values[value_handle].alias_handle {
                let resolved = Some(self.resolve_aliases(original_pointee_handle));
                let mut next_value_handle = original_pointee_handle;
                loop {
                    self.dfg.values[value_handle].alias_handle = resolved;

                    value_handle = next_value_handle;

                    if let Some(next_pointee_handle) = self.dfg.values[value_handle].alias_handle {
                        next_value_handle = next_pointee_handle;
                    } else {
                        break;
                    }
                }
            }
        }

        // step 2: propagates that resolution out into every instruction's actual operand references.
        for instruction in self.dfg.instructions.values_mut() {
            instruction.rewrite_operands(&mut self.dfg.suballocator, |value_handle| {
                match self.dfg.values[value_handle].alias_handle {
                    Some(original) => original,
                    None => value_handle,
                }
            });
        }
    }

    // NOTE: Aliases are never attached.
    fn value_is_attached(&self, value_handle: ValueHandle) -> bool {
        if self.dfg.values[value_handle].alias_handle.is_some() {
            return false;
        }
        match self.dfg.values[value_handle].origin {
            ValueOrigin::InstructionResult(instruction_handle, index) => {
                self.dfg.instruction_results[instruction_handle]
                    .get(&self.dfg.suballocator, index as usize)
                    == Some(value_handle)
            }
            ValueOrigin::Parameter(block_handle, index) => {
                self.dfg.blocks[block_handle]
                    .parameter_handles
                    .get(&self.dfg.suballocator, index as usize)
                    == Some(value_handle)
            }
            ValueOrigin::Undefined(_) => false,
        }
    }

    fn add_instruction(
        &mut self,
        instruction: Instruction<'db>,
        result_types: &[Ty<'db>],
    ) -> InstructionHandle {
        let instruction_handle = self.dfg.instructions.add(instruction);
        let result_ssa_handles: Vec<ValueHandle> = result_types
            .iter()
            .enumerate()
            .map(|(i, &ty)| {
                self.dfg.values.add(Value {
                    ty,
                    alias_handle: None,
                    origin: ValueOrigin::InstructionResult(
                        instruction_handle,
                        u16::try_from(i).unwrap(),
                    ),
                })
            })
            .collect();
        self.dfg.instruction_results.add(
            instruction_handle,
            HandleArrayList::<ValueHandle>::from(&mut self.dfg.suballocator, &result_ssa_handles),
        );
        instruction_handle
    }

    fn link_instruction_to_block(
        &mut self,
        block_handle: BlockHandle,
        instruction_handle: InstructionHandle,
    ) {
        let prev = self.layout.blocks[block_handle].last_handle;
        let node = InstructionNode {
            block_handle: Some(block_handle),
            previous_handle: prev,
            next_handle: None,
        };
        self.layout.instructions.add(instruction_handle, node);
        match prev {
            Some(prev) => self.layout.instructions[prev].next_handle = Some(instruction_handle),
            None => self.layout.blocks[block_handle].first_handle = Some(instruction_handle),
        }
        self.layout.blocks[block_handle].last_handle = Some(instruction_handle);
    }
}

impl<'db> Instruction<'db> {
    #[expect(
        clippy::match_same_arms,
        reason = "one arm per instruction, in declaration order"
    )]
    fn rewrite_operands(
        &mut self,
        suballocator: &mut HandleArrayListAllocator<ValueHandle>,
        mut f: impl FnMut(ValueHandle) -> ValueHandle,
    ) {
        match self {
            Instruction::Binary {
                operand_handles, ..
            } => {
                operand_handles[0] = f(operand_handles[0]);
                operand_handles[1] = f(operand_handles[1]);
            }
            Instruction::Unary { operand_handle, .. } => *operand_handle = f(*operand_handle),
            Instruction::Tuple { element_handles } => {
                for v in element_handles.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
            }
            Instruction::ExtractField { operand_handle, .. } => {
                *operand_handle = f(*operand_handle);
            }
            Instruction::InsertField {
                operand_handles, ..
            } => {
                operand_handles[0] = f(operand_handles[0]);
                operand_handles[1] = f(operand_handles[1]);
            }
            Instruction::Call {
                argument_handles, ..
            } => {
                for v in argument_handles.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
            }
            Instruction::CallIndirect {
                callee_and_argument_handles,
            } => {
                for v in callee_and_argument_handles.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
            }
            Instruction::Jump {
                block_argument_handles,
                ..
            } => {
                for v in block_argument_handles.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
            }
            Instruction::ConditionalBranch {
                operand_handle,
                true_block_argument_handles,
                false_block_argument_handles,
                ..
            } => {
                *operand_handle = f(*operand_handle);
                for v in true_block_argument_handles.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
                for v in false_block_argument_handles.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
            }
            Instruction::Return { output_handles } => {
                for v in output_handles.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
            }
            Instruction::IntegerLiteral { .. }
            | Instruction::BooleanLiteral { .. }
            | Instruction::Constant { .. }
            | Instruction::FunctionAddress { .. }
            | Instruction::Unreachable => {}
        }
    }
}

#[expect(
    dead_code,
    reason = "MIR API for codegen and MIR passes, which aren't written yet"
)]
impl<'a, 'db> BlockView<'a, 'db> {
    pub(crate) fn handle(&self) -> BlockHandle {
        self.block_handle
    }

    pub(crate) fn next(&self) -> Option<BlockHandle> {
        self.cfg.layout.blocks[self.block_handle].next_handle
    }

    pub(crate) fn prev(&self) -> Option<BlockHandle> {
        self.cfg.layout.blocks[self.block_handle].previous_handle
    }

    pub(crate) fn instructions(&self) -> InstructionIter<'a> {
        InstructionIter {
            layout: &self.cfg.layout,
            next_handle: self.cfg.layout.blocks[self.block_handle].first_handle,
        }
    }

    pub(crate) fn first_instruction(&self) -> Option<InstructionHandle> {
        self.cfg.layout.blocks[self.block_handle].first_handle
    }

    pub(crate) fn last_instruction(&self) -> Option<InstructionHandle> {
        self.cfg.layout.blocks[self.block_handle].last_handle
    }

    pub(crate) fn parameters(&self) -> &'a [ValueHandle] {
        self.cfg.dfg.blocks[self.block_handle]
            .parameter_handles
            .to_slice(&self.cfg.dfg.suballocator)
    }
}

impl<'a, 'db> BlockViewMut<'a, 'db> {
    pub(crate) fn append_parameter(&mut self, ty: Ty<'db>) -> ValueHandle {
        let parameter = self.cfg.dfg.values.next_handle();
        self.cfg.dfg.blocks[self.block_handle]
            .parameter_handles
            .add_last(&mut self.cfg.dfg.suballocator, parameter);
        let count = self.cfg.dfg.blocks[self.block_handle]
            .parameter_handles
            .count(&self.cfg.dfg.suballocator);
        assert!(
            u16::try_from(count).is_ok(),
            "the block has too many parameters"
        );
        self.cfg.dfg.values.add(Value {
            ty,
            alias_handle: None,
            origin: ValueOrigin::Parameter(self.block_handle, u16::try_from(count - 1).unwrap()),
        })
    }

    #[expect(
        dead_code,
        reason = "MIR API for codegen and MIR passes, which aren't written yet"
    )]
    pub(crate) fn swap_remove_parameter(&mut self, index: usize) {
        let params = self.cfg.dfg.blocks[self.block_handle]
            .parameter_handles
            .to_mut_slice(&mut self.cfg.dfg.suballocator);
        params.swap(index, params.len() - 1);
        // The value now sitting at `index` still thinks it's the parameter at its old
        // (last) position, so patch its definition to match its new slot.
        let moved_value = params[index];
        self.cfg.dfg.blocks[self.block_handle]
            .parameter_handles
            .clear_last(&mut self.cfg.dfg.suballocator);
        if let ValueOrigin::Parameter(_, num) = &mut self.cfg.dfg.values[moved_value].origin {
            *num = u16::try_from(index).unwrap();
        }
    }

    pub(crate) fn remove_parameter(&mut self, index: usize) {
        self.cfg.dfg.blocks[self.block_handle]
            .parameter_handles
            .remove(index, &mut self.cfg.dfg.suballocator);
        let parameters = self.cfg.dfg.blocks[self.block_handle].parameter_handles;
        let count = parameters.count(&self.cfg.dfg.suballocator);
        for i in index..count {
            let value_handle = parameters.get(&self.cfg.dfg.suballocator, i).unwrap();
            if let ValueOrigin::Parameter(_, num) = &mut self.cfg.dfg.values[value_handle].origin {
                *num = u16::try_from(i).unwrap();
            }
        }
    }

    #[expect(
        dead_code,
        reason = "MIR API for codegen and MIR passes, which aren't written yet"
    )]
    pub(crate) fn detach_parameters(&mut self) -> HandleArrayList<ValueHandle> {
        let params = self.cfg.dfg.blocks[self.block_handle].parameter_handles;
        self.cfg.dfg.blocks[self.block_handle].parameter_handles =
            HandleArrayList::<ValueHandle>::new();
        params
    }
}

#[expect(
    dead_code,
    reason = "MIR API for codegen and MIR passes, which aren't written yet"
)]
impl<'a, 'db> InstructionView<'a, 'db> {
    pub(crate) fn handle(&self) -> InstructionHandle {
        self.instruction_handle
    }

    pub(crate) fn next(&self) -> Option<InstructionHandle> {
        self.cfg.layout.instructions[self.instruction_handle].next_handle
    }

    pub(crate) fn prev(&self) -> Option<InstructionHandle> {
        self.cfg.layout.instructions[self.instruction_handle].previous_handle
    }

    pub(crate) fn is_terminator(&self) -> bool {
        matches!(
            self.cfg.dfg.instructions[self.instruction_handle],
            Instruction::Jump { .. }
                | Instruction::ConditionalBranch { .. }
                | Instruction::Return { .. }
                | Instruction::Unreachable
        )
    }

    #[expect(
        clippy::match_same_arms,
        reason = "one arm per instruction, in declaration order"
    )]
    pub(crate) fn inputs(&self) -> &'a [ValueHandle] {
        let suballocator = &self.cfg.dfg.suballocator;
        match &self.cfg.dfg.instructions[self.instruction_handle] {
            Instruction::Binary {
                operand_handles, ..
            } => operand_handles,
            Instruction::Unary { operand_handle, .. } => slice::from_ref(operand_handle),
            Instruction::Tuple { element_handles } => element_handles.to_slice(suballocator),
            Instruction::ExtractField { operand_handle, .. } => slice::from_ref(operand_handle),
            Instruction::InsertField {
                operand_handles, ..
            } => operand_handles,
            Instruction::Call {
                argument_handles, ..
            } => argument_handles.to_slice(suballocator),
            Instruction::CallIndirect {
                callee_and_argument_handles,
            } => callee_and_argument_handles.to_slice(suballocator),
            Instruction::Jump {
                block_argument_handles,
                ..
            } => block_argument_handles.to_slice(suballocator),
            Instruction::ConditionalBranch { operand_handle, .. } => {
                slice::from_ref(operand_handle)
            } // Only the condition is an operand here (the then/else block arguments are not operands in the traditional sense)
            Instruction::Return { output_handles } => output_handles.to_slice(suballocator),
            Instruction::IntegerLiteral { .. }
            | Instruction::BooleanLiteral { .. }
            | Instruction::Constant { .. }
            | Instruction::FunctionAddress { .. }
            | Instruction::Unreachable => &[],
        }
    }

    pub(crate) fn results(&self) -> &'a [ValueHandle] {
        self.cfg.dfg.instruction_results[self.instruction_handle]
            .to_slice(&self.cfg.dfg.suballocator)
    }

    pub(crate) fn first_result(&self) -> Option<ValueHandle> {
        self.cfg.dfg.instruction_results[self.instruction_handle].get(&self.cfg.dfg.suballocator, 0)
    }

    pub(crate) fn containing_block(&self) -> Option<BlockHandle> {
        self.cfg.layout.instructions[self.instruction_handle].block_handle
    }

    pub(crate) fn used_values(&self) -> UsedValuesIter<'a> {
        match &self.cfg.dfg.instructions[self.instruction_handle] {
            Instruction::ConditionalBranch {
                operand_handle,
                true_block_argument_handles,
                false_block_argument_handles,
                ..
            } => UsedValuesIter::Branch {
                operand_handle: Some(*operand_handle),
                then_argument_handles: true_block_argument_handles
                    .to_slice(&self.cfg.dfg.suballocator)
                    .iter(),
                else_argument_handles: false_block_argument_handles
                    .to_slice(&self.cfg.dfg.suballocator)
                    .iter(),
            },
            Instruction::Binary { .. }
            | Instruction::Unary { .. }
            | Instruction::Tuple { .. }
            | Instruction::ExtractField { .. }
            | Instruction::InsertField { .. }
            | Instruction::Call { .. }
            | Instruction::CallIndirect { .. }
            | Instruction::Jump { .. }
            | Instruction::Return { .. }
            | Instruction::IntegerLiteral { .. }
            | Instruction::BooleanLiteral { .. }
            | Instruction::Constant { .. }
            | Instruction::FunctionAddress { .. }
            | Instruction::Unreachable => UsedValuesIter::Slice(self.inputs().iter()),
        }
    }

    pub(crate) fn block_argument(
        &self,
        destination_handle: BlockHandle,
        index: usize,
    ) -> ValueHandle {
        let suballocator = &self.cfg.dfg.suballocator;
        match &self.cfg.dfg.instructions[self.instruction_handle] {
            Instruction::Jump {
                destination_handle: jump_destination_handle,
                block_argument_handles,
            } => {
                assert_eq!(
                    *jump_destination_handle, destination_handle,
                    "instruction does not jump to destination"
                );
                block_argument_handles.get(suballocator, index).unwrap()
            }
            Instruction::ConditionalBranch {
                true_block_handle,
                true_block_argument_handles,
                false_block_handle,
                false_block_argument_handles,
                ..
            } => {
                if *true_block_handle == destination_handle {
                    true_block_argument_handles
                        .get(suballocator, index)
                        .unwrap()
                } else if *false_block_handle == destination_handle {
                    false_block_argument_handles
                        .get(suballocator, index)
                        .unwrap()
                } else {
                    panic!("instruction does not branch to destination")
                }
            }
            _ => panic!("instruction is not a branch"),
        }
    }

    pub(crate) fn as_instruction_ref(&self) -> InstructionRef<'a, 'db> {
        let suballocator = &self.cfg.dfg.suballocator;
        match &self.cfg.dfg.instructions[self.instruction_handle] {
            Instruction::Binary {
                operator,
                operand_handles,
            } => InstructionRef::Binary {
                operator: *operator,
                operand_handles: [operand_handles[0], operand_handles[1]],
            },
            Instruction::Unary {
                operator,
                operand_handle,
            } => InstructionRef::Unary {
                operator: *operator,
                operand_handle: *operand_handle,
            },
            Instruction::IntegerLiteral { value } => {
                InstructionRef::IntegerLiteral { value: *value }
            }
            Instruction::Tuple { element_handles } => InstructionRef::Tuple {
                element_handles: element_handles.to_slice(suballocator),
            },
            Instruction::ExtractField {
                operand_handle,
                index,
            } => InstructionRef::ExtractField {
                operand_handle: *operand_handle,
                index: *index,
            },
            Instruction::InsertField {
                operand_handles,
                index,
            } => InstructionRef::InsertField {
                operand_handles: [operand_handles[0], operand_handles[1]],
                index: *index,
            },
            Instruction::BooleanLiteral { value } => {
                InstructionRef::BooleanLiteral { value: *value }
            }
            Instruction::Constant { constant } => InstructionRef::Constant {
                constant: *constant,
            },
            Instruction::FunctionAddress { function } => InstructionRef::FunctionAddress {
                function: *function,
            },
            Instruction::Call {
                function,
                argument_handles,
            } => InstructionRef::Call {
                function: *function,
                argument_handles: argument_handles.to_slice(suballocator),
            },
            Instruction::CallIndirect {
                callee_and_argument_handles,
            } => InstructionRef::CallIndirect {
                callee_and_argument_handles: callee_and_argument_handles.to_slice(suballocator),
            },
            Instruction::Jump {
                destination_handle,
                block_argument_handles,
            } => InstructionRef::Jump {
                destination_handle: *destination_handle,
                block_argument_handles: block_argument_handles.to_slice(suballocator),
            },
            Instruction::ConditionalBranch {
                operand_handle,
                true_block_handle,
                true_block_argument_handles,
                false_block_handle,
                false_block_argument_handles,
            } => InstructionRef::ConditionalBranch {
                operand_handle: *operand_handle,
                true_block_handle: *true_block_handle,
                true_block_argument_handles: true_block_argument_handles.to_slice(suballocator),
                false_block_handle: *false_block_handle,
                false_block_argument_handles: false_block_argument_handles.to_slice(suballocator),
            },
            Instruction::Return { output_handles } => InstructionRef::Return {
                output_handles: output_handles.to_slice(suballocator),
            },
            Instruction::Unreachable => InstructionRef::Unreachable,
        }
    }
}

impl<'a, 'db> InstructionViewMut<'a, 'db> {
    #[expect(
        dead_code,
        reason = "MIR API for codegen and MIR passes, which aren't written yet"
    )]
    pub(crate) fn rewrite_operands(&mut self, f: impl FnMut(ValueHandle) -> ValueHandle) {
        self.cfg.dfg.instructions[self.instruction_handle]
            .rewrite_operands(&mut self.cfg.dfg.suballocator, f);
    }

    pub(crate) fn append_block_argument(
        &mut self,
        destination_handle: BlockHandle,
        value: ValueHandle,
    ) {
        let suballocator = &mut self.cfg.dfg.suballocator;
        match &mut self.cfg.dfg.instructions[self.instruction_handle] {
            Instruction::Jump {
                destination_handle: jump_destination_handle,
                block_argument_handles,
            } => {
                assert_eq!(
                    *jump_destination_handle, destination_handle,
                    "instruction does not jump to destination"
                );
                block_argument_handles.add_last(suballocator, value);
            }
            Instruction::ConditionalBranch {
                true_block_handle,
                true_block_argument_handles,
                false_block_handle,
                false_block_argument_handles,
                ..
            } => {
                let mut branched = false;
                if *true_block_handle == destination_handle {
                    true_block_argument_handles.add_last(suballocator, value);
                    branched = true;
                }
                if *false_block_handle == destination_handle {
                    false_block_argument_handles.add_last(suballocator, value);
                    branched = true;
                }
                assert!(branched, "instruction does not branch to destination");
            }
            _ => panic!("instruction is not a branch"),
        }
    }
}

impl<'a, 'db> ValueView<'a, 'db> {
    pub(crate) fn ty(&self) -> Ty<'db> {
        self.cfg.dfg.values[self.value_handle].ty
    }

    pub(crate) fn origin(&self) -> ValueOrigin<'db> {
        self.cfg.dfg.values[self.cfg.resolve_aliases(self.value_handle)].origin
    }

    pub(crate) fn alias_target(&self) -> Option<ValueHandle> {
        self.cfg.dfg.values[self.value_handle].alias_handle
    }
}

impl Iterator for UsedValuesIter<'_> {
    type Item = ValueHandle;

    fn next(&mut self) -> Option<ValueHandle> {
        match self {
            UsedValuesIter::Slice(iter) => iter.next().copied(),
            UsedValuesIter::Branch {
                operand_handle,
                then_argument_handles,
                else_argument_handles,
            } => operand_handle
                .take()
                .or_else(|| then_argument_handles.next().copied())
                .or_else(|| else_argument_handles.next().copied()),
        }
    }
}

impl DoubleEndedIterator for UsedValuesIter<'_> {
    fn next_back(&mut self) -> Option<ValueHandle> {
        match self {
            UsedValuesIter::Slice(iter) => iter.next_back().copied(),
            UsedValuesIter::Branch {
                operand_handle,
                then_argument_handles,
                else_argument_handles,
            } => else_argument_handles
                .next_back()
                .copied()
                .or_else(|| then_argument_handles.next_back().copied())
                .or_else(|| operand_handle.take()),
        }
    }
}

impl Iterator for BlockIter<'_> {
    type Item = BlockHandle;
    fn next(&mut self) -> Option<BlockHandle> {
        let block = self.next_handle?;
        self.next_handle = self.layout.blocks[block].next_handle;
        Some(block)
    }
}

impl Iterator for InstructionIter<'_> {
    type Item = InstructionHandle;
    fn next(&mut self) -> Option<InstructionHandle> {
        let inst = self.next_handle?;
        self.next_handle = self.layout.instructions[inst].next_handle;
        Some(inst)
    }
}

impl<'db> DataFlowGraph<'db> {
    fn new() -> Self {
        Self {
            values: HandleMap::new(),
            instructions: HandleMap::new(),
            instruction_results: SideHandleMap::new(),
            blocks: HandleMap::new(),
            suballocator: HandleArrayListAllocator::<ValueHandle>::new(),
        }
    }
}

impl Layout {
    fn new() -> Self {
        Self {
            entry_handle: None,
            exit_handle: None,
            blocks: SideHandleMap::new(),
            instructions: SideHandleMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_block_before_entry_updates_entry() {
        let mut cfg = Cfg::new();
        let a = cfg.allocate_block();
        cfg.append_block(a);

        let b = cfg.allocate_block();
        cfg.add_block_before(b, a);

        // `b` is now the entry, and `a`'s prev is `b`.
        assert_eq!(cfg.entry().unwrap().handle(), b);
        assert_eq!(cfg.block(a).prev(), Some(b));
        assert_eq!(cfg.block(b).next(), Some(a));
        assert_eq!(cfg.block(b).prev(), None);
    }

    #[test]
    fn add_block_before_middle_updates_middle_predecessor() {
        let mut cfg = Cfg::new();
        let a = cfg.allocate_block();
        cfg.append_block(a);
        let c = cfg.allocate_block();
        cfg.append_block(c);

        let b = cfg.allocate_block();
        cfg.add_block_before(b, c);

        // entry is unchanged; `a`'s next must now point at `b`, not `c`.
        assert_eq!(cfg.entry().unwrap().handle(), a);
        assert_eq!(cfg.block(a).next(), Some(b));
        assert_eq!(cfg.block(b).prev(), Some(a));
        assert_eq!(cfg.block(b).next(), Some(c));
        assert_eq!(cfg.block(c).prev(), Some(b));
    }

    #[test]
    fn add_instruction_before_first_updates_block_head() {
        let mut cfg = Cfg::new();
        let block = cfg.allocate_block();
        cfg.append_block(block);
        let first = cfg.append_instruction(block, Instruction::Unreachable, &[]);

        let new_first = cfg.add_instruction_before(Instruction::Unreachable, &[], first);

        assert_eq!(cfg.block(block).first_instruction(), Some(new_first));
        assert_eq!(cfg.instruction(first).prev(), Some(new_first));
        assert_eq!(cfg.instruction(new_first).next(), Some(first));
    }

    #[test]
    fn add_instruction_before_middle_updates_middle_predecessor() {
        let mut cfg = Cfg::new();
        let block = cfg.allocate_block();
        cfg.append_block(block);
        let first = cfg.append_instruction(block, Instruction::Unreachable, &[]);
        let third = cfg.append_instruction(block, Instruction::Unreachable, &[]);

        let second = cfg.add_instruction_before(Instruction::Unreachable, &[], third);

        assert_eq!(cfg.block(block).first_instruction(), Some(first));
        assert_eq!(cfg.instruction(first).next(), Some(second));
        assert_eq!(cfg.instruction(second).prev(), Some(first));
        assert_eq!(cfg.instruction(second).next(), Some(third));
        assert_eq!(cfg.instruction(third).prev(), Some(second));
    }
}
