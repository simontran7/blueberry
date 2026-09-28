use crate::core::lexical_analysis::token_stream::TokenStream;

pub(crate) struct TokenDumper<'src> {
    source: &'src str,
    tokens: TokenStream,
}

impl<'src> TokenDumper<'src> {
    pub(crate) fn new(source: &'src str, tokens: TokenStream) -> Self {
        Self { source, tokens }
    }

    pub(crate) fn dump(&self) -> String {
        let mut dump = String::new();

        // dump header
        dump.push_str(&"-".repeat(32));
        dump.push('\n');
        dump.push_str(&format!("{:<8} {:<15} {:<15}\n", "Index", "Kind", "Span"));
        dump.push_str(&"-".repeat(32));
        dump.push('\n');

        // dump tokens row by row
        for (i, (kind, span)) in self.tokens.kinds().zip(self.tokens.spans()).enumerate() {
            let start = usize::from(span.start());
            let end = usize::from(span.end());

            let kind = if kind.has_lexeme() {
                self.source[start..end].to_string()
            } else {
                kind.to_string()
            };

            dump.push_str(&format!(
                "{:<8} {:<15} {:<15}\n",
                format!("#{}", i),
                format!("`{}`", kind),
                format!("[{}, {})", start, end),
            ));
        }

        dump
    }
}
