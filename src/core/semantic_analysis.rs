pub(crate) mod ast_lowering;
pub(crate) mod hir;
pub(crate) mod ids;
pub(crate) mod name_resolution;
pub(crate) mod semantic_diagnostic;
pub(crate) mod type_checking;

use std::sync::Arc;

use crate::core::common::symbol::Symbol;
use crate::core::module_map::{ModuleMap, ModulePath};
use crate::core::semantic_analysis::ast_lowering::definition_body_lowerer::DefinitionBodyLowerer;

use crate::core::semantic_analysis::hir::nodes::{
    ConstantSignature, DefinitionBody, DefinitionBodySourceMap, Expression, FunctionSignature,
    TypeAnnotation,
};

use crate::core::semantic_analysis::ids::keys::{
    BlockKey, ConstantKey, DefinitionSource, FunctionKey,
};

use crate::core::semantic_analysis::name_resolution::definition_list::{
    Definition, DefinitionList,
};

use crate::core::semantic_analysis::name_resolution::import_list::ImportList;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeTree;
use crate::core::semantic_analysis::semantic_diagnostic::{OwnerSpans, SemanticDiagnostic};
use crate::core::semantic_analysis::type_checking::type_checker::{
    TypeCheckSink, check_constant, check_function,
};
use crate::core::semantic_analysis::type_checking::types::Ty;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::RedNode;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode, File};
use crate::core::syntactic_analysis::cst_of;

#[salsa::tracked]
pub(crate) fn file_scoped_definitions_of<'db>(
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
pub(crate) fn block_scoped_definitions_of<'db>(
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
pub(crate) fn imports_of<'db>(db: &'db dyn crate::Db, file: SourceFileKey) -> Arc<ImportList<'db>> {
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

pub(crate) fn import_diagnostics_of(
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
pub(crate) fn function_signature_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> FunctionSignature<'db> {
    let node = key.to_ast_node(db);

    let name = node
        .name()
        .map(|token| Symbol::new(db, token.lexeme().to_string()))
        .unwrap();

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
    let node = key.to_ast_node(db);

    let name = node
        .name()
        .map(|token| Symbol::new(db, token.lexeme().to_string()))
        .unwrap();

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
    Arc::new(ScopeTree::new(
        function_body_of(db, key),
        key.file(db),
        key.source(db).block(),
    ))
}

#[salsa::tracked]
pub(crate) fn constant_scopes_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> Arc<ScopeTree<'db>> {
    Arc::new(ScopeTree::new(
        constant_body_of(db, key),
        key.file(db),
        key.source(db).block(),
    ))
}

#[salsa::tracked]
pub(crate) fn function_types_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> Arc<TypeCheckSink<'db>> {
    let body = function_body_of(db, key);
    let scopes = function_scopes_of(db, key);
    Arc::new(check_function(
        db,
        body,
        scopes,
        *function_signature_type_of(db, key),
    ))
}

#[salsa::tracked]
pub(crate) fn constant_types_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> Arc<TypeCheckSink<'db>> {
    let body = constant_body_of(db, key);
    let scopes = constant_scopes_of(db, key);
    Arc::new(check_constant(
        db,
        body,
        scopes,
        *constant_signature_type_of(db, key),
    ))
}

#[salsa::tracked]
pub(crate) fn function_body_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> Arc<DefinitionBody<'db>> {
    function_body_with_source_map_of(db, key).0.clone()
}

pub(crate) fn function_source_map_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> Arc<DefinitionBodySourceMap> {
    function_body_with_source_map_of(db, key).1.clone()
}

#[salsa::tracked]
pub(crate) fn constant_body_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> Arc<DefinitionBody<'db>> {
    constant_body_with_source_map_of(db, key).0.clone()
}

pub(crate) fn constant_source_map_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> Arc<DefinitionBodySourceMap> {
    constant_body_with_source_map_of(db, key).1.clone()
}

