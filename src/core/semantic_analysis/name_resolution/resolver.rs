use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::hir::nodes::{ConstantKey, FunctionKey, LocalBindingHandle};
use crate::core::semantic_analysis::name_resolution::definition_tree::Definition;
use crate::core::semantic_analysis::name_resolution::expression_scopes::ScopeView;
use crate::core::semantic_analysis::{
    block_scoped_definitions_of, file_scoped_definitions_of, imports_of, module_file_of,
};
use crate::core::source_file_key::SourceFileKey;

pub(crate) enum Resolution<'db> {
    Local(LocalBindingHandle),
    Function(FunctionKey<'db>),
    Constant(ConstantKey<'db>),
}

pub(crate) fn resolve<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
    scope: ScopeView<'_, 'db>,
    name: Symbol<'db>,
) -> Option<Resolution<'db>> {
    for scope in scope.chain() {
        if let Some(entry) = scope
            .entries()
            .iter()
            .rev()
            .find(|entry| entry.name == name)
        {
            return Some(Resolution::Local(entry.binding_handle));
        }
        if let Some(block_key) = scope.block()
            && let Some(definition) = block_scoped_definitions_of(db, block_key).find(db, name)
        {
            return Some(definition.into());
        }
    }

    for import in imports_of(db, file).iter() {
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

    file_scoped_definitions_of(db, file)
        .find(db, name)
        .map(Into::into)
}

impl<'db> From<Definition<'db>> for Resolution<'db> {
    fn from(definition: Definition<'db>) -> Self {
        match definition {
            Definition::Function(key) => Self::Function(key),
            Definition::Constant(key) => Self::Constant(key),
        }
    }
}
