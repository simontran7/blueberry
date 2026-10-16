#[salsa::interned(debug)]
pub(in crate::core) struct Symbol<'db> {
    #[returns(deref)]
    pub(in crate::core) text: String,
}

impl<'db> Symbol<'db> {
    pub(in crate::core) fn as_tuple_index(self, db: &'db dyn crate::Db) -> Option<u32> {
        let text = self.text(db);
        if text.len() > 1 && text.starts_with('0') {
            return None;
        }
        text.parse().ok()
    }
}
