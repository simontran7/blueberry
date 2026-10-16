use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::hir::nodes::{LocalBindingHandle, Path};
use crate::core::semantic_analysis::ids::keys::{BlockKey, ConstantKey, FunctionKey};
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeView;
use crate::core::semantic_analysis::{
    block_scoped_definitions_of, file_scoped_definitions_of, imports_of, module_file_of,
};
use crate::core::source_file_key::SourceFileKey;

pub(in crate::core) struct Resolver<'a, 'db> {
    pub(in crate::core) file: SourceFileKey,
    pub(in crate::core) scope_view: ScopeView<'a, 'db>,
    pub(in crate::core) enclosing_block: Option<BlockKey<'db>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) enum Resolution<'db> {
    Local(LocalBindingHandle),
    Function(FunctionKey<'db>),
    Constant(ConstantKey<'db>),
    Module(SourceFileKey),
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) enum ResolveError<'db> {
    Unresolved {
        segment_index: usize,
    },
    MissingFromModule {
        segment_index: usize,
    },
    NotAModule {
        segment_index: usize,
        found: Resolution<'db>,
    },
}

impl<'a, 'db> Resolver<'a, 'db> {
    pub(in crate::core) fn resolve_path(
        &self,
        db: &'db dyn crate::Db,
        path: Path<'db>,
    ) -> Result<Resolution<'db>, ResolveError<'db>> {
        let (first, rest) = path.segments(db).split_first().unwrap();
        let mut resolution = self
            .resolve_name(db, *first)
            .ok_or(ResolveError::Unresolved { segment_index: 0 })?;
        for (index, name) in rest.iter().enumerate() {
            let file = match resolution {
                Resolution::Module(file) => file,
                Resolution::Error => return Ok(Resolution::Error),
                _ => {
                    return Err(ResolveError::NotAModule {
                        segment_index: index,
                        found: resolution,
                    });
                }
            };
            resolution =
                resolve_in_module(db, file, *name).ok_or(ResolveError::MissingFromModule {
                    segment_index: index + 1,
                })?;
        }
        Ok(resolution)
    }

    fn resolve_name(&self, db: &'db dyn crate::Db, name: Symbol<'db>) -> Option<Resolution<'db>> {
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
        let mut ancestor = self.enclosing_block;
        while let Some(block) = ancestor {
            if let Some(definition) = block_scoped_definitions_of(db, block).find(db, name) {
                return Some(definition.into());
            }
            ancestor = block.enclosing_block(db);
        }

        // check whether `name` is a top-level definition in the current file
        if let Some(definition) = file_scoped_definitions_of(db, self.file).find(db, name) {
            return Some(definition.into());
        }

        // check whether `name` is an imported module
        let imports: Vec<_> = imports_of(db, self.file).matching(db, name).collect();
        if imports.is_empty() {
            return None;
        }
        Some(
            imports
                .into_iter()
                .find_map(|path| *module_file_of(db, path))
                .map_or(Resolution::Error, Resolution::Module),
        )
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

fn resolve_in_module<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
    name: Symbol<'db>,
) -> Option<Resolution<'db>> {
    file_scoped_definitions_of(db, file)
        .find(db, name)
        .map(Resolution::from)
}
