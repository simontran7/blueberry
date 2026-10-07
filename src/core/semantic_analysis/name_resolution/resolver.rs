use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::hir::nodes::LocalBindingHandle;
use crate::core::semantic_analysis::ids::keys::{BlockKey, ConstantKey, FunctionKey};
use crate::core::semantic_analysis::ids::red_node_id::RedNodeId;
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeView;
use crate::core::semantic_analysis::{
    block_scoped_definitions_of, file_scoped_definitions_of, imports_of, module_file_of,
};
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::SyntaxKind;

pub(crate) struct Resolver<'a, 'db> {
    pub(crate) file: SourceFileKey,
    pub(crate) scope_view: ScopeView<'a, 'db>,
    /// The block the body's definition is nested in, if any. Its definitions, and those of the
    /// blocks around it, are visible from the body.
    pub(crate) enclosing_block: Option<BlockKey<'db>>,
}

pub(crate) enum Resolution<'db> {
    Local(LocalBindingHandle),
    Function(FunctionKey<'db>),
    Constant(ConstantKey<'db>),
}

impl<'a, 'db> Resolver<'a, 'db> {
    pub(crate) fn resolve(
        &self,
        db: &'db dyn crate::Db,
        name: Symbol<'db>,
    ) -> Option<Resolution<'db>> {
        for scope_view in self.scope_view.chain() {
            // check whether `name` is a local binding
            if let Some(entry) = scope_view
                .entries()
                .iter()
                .rev()
                .find(|entry| entry.name == name)
            {
                return Some(Resolution::Local(entry.binding_handle));
            }

            // check whether `name` is a definition's name in the current block
            if let Some(block_key) = scope_view.block()
                && let Some(definition) = block_scoped_definitions_of(db, block_key).find(db, name)
            {
                return Some(definition.into());
            }
        }

        // check whether `name` is a definition in a block enclosing the body's definition,
        // innermost first
        let mut ancestor = self.enclosing_block.map(|block| block.id(db).id);
        while let Some(id) = ancestor {
            if *id.kind(db) == SyntaxKind::Block
                && let Some(definition) =
                    block_scoped_definitions_of(db, BlockKey::new(db, RedNodeId::new(id)))
                        .find(db, name)
            {
                return Some(definition.into());
            }
            ancestor = *id.parent(db);
        }

        // check whether `name` is a top-level definition in the current file
        if let Some(definition) = file_scoped_definitions_of(db, self.file).find(db, name) {
            return Some(definition.into());
        }

        // check whether `name` is imported, and that the imported file actually defines it
        for path in imports_of(db, self.file).matching(db, name) {
            if let Some(target_file) = module_file_of(db, path)
                && let Some(definition) =
                    file_scoped_definitions_of(db, *target_file).find(db, name)
            {
                return Some(definition.into());
            }
        }

        None
    }
}

impl<'db> From<Definition<'db>> for Resolution<'db> {
    fn from(definition: Definition<'db>) -> Self {
        match definition {
            Definition::Function(key) => Self::Function(key),
            Definition::Constant(key) => Self::Constant(key),
        }
    }
}
