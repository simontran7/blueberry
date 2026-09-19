use std::sync::Arc;

use crate::core::semantic_analysis::definition_tree::DefinitionTree;
use crate::core::semantic_analysis::hir::{
    BlockKey, ConstantKey, ConstantSignature, DefinitionBody, DefinitionSource, FunctionKey,
    FunctionSignature,
};
use crate::core::semantic_analysis::red_node_directory::{
    RedNodeDirectory, RedNodeDirectoryBuilder,
};
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::ast::{self, AstNode, File};
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
pub(crate) fn top_level_definitions_of<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
) -> Arc<DefinitionTree<'db>> {
    let root = RedNode::new(cst_of(db, file).clone());
    let root = File::cast(root).unwrap();
    let directory = red_node_directory_of(db, file).clone();
    collect_definitions(
        db,
        DefinitionSource::File(file),
        directory,
        root.definitions(),
    )
}

#[salsa::tracked]
pub(crate) fn block_definitions_of<'db>(
    db: &'db dyn crate::Db,
    block: BlockKey<'db>,
) -> Arc<DefinitionTree<'db>> {
    let source = DefinitionSource::Block(block);

    let file = *block.id(db).file(db);
    let directory = red_node_directory_of(db, file).clone();

    let ast_node = block.id(db).to_ast_node(db);

    collect_definitions(db, source, directory, ast_node.definitions())
}

fn collect_definitions<'db>(
    db: &'db dyn crate::Db,
    source: DefinitionSource<'db>,
    directory: Arc<RedNodeDirectory<'db>>,
    definitions: impl Iterator<Item = ast::Definition>,
) -> Arc<DefinitionTree<'db>> {
    let mut definition_tree = DefinitionTree::new(directory);
    for definition in definitions {
        if definition.name().is_none() {
            continue;
        }
        definition_tree.add_definition(db, source, definition);
    }
    Arc::new(definition_tree)
}

fn function_signature_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> FunctionSignature<'db> {
    todo!()
}

fn constant_signature_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> ConstantSignature<'db> {
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