/// Returns every definition in `file`, including the ones nested in blocks: each definition is
/// followed by the ones nested in its body.
pub(crate) fn all_definitions_of<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
) -> Vec<Definition<'db>> {
    fn visit<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
        definitions: &mut Vec<Definition<'db>>,
    ) {
        definitions.push(definition);
        let body = match definition {
            Definition::Function(key) => function_body_of(db, key),
            Definition::Constant(key) => constant_body_of(db, key),
        };
        for expression in body.expressions.values() {
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

/// Returns a definition's body, its source map, and its type checking results.
pub(crate) fn checked_body_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> (
    Arc<DefinitionBody<'db>>,
    Arc<DefinitionBodySourceMap>,
    Arc<TypeCheckSink<'db>>,
) {
    match definition {
        Definition::Function(key) => (
            function_body_of(db, key).clone(),
            function_source_map_of(db, key),
            function_types_of(db, key).clone(),
        ),
        Definition::Constant(key) => (
            constant_body_of(db, key).clone(),
            constant_source_map_of(db, key),
            constant_types_of(db, key).clone(),
        ),
    }
}

pub(crate) fn definition_diagnostics_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Vec<SemanticDiagnostic> {
    let file = match definition {
        Definition::Function(key) => key.file(db),
        Definition::Constant(key) => key.file(db),
    };
    let mut diagnostics = Vec::new();
    let type_expressions: Vec<ast::TypeExpression> = match definition {
        Definition::Function(key) => {
            let node = key.to_ast_node(db);
            node.parameter_list()
                .into_iter()
                .flat_map(|list| list.parameters().collect::<Vec<_>>())
                .filter_map(|parameter| parameter.type_expression())
                .chain(node.return_type())
                .collect()
        }
        Definition::Constant(key) => key.to_ast_node(db).type_annotation().into_iter().collect(),
    };
    for type_expression in type_expressions {
        if let Some(name) = type_expression.name()
            && Ty::to_primitive(db, name.lexeme()).is_none()
        {
            diagnostics.push(SemanticDiagnostic::UnknownType {
                name: name.lexeme().to_string(),
                span: type_expression.red().span(),
            });
        }
    }

    let owner = match definition {
        Definition::Function(key) => {
            let node = key.to_ast_node(db);
            OwnerSpans {
                file,
                name: node.name().map(|name| name.span()),
                return_type: node.return_type().map(|ty| ty.red().span()),
            }
        }
        Definition::Constant(key) => OwnerSpans {
            file,
            name: key.to_ast_node(db).name().map(|name| name.span()),
            return_type: None,
        },
    };
    let (body, source_map, types) = checked_body_of(db, definition);
    diagnostics.extend(
        types
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.resolve(db, &body, &source_map, &types, &owner)),
    );
    diagnostics
}

#[salsa::tracked]
fn function_body_with_source_map_of<'db>(
    db: &'db dyn crate::Db,
    key: FunctionKey<'db>,
) -> (Arc<DefinitionBody<'db>>, Arc<DefinitionBodySourceMap>) {
    let node = key.to_ast_node(db);
    let lowerer = DefinitionBodyLowerer::new(db, key.file(db), Definition::Function(key));
    let (body, source_map) = lowerer.lower_function(&node);
    (Arc::new(body), Arc::new(source_map))
}

#[salsa::tracked]
fn constant_body_with_source_map_of<'db>(
    db: &'db dyn crate::Db,
    key: ConstantKey<'db>,
) -> (Arc<DefinitionBody<'db>>, Arc<DefinitionBodySourceMap>) {
    let node = key.to_ast_node(db);
    let lowerer = DefinitionBodyLowerer::new(db, key.file(db), Definition::Constant(key));
    let (body, source_map) = lowerer.lower_constant(&node);
    (Arc::new(body), Arc::new(source_map))
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
        let name = Symbol::new(db, name.lexeme().to_string());
        definition_list.add_definition(db, file, source, name, definition);
    }
    Arc::new(definition_list)
}
