use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode};
use crate::core::syntactic_analysis::cst::{RedNode, RedNodeTag};
use crate::core::syntactic_analysis::cst_of;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) enum DefinitionSource<'db> {
    File(SourceFileKey),
    Block(BlockKey<'db>),
}

#[salsa::tracked(debug)]
pub(crate) struct FunctionKey<'db> {
    #[returns(copy)]
    pub(crate) file: SourceFileKey,
    #[returns(copy)]
    pub(crate) source: DefinitionSource<'db>,
    #[returns(copy)]
    pub(crate) name: Symbol<'db>,
    #[returns(copy)]
    #[tracked]
    pub(crate) node: RedNodeTag,
}

#[salsa::tracked(debug)]
pub(crate) struct ConstantKey<'db> {
    #[returns(copy)]
    pub(crate) file: SourceFileKey,
    #[returns(copy)]
    pub(crate) source: DefinitionSource<'db>,
    #[returns(copy)]
    pub(crate) name: Symbol<'db>,
    #[returns(copy)]
    #[tracked]
    pub(crate) node: RedNodeTag,
}

#[salsa::tracked(debug)]
pub(crate) struct BlockKey<'db> {
    #[returns(copy)]
    pub(crate) file: SourceFileKey,
    #[returns(copy)]
    pub(crate) owner: Definition<'db>,
    #[returns(copy)]
    pub(crate) parent: Option<BlockKey<'db>>,
    #[returns(copy)]
    #[tracked]
    pub(crate) node: RedNodeTag,
}

impl<'db> DefinitionSource<'db> {
    pub(crate) fn block(self) -> Option<BlockKey<'db>> {
        match self {
            Self::File(_) => None,
            Self::Block(block) => Some(block),
        }
    }
}

impl<'db> FunctionKey<'db> {
    pub(crate) fn to_ast_node(self, db: &'db dyn crate::Db) -> ast::FunctionDefinition {
        to_ast_node(db, self.file(db), self.node(db))
    }
}

impl<'db> ConstantKey<'db> {
    pub(crate) fn to_ast_node(self, db: &'db dyn crate::Db) -> ast::ConstantDefinition {
        to_ast_node(db, self.file(db), self.node(db))
    }
}

impl<'db> BlockKey<'db> {
    pub(crate) fn to_ast_node(self, db: &'db dyn crate::Db) -> ast::Block {
        to_ast_node(db, self.file(db), self.node(db))
    }

    pub(crate) fn enclosing_block(self, db: &'db dyn crate::Db) -> Option<BlockKey<'db>> {
        self.parent(db)
            .or_else(|| self.owner(db).source(db).block())
    }
}

fn to_ast_node<N: AstNode>(db: &dyn crate::Db, file: SourceFileKey, node: RedNodeTag) -> N {
    let root = RedNode::new(cst_of(db, file).clone());
    N::cast(node.to_red_node(&root).unwrap()).unwrap()
}
