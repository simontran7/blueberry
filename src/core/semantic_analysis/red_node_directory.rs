use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::core::common::span::Span;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::ast::{self, AstNode};
use crate::core::syntactic_analysis::cst::{RedChild, RedNode, SyntaxKind};

pub(crate) struct RedNodeDirectoryBuilder<'a, 'db> {
    db: &'db dyn crate::Db,
    file: SourceFileKey,
    root: &'a RedNode,
    directory: RedNodeDirectory<'db>,
}

#[derive(Debug, Default, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct RedNodeDirectory<'db> {
    locations: HashMap<RawRedNodeId<'db>, RedNodeLocation>,
    ids: HashMap<RedNodeLocation, RawRedNodeId<'db>>,
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
    // Hash of the node's name (or of nothing, for anonymous nodes like
    // blocks). Deriving the id from the name rather than sibling position
    // keeps it stable when an unrelated sibling is added/removed/reordered.
    pub(crate) name_hash: u64,
    // Disambiguates same-kind siblings that hash equal -- same name (or,
    // for blocks, just any two blocks under the same parent).
    pub(crate) collision_index: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) struct RedNodeLocation {
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
        self.dfs(&root, None, &mut HashMap::new());
        self.directory
    }

    fn dfs(
        &mut self,
        node: &RedNode,
        parent: Option<RawRedNodeId<'db>>,
        collisions: &mut HashMap<(SyntaxKind, u64), u32>,
    ) {
        for child in node.children() {
            let RedChild::Node(child) = child else {
                continue;
            };

            if matches!(
                child.kind(),
                SyntaxKind::Block | SyntaxKind::FunctionDefinition | SyntaxKind::ConstantDefinition
            ) {
                let name_hash = Self::name_hash(child.kind(), &child);
                let collision_index = collisions.entry((child.kind(), name_hash)).or_insert(0);
                let id = RawRedNodeId::new(self.db, self.file, parent, child.kind(), name_hash, *collision_index);
                *collision_index += 1;

                self.directory.locations.insert(id, RedNodeLocation::new(&child));
                self.directory.ids.insert(RedNodeLocation::new(&child), id);

                self.dfs(&child, Some(id), &mut HashMap::new());
            } else {
                self.dfs(&child, parent, collisions);
            }
        }
    }

    // Blocks are anonymous, so every block under the same parent hashes the
    // same and falls back to pure collision-index disambiguation -- matching
    // rust-analyzer's `BlockExprFileAstId`, which likewise carries no name.
    fn name_hash(kind: SyntaxKind, node: &RedNode) -> u64 {
        let name = match kind {
            SyntaxKind::FunctionDefinition => ast::FunctionDefinition::cast(node.clone()).and_then(|f| f.name()),
            SyntaxKind::ConstantDefinition => ast::ConstantDefinition::cast(node.clone()).and_then(|c| c.name()),
            _ => None,
        }
        .map(|token| token.lexeme().to_string());

        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish()
    }
}

impl<'db> RedNodeDirectory<'db> {
    pub(crate) fn id_of(&self, node: &RedNode) -> Option<RawRedNodeId<'db>> {
        self.ids.get(&RedNodeLocation::new(node)).copied()
    }

    pub(crate) fn location_of(&self, id: RawRedNodeId<'db>) -> Option<RedNodeLocation> {
        self.locations.get(&id).copied()
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

impl RedNodeLocation {
    fn new(node: &RedNode) -> Self {
        Self {
            kind: node.kind(),
            span: node.span(),
        }
    }

    // Descends directly toward the target span instead of scanning every
    // node: at each level, only the one child whose span contains the
    // target span can possibly lead to it. O(depth), not O(size of file) --
    // matters once this runs on every hover/goto-definition in an LSP, not
    // just once per definition in a batch compile.
    pub(crate) fn to_node(&self, root: &RedNode) -> Option<RedNode> {
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
