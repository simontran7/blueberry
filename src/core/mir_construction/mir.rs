use std::slice;

use handlemap::handle_map::HandleMap;
use handlemap::handle_map::SideHandleMap;

use crate::core::common::string_interner::Symbol;
use crate::core::common::types::TypeId;
use crate::front_end::semantic_analysis::hir::DefinitionBindingId;
use crate::front_end::syntactic_analysis::cst::operators::{BinOp, UnOp};
use crate::middle_end::handle_list::{HandleList, HandleListSubAllocator};

pub(crate) struct Mir {
    functions: Vec<Function>,
}

impl Mir {
    pub(crate) fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }

    pub(crate) fn add_function(&mut self, function: Function) {
        self.functions.push(function);
    }

    pub(crate) fn functions(&self) -> impl Iterator<Item = &Function> {
        self.functions.iter()
    }
}

pub(crate) struct Function {
    pub(crate) definition_binding_id: DefinitionBindingId,
    pub(crate) name: Symbol,
    pub(crate) signature: Signature,
    pub(crate) body: Cfg,
}

pub(crate) struct Cfg {
    dfg: DataFlowGraph,
    layout: Layout,
}

pub(crate) struct Value {
    type_id: TypeId,
    alias_id: Option<ValueId>,
    origin: ValueOrigin,
}

#[derive(Clone, Copy)]
pub(crate) enum ValueOrigin {
    InstructionResult(InstructionId, u16),
    Parameter(BlockId, u16),
    Undefined(TypeId),
}

pub(crate) struct Block {
    parameter_ids: HandleList<ValueId>,
}

pub(crate) enum InstructionKind<T> {
    // Arithmetic
    Binary {
        operator: BinOp,
        operand_ids: [ValueId; 2],
    },
    Unary {
        operator: UnOp,
        operand_id: ValueId,
    },

    // Literals
    IntegerLiteral {
        value: u128,
    },
    BooleanLiteral {
        value: bool,
    },

    // Calls
    Call {
        callee_id: FunctionReferenceId,
        argument_ids: T,
    },

    // Terminators
    Jump {
        destination_id: BlockId,
        block_argument_ids: T,
    },
    ConditionalBranch {
        operand_id: ValueId,
        true_block_id: BlockId,
        true_block_argument_ids: T,
        false_block_id: BlockId,
        false_block_argument_ids: T,
    },
    Return {
        output_ids: T,
    },
    Unreachable,
}

pub(crate) type Instruction = InstructionKind<HandleList<ValueId>>;

pub(crate) struct FunctionReference {
    pub(crate) definition_binding_id: DefinitionBindingId,
    pub(crate) name: Symbol,
    pub(crate) signature_id: SignatureId,
}

pub(crate) struct Signature {
    pub(crate) parameter_type_ids: Vec<TypeId>,
    pub(crate) return_type_id: TypeId,
}

// Opaque, 4-byte handles into the tables above.
handlemap::handle_impl!(pub(crate) BlockId);
handlemap::handle_impl!(pub(crate) InstructionId);
handlemap::handle_impl!(pub(crate) FunctionReferenceId);
handlemap::handle_impl!(pub(crate) SignatureId);
handlemap::handle_impl!(pub(crate) ValueId);

pub(crate) struct BlockView<'a> {
    block_id: BlockId,
    cfg: &'a Cfg,
}

pub(crate) struct BlockViewMut<'a> {
    block_id: BlockId,
    cfg: &'a mut Cfg,
}

pub(crate) struct InstructionView<'a> {
    instruction_id: InstructionId,
    cfg: &'a Cfg,
}

pub(crate) type InstructionRef<'a> = InstructionKind<&'a [ValueId]>;

pub(crate) struct InstructionViewMut<'a> {
    instruction_id: InstructionId,
    cfg: &'a mut Cfg,
}

pub(crate) struct ValueView<'a> {
    value_id: ValueId,
    cfg: &'a Cfg,
}

pub(crate) enum UsedValuesIter<'a> {
    Slice(slice::Iter<'a, ValueId>),
    Branch {
        operand_id: Option<ValueId>,
        then_argument_ids: slice::Iter<'a, ValueId>,
        else_argument_ids: slice::Iter<'a, ValueId>,
    },
}

pub(crate) struct BlockIter<'a> {
    layout: &'a Layout,
    next_id: Option<BlockId>,
}

pub(crate) struct InstructionIter<'a> {
    layout: &'a Layout,
    next_id: Option<InstructionId>,
}

struct DataFlowGraph {
    values: HandleMap<ValueId, Value>,
    instructions: HandleMap<InstructionId, Instruction>,
    instruction_results: SideHandleMap<InstructionId, HandleList<ValueId>>,
    blocks: HandleMap<BlockId, Block>,
    function_references: HandleMap<FunctionReferenceId, FunctionReference>,
    signatures: HandleMap<SignatureId, Signature>,
    suballocator: HandleListSubAllocator<ValueId>,
}

