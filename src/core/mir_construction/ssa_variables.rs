//! SSA construction, following Braun et al., "Simple and Efficient Construction of Static Single
//! Assignment Form" (2013), with block parameters in place of phi nodes.

use std::collections::HashMap;

use crate::core::common::handle_collections::handle_list::{
    HandleArrayList, HandleArrayListAllocator,
};
use crate::core::common::handle_collections::handle_map::SideHandleMap;
use crate::core::mir_construction::mir::{
    BlockHandle, Cfg, InstructionHandle, ValueHandle, ValueOrigin,
};
use crate::core::semantic_analysis::ast_lowering::nodes::LocalBindingHandle;
use crate::core::semantic_analysis::type_checking::types::Ty;

/// The value each variable has in each block, and what's needed to find it when a block doesn't
/// define it itself.
pub(crate) struct SsaVariables {
    current_defs: HashMap<(LocalBindingHandle, BlockHandle), ValueHandle>,
    incomplete_placeholders: HashMap<BlockHandle, Vec<(LocalBindingHandle, ValueHandle)>>,
    predecessor_edge_suballocator: HandleArrayListAllocator<InstructionHandle>,
    blocks: SideHandleMap<BlockHandle, SsaBlockState>,
}

// Clone: required by `SideHandleMap::add` for resize padding
#[derive(Clone, Default)]
struct SsaBlockState {
    predecessors: HandleArrayList<InstructionHandle>,
    sealed: bool,
}

impl SsaVariables {
    pub(crate) fn new() -> Self {
        Self {
            current_defs: HashMap::new(),
            incomplete_placeholders: HashMap::new(),
            predecessor_edge_suballocator: HandleArrayListAllocator::new(),
            blocks: SideHandleMap::new(),
        }
    }

    pub(crate) fn declare_block(&mut self, block_handle: BlockHandle) {
        self.blocks.add(block_handle, SsaBlockState::default());
    }

    /// Records that `instruction_handle`, a jump or branch, leads into `block_handle`.
    pub(crate) fn declare_predecessor(
        &mut self,
        block_handle: BlockHandle,
        instruction_handle: InstructionHandle,
    ) {
        assert!(
            !self.blocks[block_handle].sealed,
            "cannot add a predecessor to an already-sealed block"
        );
        self.blocks[block_handle]
            .predecessors
            .add_last(&mut self.predecessor_edge_suballocator, instruction_handle);
    }

    /// Declares that `block_handle` has no more predecessors, and completes the placeholders
    /// created while it might still have had more.
    pub(crate) fn seal_block<'db>(&mut self, cfg: &mut Cfg<'db>, block_handle: BlockHandle) {
        assert!(
            !self.blocks[block_handle].sealed,
            "block {block_handle:?} is already sealed"
        );

        if let Some(placeholders) = self.incomplete_placeholders.remove(&block_handle) {
            for (variable_handle, block_parameter) in placeholders {
                let ty = cfg.value(block_parameter).ty();
                let block_args =
                    self.collect_block_arguments(cfg, variable_handle, ty, block_handle);
                Self::finalize_block_parameter(cfg, block_parameter, ty, block_handle, &block_args);
            }
        }

        self.blocks[block_handle].sealed = true;
    }

    pub(crate) fn write_variable(
        &mut self,
        variable_handle: LocalBindingHandle,
        block_handle: BlockHandle,
        value_handle: ValueHandle,
    ) {
        self.current_defs
            .insert((variable_handle, block_handle), value_handle);
    }

    pub(crate) fn read_variable<'db>(
        &mut self,
        cfg: &mut Cfg<'db>,
        variable: LocalBindingHandle,
        ty: Ty<'db>,
        block: BlockHandle,
    ) -> ValueHandle {
        if let Some(&value_handle) = self.current_defs.get(&(variable, block)) {
            return value_handle;
        }
        self.read_variable_recursive(cfg, variable, ty, block)
    }

    fn read_variable_recursive<'db>(
        &mut self,
        cfg: &mut Cfg<'db>,
        variable_handle: LocalBindingHandle,
        ty: Ty<'db>,
        block_handle: BlockHandle,
    ) -> ValueHandle {
        let value_handle = if !self.blocks[block_handle].sealed {
            // case 1: incomplete CFG
            let block_parameter = cfg.block_mut(block_handle).append_parameter(ty);
            self.incomplete_placeholders
                .entry(block_handle)
                .or_default()
                .push((variable_handle, block_parameter));
            block_parameter
        } else if self.blocks[block_handle]
            .predecessors
            .count(&self.predecessor_edge_suballocator)
            == 1
        {
            // case 2: common case of one predecessor (no block parameter needed)
            let instruction_handle = self.blocks[block_handle]
                .predecessors
                .get(&self.predecessor_edge_suballocator, 0)
                .unwrap();
            let predecessor_handle = cfg
                .instruction(instruction_handle)
                .containing_block()
                .unwrap();
            self.read_variable(cfg, variable_handle, ty, predecessor_handle)
        } else {
            // case 3: general case with predecessors
            let block_parameter = cfg.block_mut(block_handle).append_parameter(ty);
            self.write_variable(variable_handle, block_handle, block_parameter);
            let block_args = self.collect_block_arguments(cfg, variable_handle, ty, block_handle);
            Self::finalize_block_parameter(cfg, block_parameter, ty, block_handle, &block_args)
        };
        self.write_variable(variable_handle, block_handle, value_handle);
        value_handle
    }

    fn collect_block_arguments<'db>(
        &mut self,
        cfg: &mut Cfg<'db>,
        variable: LocalBindingHandle,
        ty: Ty<'db>,
        block_handle: BlockHandle,
    ) -> Vec<(InstructionHandle, ValueHandle)> {
        let mut block_args = Vec::new();
        let predecessor_edges = self.blocks[block_handle]
            .predecessors
            .to_slice(&self.predecessor_edge_suballocator)
            .to_vec();

        for instruction_handle in predecessor_edges {
            let predecessor = cfg
                .instruction(instruction_handle)
                .containing_block()
                .unwrap();
            let value = self.read_variable(cfg, variable, ty, predecessor);
            block_args.push((instruction_handle, value));
        }

        block_args
    }

    fn finalize_block_parameter<'db>(
        cfg: &mut Cfg<'db>,
        block_parameter: ValueHandle,
        ty: Ty<'db>,
        block_handle: BlockHandle,
        block_args: &[(InstructionHandle, ValueHandle)],
    ) -> ValueHandle {
        let mut common_value: Option<ValueHandle> = None;

        for &(_, block_arg) in block_args {
            let resolved_block_arg = cfg.resolve_aliases(block_arg);

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
                    cfg.instruction_mut(instruction_handle)
                        .append_block_argument(block_handle, value);
                }
                return block_parameter;
            }
        }

        // trivial: converges to a common value, or to a fresh undefined if there wasn't one
        let replacement_value = common_value.unwrap_or_else(|| cfg.add_undefined(ty));

        // remove the block parameter
        let index = match cfg.value(block_parameter).origin() {
            ValueOrigin::Parameter(_, index) => index as usize,
            ValueOrigin::InstructionResult(..) | ValueOrigin::Undefined(_) => {
                unreachable!("`block_parameter` must originate from a block parameter slot")
            }
        };
        cfg.block_mut(block_handle).remove_parameter(index);

        // alias to replacement_value; stale references are fixed up later by `flush_aliases`, not now
        cfg.mark_as_alias(block_parameter, replacement_value);

        replacement_value
    }
}
