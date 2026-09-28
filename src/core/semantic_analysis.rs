use std::sync::Arc;

use crate::core::common::symbol::Symbol;
use crate::core::module_map::{ModuleMap, ModulePath};
use crate::core::semantic_analysis::ast_lowering::definition_body_lowerer::DefinitionBodyLowerer;
use crate::core::semantic_analysis::hir::nodes::{
    ConstantSignature, DefinitionBody, DefinitionBodySourceMap, FunctionSignature, Path,
    TypeAnnotation,
};
use crate::core::semantic_analysis::ids::keys::{BlockKey, ConstantKey, DefinitionSource, FunctionKey};
use crate::core::semantic_analysis::name_resolution::definition_tree::DefinitionTree;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeTree;
use crate::core::semantic_analysis::ids::red_node_directory::{
    RedNodeDirectory, RedNodeDirectoryBuilder,
};
use crate::core::semantic_analysis::type_checking::type_checker::{TypeCheckResult, TypeChecker};
use crate::core::semantic_analysis::type_checking::types::Ty;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode, File};
use crate::core::syntactic_analysis::cst::RedNode;
use crate::core::syntactic_analysis::cst_of;

pub(crate) mod ast_lowering;
pub(crate) mod hir;
pub(crate) mod ids;
pub(crate) mod name_resolution;
pub(crate) mod old_sema;
pub(crate) mod type_checking;

#[salsa::tracked]
pub(crate) fn file_scoped_definitions_of<'db>(
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
        root.items().filter_map(|item| match item {
            ast::Item::Definition(definition) => Some(definition),
            ast::Item::Import(_) => None,
        }),
    )
}

#[salsa::tracked]
pub(crate) fn block_scoped_definitions_of<'db>(
    db: &'db dyn crate::Db,
    block: BlockKey<'db>,
) -> Arc<DefinitionTree<'db>> {
    let source = DefinitionSource::Block(block);

    let file = *block.id(db).file(db);
    let directory = red_node_directory_of(db, file).clone();

    let ast_node = block.id(db).to_ast_node(db);

    collect_definitions(db, source, directory, ast_node.definitions())
}

#[salsa::tracked]
pub(crate) fn imports_of<'db>(db: &'db dyn crate::Db, file: SourceFileKey) -> Arc<Vec<Path<'db>>> {
    let root = RedNode::new(cst_of(db, file).clone());
    let root = File::cast(root).unwrap();

    let imports = root
        .items()
        .filter_map(|item| match item {
            ast::Item::Import(import_declaration) => import_declaration.path(),
            ast::Item::Definition(_) => None,
        })
        .filter_map(|path| ast_lowering::lower_path(db, &path))
        .collect();

    Arc::new(imports)
}

#[salsa::tracked]
pub(crate) fn module_file_of<'db>(
    db: &'db dyn crate::Db,
    path: hir::nodes::Path<'db>,
) -> Option<SourceFileKey> {
    let modules = ModuleMap::try_get(db)?;
    let path = ModulePath::from_path(db, path);
    modules.files(db).get(&path).copied()
}

#[salsa::tracked]
pub(crate) fn red_node_directory_of<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
) -> Arc<RedNodeDirectory<'db>> {
    let root = RedNode::new(cst_of(db, file).clone());
    Arc::new(RedNodeDirectoryBuilder::new(db, file, &root).build())
}

#[salsa::tracked]
pub(crate) fn function_signature_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> FunctionSignature<'db> {
    let node = key.id(db).to_ast_node(db);

    let name = node
        .name()
        .map(|token| Symbol::new(db, token.lexeme().to_string()))
        .expect("nameless definitions are skipped in `collect_definitions`");

    let parameters = match node.parameter_list() {
        Some(list) => list
            .parameters()
            .map(|parameter| {
                parameter
                    .type_expression()
                    .map_or(TypeAnnotation::Error, |type_expression| {
                        TypeAnnotation::from_type_expression(db, &type_expression)
                    })
            })
            .collect(),
        None => Vec::new(),
    };

    let return_type_annotation = node
        .return_type()
        .map(|ty| TypeAnnotation::from_type_expression(db, &ty));

    FunctionSignature {
        name,
        parameters,
        return_type_annotation,
    }
}