struct Layout {
    entry_id: Option<BlockId>,
    exit_id: Option<BlockId>,
    blocks: SideHandleMap<BlockId, BlockNode>,
    instructions: SideHandleMap<InstructionId, InstructionNode>,
}

// Clone: required by `SideHandleMap::add` for resize padding
#[derive(Clone, Default)]
struct BlockNode {
    previous_id: Option<BlockId>,
    first_id: Option<InstructionId>,
    last_id: Option<InstructionId>,
    next_id: Option<BlockId>,
}

// Clone: required by `SideHandleMap::add` for resize padding
#[derive(Clone, Default)]
struct InstructionNode {
    previous_id: Option<InstructionId>,
    block_id: Option<BlockId>,
    next_id: Option<InstructionId>,
}

impl Cfg {
    pub(crate) fn new() -> Self {
        Self {
            dfg: DataFlowGraph::new(),
            layout: Layout::new(),
        }
    }

    pub(crate) fn allocate_block(&mut self) -> BlockId {
        self.dfg.blocks.add(Block {
            parameter_ids: HandleList::<ValueId>::new(),
        })
    }

    pub(crate) fn entry(&self) -> Option<BlockView<'_>> {
        self.layout.entry_id.map(|block_id| BlockView {
            block_id,
            cfg: self,
        })
    }

    pub(crate) fn exit(&self) -> Option<BlockView<'_>> {
        self.layout.exit_id.map(|block_id| BlockView {
            block_id,
            cfg: self,
        })
    }

    pub(crate) fn block_ids(&self) -> BlockIter<'_> {
        BlockIter {
            layout: &self.layout,
            next_id: self.layout.entry_id,
        }
    }

    pub(crate) fn ssa_ids(&self) -> impl Iterator<Item = ValueId> + '_ {
        self.dfg.values.keys()
    }

    pub(crate) fn get_block(&self, block_id: BlockId) -> BlockView<'_> {
        BlockView {
            block_id,
            cfg: self,
        }
    }

    pub(crate) fn get_block_mut(&mut self, block_id: BlockId) -> BlockViewMut<'_> {
        BlockViewMut {
            block_id,
            cfg: self,
        }
    }

    pub(crate) fn contains(&self, block_id: BlockId) -> bool {
        Some(block_id) == self.layout.entry_id
            || self
                .layout
                .blocks
                .get(block_id)
                .is_some_and(|node| node.previous_id.is_some())
    }

    pub(crate) fn append_block(&mut self, block_id: BlockId) {
        assert!(
            !self.contains(block_id),
            "cannot append a block that is already in the cfg"
        );
        let node = BlockNode {
            previous_id: self.layout.exit_id,
            next_id: None,
            first_id: None,
            last_id: None,
        };
        self.layout.blocks.add(block_id, node);
        if let Some(exit) = self.layout.exit_id {
            self.layout.blocks[exit].next_id = Some(block_id);
        } else {
            self.layout.entry_id = Some(block_id);
        }
        self.layout.exit_id = Some(block_id);
    }

    pub(crate) fn add_block_before(&mut self, block_id: BlockId, before_id: BlockId) {
        assert!(
            !self.contains(block_id),
            "cannot insert a block that is already in the cfg"
        );
        assert!(
            self.contains(before_id),
            "block insertion point is not in the cfg"
        );
        let old_prev_id = self.layout.blocks[before_id].previous_id;
        let node = BlockNode {
            previous_id: old_prev_id,
            next_id: Some(before_id),
            first_id: None,
            last_id: None,
        };
        self.layout.blocks.add(block_id, node);
        self.layout.blocks[before_id].previous_id = Some(block_id);
        match old_prev_id {
            Some(old_prev_id) => self.layout.blocks[old_prev_id].next_id = Some(block_id),
            None => self.layout.entry_id = Some(block_id),
        }
    }

    pub(crate) fn add_block_after(&mut self, block_id: BlockId, after_id: BlockId) {
        assert!(
            !self.contains(block_id),
            "cannot insert a block that is already in the cfg"
        );
        assert!(
            self.contains(after_id),
            "block insertion point is not in the cfg"
        );
        let before_id = self.layout.blocks[after_id].next_id;
        let node = BlockNode {
            previous_id: Some(after_id),
            next_id: before_id,
            first_id: None,
            last_id: None,
        };
        self.layout.blocks.add(block_id, node);
        self.layout.blocks[after_id].next_id = Some(block_id);
        match before_id {
            None => self.layout.exit_id = Some(block_id),
            Some(before_id) => self.layout.blocks[before_id].previous_id = Some(block_id),
        }
    }

    pub(crate) fn remove_block(&mut self, block_id: BlockId) {
        assert!(
            self.contains(block_id),
            "block pointed by `block_id` is not in the cfg"
        );
        assert!(
            self.layout.blocks[block_id].first_id.is_none(),
            "cannot remove a block that still has instructions"
        );
        let prev = self.layout.blocks[block_id].previous_id;
        let next = self.layout.blocks[block_id].next_id;
        self.layout.blocks[block_id].previous_id = None;
        self.layout.blocks[block_id].next_id = None;
        match prev {
            None => self.layout.entry_id = next,
            Some(p) => self.layout.blocks[p].next_id = next,
        }
        match next {
            None => self.layout.exit_id = prev,
            Some(n) => self.layout.blocks[n].previous_id = prev,
        }
    }

    pub(crate) fn clear_block(&mut self, block_id: BlockId) {
        assert!(
            self.contains(block_id),
            "block pointed by `block_id` is not in the cfg"
        );
        while let Some(instruction_id) = self.layout.blocks[block_id].first_id {
            self.remove_instruction(instruction_id);
        }
    }

    pub(crate) fn get_instruction(&self, instruction_id: InstructionId) -> InstructionView<'_> {
        InstructionView {
            instruction_id,
            cfg: self,
        }
    }

    pub(crate) fn get_instruction_mut(
        &mut self,
        instruction_id: InstructionId,
    ) -> InstructionViewMut<'_> {
        InstructionViewMut {
            instruction_id,
            cfg: self,
        }
    }

    fn add_instruction(
        &mut self,
        instruction: Instruction,
        result_type_ids: &[TypeId],
    ) -> InstructionId {
        let instruction_id = self.dfg.instructions.add(instruction);
        let result_ssa_ids: Vec<ValueId> = result_type_ids
            .iter()
            .enumerate()
            .map(|(i, &type_id)| {
                self.dfg.values.add(Value {
                    type_id,
                    alias_id: None,
                    origin: ValueOrigin::InstructionResult(instruction_id, i as u16),
                })
            })
            .collect();
        self.dfg.instruction_results.add(
            instruction_id,
            HandleList::<ValueId>::from(&mut self.dfg.suballocator, &result_ssa_ids),
        );
        instruction_id
    }

    fn link_instruction_to_block(&mut self, block_id: BlockId, instruction_id: InstructionId) {
        let prev = self.layout.blocks[block_id].last_id;
        let node = InstructionNode {
            block_id: Some(block_id),
            previous_id: prev,
            next_id: None,
        };
        self.layout.instructions.add(instruction_id, node);
        match prev {
            Some(prev) => self.layout.instructions[prev].next_id = Some(instruction_id),
            None => self.layout.blocks[block_id].first_id = Some(instruction_id),
        }
        self.layout.blocks[block_id].last_id = Some(instruction_id);
    }

    pub(crate) fn add_instruction_before(
        &mut self,
        instruction: Instruction,
        result_type_ids: &[TypeId],
        before_id: InstructionId,
    ) -> InstructionId {
        let instruction_id = self.add_instruction(instruction, result_type_ids);

        let block_id = self.layout.instructions[before_id]
            .block_id
            .expect("instruction insertion point is not in the cfg");

        let old_prev = self.layout.instructions[before_id].previous_id;

        self.layout.instructions.add(
            instruction_id,
            InstructionNode {
                block_id: Some(block_id),
                previous_id: old_prev,
                next_id: Some(before_id),
            },
        );

        self.layout.instructions[before_id].previous_id = Some(instruction_id);

        match old_prev {
            None => self.layout.blocks[block_id].first_id = Some(instruction_id),
            Some(before_prev_id) => {
                self.layout.instructions[before_prev_id].next_id = Some(instruction_id)
            }
        }

        instruction_id
    }

    pub(crate) fn append_instruction(
        &mut self,
        block_id: BlockId,
        instruction: Instruction,
        result_type_ids: &[TypeId],
    ) -> InstructionId {
        let instruction_id = self.add_instruction(instruction, result_type_ids);
        self.link_instruction_to_block(block_id, instruction_id);
        instruction_id
    }

    pub(crate) fn set_terminator(&mut self, block_id: BlockId, terminator: Instruction) {
        let instruction_id = self.add_instruction(terminator, &[]);
        self.link_instruction_to_block(block_id, instruction_id);
    }

    // NOTE: Its DFG data (and any values it defines) remains valid but is no longer
    // reachable via layout traversal.
    pub(crate) fn remove_instruction(&mut self, instruction_id: InstructionId) {
        let block_id = self.layout.instructions[instruction_id]
            .block_id
            .expect("instruction is not in the cfg");

        let old_prev = self.layout.instructions[instruction_id].previous_id;
        let old_next = self.layout.instructions[instruction_id].next_id;

        self.layout.instructions[instruction_id].block_id = None;
        self.layout.instructions[instruction_id].previous_id = None;
        self.layout.instructions[instruction_id].next_id = None;

        match old_prev {
            None => self.layout.blocks[block_id].first_id = old_next,
            Some(old_prev_id) => self.layout.instructions[old_prev_id].next_id = old_next,
        }
        match old_next {
            None => self.layout.blocks[block_id].last_id = old_prev,
            Some(old_next_id) => self.layout.instructions[old_next_id].previous_id = old_prev,
        }
    }

    pub(crate) fn split_block(&mut self, new_block_id: BlockId, split_point_id: InstructionId) {
        assert!(
            !self.contains(new_block_id),
            "cannot split into a block that is already in the cfg"
        );

        let old_block_id = self.layout.instructions[split_point_id]
            .block_id
            .expect("split point instruction is not in the cfg");

        self.layout.blocks.add(
            new_block_id,
            BlockNode {
                previous_id: Some(old_block_id),
                next_id: self.layout.blocks[old_block_id].next_id,
                first_id: Some(split_point_id),
                last_id: self.layout.blocks[old_block_id].last_id,
            },
        );

        let old_next = self.layout.blocks[old_block_id].next_id;
        self.layout.blocks[old_block_id].next_id = Some(new_block_id);
        match old_next {
            None => self.layout.exit_id = Some(new_block_id),
            Some(old_next_id) => self.layout.blocks[old_next_id].previous_id = Some(new_block_id),
        }

        let before_split_id = self.layout.instructions[split_point_id].previous_id;
        self.layout.instructions[split_point_id].previous_id = None;
        self.layout.blocks[old_block_id].last_id = before_split_id;

        match before_split_id {
            None => self.layout.blocks[old_block_id].first_id = None,
            Some(instruction_id) => self.layout.instructions[instruction_id].next_id = None,
        }

        let mut current = Some(split_point_id);
        while let Some(instruction_id) = current {
            self.layout.instructions[instruction_id].block_id = Some(new_block_id);
            current = self.layout.instructions[instruction_id].next_id;
        }
    }

    pub(crate) fn allocate_binary(
        &self,
        operator: BinOp,
        operand_ids: [ValueId; 2],
    ) -> Instruction {
        Instruction::Binary {
            operator,
            operand_ids,
        }
    }

    pub(crate) fn allocate_unary(&self, operator: UnOp, operand_id: ValueId) -> Instruction {
        Instruction::Unary {
            operator,
            operand_id,
        }
    }

    pub(crate) fn allocate_integer_literal(&self, value: u128) -> Instruction {
        Instruction::IntegerLiteral { value }
    }

    pub(crate) fn allocate_boolean_literal(&self, value: bool) -> Instruction {
        Instruction::BooleanLiteral { value }
    }

    pub(crate) fn allocate_unreachable(&self) -> Instruction {
        Instruction::Unreachable
    }

    pub(crate) fn allocate_jump(
        &mut self,
        destination_id: BlockId,
        block_argument_ids: &[ValueId],
    ) -> Instruction {
        Instruction::Jump {
            destination_id,
            block_argument_ids: HandleList::<ValueId>::from(
                &mut self.dfg.suballocator,
                block_argument_ids,
            ),
        }
    }

    pub(crate) fn allocate_return(&mut self, output_ids: &[ValueId]) -> Instruction {
        Instruction::Return {
            output_ids: HandleList::<ValueId>::from(&mut self.dfg.suballocator, output_ids),
        }
    }

    pub(crate) fn allocate_call(
        &mut self,
        callee_id: FunctionReferenceId,
        argument_ids: &[ValueId],
    ) -> Instruction {
        Instruction::Call {
            callee_id,
            argument_ids: HandleList::<ValueId>::from(&mut self.dfg.suballocator, argument_ids),
        }
    }

    pub(crate) fn allocate_conditional_branch(
        &mut self,
        operand_id: ValueId,
        true_block_id: BlockId,
        true_block_argument_ids: &[ValueId],
        false_block_id: BlockId,
        false_block_argument_ids: &[ValueId],
    ) -> Instruction {
        Instruction::ConditionalBranch {
            operand_id,
            true_block_id,
            true_block_argument_ids: HandleList::<ValueId>::from(
                &mut self.dfg.suballocator,
                true_block_argument_ids,
            ),
            false_block_id,
            false_block_argument_ids: HandleList::<ValueId>::from(
                &mut self.dfg.suballocator,
                false_block_argument_ids,
            ),
        }
    }

    pub(crate) fn get_value(&self, value_id: ValueId) -> ValueView<'_> {
        ValueView {
            value_id,
            cfg: self,
        }
    }

    pub(crate) fn block_argument(
        &self,
        block_parameter: ValueId,
        predecessor_edge: InstructionId,
    ) -> ValueId {
        let (block_id, index) = match self.get_value(block_parameter).origin() {
            ValueOrigin::Parameter(block_id, index) => (block_id, index as usize),
            ValueOrigin::InstructionResult(..) | ValueOrigin::Undefined(_) => {
                panic!("value is not a block parameter")
            }
        };
        self.get_instruction(predecessor_edge)
            .block_argument(block_id, index)
    }

    pub(crate) fn add_undefined(&mut self, type_id: TypeId) -> ValueId {
        self.dfg.values.add(Value {
            type_id,
            alias_id: None,
            origin: ValueOrigin::Undefined(type_id),
        })
    }

    // NOTE: Aliases are never attached.
    fn value_is_attached(&self, value_id: ValueId) -> bool {
        if self.dfg.values[value_id].alias_id.is_some() {
            return false;
        }
        match self.dfg.values[value_id].origin {
            ValueOrigin::InstructionResult(instruction_id, index) => {
                self.dfg.instruction_results[instruction_id]
                    .get(&self.dfg.suballocator, index as usize)
                    == Some(value_id)
            }
            ValueOrigin::Parameter(block_id, index) => {
                self.dfg.blocks[block_id]
                    .parameter_ids
                    .get(&self.dfg.suballocator, index as usize)
                    == Some(value_id)
            }
            ValueOrigin::Undefined(_) => false,
        }
    }

    pub(crate) fn resolve_aliases(&self, value_id: ValueId) -> ValueId {
        let mut current_value = value_id;
        for _ in 0..=self.dfg.values.count() {
            match self.dfg.values[current_value].alias_id {
                Some(original) => current_value = original,
                None => return current_value,
            }
        }
        panic!("value alias loop detected");
    }

    pub(crate) fn mark_as_alias(&mut self, destination_id: ValueId, source_id: ValueId) {
        assert!(
            !self.value_is_attached(destination_id),
            "cannot alias a value that is still attached to an instruction or block"
        );

        let original = self.resolve_aliases(source_id);

        assert!(
            destination_id != original,
            "aliasing a value to itself would create a loop"
        );

        assert!(
            self.dfg.values[destination_id].type_id == self.dfg.values[original].type_id,
            "aliasing values of different types"
        );

        self.dfg.values[destination_id].alias_id = Some(original);
    }

    pub(crate) fn flush_aliases(&mut self) {
        // step 1: compresses every alias chain so each aliased value points directly at its final target
        let value_ids: Vec<ValueId> = self.dfg.values.keys().collect();
        for mut value_id in value_ids {
            if let Some(original_pointee_id) = self.dfg.values[value_id].alias_id {
                let resolved = Some(self.resolve_aliases(original_pointee_id));
                let mut next_value_id = original_pointee_id;
                loop {
                    self.dfg.values[value_id].alias_id = resolved;

                    value_id = next_value_id;

                    if let Some(next_pointee_id) = self.dfg.values[value_id].alias_id {
                        next_value_id = next_pointee_id;
                    } else {
                        break;
                    }
                }
            }
        }

        // step 2: propagates that resolution out into every instruction's actual operand references.
        for instruction in self.dfg.instructions.values_mut() {
            instruction.rewrite_operands(&mut self.dfg.suballocator, |value_id| {
                match self.dfg.values[value_id].alias_id {
                    Some(original) => original,
                    None => value_id,
                }
            });
        }
    }

    pub(crate) fn add_signature(&mut self, signature: Signature) -> SignatureId {
        self.dfg.signatures.add(signature)
    }

    pub(crate) fn add_function_reference(
        &mut self,
        definition_binding_id: DefinitionBindingId,
        name: Symbol,
        signature_id: SignatureId,
    ) -> FunctionReferenceId {
        self.dfg.function_references.add(FunctionReference {
            definition_binding_id,
            name,
            signature_id,
        })
    }

    pub(crate) fn get_signature(&self, signature_id: SignatureId) -> &Signature {
        &self.dfg.signatures[signature_id]
    }

    pub(crate) fn get_function_reference(
        &self,
        function_reference_id: FunctionReferenceId,
    ) -> &FunctionReference {
        &self.dfg.function_references[function_reference_id]
    }
}

