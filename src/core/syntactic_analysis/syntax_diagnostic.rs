use crate::core::common::diagnostic::{DiagnosticDescription, DiagnosticLabel, LabelSeverity};
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

    pub(crate) fn describe(&self) -> DiagnosticDescription {
        let Self::UnexpectedToken {
            span,
            expected,
            found,
        } = self;
        let span = span.expect("diagnostic span not yet resolved");
        DiagnosticDescription {
            code: "E0101",
            message: format!("expected `{expected}`, found `{found}`"),
            anchor: span,
            labels: vec![DiagnosticLabel {
                span,
                message: Some(format!("expected `{expected}`")),
                severity: LabelSeverity::Primary,
            }],
        }
    }
}
