use crate::core::common::types::TypeInterner;

#[derive(Clone)]
pub struct CompilerContext {
    pub(crate) type_interner: TypeInterner,
}

impl CompilerContext {
    pub fn new() -> Self {
        Self {
            type_interner: TypeInterner::new(),
        }
    }
}
