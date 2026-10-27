pub(in crate::core) mod ast_lowering;
pub(in crate::core) mod keys;
pub(in crate::core) mod name_resolution;
pub(in crate::core) mod semantic_diagnostic;
pub(in crate::core) mod type_checking;

use std::sync::Arc;

use crate::core::common::span::TextSize;
use crate::core::module_map::{ModuleMap, ModulePath};
use crate::core::semantic_analysis::ast_lowering::body_lowerer::BodyLowerer;
use crate::core::semantic_analysis::ast_lowering::{
    lower_constant_signature, lower_function_signature,
};

use crate::core::semantic_analysis::ast_lowering::nodes::{
    Body, Expression, HirNodeSourceMap, HirNodeStore, Signature, TypeExpression,
    TypeExpressionHandle,
};

use crate::core::semantic_analysis::keys::{BlockKey, DefinitionSource};

use crate::core::semantic_analysis::name_resolution::definition_list::{
    Definition, DefinitionList,
};

use crate::core::semantic_analysis::name_resolution::import_list::ImportList;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeTree;
use crate::core::semantic_analysis::semantic_diagnostic::{OwnerSpans, SemanticDiagnostic};
use crate::core::semantic_analysis::type_checking::type_checker::{
    TypeCheckResult, check_constant, check_function,
};
use crate::core::semantic_analysis::type_checking::type_expression_lowering::lower_type_expression;
use crate::core::semantic_analysis::type_checking::types::Ty;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::RedNode;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode, File};
use crate::core::syntactic_analysis::cst_of;

#[salsa::tracked]
pub(in crate::core) fn types_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Arc<TypeCheckResult<'db>> {
    let body = body_of(db, definition);
    let scopes = scopes_of(db, definition);
    let signature_type = *signature_type_of(db, definition);
    Arc::new(match definition {
        Definition::Function(_) => check_function(db, body, scopes, signature_type),
        Definition::Constant(_) => check_constant(db, body, scopes, signature_type),
    })
}

#[salsa::tracked]
pub(in crate::core) fn body_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Arc<Body<'db>> {
    body_with_source_map_of(db, definition).0.clone()
}

pub(in crate::core) fn body_source_map_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Arc<HirNodeSourceMap> {
    body_with_source_map_of(db, definition).1.clone()
}

#[salsa::tracked]
pub(in crate::core) fn scopes_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Arc<ScopeTree<'db>> {
    Arc::new(ScopeTree::new(
        body_of(db, definition),
        definition.file(db),
        definition.source(db).block(),
    ))
}

#[salsa::tracked]
pub(in crate::core) fn signature_type_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Ty<'db> {
    let signature = signature_of(db, definition);
    let lower = |annotation_handle| {
        lower_type_expression(db, signature.store(), annotation_handle, &mut |_| {})
    };
    match &**signature {
        Signature::Function(function) => {
            let parameters = function
                .parameter_annotation_handles
                .iter()
                .map(|&annotation_handle| lower(annotation_handle))
                .collect();
            let return_type = function
                .return_type_annotation
                .map_or_else(|| Ty::unit(db), lower);
            Ty::function(db, parameters, return_type)
        }
        Signature::Constant(constant) => constant.annotation.map_or_else(|| Ty::error(db), lower),
    }
}

#[salsa::tracked]
pub(in crate::core) fn signature_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Arc<Signature<'db>> {
    signature_with_source_map_of(db, definition).0.clone()
}

pub(in crate::core) fn signature_source_map_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Arc<HirNodeSourceMap> {
    signature_with_source_map_of(db, definition).1.clone()
}

#[salsa::tracked]
pub(in crate::core) fn file_scoped_definitions_of<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
) -> Arc<DefinitionList<'db>> {
    let root = RedNode::new(cst_of(db, file).clone());
    let root = File::cast(root).unwrap();
    collect_definitions(
        db,
        file,
        DefinitionSource::File(file),
        root.items().filter_map(|item| match item {
            ast::Item::Definition(definition) => Some(definition),
            ast::Item::Import(_) => None,
        }),
    )
}

#[salsa::tracked]
pub(in crate::core) fn block_scoped_definitions_of<'db>(
    db: &'db dyn crate::Db,
    block: BlockKey<'db>,
) -> Arc<DefinitionList<'db>> {
    collect_definitions(
        db,
        block.file(db),
        DefinitionSource::Block(block),
        block.to_ast_node(db).definitions(),
    )
}

#[salsa::tracked]
pub(in crate::core) fn imports_of<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
) -> Arc<ImportList<'db>> {
    let root = RedNode::new(cst_of(db, file).clone());
    let root = File::cast(root).unwrap();

    let paths = root
        .items()
        .filter_map(|item| match item {
            ast::Item::Import(import_declaration) => import_declaration.path(),
            ast::Item::Definition(_) => None,
        })
        .filter_map(|path| ast_lowering::lower_path(db, &path))
        .collect();

    Arc::new(ImportList::new(paths))
}

#[salsa::tracked]
pub(in crate::core) fn module_file_of<'db>(
    db: &'db dyn crate::Db,
    path: ast_lowering::nodes::Path<'db>,
) -> Option<SourceFileKey> {
    let modules = ModuleMap::try_get(db)?;
    let path = ModulePath::from_path(db, path);
    modules.files(db).get(&path).copied()
}

