use crate::core::lexical_analysis::lexical_diagnostic::LexicalDiagnostic;
use crate::core::semantic_analysis::semantic_diagnostic::SemanticDiagnostic;
use crate::core::syntactic_analysis::syntax_diagnostic::SyntaxDiagnostic;

#[salsa::accumulator]
pub(crate) struct DiagnosticAccumulator(pub(crate) Diagnostic);

#[derive(Debug, Clone)]
pub(crate) enum Diagnostic {
    Lexical(LexicalDiagnostic),
    Syntax(SyntaxDiagnostic),
    Semantic(SemanticDiagnostic),
}

impl Diagnostic {
    pub(crate) fn render(&self, filename: &str, source: &str) {
        match self {
            Self::Lexical(diagnostic) => diagnostic.render(filename, source),
            Self::Syntax(diagnostic) => diagnostic.render(filename, source),
            Self::Semantic(diagnostic) => diagnostic.render(filename, source),
        }
    }
}
