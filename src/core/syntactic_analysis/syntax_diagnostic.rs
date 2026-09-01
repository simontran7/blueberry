use ariadne::{Color, Label, Report, ReportKind, Source};

use crate::core::common::text_size::TextRange;

#[derive(Debug, Clone)]
pub(crate) enum SyntaxDiagnostic {
    UnexpectedToken {
        span: Option<TextRange>,
        expected: String,
        found: String,
    },
}

impl SyntaxDiagnostic {
    pub(crate) fn new(expected: String, found: String) -> Self {
        Self::UnexpectedToken {
            span: None,
            expected,
            found,
        }
    }

    pub(crate) fn resolve(&mut self, span: TextRange) {
        let Self::UnexpectedToken { span: s, .. } = self;
        *s = Some(span);
    }

    pub(crate) fn render(&self, filename: &str, source: &str) {
        let Self::UnexpectedToken {
            span,
            expected,
            found,
        } = self;
        let span = span.expect("diagnostic span not yet resolved");
        let report = Report::build(ReportKind::Error, filename, usize::from(span.start()))
            .with_code("E0101")
            .with_message(format!("expected `{expected}`, found `{found}`"))
            .with_label(
                Label::new((filename, span.into()))
                    .with_message(format!("expected `{expected}`"))
                    .with_color(Color::Red),
            )
            .finish();
        report.eprint((filename, Source::from(source))).unwrap();
    }
}
