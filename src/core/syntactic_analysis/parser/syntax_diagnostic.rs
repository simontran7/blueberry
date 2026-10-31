use crate::core::common::diagnostic::{Diagnostic, DiagnosticLabel};
use crate::core::common::text_range::AbsoluteTextRange;

/// A syntax error the parser found, whose range isn't known until the CST is assembled. This is because the parser
/// never sees trivia, so it can't tell where a token is in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnresolvedSyntaxDiagnostic {
    expected: String,
    found: String,
}

impl UnresolvedSyntaxDiagnostic {
    pub(crate) fn new(expected: String, found: String) -> Self {
        Self { expected, found }
    }

    pub(crate) fn resolve(&self, range: AbsoluteTextRange) -> Diagnostic {
        let Self { expected, found } = self;
        Diagnostic::new(
            "E0101",
            &format!("expected `{expected}`, found `{found}`"),
            range,
            vec![DiagnosticLabel::primary(
                range,
                format!("expected `{expected}`"),
            )],
        )
    }
}
