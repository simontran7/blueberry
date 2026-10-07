use crate::core::common::span::Span;
use crate::core::lexical_analysis::lexical_diagnostic::LexicalDiagnostic;
use crate::core::semantic_analysis::semantic_diagnostic::SemanticDiagnostic;
use crate::core::syntactic_analysis::parser::syntax_diagnostic::SyntaxDiagnostic;

#[salsa::accumulator]
pub(crate) struct DiagnosticAccumulator(pub(crate) Diagnostic);

#[derive(Debug, Clone)]
pub(crate) enum Diagnostic {
    Lexical(LexicalDiagnostic),
    Syntax(SyntaxDiagnostic),
    Semantic(SemanticDiagnostic),
}

pub(crate) struct DiagnosticDescription {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) span: Span,
    pub(crate) labels: Vec<DiagnosticLabel>,
    /// Shown after the source snippet as `= note: ...`.
    pub(crate) notes: Vec<String>,
    /// Shown after the source snippet as `= help: ...`.
    pub(crate) helps: Vec<String>,
}

pub(crate) struct DiagnosticLabel {
    pub(crate) span: Span,
    pub(crate) message: Option<String>,
    pub(crate) severity: LabelSeverity,
}

pub(crate) enum LabelSeverity {
    Primary,
    Secondary,
}

impl Diagnostic {
    pub(crate) fn describe(&self) -> DiagnosticDescription {
        match self {
            Self::Lexical(diagnostic) => diagnostic.describe(),
            Self::Syntax(diagnostic) => diagnostic.describe(),
            Self::Semantic(diagnostic) => diagnostic.describe(),
        }
    }
}
