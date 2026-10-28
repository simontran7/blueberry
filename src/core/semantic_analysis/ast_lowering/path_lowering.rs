use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ast_lowering::nodes;
use crate::core::syntactic_analysis::cst::RedToken;
use crate::core::syntactic_analysis::cst::ast;

pub(crate) fn lower_name<'db>(db: &'db dyn crate::Db, token: &RedToken) -> Symbol<'db> {
    Symbol::new(db, token.lexeme().to_string())
}

pub(crate) fn lower_path<'db>(
    db: &'db dyn crate::Db,
    path: &ast::Path,
) -> Option<nodes::Path<'db>> {
    let mut segments = Vec::new();
    let mut current = Some(path.clone());

    while let Some(path) = current {
        let name = path.segment()?.name()?;
        segments.push(lower_name(db, &name));
        current = path.qualifier();
    }

    segments.reverse();

    Some(nodes::Path::new(db, segments.into_boxed_slice()))
}
