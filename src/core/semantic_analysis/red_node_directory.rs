use std::collections::HashMap;
use std::marker::PhantomData;

use crate::core::common::span::Span;
use crate::core::common::symbol::Symbol;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::ast::{self, AstNode};
use crate::core::syntactic_analysis::cst::{RedChild, RedNode, SyntaxKind};
use crate::core::syntactic_analysis::cst_of;

pub(crate) struct RedNodeDirectoryBuilder<'a, 'db> {
    db: &'db dyn crate::Db,
    file: SourceFileKey,
    root: &'a RedNode,
    directory: RedNodeDirectory<'db>,
}

#[derive(Debug, Default, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct RedNodeDirectory<'db> {
    tags: HashMap<RawRedNodeId<'db>, RedNodeTag>,
    ids: HashMap<RedNodeTag, RawRedNodeId<'db>>,
}

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) struct RedNodeTag {
    pub(crate) kind: SyntaxKind,
    pub(crate) span: Span,
}

impl<'a, 'db> RedNodeDirectoryBuilder<'a, 'db> {
    pub(crate) fn new(db: &'db dyn crate::Db, file: SourceFileKey, root: &'a RedNode) -> Self {
        RedNodeDirectoryBuilder {
            db,
            file,
            root,
            directory: RedNodeDirectory::default(),
        }
    }

    pub(crate) fn build(mut self) -> RedNodeDirectory<'db> {
        let root = self.root.clone();
        self.assign_ids(&root, None, &mut HashMap::new());
        self.directory
    }

    /// Assigns a stable id of kind `RawRedNodeId` to every `Block`, `FunctionDefinition` and `ConstantDefinition`
    fn assign_ids(
        &mut self,
        node: &RedNode,
        parent: Option<RawRedNodeId<'db>>,
        collisions: &mut HashMap<(SyntaxKind, Option<Symbol<'db>>), u32>,
    ) {
        for child in node.children() {
            let RedChild::Node(child) = child else {
                continue;
            };

            if matches!(
                child.kind(),
                SyntaxKind::Block | SyntaxKind::FunctionDefinition | SyntaxKind::ConstantDefinition
            ) {
                let name = {
                    let token = ast::Definition::cast(child.clone())
                        .and_then(|definition| definition.name());
                    if let Some(token) = token {
                        Some(Symbol::new(self.db, token.lexeme().to_string()))
                    } else {
                        None
                    }
                };
                let collision_index = collisions.entry((child.kind(), name)).or_insert(0);
                let id = RawRedNodeId::new(
                    self.db,
                    self.file,
                    parent,
                    child.kind(),
                    name,
                    *collision_index,
                );
                *collision_index += 1;

                self.directory.tags.insert(id, RedNodeTag::new(&child));
                self.directory.ids.insert(RedNodeTag::new(&child), id);

                self.assign_ids(&child, Some(id), &mut HashMap::new());
            } else {
                self.assign_ids(&child, parent, collisions);
            }
        }
    }
}

impl<'db> RedNodeDirectory<'db> {
    pub(crate) fn id_of(&self, node: &RedNode) -> Option<RawRedNodeId<'db>> {
        self.ids.get(&RedNodeTag::new(node)).copied()
    }

    pub(crate) fn tag_of(&self, id: RawRedNodeId<'db>) -> Option<RedNodeTag> {
        self.tags.get(&id).copied()
    }
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
        let directory = super::red_node_directory_of(db, file);
        let tag = directory.tag_of(self.id).unwrap();
        let root = RedNode::new(cst_of(db, file).clone());
        N::cast(tag.to_red_node(&root).unwrap()).unwrap()
    }
}

impl RedNodeTag {
    fn new(node: &RedNode) -> Self {
        Self {
            kind: node.kind(),
            span: node.span(),
        }
    }

    fn to_red_node(&self, root: &RedNode) -> Option<RedNode> {
        let mut current = root.clone();
        loop {
            if current.kind() == self.kind && current.span() == self.span {
                return Some(current);
            }
            let next = current.children().find_map(|child| match child {
                RedChild::Node(node) if node.span().contains(self.span) => Some(node),
                _ => None,
            })?;
            current = next;
        }
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
