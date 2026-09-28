use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::hir::nodes::LocalBindingHandle;
use crate::core::semantic_analysis::ids::keys::{ConstantKey, FunctionKey};
use crate::core::semantic_analysis::name_resolution::definition_tree::Definition;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeView;
use crate::core::semantic_analysis::{
    block_scoped_definitions_of, file_scoped_definitions_of, imports_of, module_file_of,
};
use crate::core::source_file_key::SourceFileKey;

pub(crate) struct Resolver<'a, 'db> {
    pub(crate) file: SourceFileKey,
    pub(crate) scope: ScopeView<'a, 'db>,
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
        for scope in self.scope.chain() {
            // check whether `name` is a local binding
            if let Some(entry) = scope
                .entries()
                .iter()
                .rev()
                .find(|entry| entry.name == name)
            {
                return Some(Resolution::Local(entry.binding_handle));
            }

            // check whether `name` is a definition's name in the current block (if any)
            if let Some(block_key) = scope.block()
                && let Some(definition) = block_scoped_definitions_of(db, block_key).find(db, name)
            {
                return Some(definition.into());
            }
        }

        // check whether `name` is a path
        for import in imports_of(db, self.file).iter() {
            if import.segments(db).last() != Some(&name) {
                continue;
            }
            let Some(target_file) = module_file_of(db, *import) else {
                continue;
            };
            if let Some(definition) = file_scoped_definitions_of(db, *target_file).find(db, name) {
                return Some(definition.into());
            }
        }

        // check whether `name` is a definition's name in the current file
        file_scoped_definitions_of(db, self.file)
            .find(db, name)
            .map(Into::into)
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
