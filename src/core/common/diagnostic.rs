use crate::core::common::text_range::AbsoluteTextRange;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Diagnostic {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) range: AbsoluteTextRange,
    pub(crate) labels: Box<[DiagnosticLabel]>,
    /// Shown after the source snippet as `= note: ...`.
    pub(crate) notes: Box<[String]>,
    /// Shown after the source snippet as `= help: ...`.
    pub(crate) helps: Box<[String]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiagnosticLabel {
    pub(crate) range: AbsoluteTextRange,
    pub(crate) message: Option<String>,
    pub(crate) kind: LabelKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LabelKind {
    Primary,
    Secondary,
}

impl Diagnostic {
    pub(crate) fn new(
        code: &'static str,
        message: &str,
        range: AbsoluteTextRange,
        labels: Vec<DiagnosticLabel>,
    ) -> Self {
        Self {
            code,
            message: message.to_string(),
            range,
            labels: labels.into_boxed_slice(),
            notes: Box::new([]),
            helps: Box::new([]),
        }
    }
}

impl DiagnosticLabel {
    pub(crate) fn primary(range: AbsoluteTextRange, message: impl Into<String>) -> Self {
        Self {
            range,
            message: Some(message.into()),
            kind: LabelKind::Primary,
        }
    }

    pub(crate) fn secondary(range: AbsoluteTextRange, message: impl Into<String>) -> Self {
        Self {
            range,
            message: Some(message.into()),
            kind: LabelKind::Secondary,
        }
    }

    pub(crate) fn unlabeled(range: AbsoluteTextRange) -> Self {
        Self {
            range,
            message: None,
            kind: LabelKind::Primary,
        }
    }
}
