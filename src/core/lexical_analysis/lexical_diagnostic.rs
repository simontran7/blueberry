use crate::core::common::diagnostic::{DiagnosticDescription, DiagnosticLabel, LabelSeverity};
use crate::core::common::text_range::AbsoluteTextRange;

#[derive(Debug, Clone)]
pub(crate) enum LexicalDiagnostic {
    UnknownToken {
        character: char,
        range: AbsoluteTextRange,
    },
}

impl LexicalDiagnostic {
    pub(crate) fn describe(&self) -> DiagnosticDescription {
        match self {
            Self::UnknownToken { character, range } => DiagnosticDescription {
                code: "E0102",
                message: format!("unknown start of token: `{character}`"),
                range: *range,
                labels: Box::new([DiagnosticLabel {
                    range: *range,
                    message: Some("unrecognized character".to_string()),
                    severity: LabelSeverity::Primary,
                }]),
                notes: Box::new([]),
                helps: Box::new([]),
            },
        }
    }
}
