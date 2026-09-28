use std::collections::HashMap;

use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ids::red_node_id::RawRedNodeId;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode};
use crate::core::syntactic_analysis::cst::{RedChild, RedNode, RedNodeTag, SyntaxKind};

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
                let name = ast::Definition::cast(child.clone())
                    .and_then(|definition| definition.name())
                    .map(|token| Symbol::new(self.db, token.lexeme().to_string()));
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