impl InstructionKind<HandleList<ValueId>> {
    fn rewrite_operands(
        &mut self,
        suballocator: &mut HandleListSubAllocator<ValueId>,
        mut f: impl FnMut(ValueId) -> ValueId,
    ) {
        match self {
            Instruction::Binary { operand_ids, .. } => {
                operand_ids[0] = f(operand_ids[0]);
                operand_ids[1] = f(operand_ids[1]);
            }
            Instruction::Unary { operand_id, .. } => *operand_id = f(*operand_id),
            Instruction::Call { argument_ids, .. } => {
                for v in argument_ids.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
            }
            Instruction::Jump {
                block_argument_ids, ..
            } => {
                for v in block_argument_ids.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
            }
            Instruction::ConditionalBranch {
                operand_id,
                true_block_argument_ids,
                false_block_argument_ids,
                ..
            } => {
                *operand_id = f(*operand_id);
                for v in true_block_argument_ids.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
                for v in false_block_argument_ids.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
            }
            Instruction::Return { output_ids } => {
                for v in output_ids.to_mut_slice(suballocator) {
                    *v = f(*v);
                }
            }
            Instruction::IntegerLiteral { .. }
            | Instruction::BooleanLiteral { .. }
            | Instruction::Unreachable => {}
        }
    }
}

