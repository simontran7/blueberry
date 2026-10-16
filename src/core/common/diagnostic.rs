use crate::core::common::span::Span;
use crate::core::lexical_analysis::lexical_diagnostic::LexicalDiagnostic;
use crate::core::semantic_analysis::semantic_diagnostic::SemanticDiagnostic;
use crate::core::syntactic_analysis::parser::syntax_diagnostic::SyntaxDiagnostic;

#[salsa::accumulator]
pub(in crate::core) struct DiagnosticAccumulator(pub(in crate::core) Diagnostic);

#[derive(Debug, Clone)]
pub(in crate::core) enum Diagnostic {
    Lexical(LexicalDiagnostic),
    Syntax(SyntaxDiagnostic),
    Semantic(SemanticDiagnostic),
}

pub(crate) struct DiagnosticDescription {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) span: Span,
    pub(crate) labels: Box<[DiagnosticLabel]>,
    /// Shown after the source snippet as `= note: ...`.
    pub(crate) notes: Box<[String]>,
    /// Shown after the source snippet as `= help: ...`.
    pub(crate) helps: Box<[String]>,
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
    pub(in crate::core) fn describe(&self) -> DiagnosticDescription {
        match self {
            Self::Lexical(diagnostic) => diagnostic.describe(),
            Self::Syntax(diagnostic) => diagnostic.describe(),
            Self::Semantic(diagnostic) => diagnostic.describe(),
        }
    }
}
