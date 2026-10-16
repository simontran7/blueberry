use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::hir::nodes::Path;

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) struct ImportList<'db> {
    paths: Box<[Path<'db>]>,
}

impl<'db> ImportList<'db> {
    pub(in crate::core) fn new(paths: Box<[Path<'db>]>) -> Self {
        Self { paths }
    }

    pub(in crate::core) fn paths(&self) -> &[Path<'db>] {
        &self.paths
    }

    pub(in crate::core) fn matching(
        &self,
        db: &'db dyn crate::Db,
        name: Symbol<'db>,
    ) -> impl Iterator<Item = Path<'db>> {
        self.paths
            .iter()
            .copied()
            .filter(move |path| path.segments(db).last() == Some(&name))
    }
}