impl<'a> BlockView<'a> {
    pub(crate) fn id(&self) -> BlockId {
        self.block_id
    }

    pub(crate) fn next(&self) -> Option<BlockId> {
        self.cfg.layout.blocks[self.block_id].next_id
    }

    pub(crate) fn prev(&self) -> Option<BlockId> {
        self.cfg.layout.blocks[self.block_id].previous_id
    }

    pub(crate) fn instructions(&self) -> InstructionIter<'a> {
        InstructionIter {
            layout: &self.cfg.layout,
            next_id: self.cfg.layout.blocks[self.block_id].first_id,
        }
    }

    pub(crate) fn first_instruction(&self) -> Option<InstructionId> {
        self.cfg.layout.blocks[self.block_id].first_id
    }

    pub(crate) fn last_instruction(&self) -> Option<InstructionId> {
        self.cfg.layout.blocks[self.block_id].last_id
    }

    pub(crate) fn parameters(&self) -> &'a [ValueId] {
        self.cfg.dfg.blocks[self.block_id]
            .parameter_ids
            .to_slice(&self.cfg.dfg.suballocator)
    }
}

impl<'a> BlockViewMut<'a> {
    pub(crate) fn append_parameter(&mut self, ty: TypeId) -> ValueId {
        let parameter = self.cfg.dfg.values.next_key();
        self.cfg.dfg.blocks[self.block_id]
            .parameter_ids
            .add_last(&mut self.cfg.dfg.suballocator, parameter);
        let count = self.cfg.dfg.blocks[self.block_id]
            .parameter_ids
            .count(&self.cfg.dfg.suballocator);
        assert!(
            count <= u16::MAX as usize,
            "the block has too many parameters"
        );
        self.cfg.dfg.values.add(Value {
            type_id: ty,
            alias_id: None,
            origin: ValueOrigin::Parameter(self.block_id, (count - 1) as u16),
        })
    }