#[salsa::tracked]
pub(in crate::core) fn definition_diagnostics_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Arc<[SemanticDiagnostic]> {
    let file = definition.file(db);
    let signature = signature_of(db, definition);
    let anchor = definition.anchor(db);
    let mut diagnostics = unknown_type_diagnostics(
        db,
        signature.store(),
        &signature_source_map_of(db, definition),
        anchor,
        signature.annotation_handles(),
    );

    let owner = match definition {
        Definition::Function(key) => {
            let node = key.to_ast_node(db);
            OwnerSpans {
                file,
                anchor,
                name: node.name().map(|name| name.span()),
                return_type: node
                    .return_type_annotation()
                    .map(|annotation| annotation.red().span()),
            }
        }
        Definition::Constant(key) => OwnerSpans {
            file,
            anchor,
            name: key.to_ast_node(db).name().map(|name| name.span()),
            return_type: None,
        },
    };
    let body = body_of(db, definition);
    let source_map = body_source_map_of(db, definition);
    let types = types_of(db, definition);
    diagnostics.extend(
        types
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.resolve(db, &body, &source_map, &types, &owner)),
    );
    diagnostics.into()
}

pub(in crate::core) fn import_diagnostics_of(
    db: &dyn crate::Db,
    file: SourceFileKey,
) -> Vec<SemanticDiagnostic> {
    let root = RedNode::new(cst_of(db, file).clone());
    let root = File::cast(root).unwrap();
    root.items()
        .filter_map(|item| match item {
            ast::Item::Import(import_declaration) => import_declaration.path(),
            ast::Item::Definition(_) => None,
        })
        .filter_map(|path| {
            let lowered = ast_lowering::lower_path(db, &path)?;
            module_file_of(db, lowered)
                .is_none()
                .then(|| SemanticDiagnostic::UnresolvedImport {
                    path: lowered
                        .segments(db)
                        .iter()
                        .map(|segment| segment.text(db).to_string())
                        .collect(),
                    span: path.red().span(),
                })
        })
        .collect()
}

pub(in crate::core) fn definition_at<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
    offset: TextSize,
) -> Option<Definition<'db>> {
    all_definitions_of(db, file)
        .into_iter()
        .filter(|definition| definition.span(db).contains_offset(offset))
        .min_by_key(|definition| definition.span(db).len())
}

pub(in crate::core) fn all_definitions_of<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
) -> Vec<Definition<'db>> {
    fn visit<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
        definitions: &mut Vec<Definition<'db>>,
    ) {
        definitions.push(definition);
        let body = body_of(db, definition);
        for expression in body.store.expressions().map(|(_, expression)| expression) {
            if let Expression::Block {
                block_key: Some(block_key),
                ..
            } = expression
            {
                for nested in block_scoped_definitions_of(db, *block_key).definitions() {
                    visit(db, *nested, definitions);
                }
            }
        }
    }

    let mut definitions = Vec::new();
    for definition in file_scoped_definitions_of(db, file).definitions() {
        visit(db, *definition, &mut definitions);
    }
    definitions
}

#[salsa::tracked]
fn body_with_source_map_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> (Arc<Body<'db>>, Arc<HirNodeSourceMap>) {
    let lowerer = BodyLowerer::new(db, definition.file(db), definition);
    let (body, source_map) = match definition {
        Definition::Function(key) => lowerer.lower_function(&key.to_ast_node(db)),
        Definition::Constant(key) => lowerer.lower_constant(&key.to_ast_node(db)),
    };
    (Arc::new(body), Arc::new(source_map))
}

#[salsa::tracked]
fn signature_with_source_map_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> (Arc<Signature<'db>>, Arc<HirNodeSourceMap>) {
    let (signature, source_map) = match definition {
        Definition::Function(key) => {
            let (function, source_map) =
                lower_function_signature(db, &key.to_ast_node(db), definition.anchor(db));
            (Signature::Function(function), source_map)
        }
        Definition::Constant(key) => {
            let (constant, source_map) =
                lower_constant_signature(db, &key.to_ast_node(db), definition.anchor(db));
            (Signature::Constant(constant), source_map)
        }
    };
    (Arc::new(signature), Arc::new(source_map))
}

fn collect_definitions<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
    source: DefinitionSource<'db>,
    definitions: impl Iterator<Item = ast::Definition>,
) -> Arc<DefinitionList<'db>> {
    let mut definition_list = DefinitionList::new();
    for definition in definitions {
        let Some(name) = definition.name() else {
            continue;
        };
        let name = ast_lowering::lower_name(db, &name);
        definition_list.add_definition(db, file, source, name, definition);
    }
    Arc::new(definition_list)
}

fn unknown_type_diagnostics<'db>(
    db: &'db dyn crate::Db,
    store: &HirNodeStore<'db>,
    source_map: &HirNodeSourceMap,
    anchor: TextSize,
    annotation_handles: impl IntoIterator<Item = TypeExpressionHandle>,
) -> Vec<SemanticDiagnostic> {
    let mut diagnostics = Vec::new();
    for annotation_handle in annotation_handles {
        lower_type_expression(
            db,
            store,
            annotation_handle,
            &mut |type_expression_handle| {
                let TypeExpression::Path(path) = &store[type_expression_handle] else {
                    unreachable!("only path type expressions can be unknown")
                };
                diagnostics.push(SemanticDiagnostic::UnknownType {
                    name: path.display(db).to_string(),
                    span: source_map.type_expression_span(type_expression_handle, anchor),
                });
            },
        );
    }
    diagnostics
}
