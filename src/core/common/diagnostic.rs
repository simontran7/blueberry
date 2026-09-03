use crate::core::common::text_size::TextRange;
use crate::core::lexical_analysis::lexical_diagnostic::LexicalDiagnostic;
// TODO: bring back once semantic_diagnostic.rs is rebuilt.
// use crate::core::semantic_analysis::semantic_diagnostic::SemanticDiagnostic;
use crate::core::syntactic_analysis::syntax_diagnostic::SyntaxDiagnostic;

#[salsa::accumulator]
pub(crate) struct DiagnosticAccumulator(pub(crate) Diagnostic);

#[derive(Debug, Clone)]
pub(crate) enum Diagnostic {
    Lexical(LexicalDiagnostic),
    Syntax(SyntaxDiagnostic),
    // Semantic(SemanticDiagnostic),
}

impl Diagnostic {
    pub(crate) fn describe(&self) -> DiagnosticDescription {
        match self {
            Self::Lexical(diagnostic) => diagnostic.describe(),
            Self::Syntax(diagnostic) => diagnostic.describe(),
            // Self::Semantic(diagnostic) => diagnostic.describe(),
        }
    }
}

pub(crate) struct DiagnosticDescription {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) span: TextRange,
    pub(crate) labels: Vec<DiagnosticLabel>,
}

pub(crate) struct DiagnosticLabel {
    pub(crate) span: TextRange,
    pub(crate) message: Option<String>,
    pub(crate) severity: LabelSeverity,
}

pub(crate) enum LabelSeverity {
    Primary,
    Secondary,
}