    pub(crate) fn swap_remove_parameter(&mut self, index: usize) {
        let params = self.cfg.dfg.blocks[self.block_id]
            .parameter_ids
            .to_mut_slice(&mut self.cfg.dfg.suballocator);
        params.swap(index, params.len() - 1);
        // The value now sitting at `index` still thinks it's the parameter at its old
        // (last) position, so patch its definition to match its new slot.
        let moved_value = params[index];
        self.cfg.dfg.blocks[self.block_id]
            .parameter_ids
            .clear_last(&mut self.cfg.dfg.suballocator);
        if let ValueOrigin::Parameter(_, num) = &mut self.cfg.dfg.values[moved_value].origin {
            *num = index as u16;
        }
    }

    pub(crate) fn remove_parameter(&mut self, index: usize) {
        self.cfg.dfg.blocks[self.block_id]
            .parameter_ids
            .remove(index, &mut self.cfg.dfg.suballocator);
        let parameters = self.cfg.dfg.blocks[self.block_id].parameter_ids;
        let count = parameters.count(&self.cfg.dfg.suballocator);
        for i in index..count {
            let value_id = parameters.get(&self.cfg.dfg.suballocator, i).unwrap();
            if let ValueOrigin::Parameter(_, num) = &mut self.cfg.dfg.values[value_id].origin {
                *num = i as u16;
            }
        }
    }

