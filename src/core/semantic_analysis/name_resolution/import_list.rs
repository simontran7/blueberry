use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ast_lowering::nodes;

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct ImportList<'db> {
    paths: Box<[nodes::Path<'db>]>,
}

impl<'db> ImportList<'db> {
    pub(crate) fn new(paths: Box<[nodes::Path<'db>]>) -> Self {
        Self { paths }
    }

    pub(crate) fn paths(&self) -> &[nodes::Path<'db>] {
        &self.paths
    }

    pub(crate) fn matching(
        &self,
        db: &'db dyn crate::Db,
        name: Symbol<'db>,
    ) -> impl Iterator<Item = nodes::Path<'db>> {
        self.paths
            .iter()
            .copied()
            .filter(move |path| path.segments(db).last() == Some(&name))
    }
}
