pub(crate) mod cst;
pub(crate) mod parser;

use std::sync::Arc;

use salsa::Accumulator;

use crate::core::common::diagnostic::{Diagnostic, DiagnosticAccumulator};
use crate::core::lexical_analysis::tokens_of;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::GreenNode;
use crate::core::syntactic_analysis::cst::cst_builder::CstBuilder;
use crate::core::syntactic_analysis::parser::Parser;

#[salsa::tracked]
pub(crate) fn cst_of(db: &dyn crate::Db, file: SourceFileKey) -> Arc<GreenNode> {
    let tokens = tokens_of(db, file);
    let (sink, unresolved_diagnostics) = Parser::new(tokens).parse();
    let (cst, diagnostics) =
        CstBuilder::new(file.contents(db), tokens, sink, unresolved_diagnostics).build();
    for diagnostic in diagnostics {
        DiagnosticAccumulator(Diagnostic::Syntax(diagnostic)).accumulate(db);
    }
    cst
}
