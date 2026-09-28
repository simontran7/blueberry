use crate::core::semantic_analysis::ids::red_node_id::RedNodeId;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::ast;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) enum DefinitionSource<'db> {
    File(SourceFileKey),
    Block(BlockKey<'db>),
}

#[salsa::interned(debug)]
pub(crate) struct BlockKey<'db> {
    pub(crate) id: RedNodeId<'db, ast::Block>,
}

#[salsa::interned(debug)]
pub(crate) struct FunctionKey<'db> {
    pub(crate) source: DefinitionSource<'db>,
    pub(crate) id: RedNodeId<'db, ast::FunctionDefinition>,
}

#[salsa::interned(debug)]
pub(crate) struct ConstantKey<'db> {
    pub(crate) source: DefinitionSource<'db>,
    pub(crate) id: RedNodeId<'db, ast::ConstantDefinition>,
}
