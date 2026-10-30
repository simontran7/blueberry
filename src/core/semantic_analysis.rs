pub(crate) mod ast_lowering;
pub(crate) mod keys;
pub(crate) mod name_resolution;
pub(crate) mod semantic_diagnostic;
pub(crate) mod type_checking;

use std::sync::Arc;

use crate::core::common::text_range::{AbsoluteTextRange, TextOffset};
use crate::core::module_map::{ModuleMap, ModulePath};

use crate::core::semantic_analysis::ast_lowering::nodes::{
    self, HirNodeSourceMap, HirNodeStore, TypeExpressionHandle,
};

use crate::core::semantic_analysis::keys::{BlockKey, DefinitionSource};

use crate::core::semantic_analysis::name_resolution::definition_list::{
    Definition, DefinitionList,
};

use crate::core::semantic_analysis::name_resolution::import_list::ImportList;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeTree;
use crate::core::semantic_analysis::semantic_diagnostic::{OwnerSpans, SemanticDiagnostic};
use crate::core::semantic_analysis::type_checking::type_expression_resolution::resolve_type_expression;
use crate::core::semantic_analysis::type_checking::types::Ty;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::RedNode;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode, File};
use crate::core::syntactic_analysis::cst_of;

// ---- Signatures ----
pub(crate) mod signature {
    use std::sync::Arc;

    use crate::core::semantic_analysis::ast_lowering::nodes::{
        HirNodeSourceMap, Signature, SignatureKind,
    };
    use crate::core::semantic_analysis::ast_lowering::signature_lowerer::SignatureLowerer;
    use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
    use crate::core::semantic_analysis::type_checking::type_expression_resolution::resolve_type_expression;
    use crate::core::semantic_analysis::type_checking::types::Ty;

    /// Returns the type of `definition`, from its signature's annotations alone.
    #[salsa::tracked]
    pub(crate) fn type_of<'db>(db: &'db dyn crate::Db, definition: Definition<'db>) -> Ty<'db> {
        let signature = hir_of(db, definition);
        // the unknown paths are reported by `definition_diagnostics_of`
        let resolve = |annotation_handle| {
            resolve_type_expression(db, signature.store(), annotation_handle).ty
        };
        match signature.kind() {
            SignatureKind::Function(function) => {
                let parameters = function
                    .parameter_annotations()
                    .iter()
                    .map(|&annotation_handle| resolve(annotation_handle))
                    .collect();
                let return_type = function
                    .return_type_annotation()
                    .map_or_else(|| Ty::unit(db), resolve);
                Ty::function(db, parameters, return_type)
            }
            SignatureKind::Constant(constant) => {
                constant.annotation().map_or_else(|| Ty::error(db), resolve)
            }
        }
    }

    /// Returns the HIR of `definition`'s signature.
    #[salsa::tracked]
    pub(crate) fn hir_of<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
    ) -> Arc<Signature<'db>> {
        hir_and_source_map_of(db, definition).0.clone()
    }

    /// Returns where each node of `definition`'s signature comes from in the text.
    pub(crate) fn source_map_of<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
    ) -> Arc<HirNodeSourceMap> {
        hir_and_source_map_of(db, definition).1.clone()
    }

    #[salsa::tracked]
    fn hir_and_source_map_of<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
    ) -> (Arc<Signature<'db>>, Arc<HirNodeSourceMap>) {
        let lowerer = SignatureLowerer::new(db, definition);
        let (signature, source_map) = match definition {
            Definition::Function(key) => lowerer.lower_function(&key.to_ast_node(db)),
            Definition::Constant(key) => lowerer.lower_constant(&key.to_ast_node(db)),
        };
        (Arc::new(signature), Arc::new(source_map))
    }
}

// ---- Bodies ----
pub(crate) mod body {
    use std::sync::Arc;

