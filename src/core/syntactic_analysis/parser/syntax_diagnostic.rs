use crate::core::common::diagnostic::{DiagnosticDescription, DiagnosticLabel, LabelSeverity};
use crate::core::common::text_range::AbsoluteTextRange;

#[derive(Debug, Clone)]
pub(crate) enum SyntaxDiagnostic {
    UnexpectedToken {
        range: Option<AbsoluteTextRange>,
        expected: String,
        found: String,
    },
}

impl SyntaxDiagnostic {
    pub(crate) fn new(expected: String, found: String) -> Self {
        Self::UnexpectedToken {
            range: None,
            expected,
            found,
        }
    }

    pub(crate) fn describe(&self) -> DiagnosticDescription {
        let Self::UnexpectedToken {
            range,
            expected,
            found,
        } = self;
        let range = range.unwrap();
        DiagnosticDescription {
            code: "E0101",
            message: format!("expected `{expected}`, found `{found}`"),
            range,
            labels: Box::new([DiagnosticLabel {
                range,
                message: Some(format!("expected `{expected}`")),
                severity: LabelSeverity::Primary,
            }]),
            notes: Box::new([]),
            helps: Box::new([]),
        }
    }

    pub(crate) fn resolve(&mut self, range: AbsoluteTextRange) {
        let Self::UnexpectedToken { range: s, .. } = self;
        *s = Some(range);
    }
}
