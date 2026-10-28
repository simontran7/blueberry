use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode};
use crate::core::syntactic_analysis::cst::{RedNode, RedNodeTag};
use crate::core::syntactic_analysis::cst_of;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(in crate::core) enum DefinitionSource<'db> {
    TopLevel(SourceFileKey),
    Block(BlockKey<'db>),
}

#[salsa::tracked(debug)]
pub(in crate::core) struct FunctionKey<'db> {
    #[returns(copy)]
    pub(in crate::core) file: SourceFileKey,
    #[returns(copy)]
    pub(in crate::core) source: DefinitionSource<'db>,
    #[returns(copy)]
    pub(in crate::core) name: Symbol<'db>,
    #[returns(copy)]
    #[tracked]
    pub(in crate::core) node: RedNodeTag,
}

#[salsa::tracked(debug)]
pub(in crate::core) struct ConstantKey<'db> {
    #[returns(copy)]
    pub(in crate::core) file: SourceFileKey,
    #[returns(copy)]
    pub(in crate::core) source: DefinitionSource<'db>,
    #[returns(copy)]
    pub(in crate::core) name: Symbol<'db>,
    #[returns(copy)]
    #[tracked]
    pub(in crate::core) node: RedNodeTag,
}

#[salsa::tracked(debug)]
pub(in crate::core) struct BlockKey<'db> {
    #[returns(copy)]
    pub(in crate::core) file: SourceFileKey,
    #[returns(copy)]
    pub(in crate::core) owner: Definition<'db>,
    #[returns(copy)]
    pub(in crate::core) parent: Option<BlockKey<'db>>,
    #[returns(copy)]
    #[tracked]
    pub(in crate::core) node: RedNodeTag,
}

impl<'db> DefinitionSource<'db> {
    pub(in crate::core) fn block(self) -> Option<BlockKey<'db>> {
        match self {
            Self::TopLevel(_) => None,
            Self::Block(block) => Some(block),
        }
    }
}

impl<'db> FunctionKey<'db> {
    pub(in crate::core) fn to_ast_node(self, db: &'db dyn crate::Db) -> ast::FunctionDefinition {
        to_ast_node(db, self.file(db), self.node(db))
    }
}

impl<'db> ConstantKey<'db> {
    pub(in crate::core) fn to_ast_node(self, db: &'db dyn crate::Db) -> ast::ConstantDefinition {
        to_ast_node(db, self.file(db), self.node(db))
    }
}

impl<'db> BlockKey<'db> {
    pub(in crate::core) fn to_ast_node(self, db: &'db dyn crate::Db) -> ast::Block {
        to_ast_node(db, self.file(db), self.node(db))
    }

    pub(in crate::core) fn enclosing_block(self, db: &'db dyn crate::Db) -> Option<BlockKey<'db>> {
        self.parent(db)
            .or_else(|| self.owner(db).source(db).block())
    }
}

fn to_ast_node<N: AstNode>(db: &dyn crate::Db, file: SourceFileKey, node: RedNodeTag) -> N {
    let root = RedNode::new(cst_of(db, file).clone());
    N::cast(node.to_red_node(&root).unwrap()).unwrap()
}
