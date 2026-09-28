use crate::core::lexical_analysis::token_stream::{TokenKind, TokenStream};

/// The parser's input: only the significant tokens of a `TokenStream`, with trivia removed. Like
/// the `TokenStream`, it always ends with an `Eof` token.
///
/// The parser never sees trivia. The `CstBuilder` walks the full `TokenStream` and puts the trivia
/// back into the tree, using `token_index` to map a significant token back to its full-stream index.
pub(crate) struct Source {
    kinds: Vec<TokenKind>,
    /// `token_indices[i]` is the full-stream index of significant token `i`.
    token_indices: Vec<usize>,
}

impl Source {
    pub(crate) fn new(tokens: &TokenStream) -> Self {
        let (kinds, token_indices) = tokens
            .kinds()
            .enumerate()
            .filter(|(_, kind)| !kind.is_trivia())
            .map(|(index, kind)| (kind, index))
            .unzip();
        Self {
            kinds,
            token_indices,
        }
    }

    /// Returns the kind of the `index`th significant token. Any index past the end is `Eof`.
    pub(crate) fn kind_at(&self, index: usize) -> TokenKind {
        self.kinds[index.min(self.kinds.len() - 1)]
    }

    /// Returns the full-stream index of the `index`th significant token.
    pub(crate) fn token_index(&self, index: usize) -> usize {
        self.token_indices[index]
    }
}
