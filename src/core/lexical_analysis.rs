pub(crate) mod lexical_diagnostic;
pub(crate) mod token_stream;
pub(crate) mod token_stream_dumper;
pub(crate) mod tokenizer;

use salsa::Accumulator;

use crate::core::common::diagnostic::{Diagnostic, DiagnosticAccumulator};
use crate::core::lexical_analysis::token_stream::TokenStream;
use crate::core::lexical_analysis::tokenizer::Tokenizer;
use crate::core::source_file::SourceFile;

#[salsa::tracked]
pub(crate) fn tokens_of(db: &dyn crate::Db, file: SourceFile) -> TokenStream {
    let mut tokenizer = Tokenizer::new(file.contents(db));
    let (tokens, diagnostics) = tokenizer.tokenize();
    for diagnostic in diagnostics {
        DiagnosticAccumulator(Diagnostic::Lexical(diagnostic)).accumulate(db);
    }
    tokens
}