    use crate::core::semantic_analysis::ast_lowering::body_lowerer::BodyLowerer;
    use crate::core::semantic_analysis::ast_lowering::nodes::{Body, HirNodeSourceMap};
    use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
    use crate::core::semantic_analysis::type_checking::type_checker::{
        BodySemantics, check_constant, check_function,
    };
    use crate::core::semantic_analysis::{scopes_of, signature};

    /// Returns the types of `definition`'s body: what each name in it refers to, and what type
    /// each expression has.
    #[salsa::tracked]
    pub(crate) fn types_of<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
    ) -> Arc<BodySemantics<'db>> {
        let body = hir_of(db, definition);
        let scopes = scopes_of(db, definition);
        let signature_type = *signature::type_of(db, definition);
        Arc::new(match definition {
            Definition::Function(_) => check_function(db, body, scopes, signature_type),
            Definition::Constant(_) => check_constant(db, body, scopes, signature_type),
        })
    }

    /// Returns the HIR of `definition`'s body.
    #[salsa::tracked]
    pub(crate) fn hir_of<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
    ) -> Arc<Body<'db>> {
        hir_and_source_map_of(db, definition).0.clone()
    }

    /// Returns where each node of `definition`'s body comes from in the text.
    pub(crate) fn source_map_of<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
    ) -> Arc<HirNodeSourceMap> {
        hir_and_source_map_of(db, definition).1.clone()
    }

    #[salsa::tracked]
    fn hir_and_source_map_of<'db>(
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
}

// ---- Name Collecting ----
#[salsa::tracked]
pub(crate) fn scopes_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Arc<ScopeTree<'db>> {
    Arc::new(ScopeTree::new(
        body::hir_of(db, definition),
        definition.file(db),
        definition.source(db).block(),
    ))
}

#[salsa::tracked]
pub(crate) fn top_level_definitions_of<'db>(
    db: &'db dyn crate::Db,
    file: SourceFileKey,
) -> Arc<DefinitionList<'db>> {
    let root = RedNode::new(cst_of(db, file).clone());
    let root = File::cast(root).unwrap();
    collect_definitions(
        db,
        file,
        DefinitionSource::TopLevel(file),
        root.items().filter_map(|item| match item {
            ast::Item::Definition(definition) => Some(definition),
            ast::Item::Import(_) => None,
        }),
    )
}

#[salsa::tracked]
pub(crate) fn block_definitions_of<'db>(
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
    let root_node = RedNode::new(cst_of(db, file).clone());
    let root_file = File::cast(root_node).unwrap();

    let paths = root_file
        .items()
        .filter_map(|item| match item {
            ast::Item::Import(import_declaration) => import_declaration.path(),
            ast::Item::Definition(_) => None,
        })
        .filter_map(|path| ast_lowering::path_lowering::lower_path(db, &path))
        .collect();

    Arc::new(ImportList::new(paths))
}

/// Takes an import's path and returns that module's [`SourceFileKey`]
#[salsa::tracked]
pub(crate) fn module_file_of<'db>(
    db: &'db dyn crate::Db,
    path: ast_lowering::nodes::Path<'db>,
) -> Option<SourceFileKey> {
    let modules = ModuleMap::try_get(db)?;
    let path = ModulePath::new(
        path.segments(db)
            .iter()
            .map(|segment| segment.text(db).to_string())
            .collect(),
    );
    modules.files(db).get(&path).copied()
}

