pub(crate) mod cst;
pub(crate) mod parser;

use std::sync::Arc;

use crate::core::common::diagnostic::Diagnostic;
use crate::core::lexical_analysis::tokens_of;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::GreenNode;
use crate::core::syntactic_analysis::cst::cst_assembler::CstAssembler;
use crate::core::syntactic_analysis::parser::Parser;

/// Returns the CST of `file`.
#[salsa::tracked]
pub(crate) fn cst_of(db: &dyn crate::Db, file: SourceFileKey) -> Arc<GreenNode> {
    cst_and_diagnostics_of(db, file).0.clone()
}

/// Returns the errors found while parsing `file`.
pub(crate) fn syntax_diagnostics_of(db: &dyn crate::Db, file: SourceFileKey) -> Arc<[Diagnostic]> {
    cst_and_diagnostics_of(db, file).1.clone()
}

#[salsa::tracked]
fn cst_and_diagnostics_of(
    db: &dyn crate::Db,
    file: SourceFileKey,
) -> (Arc<GreenNode>, Arc<[Diagnostic]>) {
    let tokens = tokens_of(db, file);
    let (sink, unresolved_diagnostics) = Parser::new(tokens).parse();
    let (cst, diagnostics) =
        CstAssembler::new(file.contents(db), tokens, sink, unresolved_diagnostics).assemble();
    (cst, diagnostics.into())
}
