use std::path::PathBuf;

#[salsa::input(debug)]
pub(crate) struct SourceFileKey {
    pub(in crate::core) path: PathBuf,
    #[returns(deref)]
    pub(in crate::core) contents: String,
}