    pub(crate) fn detach_parameters(&mut self) -> HandleList<ValueId> {
        let params = self.cfg.dfg.blocks[self.block_id].parameter_ids;
        self.cfg.dfg.blocks[self.block_id].parameter_ids = HandleList::<ValueId>::new();
        params
    }
}

impl<'a> InstructionView<'a> {
    pub(crate) fn id(&self) -> InstructionId {
        self.instruction_id
    }

    pub(crate) fn next(&self) -> Option<InstructionId> {
        self.cfg.layout.instructions[self.instruction_id].next_id
    }

    pub(crate) fn prev(&self) -> Option<InstructionId> {
        self.cfg.layout.instructions[self.instruction_id].previous_id
    }

    pub(crate) fn is_terminator(&self) -> bool {
        matches!(
            self.cfg.dfg.instructions[self.instruction_id],
            Instruction::Jump { .. }
                | Instruction::ConditionalBranch { .. }
                | Instruction::Return { .. }
                | Instruction::Unreachable
        )
    }

    pub(crate) fn inputs(&self) -> &'a [ValueId] {
        let suballocator = &self.cfg.dfg.suballocator;
        match &self.cfg.dfg.instructions[self.instruction_id] {
            Instruction::Binary { operand_ids, .. } => operand_ids,
            Instruction::Unary { operand_id, .. } => slice::from_ref(operand_id),
            Instruction::Call { argument_ids, .. } => argument_ids.to_slice(suballocator),
            Instruction::Jump {
                block_argument_ids, ..
            } => block_argument_ids.to_slice(suballocator),
            Instruction::ConditionalBranch { operand_id, .. } => slice::from_ref(operand_id), // Only the condition is an operand here (the then/else block arguments are not operands in the traditional sense)
            Instruction::Return { output_ids } => output_ids.to_slice(suballocator),
            Instruction::IntegerLiteral { .. }
            | Instruction::BooleanLiteral { .. }
            | Instruction::Unreachable => &[],
        }
    }

    pub(crate) fn results(&self) -> &'a [ValueId] {
        self.cfg.dfg.instruction_results[self.instruction_id].to_slice(&self.cfg.dfg.suballocator)
    }

    pub(crate) fn first_result(&self) -> Option<ValueId> {
        self.cfg.dfg.instruction_results[self.instruction_id].get(&self.cfg.dfg.suballocator, 0)
    }

    pub(crate) fn containing_block(&self) -> Option<BlockId> {
        self.cfg.layout.instructions[self.instruction_id].block_id
    }

    pub(crate) fn used_values(&self) -> UsedValuesIter<'a> {
        match &self.cfg.dfg.instructions[self.instruction_id] {
            Instruction::ConditionalBranch {
                operand_id,
                true_block_argument_ids,
                false_block_argument_ids,
                ..
            } => UsedValuesIter::Branch {
                operand_id: Some(*operand_id),
                then_argument_ids: true_block_argument_ids
                    .to_slice(&self.cfg.dfg.suballocator)
                    .iter(),
                else_argument_ids: false_block_argument_ids
                    .to_slice(&self.cfg.dfg.suballocator)
                    .iter(),
            },
            Instruction::Binary { .. }
            | Instruction::Unary { .. }
            | Instruction::Call { .. }
            | Instruction::Jump { .. }
            | Instruction::Return { .. }
            | Instruction::IntegerLiteral { .. }
            | Instruction::BooleanLiteral { .. }
            | Instruction::Unreachable => UsedValuesIter::Slice(self.inputs().iter()),
        }
    }

    pub(crate) fn block_argument(&self, destination_id: BlockId, index: usize) -> ValueId {
        let suballocator = &self.cfg.dfg.suballocator;
        match &self.cfg.dfg.instructions[self.instruction_id] {
            Instruction::Jump {
                destination_id: jump_destination_id,
                block_argument_ids,
            } => {
                assert_eq!(
                    *jump_destination_id, destination_id,
                    "instruction does not jump to destination"
                );
                block_argument_ids
                    .get(suballocator, index)
                    .expect("branch argument index out of bounds")
            }
            Instruction::ConditionalBranch {
                true_block_id,
                true_block_argument_ids,
                false_block_id,
                false_block_argument_ids,
                ..
            } => {
                if *true_block_id == destination_id {
                    true_block_argument_ids
                        .get(suballocator, index)
                        .expect("branch argument index out of bounds")
                } else if *false_block_id == destination_id {
                    false_block_argument_ids
                        .get(suballocator, index)
                        .expect("branch argument index out of bounds")
                } else {
                    panic!("instruction does not branch to destination")
                }
            }
            _ => panic!("instruction is not a branch"),
        }
    }

    pub(crate) fn as_instruction_ref(&self) -> InstructionRef<'a> {
        let suballocator = &self.cfg.dfg.suballocator;
        match &self.cfg.dfg.instructions[self.instruction_id] {
            Instruction::Binary {
                operator,
                operand_ids,
            } => InstructionRef::Binary {
                operator: *operator,
                operand_ids: [operand_ids[0], operand_ids[1]],
            },
            Instruction::Unary {
                operator,
                operand_id,
            } => InstructionRef::Unary {
                operator: *operator,
                operand_id: *operand_id,
            },
            Instruction::IntegerLiteral { value } => {
                InstructionRef::IntegerLiteral { value: *value }
            }
            Instruction::BooleanLiteral { value } => {
                InstructionRef::BooleanLiteral { value: *value }
            }
            Instruction::Call {
                callee_id,
                argument_ids,
            } => InstructionRef::Call {
                callee_id: *callee_id,
                argument_ids: argument_ids.to_slice(suballocator),
            },
            Instruction::Jump {
                destination_id,
                block_argument_ids,
            } => InstructionRef::Jump {
                destination_id: *destination_id,
                block_argument_ids: block_argument_ids.to_slice(suballocator),
            },
            Instruction::ConditionalBranch {
                operand_id,
                true_block_id,
                true_block_argument_ids,
                false_block_id,
                false_block_argument_ids,
            } => InstructionRef::ConditionalBranch {
                operand_id: *operand_id,
                true_block_id: *true_block_id,
                true_block_argument_ids: true_block_argument_ids.to_slice(suballocator),
                false_block_id: *false_block_id,
                false_block_argument_ids: false_block_argument_ids.to_slice(suballocator),
            },
            Instruction::Return { output_ids } => InstructionRef::Return {
                output_ids: output_ids.to_slice(suballocator),
            },
            Instruction::Unreachable => InstructionRef::Unreachable,
        }
    }
}

