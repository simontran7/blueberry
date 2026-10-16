use crate::core::lexical_analysis::token_stream::{TokenKind, TokenStream};

/// The parser's input: only the significant tokens of a `TokenStream`, with trivia removed. Like
/// the `TokenStream`, it always ends with an `Eof` token.
///
/// The parser never sees trivia. The `CstBuilder` walks the full `TokenStream` and puts the trivia
/// back into the tree, using `token_index` to map a significant token back to its full-stream index.
pub(in crate::core) struct Source {
    kinds: Box<[TokenKind]>,
    /// `token_indices[i]` is the full-stream index of significant token `i`.
    token_indices: Box<[usize]>,
}

impl Source {
    pub(in crate::core) fn new(tokens: &TokenStream) -> Self {
        let (kinds, token_indices): (Vec<_>, Vec<_>) = tokens
            .kinds()
            .enumerate()
            .filter(|(_, kind)| !kind.is_trivia())
            .map(|(index, kind)| (kind, index))
            .unzip();
        Self {
            kinds: kinds.into_boxed_slice(),
            token_indices: token_indices.into_boxed_slice(),
        }
    }

    /// Returns the kind of the `index`th significant token. Any index past the end is `Eof`.
    pub(in crate::core) fn kind_at(&self, index: usize) -> TokenKind {
        self.kinds[index.min(self.kinds.len() - 1)]
    }

    /// Returns the full-stream index of the `index`th significant token.
    pub(in crate::core) fn token_index(&self, index: usize) -> usize {
        self.token_indices[index]
    }
}