// ---- Diagnostics ----
#[salsa::tracked]
pub(crate) fn definition_diagnostics_of<'db>(
    db: &'db dyn crate::Db,
    definition: Definition<'db>,
) -> Arc<[SemanticDiagnostic]> {
    let file = definition.file(db);
    let signature = signature::hir_of(db, definition);
    let anchor = definition.anchor(db);
    let mut diagnostics = unknown_type_diagnostics(
        db,
        signature.store(),
        &signature::source_map_of(db, definition),
        anchor,
        signature.annotation_handles(),
    );

    let owner = match definition {
        Definition::Function(key) => {
            let node = key.to_ast_node(db);
            OwnerSpans {
                file,
                anchor,
                name: node.name().map(|name| name.range()),
                return_type: node
                    .return_type_annotation()
                    .map(|annotation| annotation.red().range()),
            }
        }
        Definition::Constant(key) => OwnerSpans {
            file,
            anchor,
            name: key.to_ast_node(db).name().map(|name| name.range()),
            return_type: None,
        },
    };
    let body = body::hir_of(db, definition);
    let source_map = body::source_map_of(db, definition);
    let semantics = body::types_of(db, definition);
    diagnostics.extend(
        semantics
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.resolve(db, body, &source_map, semantics, &owner)),
    );
    diagnostics.into()
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
            let lowered = ast_lowering::path_lowering::lower_path(db, &path)?;
            module_file_of(db, lowered)
                .is_none()
                .then(|| SemanticDiagnostic::UnresolvedImport {
                    path: lowered
                        .segments(db)
                        .iter()
                        .map(|segment| segment.text(db).to_string())
                        .collect(),
                    range: path.red().range(),
                })
        })
        .collect()
}

pub(crate) fn entry_point_diagnostics_of(
    db: &dyn crate::Db,
    file: SourceFileKey,
) -> Vec<SemanticDiagnostic> {
    let main = top_level_definitions_of(db, file)
        .definitions()
        .iter()
        .copied()
        .find(|definition| {
            matches!(definition, Definition::Function(_)) && definition.name(db).text(db) == "main"
        });
    let Some(main) = main else {
        let file_name = file.path(db).file_name().unwrap_or_default();
        let end = TextOffset::new(file.contents(db).len());
        return vec![SemanticDiagnostic::MainNotFound {
            file: file_name.to_string_lossy().into_owned(),
            range: AbsoluteTextRange::new(end, end),
        }];
    };

    let ty = *signature::type_of(db, main);
    let expected = Ty::function(db, Box::new([]), Ty::unit(db));
    // a poisoned signature (e.g., with an unknown type) was already reported
    if ty == expected || ty.is_poisoned(db) {
        return Vec::new();
    }
    let Definition::Function(key) = main else {
        unreachable!("`main` was found among the functions");
    };
    let range = key
        .to_ast_node(db)
        .name()
        .map_or_else(|| main.range(db), |name| name.range());
    vec![SemanticDiagnostic::MainWrongType {
        expected: expected.display(db).to_string(),
        found: ty.display(db).to_string(),
        range,
    }]
}

// ---- Finding Definitions ----
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
        let body = body::hir_of(db, definition);
        for expression in body.store().expressions().map(|(_, expression)| expression) {
            if let nodes::Expression::Block {
                block_key: Some(block_key),
                ..
            } = expression
            {
                for nested in block_definitions_of(db, *block_key).definitions() {
                    visit(db, *nested, definitions);
                }
            }
        }
    }

    let mut definitions = Vec::new();
    for definition in top_level_definitions_of(db, file).definitions() {
        visit(db, *definition, &mut definitions);
    }
    definitions
}

// ---- Helpers ----
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
        let name = ast_lowering::path_lowering::lower_name(db, &name);
        definition_list.add_definition(db, file, source, name, definition);
    }
    Arc::new(definition_list)
}

fn unknown_type_diagnostics<'db>(
    db: &'db dyn crate::Db,
    store: &HirNodeStore<'db>,
    source_map: &HirNodeSourceMap,
    anchor: TextOffset,
    annotation_handles: impl IntoIterator<Item = TypeExpressionHandle>,
) -> Vec<SemanticDiagnostic> {
    let mut diagnostics = Vec::new();
    for annotation_handle in annotation_handles {
        let resolved = resolve_type_expression(db, store, annotation_handle);
        for unknown_path in resolved.unknown_paths {
            diagnostics.push(SemanticDiagnostic::UnknownType {
                name: unknown_path.path.display(db).to_string(),
                range: source_map.type_expression_range(unknown_path.handle, anchor),
            });
        }
    }
    diagnostics
}
