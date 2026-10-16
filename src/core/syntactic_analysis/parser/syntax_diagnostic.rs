use crate::core::common::diagnostic::{DiagnosticDescription, DiagnosticLabel, LabelSeverity};
use crate::core::common::span::Span;

#[derive(Debug, Clone)]
pub(in crate::core) enum SyntaxDiagnostic {
    UnexpectedToken {
        span: Option<Span>,
        expected: String,
        found: String,
    },
}

impl SyntaxDiagnostic {
    pub(in crate::core) fn new(expected: String, found: String) -> Self {
        Self::UnexpectedToken {
            span: None,
            expected,
            found,
        }
    }

    pub(in crate::core) fn describe(&self) -> DiagnosticDescription {
        let Self::UnexpectedToken {
            span,
            expected,
            found,
        } = self;
        let span = span.unwrap();
        DiagnosticDescription {
            code: "E0101",
            message: format!("expected `{expected}`, found `{found}`"),
            span,
            labels: Box::new([DiagnosticLabel {
                span,
                message: Some(format!("expected `{expected}`")),
                severity: LabelSeverity::Primary,
            }]),
            notes: Box::new([]),
            helps: Box::new([]),
        }
    }

    pub(in crate::core) fn resolve(&mut self, span: Span) {
        let Self::UnexpectedToken { span: s, .. } = self;
        *s = Some(span);
    }
}
