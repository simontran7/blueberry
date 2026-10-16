use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path as FilePath;

use crate::core::semantic_analysis::hir::nodes::Path;
use crate::core::source_file_key::SourceFileKey;

#[salsa::input(singleton, debug)]
pub(in crate::core) struct ModuleMap {
    #[returns(ref)]
    pub(in crate::core) files: BTreeMap<ModulePath, SourceFileKey>,
}

/// A module path's segments, e.g. `["a", "b"]` for an import declaration `import a::b;`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::core) struct ModulePath(Box<[String]>);

impl ModuleMap {
    pub(in crate::core) fn load(db: &dyn crate::Db, root: &FilePath) -> io::Result<Self> {
        let mut files = BTreeMap::new();
        let mut directories = vec![root.to_path_buf()];
        while let Some(directory) = directories.pop() {
            for entry in fs::read_dir(&directory)? {
                let path = entry?.path();
                if path.is_dir() {
                    directories.push(path);
                } else if path.extension().is_some_and(|extension| extension == "bb") {
                    let module_path = ModulePath::from_file_path(path.strip_prefix(root).unwrap());
                    let contents = fs::read_to_string(&path)?;
                    files.insert(module_path, SourceFileKey::new(db, path, contents));
                }
            }
        }
        Ok(Self::new(db, files))
    }
}

impl ModulePath {
    pub(in crate::core) fn from_file_path(relative_path: &FilePath) -> Self {
        Self(
            relative_path
                .with_extension("")
                .components()
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .collect(),
        )
    }

    pub(in crate::core) fn from_path<'db>(db: &'db dyn crate::Db, path: Path<'db>) -> Self {
        Self(
            path.segments(db)
                .iter()
                .map(|segment| segment.text(db).to_string())
                .collect(),
        )
    }
}
