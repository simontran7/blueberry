//! `SourceFile` itself is core-compiler infrastructure -- every producer
//! query takes one. `source_file_of` (which reads from disk) is currently
//! the only way to construct one, and is really batch-flavored: a future
//! `lsp` consumer would construct `SourceFile` inputs from editor-provided
//! text (`textDocument/didOpen`/`didChange`), not by reading disk, the same
//! way rust-analyzer's VFS is populated from editor events rather than a
//! query doing its own file I/O.

use std::fs;
use std::io;
use std::path::PathBuf;

#[salsa::input(debug)]
pub(crate) struct SourceFile {
    pub(crate) path: PathBuf,
    #[returns(deref)]
    pub(crate) contents: String,
}

pub(crate) fn source_file_of(db: &dyn crate::Db, path: PathBuf) -> io::Result<SourceFile> {
    let contents = fs::read_to_string(&path)?;
    Ok(SourceFile::new(db, path, contents))
}
