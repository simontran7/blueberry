use std::sync::Arc;

use crate::core::semantic_analysis::definition_tree::{DefinitionTree, RawDefinition};
use crate::core::semantic_analysis::hir::{
    ConstantKey, ConstantSignature, DefinitionBody, FunctionKey, FunctionSignature,
};
use crate::core::semantic_analysis::red_node_directory::{RedNodeDirectory, RedNodeDirectoryBuilder};
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::ast::{self, AstNode};
use crate::core::syntactic_analysis::cst::RedNode;
use crate::core::syntactic_analysis::cst_of;

pub(crate) mod constraints;
pub(crate) mod definition_tree;
pub(crate) mod hir;
pub(crate) mod hir_dumper;
pub(crate) mod red_node_directory;
pub(crate) mod semantic_analyzer;
pub(crate) mod semantic_diagnostic;
pub(crate) mod symbol_table;
pub(crate) mod unification_table;

#[salsa::tracked]
pub(crate) fn definitions_of<'db>(db: &'db dyn crate::Db, file: SourceFileKey) -> Arc<DefinitionTree<'db>> {
    let root = RedNode::new(cst_of(db, file).clone());
    let root = ast::File::cast(root).unwrap();
    let directory = red_node_directory_of(db, file).clone();
    let mut definition_tree = DefinitionTree::new(directory);
 
    for definition in root.definitions() {
        // error recovery: a definition missing its name (e.g. `func () {}`) still parses, but has nothing to bind.
        let has_name = match &definition {
            ast::Definition::FunctionDefinition(f) => f.name().is_some(),
            ast::Definition::ConstantDefinition(c) => c.name().is_some(),
        };
        if !has_name {
            continue; 
        }

        definition_tree.add_id(definition.red());
        definition_tree.add_definition(definition);
    }

    Arc::new(definition_tree)
}

fn function_signature_of<'db>(db: &'db dyn crate::Db, key: FunctionKey<'db>) -> FunctionSignature<'db> {
    todo!()
}

fn constant_signature_of<'db>(db: &'db dyn crate::Db, key: ConstantKey<'db>) -> ConstantSignature<'db> {
    todo!()
}

fn function_body_of<'db>(db: &'db dyn crate::Db, key: FunctionKey<'db>) -> DefinitionBody<'db> {
    todo!()
}

fn constant_body_of<'db>(db: &'db dyn crate::Db, key: ConstantKey<'db>) -> DefinitionBody<'db> {
    todo!()
}

#[salsa::tracked]
pub(crate) fn red_node_directory_of<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
) -> Arc<RedNodeDirectory<'db>> {
    let root = RedNode::new(cst_of(db, file).clone());
    Arc::new(RedNodeDirectoryBuilder::new(db, file, &root).build())
}
