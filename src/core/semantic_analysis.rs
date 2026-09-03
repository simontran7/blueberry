pub(crate) mod constraints;
pub(crate) mod hir;
pub(crate) mod hir_dumper;
pub(crate) mod semantic_analyzer;
pub(crate) mod semantic_diagnostic;
pub(crate) mod symbol_table;
pub(crate) mod unification_table;

/* Query layer being redesigned from scratch to match rust-analyzer's
   shape/infer query split -- kept here for reference during the rewrite.
use salsa::Accumulator;

use crate::core::common::diagnostic::{Diagnostic, DiagnosticAccumulator};
use crate::core::common::text_size::TextRange;
use crate::core::common::types::Ty;
use crate::core::source_file::SourceFile;
use crate::core::semantic_analysis::hir::{Hir, HirSourceMaps, ResolvedTypes};
use crate::core::semantic_analysis::semantic_analyzer::{
    BlockId, DefinitionKey, ScopeId, SemanticAnalyzer,
};
use crate::core::syntactic_analysis::cst_of;

#[salsa::tracked]
pub(crate) fn hir_of(db: &dyn crate::Db, file: SourceFile) {
    todo!()
}

#[salsa::tracked]
pub(crate) fn signatures_of(
    db: &dyn crate::Db,
    file: SourceFile,
) -> Vec<(DefinitionKey, Ty, TextRange)> {
    let cst = cst_of(db, file).clone();
    let (signatures, diagnostics) = SemanticAnalyzer::new(cst, db).collect_signatures();
    for diagnostic in diagnostics {
        DiagnosticAccumulator(Diagnostic::Semantic(diagnostic)).accumulate(db);
    }
    signatures
}

#[salsa::tracked]
pub(crate) fn definition_keys_of(db: &dyn crate::Db, file: SourceFile) -> Vec<DefinitionKey> {
    let mut keys = Vec::new();
    for (key, _, _) in signatures_of(db, file) {
        keys.push(key.clone());
        collect_nested_keys(db, file, ScopeId::Definition(key.clone()), &mut keys);
    }
    keys
}

fn collect_nested_keys(
    db: &dyn crate::Db,
    file: SourceFile,
    scope: ScopeId,
    out: &mut Vec<DefinitionKey>,
) {
    let (definitions, blocks) = scope_contents_of(db, file, scope);
    for (key, _, _) in definitions {
        out.push(key.clone());
        collect_nested_keys(db, file, ScopeId::Definition(key.clone()), out);
    }
    for block_id in blocks {
        collect_nested_keys(db, file, ScopeId::Block(block_id.clone()), out);
    }
}

#[salsa::tracked]
pub(crate) fn scope_contents_of(
    db: &dyn crate::Db,
    file: SourceFile,
    scope: ScopeId,
) -> (Vec<(DefinitionKey, Ty, TextRange)>, Vec<BlockId>) {
    let cst = cst_of(db, file).clone();
    // No ancestor seeding needed: `collect_scope_at` locates `scope`'s own
    // syntax via a pure walk from the file root (no name resolution), and
    // `collect_definition` only resolves builtin type-annotation names --
    // neither needs anything from an enclosing scope's symbol table.
    let (definitions, blocks, diagnostics) =
        SemanticAnalyzer::new(cst, db).collect_scope_at(&scope);
    for diagnostic in diagnostics {
        DiagnosticAccumulator(Diagnostic::Semantic(diagnostic)).accumulate(db);
    }
    (definitions, blocks)
}

#[salsa::tracked(no_eq)]
pub(crate) fn body_hir_with_source_map_of<'db>(
    db: &'db dyn crate::Db,
    file: SourceFile,
    key: DefinitionKey,
) -> (Hir, HirSourceMaps, ResolvedTypes<'db>) {
    let cst = cst_of(db, file).clone();
    let levels = levels_for(db, file, &key.parent);
    let (_analyzer, _own_binding_id) =
        SemanticAnalyzer::new(cst, db).seed_signatures(&levels, &key);
    // TODO: rebuild per the new two-pass design -- lower_body (pure shape,
    // no types) then infer_body (unification over that already-built
    // shape). The old fused typecheck_one and its whole call graph are
    // commented out in semantic_analyzer.rs, kept for reference.
    todo!("rebuild body lowering + inference per the new two-pass design")
}

fn levels_for(
    db: &dyn crate::Db,
    file: SourceFile,
    target: &ScopeId,
) -> Vec<Vec<(DefinitionKey, Ty, TextRange)>> {
    scope_chain(target)
        .into_iter()
        .map(|scope| match scope {
            ScopeId::File => signatures_of(db, file).clone(),
            other => scope_contents_of(db, file, other).0.clone(),
        })
        .collect()
}

fn scope_chain(scope: &ScopeId) -> Vec<ScopeId> {
    let mut chain = match scope {
        ScopeId::File => Vec::new(),
        ScopeId::Definition(key) => scope_chain(&key.parent),
        ScopeId::Block(block_id) => scope_chain(&block_id.parent),
    };
    chain.push(scope.clone());
    chain
}

#[salsa::tracked]
pub(crate) fn body_hir_of(db: &dyn crate::Db, file: SourceFile, key: DefinitionKey) -> Hir {
    let (hir, _source_maps, _resolved_types) = body_hir_with_source_map_of(db, file, key);
    hir.clone()
}

#[salsa::tracked]
pub(crate) fn body_resolved_types_of<'db>(
    db: &'db dyn crate::Db,
    file: SourceFile,
    key: DefinitionKey,
) -> ResolvedTypes<'db> {
    let (_hir, _source_maps, resolved_types) = body_hir_with_source_map_of(db, file, key);
    resolved_types.clone()
}
*/
