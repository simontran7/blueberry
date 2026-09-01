#[salsa::interned(debug)]
pub(crate) struct Symbol<'db> {
    #[returns(deref)]
    pub(crate) text: String,
}
