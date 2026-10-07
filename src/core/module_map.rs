use std::collections::BTreeMap;

use crate::core::semantic_analysis::hir::nodes::Path;
use crate::core::source_file_key::SourceFileKey;

#[salsa::input(singleton, debug)]
pub(crate) struct ModuleMap {
    #[returns(ref)]
    pub(crate) files: BTreeMap<ModulePath, SourceFileKey>,
}

/// A module path's segments, e.g. `["a", "b"]` for an import declaration `import a::b;`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ModulePath(Vec<String>);

impl ModulePath {
    pub(crate) fn from_path<'db>(db: &'db dyn crate::Db, path: Path<'db>) -> Self {
        Self(
            path.segments(db)
                .iter()
                .map(|segment| segment.text(db).to_string())
                .collect(),
        )
    }
}
