use crate::core::common::span::{Span, TextSize};
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::keys::{ConstantKey, DefinitionSource, FunctionKey};
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::RedNodeTag;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode};

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct DefinitionList<'db> {
    definitions: Vec<Definition<'db>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::Supertype, salsa::SalsaValue)]
pub(crate) enum Definition<'db> {
    Function(FunctionKey<'db>),
    Constant(ConstantKey<'db>),
}

impl<'db> DefinitionList<'db> {
    pub(crate) fn new() -> Self {
        Self {
            definitions: Vec::new(),
        }
    }

    pub(crate) fn definitions(&self) -> &[Definition<'db>] {
        &self.definitions
    }

    pub(crate) fn find(
        &self,
        db: &'db dyn crate::Db,
        name: Symbol<'db>,
    ) -> Option<Definition<'db>> {
        self.definitions
            .iter()
            .copied()
            .find(|definition| definition.name(db) == name)
    }

    pub(crate) fn add_definition(
        &mut self,
        db: &'db dyn crate::Db,
        file: SourceFileKey,
        source: DefinitionSource<'db>,
        name: Symbol<'db>,
        definition: ast::Definition,
    ) {
        let node = RedNodeTag::new(definition.red());
        let key = match definition {
            ast::Definition::FunctionDefinition(_) => {
                Definition::Function(FunctionKey::new(db, file, source, name, node))
            }
            ast::Definition::ConstantDefinition(_) => {
                Definition::Constant(ConstantKey::new(db, file, source, name, node))
            }
        };
        self.definitions.push(key);
    }
}

impl<'db> Definition<'db> {
    pub(crate) fn name(self, db: &'db dyn crate::Db) -> Symbol<'db> {
        match self {
            Self::Function(key) => key.name(db),
            Self::Constant(key) => key.name(db),
        }
    }

    pub(crate) fn anchor(self, db: &'db dyn crate::Db) -> TextSize {
        self.span(db).start()
    }

    pub(crate) fn span(self, db: &'db dyn crate::Db) -> Span {
        match self {
            Self::Function(key) => key.node(db).span,
            Self::Constant(key) => key.node(db).span,
        }
    }

    pub(crate) fn source(self, db: &'db dyn crate::Db) -> DefinitionSource<'db> {
        match self {
            Self::Function(key) => key.source(db),
            Self::Constant(key) => key.source(db),
        }
    }

    pub(crate) fn file(self, db: &'db dyn crate::Db) -> SourceFileKey {
        match self {
            Self::Function(key) => key.file(db),
            Self::Constant(key) => key.file(db),
        }
    }
}
