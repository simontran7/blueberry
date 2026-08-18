use crate::common::string_interner::StringInterner;
use crate::common::types::TypeInterner;

pub struct CompilerContext {
    pub(crate) string_interner: StringInterner,
    pub(crate) type_interner: TypeInterner,
}

impl<'src> CompilerContext {
    pub fn new() -> Self {
        Self {
            string_interner: StringInterner::new(),
            type_interner: TypeInterner::new(),
        }
    }
}