impl<'a> InstructionViewMut<'a> {
    pub(crate) fn rewrite_operands(&mut self, f: impl FnMut(ValueId) -> ValueId) {
        self.cfg.dfg.instructions[self.instruction_id]
            .rewrite_operands(&mut self.cfg.dfg.suballocator, f);
    }

    pub(crate) fn append_block_argument(&mut self, destination_id: BlockId, value: ValueId) {
        let suballocator = &mut self.cfg.dfg.suballocator;
        match &mut self.cfg.dfg.instructions[self.instruction_id] {
            Instruction::Jump {
                destination_id: jump_destination_id,
                block_argument_ids,
            } => {
                assert_eq!(
                    *jump_destination_id, destination_id,
                    "instruction does not jump to destination"
                );
                block_argument_ids.add_last(suballocator, value);
            }
            Instruction::ConditionalBranch {
                true_block_id,
                true_block_argument_ids,
                false_block_id,
                false_block_argument_ids,
                ..
            } => {
                let mut branched = false;
                if *true_block_id == destination_id {
                    true_block_argument_ids.add_last(suballocator, value);
                    branched = true;
                }
                if *false_block_id == destination_id {
                    false_block_argument_ids.add_last(suballocator, value);
                    branched = true;
                }
                assert!(branched, "instruction does not branch to destination");
            }
            _ => panic!("instruction is not a branch"),
        }
    }
}

