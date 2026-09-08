use std::sync::Arc;

use crate::core::{
    semantic_analysis::red_node_directory::{RawRedNodeId, RedNodeDirectory},
    syntactic_analysis::{ast, cst::RedNode},
};

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct DefinitionTree<'db> {
    directory: Arc<RedNodeDirectory<'db>>,
    definitions: Vec<RawDefinition>,
    ids: Vec<RawRedNodeId<'db>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawDefinition {
    Function,
    Constant,
}

impl<'db> DefinitionTree<'db> {
    pub(crate) fn new(directory: Arc<RedNodeDirectory<'db>>) -> Self {
        Self {
            directory,
            definitions: Vec::new(),
            ids: Vec::new(),
        }
    }

    pub(crate) fn add_definition(&mut self, definition: ast::Definition) {
        let raw_definition = match definition {
            ast::Definition::FunctionDefinition(_) => RawDefinition::Function,
            ast::Definition::ConstantDefinition(_) => RawDefinition::Constant,
        };
        self.definitions.push(raw_definition);
    }

    pub(crate) fn add_id(&mut self, node: &RedNode) {
        self.ids.push(self.directory.id_of(node).unwrap());
    }
}
