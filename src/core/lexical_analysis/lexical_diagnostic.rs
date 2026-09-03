use crate::core::common::diagnostic::{DiagnosticDescription, DiagnosticLabel, LabelSeverity};
use crate::core::common::text_size::TextRange;

#[derive(Debug, Clone)]
pub(crate) enum LexicalDiagnostic {
    UnknownToken { character: char, span: TextRange },
}

impl LexicalDiagnostic {
    pub(crate) fn describe(&self) -> DiagnosticDescription {
        match self {
            Self::UnknownToken { character, span } => DiagnosticDescription {
                code: "E0102",
                message: format!("unknown start of token: `{character}`"),
                span: *span,
                labels: vec![DiagnosticLabel {
                    span: *span,
                    message: Some("unrecognized character".to_string()),
                    severity: LabelSeverity::Primary,
                }],
            },
        }
    }
}
