use std::sync::Arc;

use crate::core::{
    semantic_analysis::{
        hir::{ConstantKey, DefinitionSource, FunctionKey},
        red_node_directory::{RedNodeDirectory, RedNodeId},
    },
    syntactic_analysis::{ast, ast::AstNode},
};

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct DefinitionTree<'db> {
    directory: Arc<RedNodeDirectory<'db>>,
    definitions: Vec<Definition<'db>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum Definition<'db> {
    Function(FunctionKey<'db>),
    Constant(ConstantKey<'db>),
}

impl<'db> DefinitionTree<'db> {
    pub(crate) fn new(directory: Arc<RedNodeDirectory<'db>>) -> Self {
        Self {
            directory,
            definitions: Vec::new(),
        }
    }

    pub(crate) fn definitions(&self) -> &[Definition<'db>] {
        &self.definitions
    }

    pub(crate) fn add_definition(
        &mut self,
        db: &'db dyn crate::Db,
        source: DefinitionSource<'db>,
        definition: ast::Definition,
    ) {
        let id = self.directory.id_of(definition.red()).unwrap();
        let key = match definition {
            ast::Definition::FunctionDefinition(_) => {
                Definition::Function(FunctionKey::new(db, source, RedNodeId::new(id)))
            }
            ast::Definition::ConstantDefinition(_) => {
                Definition::Constant(ConstantKey::new(db, source, RedNodeId::new(id)))
            }
        };
        self.definitions.push(key);
    }
}
