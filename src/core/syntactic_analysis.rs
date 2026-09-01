//! Producer: part of the core compiler. Pure, memoized salsa queries only --
//! no I/O, no consumer-specific policy. Consumed by both the batch (`cli`)
//! and, eventually, interactive (`lsp`) consumers.

pub(crate) mod ast;
pub(crate) mod cst;
pub(crate) mod cst_builder;
pub(crate) mod cst_dumper;
pub(crate) mod parser;
pub(crate) mod syntax_diagnostic;

use std::sync::Arc;

use salsa::Accumulator;

use crate::core::common::diagnostic::{Diagnostic, DiagnosticAccumulator};
use crate::core::file_scanning::SourceFile;
use crate::core::lexical_analysis::tokens_of;
use crate::core::syntactic_analysis::cst::GreenNode;
use crate::core::syntactic_analysis::cst_builder::CstBuilder;
use crate::core::syntactic_analysis::parser::Parser;

#[salsa::tracked]
pub(crate) fn cst_of(db: &dyn crate::Db, file: SourceFile) -> Arc<GreenNode> {
    let tokens = tokens_of(db, file);
    let (events, diagnostics) = Parser::new(tokens).parse();
    let (cst, diagnostics) = CstBuilder::new(file.contents(db), tokens, events, diagnostics).build();
    for diagnostic in diagnostics {
        DiagnosticAccumulator(Diagnostic::Syntax(diagnostic)).accumulate(db);
    }
    cst
}
