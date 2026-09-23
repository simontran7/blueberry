use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::hir::nodes::Path;
use crate::core::syntactic_analysis::ast;

pub(crate) mod definition_body_builder;
pub(crate) mod definition_body_lowerer;

/// Walks an `ast::Path`'s qualifier chain and interns its names, in source order.
/// Returns `None` if any segment is missing (a broken path after a parse error).
pub(crate) fn lower_path<'db>(db: &'db dyn crate::Db, path: &ast::Path) -> Option<Path<'db>> {
    let mut segments = Vec::new();
    let mut current = Some(path.clone());

    while let Some(path) = current {
        let name = path.segment()?.name()?;
        segments.push(Symbol::new(db, name.lexeme().to_string()));
        current = path.qualifier();
    }

    segments.reverse();

    Some(Path::new(db, segments))
}
