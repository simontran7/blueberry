pub(crate) mod constraints;
pub(crate) mod hir;
pub(crate) mod hir_dumper;
pub(crate) mod semantic_analyzer;
pub(crate) mod semantic_diagnostic;
pub(crate) mod symbol_table;
pub(crate) mod unification_table;

use salsa::Accumulator;

use crate::core::common::context::CompilerContext;
use crate::core::common::diagnostic::{Diagnostic, DiagnosticAccumulator};
use crate::core::common::text_size::TextRange;
use crate::core::common::types::Ty;
use crate::core::file_scanning::SourceFile;
use crate::core::semantic_analysis::hir::{Hir, HirSourceMaps};
use crate::core::semantic_analysis::semantic_analyzer::{DefinitionKey, SemanticAnalyzer};
use crate::core::syntactic_analysis::cst_of;

#[salsa::tracked]
pub(crate) fn hir_of(db: &dyn crate::Db, file: SourceFile) {
    cst_of(db, file);
    check_signatures_of(db, file);
    for key in definition_keys_of(db, file) {
        body_hir_of(db, file, key.clone());
    }
}

#[salsa::tracked]
pub(crate) fn signatures_of(
    db: &dyn crate::Db,
    file: SourceFile,
) -> Vec<(DefinitionKey, Ty, TextRange)> {
    let cst = cst_of(db, file).clone();
    let mut ctx = CompilerContext::new();
    let (hir, _symbol_table, binding_ids, _diagnostics) =
        SemanticAnalyzer::new(cst, &mut ctx, db).collect_signatures();
    binding_ids
        .into_iter()
        .map(|(key, binding_id)| {
            let binding_view = hir.get_definition_binding(binding_id);
            let ty = ctx
                .type_interner
                .resolve(binding_view.ty())
                .expect("just interned")
                .clone();
            (key, ty, binding_view.text_range())
        })
        .collect()
}

#[salsa::tracked]
pub(crate) fn definition_keys_of(db: &dyn crate::Db, file: SourceFile) -> Vec<DefinitionKey> {
    signatures_of(db, file)
        .iter()
        .map(|(key, _, _)| key.clone())
        .collect()
}

#[salsa::tracked]
pub(crate) fn check_signatures_of(db: &dyn crate::Db, file: SourceFile) {
    let cst = cst_of(db, file).clone();
    let mut ctx = CompilerContext::new();
    let (_hir, _symbol_table, _binding_ids, diagnostics) =
        SemanticAnalyzer::new(cst, &mut ctx, db).collect_signatures();
    for diagnostic in diagnostics {
        DiagnosticAccumulator(Diagnostic::Semantic(diagnostic)).accumulate(db);
    }
}

#[salsa::tracked(no_eq)]
pub(crate) fn body_hir_of(
    db: &dyn crate::Db,
    file: SourceFile,
    key: DefinitionKey,
) -> (Hir, HirSourceMaps, CompilerContext) {
    let cst = cst_of(db, file).clone();
    let mut ctx = CompilerContext::new();
    let signatures = signatures_of(db, file).clone();
    let (analyzer, binding_ids) =
        SemanticAnalyzer::new(cst, &mut ctx, db).seed_signatures(&signatures);
    let own_binding_id = binding_ids
        .into_iter()
        .find(|(candidate, _)| *candidate == key)
        .expect("caller-supplied key must belong to this file")
        .1;
    let (hir, source_maps, diagnostics) = analyzer.typecheck_one(&key, own_binding_id);
    for diagnostic in diagnostics {
        DiagnosticAccumulator(Diagnostic::Semantic(diagnostic)).accumulate(db);
    }
    (hir, source_maps, ctx)
}
