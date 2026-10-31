pub(crate) mod ast_lowering;
pub(crate) mod keys;
pub(crate) mod name_resolution;
pub(crate) mod semantic_diagnostic;
pub(crate) mod type_checking;

use std::sync::Arc;

use crate::core::module_map::{ModuleMap, ModulePath};

use crate::core::semantic_analysis::ast_lowering::nodes;

use crate::core::semantic_analysis::keys::{BlockKey, DefinitionSource};

use crate::core::semantic_analysis::name_resolution::definition_list::{
    Definition, DefinitionList,
};

use crate::core::common::diagnostic::Diagnostic;
use crate::core::semantic_analysis::name_resolution::import_list::ImportList;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeTree;
use crate::core::semantic_analysis::semantic_diagnostic::{unknown_type, unresolved_import};
use crate::core::semantic_analysis::type_checking::type_expression_resolution::resolve_type_expression;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::RedNode;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode, File};
use crate::core::syntactic_analysis::cst_of;

// ---- Signatures ----
pub(crate) mod signature {
    use std::sync::Arc;

    use crate::core::semantic_analysis::ast_lowering::nodes::{
        HirNodeRanges, Signature, SignatureKind,
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
        hir_and_ranges_of(db, definition).0.clone()
    }

    /// Returns where each node of `definition`'s signature comes from in the text.
    pub(crate) fn ranges_of<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
    ) -> Arc<HirNodeRanges> {
        hir_and_ranges_of(db, definition).1.clone()
    }

    #[salsa::tracked]
    fn hir_and_ranges_of<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
    ) -> (Arc<Signature<'db>>, Arc<HirNodeRanges>) {
        let lowerer = SignatureLowerer::new(db, definition);
        let (signature, ranges) = match definition {
            Definition::Function(key) => lowerer.lower_function(&key.to_ast_node(db)),
            Definition::Constant(key) => lowerer.lower_constant(&key.to_ast_node(db)),
        };
        (Arc::new(signature), Arc::new(ranges))
    }
}

// ---- Bodies ----
pub(crate) mod body {
    use std::sync::Arc;

    use crate::core::semantic_analysis::ast_lowering::body_lowerer::BodyLowerer;
    use crate::core::semantic_analysis::ast_lowering::nodes::{Body, HirNodeRanges};
    use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
    use crate::core::semantic_analysis::type_checking::type_checker::{
        BodySemantics, check_constant, check_function,
    };
    use crate::core::semantic_analysis::{scopes_of, signature};

    /// Returns the types of `definition`'s body: what each name in it refers to, and what type
    /// each expression has.
    #[salsa::tracked]
    pub(crate) fn semantics_of<'db>(
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
        hir_and_ranges_of(db, definition).0.clone()
    }

    /// Returns where each node of `definition`'s body comes from in the text.
    pub(crate) fn ranges_of<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
    ) -> Arc<HirNodeRanges> {
        hir_and_ranges_of(db, definition).1.clone()
    }

    #[salsa::tracked]
    fn hir_and_ranges_of<'db>(
        db: &'db dyn crate::Db,
        definition: Definition<'db>,
    ) -> (Arc<Body<'db>>, Arc<HirNodeRanges>) {
        let lowerer = BodyLowerer::new(db, definition.file(db), definition);
        let (body, ranges) = match definition {
            Definition::Function(key) => lowerer.lower_function(&key.to_ast_node(db)),
            Definition::Constant(key) => lowerer.lower_constant(&key.to_ast_node(db)),
        };
        (Arc::new(body), Arc::new(ranges))
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
            ast::TopLevelItem::Definition(definition) => Some(definition),
            ast::TopLevelItem::Import(_) => None,
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

/// Returns every definition in `file`, including nested ones, depth-first: each definition is
/// followed by the ones nested inside it.
///
/// Used for diagnostics and MIR dumping.
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
        for expression in body::hir_of(db, definition)
            .store()
            .expressions()
            .map(|(_, expression)| expression)
        {
            if let nodes::Expression::Block {
                block_key: Some(block_key),
                ..
            } = expression
            {
                for nested_definition in block_definitions_of(db, *block_key).definitions() {
                    visit(db, *nested_definition, definitions);
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

#[salsa::tracked]
pub(crate) fn imports_of<'db>(db: &'db dyn crate::Db, file: SourceFileKey) -> Arc<ImportList<'db>> {
    let root_node = RedNode::new(cst_of(db, file).clone());
    let root_file = File::cast(root_node).unwrap();

    let paths = root_file
        .items()
        .filter_map(|item| match item {
            ast::TopLevelItem::Import(import_declaration) => import_declaration.path(),
            ast::TopLevelItem::Definition(_) => None,
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
) -> Arc<[Diagnostic]> {
    // fetch the signature
    let signature = signature::hir_of(db, definition);
    let signature_ranges = signature::ranges_of(db, definition);
    let anchor = definition.anchor(db);

    // report unknown types in the signature
    let mut diagnostics = Vec::new();
    for annotation_handle in signature.annotation_handles() {
        let type_expression = resolve_type_expression(db, signature.store(), annotation_handle);
        for unknown_path in type_expression.unknown_paths {
            diagnostics.push(unknown_type(
                &unknown_path.path.display(db).to_string(),
                signature_ranges.type_expression_absolute_range(unknown_path.handle, anchor),
            ));
        }
    }

    // resolve body semantic diagnostics
    let hir = body::hir_of(db, definition);
    let ranges = body::ranges_of(db, definition);
    let semantics = body::semantics_of(db, definition);
    diagnostics.extend(
        semantics
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.resolve(db, hir, &ranges, semantics, definition)),
    );

    diagnostics.into()
}

pub(crate) fn import_diagnostics_of(db: &dyn crate::Db, file: SourceFileKey) -> Vec<Diagnostic> {
    let green_root = cst_of(db, file).clone();
    let red_root = RedNode::new(green_root);
    let ast_root = File::cast(red_root).unwrap();

    ast_root
        .items()
        .filter_map(|item| match item {
            ast::TopLevelItem::Import(import_declaration) => import_declaration.path(),
            ast::TopLevelItem::Definition(_) => None,
        }) // retain only imports
        .filter_map(|ast_path| {
            let hir_path = ast_lowering::path_lowering::lower_path(db, &ast_path)?;
            module_file_of(db, hir_path).is_none().then(|| {
                let segments: Vec<String> = hir_path
                    .segments(db)
                    .iter()
                    .map(|segment| segment.text(db).to_string())
                    .collect();
                unresolved_import(&segments, ast_path.red().range())
            }) // produce a diagnostic if hir `path` is not found in the module map
        })
        .collect()
}
