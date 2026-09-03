pub(crate) mod ast;
pub(crate) mod cst;
pub(crate) mod cst_builder;
pub(crate) mod cst_dumper;
pub(crate) mod parser;
pub(crate) mod syntax_diagnostic;

use std::sync::Arc;

use salsa::Accumulator;

use crate::core::common::diagnostic::{Diagnostic, DiagnosticAccumulator};
use crate::core::source_file::SourceFile;
use crate::core::lexical_analysis::tokens_of;
use crate::core::syntactic_analysis::cst::GreenNode;
use crate::core::syntactic_analysis::cst_builder::CstBuilder;
use crate::core::syntactic_analysis::parser::Parser;

#[salsa::tracked]
pub(crate) fn cst_of(db: &dyn crate::Db, file: SourceFile) -> Arc<GreenNode> {
    let tokens = tokens_of(db, file);
    let (events, unresolved_diagnostics) = Parser::new(tokens).parse();
    let (cst, diagnostics) = CstBuilder::new(file.contents(db), tokens, events, unresolved_diagnostics).build();
    for diagnostic in diagnostics {
        DiagnosticAccumulator(Diagnostic::Syntax(diagnostic)).accumulate(db);
    }
    cst
}
