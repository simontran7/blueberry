use std::ops::Range;

use ariadne::{Color, Label, Report, ReportKind, Source};

#[derive(Debug, Clone)]
pub(crate) enum ParserDiagnostic {
    UnexpectedToken {
        span: Option<Range<usize>>,
        expected: String,
        found: String,
    },
}

impl ParserDiagnostic {
    pub(crate) fn new(expected: String, found: String) -> Self {
        Self::UnexpectedToken { span: None, expected, found }
    }

    pub(crate) fn resolve(&mut self, span: Range<usize>) {
        match self {
            Self::UnexpectedToken { span: s, .. } => *s = Some(span),
        }
    }

    pub(crate) fn render(&self, filename: &str, source: &str) {
        let span = match self {
            Self::UnexpectedToken { span, .. } => {
                span.clone().expect("diagnostic span not yet resolved")
            }
        };
        let report = match self {
            Self::UnexpectedToken { expected, found, .. } => {
                Report::build(ReportKind::Error, filename, span.start)
                    .with_code("E0101")
                    .with_message(format!("expected `{expected}`, found `{found}`"))
                    .with_label(
                        Label::new((filename, span))
                            .with_message(format!("expected `{expected}`"))
                            .with_color(Color::Red),
                    )
                    .finish()
            }
        };
        report.eprint((filename, Source::from(source))).unwrap();
    }
}
