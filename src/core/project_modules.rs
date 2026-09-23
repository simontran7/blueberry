use std::collections::BTreeMap;

use crate::core::source_file_key::SourceFileKey;

#[salsa::input(singleton, debug)]
pub(crate) struct ProjectModules {
    #[returns(ref)]
    pub(crate) modules: BTreeMap<Vec<String>, SourceFileKey>,
}