impl<'a> ValueView<'a> {
    pub(crate) fn ty(&self) -> TypeId {
        self.cfg.dfg.values[self.value_id].type_id
    }

    pub(crate) fn origin(&self) -> ValueOrigin {
        self.cfg.dfg.values[self.cfg.resolve_aliases(self.value_id)].origin
    }

    pub(crate) fn alias_target(&self) -> Option<ValueId> {
        self.cfg.dfg.values[self.value_id].alias_id
    }
}

impl Iterator for UsedValuesIter<'_> {
    type Item = ValueId;

    fn next(&mut self) -> Option<ValueId> {
        match self {
            UsedValuesIter::Slice(iter) => iter.next().copied(),
            UsedValuesIter::Branch {
                operand_id,
                then_argument_ids,
                else_argument_ids,
            } => operand_id
                .take()
                .or_else(|| then_argument_ids.next().copied())
                .or_else(|| else_argument_ids.next().copied()),
        }
    }
}

impl DoubleEndedIterator for UsedValuesIter<'_> {
    fn next_back(&mut self) -> Option<ValueId> {
        match self {
            UsedValuesIter::Slice(iter) => iter.next_back().copied(),
            UsedValuesIter::Branch {
                operand_id,
                then_argument_ids,
                else_argument_ids,
            } => else_argument_ids
                .next_back()
                .copied()
                .or_else(|| then_argument_ids.next_back().copied())
                .or_else(|| operand_id.take()),
        }
    }
}

impl Iterator for BlockIter<'_> {
    type Item = BlockId;
    fn next(&mut self) -> Option<BlockId> {
        let block = self.next_id?;
        self.next_id = self.layout.blocks[block].next_id;
        Some(block)
    }
}

impl Iterator for InstructionIter<'_> {
    type Item = InstructionId;
    fn next(&mut self) -> Option<InstructionId> {
        let inst = self.next_id?;
        self.next_id = self.layout.instructions[inst].next_id;
        Some(inst)
    }
}

impl DataFlowGraph {
    fn new() -> Self {
        Self {
            values: HandleMap::new(),
            instructions: HandleMap::new(),
            instruction_results: SideHandleMap::new(),
            blocks: HandleMap::new(),
            function_references: HandleMap::new(),
            signatures: HandleMap::new(),
            suballocator: HandleListSubAllocator::<ValueId>::new(),
        }
    }
}

impl Layout {
    fn new() -> Self {
        Self {
            entry_id: None,
            exit_id: None,
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
        assert_eq!(cfg.entry().unwrap().id(), b);
        assert_eq!(cfg.get_block(a).prev(), Some(b));
        assert_eq!(cfg.get_block(b).next(), Some(a));
        assert_eq!(cfg.get_block(b).prev(), None);
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
        assert_eq!(cfg.entry().unwrap().id(), a);
        assert_eq!(cfg.get_block(a).next(), Some(b));
        assert_eq!(cfg.get_block(b).prev(), Some(a));
        assert_eq!(cfg.get_block(b).next(), Some(c));
        assert_eq!(cfg.get_block(c).prev(), Some(b));
    }

    #[test]
    fn add_instruction_before_first_updates_block_head() {
        let mut cfg = Cfg::new();
        let block = cfg.allocate_block();
        cfg.append_block(block);
        let first = cfg.append_instruction(block, Instruction::Unreachable, &[]);

        let new_first = cfg.add_instruction_before(Instruction::Unreachable, &[], first);

        assert_eq!(cfg.get_block(block).first_instruction(), Some(new_first));
        assert_eq!(cfg.get_instruction(first).prev(), Some(new_first));
        assert_eq!(cfg.get_instruction(new_first).next(), Some(first));
    }

    #[test]
    fn add_instruction_before_middle_updates_middle_predecessor() {
        let mut cfg = Cfg::new();
        let block = cfg.allocate_block();
        cfg.append_block(block);
        let first = cfg.append_instruction(block, Instruction::Unreachable, &[]);
        let third = cfg.append_instruction(block, Instruction::Unreachable, &[]);

        let second = cfg.add_instruction_before(Instruction::Unreachable, &[], third);

        assert_eq!(cfg.get_block(block).first_instruction(), Some(first));
        assert_eq!(cfg.get_instruction(first).next(), Some(second));
        assert_eq!(cfg.get_instruction(second).prev(), Some(first));
        assert_eq!(cfg.get_instruction(second).next(), Some(third));
        assert_eq!(cfg.get_instruction(third).prev(), Some(second));
    }
}
