#[salsa::db]
#[derive(Default)]
pub(crate) struct BlueberryDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for BlueberryDatabase {}
