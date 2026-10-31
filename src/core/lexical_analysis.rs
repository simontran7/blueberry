pub(crate) mod token_stream;
pub(crate) mod token_stream_dumper;
pub(crate) mod tokenizer;

use std::sync::Arc;

use crate::core::common::diagnostic::Diagnostic;
use crate::core::lexical_analysis::token_stream::TokenStream;
use crate::core::lexical_analysis::tokenizer::Tokenizer;
use crate::core::source_file_key::SourceFileKey;

/// Returns the tokens of `file`.
#[salsa::tracked]
pub(crate) fn tokens_of(db: &dyn crate::Db, file: SourceFileKey) -> Arc<TokenStream> {
    tokens_and_diagnostics_of(db, file).0.clone()
}

/// Returns the errors found while tokenizing `file`.
pub(crate) fn lexical_diagnostics_of(db: &dyn crate::Db, file: SourceFileKey) -> Arc<[Diagnostic]> {
    tokens_and_diagnostics_of(db, file).1.clone()
}

#[salsa::tracked]
fn tokens_and_diagnostics_of(
    db: &dyn crate::Db,
    file: SourceFileKey,
) -> (Arc<TokenStream>, Arc<[Diagnostic]>) {
    let (tokens, diagnostics) = Tokenizer::new(file.contents(db)).tokenize();
    (Arc::new(tokens), diagnostics.into())
}
