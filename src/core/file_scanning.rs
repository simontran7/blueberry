use std::path::PathBuf;

#[salsa::input(debug)]
pub(crate) struct SourceFile {
    pub(crate) path: PathBuf,
    #[returns(deref)]
    pub(crate) contents: String,
}
