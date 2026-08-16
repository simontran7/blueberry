use crate::lexical_analysis::token_stream::TokenStream;

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
        let mut offset: usize = 0;

        for (i, (kind, width)) in self.tokens.kinds().zip(self.tokens.widths()).enumerate() {
            let start = offset;
            let end = offset + usize::from(width);

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

            offset = end;
        }

        dump
    }
}
