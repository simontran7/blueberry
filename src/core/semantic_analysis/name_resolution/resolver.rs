use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ast_lowering::nodes::{self, LocalBindingHandle};
use crate::core::semantic_analysis::keys::{BlockKey, ConstantKey, FunctionKey};
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeView;
use crate::core::semantic_analysis::{
    block_definitions_of, imports_of, module_file_of, top_level_definitions_of,
};
use crate::core::source_file_key::SourceFileKey;

pub(crate) struct Resolver<'a, 'db> {
    pub(in crate::core::semantic_analysis::name_resolution) file: SourceFileKey,
    pub(in crate::core::semantic_analysis::name_resolution) scope_view: ScopeView<'a, 'db>,
    pub(in crate::core::semantic_analysis::name_resolution) enclosing_block: Option<BlockKey<'db>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum Resolution<'db> {
    Local(LocalBindingHandle),
    Function(FunctionKey<'db>),
    Constant(ConstantKey<'db>),
    Module(SourceFileKey),
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum ResolveError<'db> {
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
    pub(crate) fn resolve_path(
        &self,
        db: &'db dyn crate::Db,
        path: nodes::Path<'db>,
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
            // `name` is looked up among the top-level definitions of the module that the previous segment named
            resolution = top_level_definitions_of(db, file)
                .find(db, *name)
                .map(Resolution::from)
                .ok_or(ResolveError::MissingFromModule {
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
                .find(|entry| entry.name() == name)
            {
                return Some(Resolution::Local(entry.binding()));
            }

            // check whether `name` is a definition's name in the current block
            if let Some(block_key) = scope_view.block()
                && let Some(definition) = block_definitions_of(db, block_key).find(db, name)
            {
                return Some(definition.into());
            }
        }

        // check whether `name` is a definition in a block enclosing the body's definition,
        // innermost first
        let mut ancestor = self.enclosing_block;
        while let Some(block) = ancestor {
            if let Some(definition) = block_definitions_of(db, block).find(db, name) {
                return Some(definition.into());
            }
            ancestor = block.enclosing_block(db);
        }

        // check whether `name` is a top-level definition in the current file
        if let Some(definition) = top_level_definitions_of(db, self.file).find(db, name) {
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