#[salsa::tracked]
pub(crate) fn constant_signature_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> ConstantSignature<'db> {
    let node = key.id(db).to_ast_node(db);

    let name = node
        .name()
        .map(|token| Symbol::new(db, token.lexeme().to_string()))
        .expect("nameless definitions are skipped in `collect_definitions`");

    let type_annotation = node
        .type_annotation()
        .map(|ty| TypeAnnotation::from_type_expression(db, &ty));

    ConstantSignature {
        name,
        type_annotation,
    }
}

#[salsa::tracked]
pub(crate) fn function_signature_type_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> Ty<'db> {
    let signature = function_signature_of(db, key);

    let parameters = signature
        .parameters
        .iter()
        .map(|parameter| parameter.to_ty(db))
        .collect();

    let return_type = signature
        .return_type_annotation
        .as_ref()
        .map_or_else(|| Ty::unit(db), |annotation| annotation.to_ty(db));

    Ty::function(db, parameters, return_type)
}

#[salsa::tracked]
pub(crate) fn constant_signature_type_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> Ty<'db> {
    constant_signature_of(db, key)
        .type_annotation
        .as_ref()
        .map_or_else(
            || Ty::error(db),
            |type_annotation| type_annotation.to_ty(db),
        )
}

#[salsa::tracked]
pub(crate) fn function_scopes_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> Arc<ScopeTree<'db>> {
    Arc::new(ScopeTree::new(&function_body_of(db, key)))
}

#[salsa::tracked]
pub(crate) fn constant_scopes_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> Arc<ScopeTree<'db>> {
    Arc::new(ScopeTree::new(&constant_body_of(db, key)))
}

#[salsa::tracked]
pub(crate) fn function_types_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> Arc<TypeCheckResult<'db>> {
    let body = function_body_of(db, key);
    let checker = TypeChecker::new(db, &body);
    Arc::new(checker.finish())
}

#[salsa::tracked]
pub(crate) fn constant_types_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> Arc<TypeCheckResult<'db>> {
    todo!()
}

#[salsa::tracked]
fn function_body_with_source_map_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> (Arc<DefinitionBody<'db>>, Arc<DefinitionBodySourceMap>) {
    let node = key.id(db).to_ast_node(db);
    let file = *key.id(db).file(db);
    let lowerer = DefinitionBodyLowerer::new(db, red_node_directory_of(db, file).clone());
    let (body, source_map) = lowerer.lower_function(&node);
    (Arc::new(body), Arc::new(source_map))
}

#[salsa::tracked]
pub(crate) fn function_body_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> Arc<DefinitionBody<'db>> {
    function_body_with_source_map_of(db, key).0.clone()
}

fn function_source_map_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> Arc<DefinitionBodySourceMap> {
    function_body_with_source_map_of(db, key).1.clone()
}

#[salsa::tracked]
fn constant_body_with_source_map_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> (Arc<DefinitionBody<'db>>, Arc<DefinitionBodySourceMap>) {
    let node = key.id(db).to_ast_node(db);
    let file = *key.id(db).file(db);
    let lowerer = DefinitionBodyLowerer::new(db, red_node_directory_of(db, file).clone());
    let (body, source_map) = lowerer.lower_constant(&node);
    (Arc::new(body), Arc::new(source_map))
}

#[salsa::tracked]
pub(crate) fn constant_body_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> Arc<DefinitionBody<'db>> {
    constant_body_with_source_map_of(db, key).0.clone()
}

fn constant_source_map_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> Arc<DefinitionBodySourceMap> {
    constant_body_with_source_map_of(db, key).1.clone()
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
