use std::path::PathBuf;

#[salsa::input(debug)]
pub(crate) struct SourceFileKey {
    pub(crate) path: PathBuf,
    #[returns(deref)]
    pub(crate) contents: String,
}
