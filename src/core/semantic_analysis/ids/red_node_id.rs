use std::marker::PhantomData;

use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::red_node_directory_of;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::ast::AstNode;
use crate::core::syntactic_analysis::cst::{RedNode, SyntaxKind};
use crate::core::syntactic_analysis::cst_of;

pub(crate) struct RedNodeId<'db, N> {
    pub(crate) id: RawRedNodeId<'db>,
    _marker: PhantomData<fn() -> N>,
}

#[salsa::interned(debug)]
pub(crate) struct RawRedNodeId<'db> {
    pub(crate) file: SourceFileKey,
    pub(crate) parent: Option<RawRedNodeId<'db>>,
    pub(crate) kind: SyntaxKind,
    pub(crate) name: Option<Symbol<'db>>,
    pub(crate) collision_index: u32,
}

impl<'db, N> RedNodeId<'db, N> {
    pub(crate) fn new(id: RawRedNodeId<'db>) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }
}
impl<'db, N: AstNode> RedNodeId<'db, N> {
    pub(crate) fn to_ast_node(self, db: &'db dyn crate::Db) -> N {
        let file = *self.id.file(db);
        let directory = red_node_directory_of(db, file);
        let tag = directory.tag_of(self.id).unwrap();
        let root = RedNode::new(cst_of(db, file).clone());
        N::cast(tag.to_red_node(&root).unwrap()).unwrap()
    }
}

unsafe impl<'db, N> salsa::SalsaValue for RedNodeId<'db, N> {}

impl<'db, N> std::ops::Deref for RedNodeId<'db, N> {
    type Target = RawRedNodeId<'db>;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<'db, N> Clone for RedNodeId<'db, N> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'db, N> Copy for RedNodeId<'db, N> {}

impl<'db, N> PartialEq for RedNodeId<'db, N> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<'db, N> Eq for RedNodeId<'db, N> {}

impl<'db, N> std::hash::Hash for RedNodeId<'db, N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<'db, N> std::fmt::Debug for RedNodeId<'db, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedNodeId").field("id", &self.id).finish()
    }
}
