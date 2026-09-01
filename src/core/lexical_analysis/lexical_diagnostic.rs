use ariadne::{Color, Label, Report, ReportKind, Source};

use crate::core::common::text_size::TextRange;

#[derive(Debug, Clone)]
pub(crate) enum LexicalDiagnostic {
    UnknownToken { character: char, span: TextRange },
}

impl LexicalDiagnostic {
    pub(crate) fn render(&self, filename: &str, source: &str) {
        let report = match self {
            Self::UnknownToken { character, span } => {
                Report::build(ReportKind::Error, filename, usize::from(span.start()))
                    .with_code("E0102")
                    .with_message(format!("unknown start of token: `{character}`"))
                    .with_label(
                        Label::new((filename, span.into()))
                            .with_message("unrecognized character")
                            .with_color(Color::Red),
                    )
                    .finish()
            }
        };
        report.eprint((filename, Source::from(source))).unwrap();
    }
}
