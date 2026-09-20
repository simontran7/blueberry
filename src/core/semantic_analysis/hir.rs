use std::fmt;

use crate::core::common::handle_collections::handle_impl;
use crate::core::common::handle_collections::handle_list::{AppendOnlyHandleList, HandleRange};
use crate::core::common::handle_collections::handle_map::{HandleMap, SideHandleMap};
use crate::core::common::symbol::Symbol;
use crate::core::common::types::Ty;
use crate::core::semantic_analysis::red_node_directory::{RedNodeId, RedNodeTag};
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::ast;
use crate::core::syntactic_analysis::cst::SyntaxKind;

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

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct FunctionSignature<'db> {
    pub(crate) name: Symbol<'db>,
    pub(crate) parameters: Vec<TypeAnnotation<'db>>,
    pub(crate) return_type_annotation: Option<TypeAnnotation<'db>>,
}
#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct ConstantSignature<'db> {
    pub(crate) name: Symbol<'db>,
    pub(crate) type_annotation: Option<TypeAnnotation<'db>>,
}
#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct DefinitionBody<'db> {
    pub(crate) root: ExpressionHandle,
    pub(crate) parameters: HandleRange<LocalBindingHandle>,
    pub(crate) expressions: HandleMap<ExpressionHandle, Expression<'db>>,
    pub(crate) statements: HandleMap<StatementHandle, Statement>,
    pub(crate) local_bindings: HandleMap<LocalBindingHandle, LocalBinding<'db>>,
    pub(crate) type_annotations: HandleMap<TypeAnnotationHandle, TypeAnnotation<'db>>,

    pub(crate) binding_children: AppendOnlyHandleList<LocalBindingHandle>,
    pub(crate) expression_children: AppendOnlyHandleList<ExpressionHandle>,
    pub(crate) statement_children: AppendOnlyHandleList<StatementHandle>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct DefinitionBodySourceMap {
    pub(crate) expressions: SideHandleMap<ExpressionHandle, RedNodeTag>,
    pub(crate) statements: SideHandleMap<StatementHandle, RedNodeTag>,
    pub(crate) local_bindings: SideHandleMap<LocalBindingHandle, RedNodeTag>,
    pub(crate) type_annotations: SideHandleMap<TypeAnnotationHandle, RedNodeTag>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum Expression<'db> {
    Unit,
    Integer(u128),
    Boolean(bool),
    Path(Path<'db>),
    If {
        condition: ExpressionHandle,
        then_branch: ExpressionHandle,
        else_branch: Option<ExpressionHandle>,
    },
    Block {
        // `Some` only when this block has nested definitions inside it
        key: Option<BlockKey<'db>>,
        statements: HandleRange<StatementHandle>,
        tail: Option<ExpressionHandle>,
    },
    Loop {
        source: LoopSource,
        body: ExpressionHandle,
    },
    Call {
        callee: ExpressionHandle,
        arguments: HandleRange<ExpressionHandle>,
    },
    Continue,
    Break {
        value: Option<ExpressionHandle>,
    },
    Return {
        value: Option<ExpressionHandle>,
    },
    UnaryOperation {
        operator: UnaryOperator,
        operand: ExpressionHandle,
    },
    BinaryOperation {
        lhs: ExpressionHandle,
        operator: Option<BinaryOperator>,
        rhs: ExpressionHandle,
    },
    Assignment {
        target: ExpressionHandle,
        value: ExpressionHandle,
    },
    Hole,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct Path<'db> {
    pub(crate) segments: Vec<Symbol<'db>>,
}

impl<'db> Path<'db> {
    pub(crate) fn lower(db: &'db dyn crate::Db, path: &ast::Path) -> Option<Self> {
        let mut segments = Vec::new();
        let mut current = Some(path.clone());
        while let Some(path) = current {
            let name = path.segment()?.name()?;
            segments.push(Symbol::new(db, name.lexeme().to_string()));
            current = path.qualifier();
        }
        segments.reverse();
        Some(Self { segments })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum Statement {
    Let {
        name: LocalBindingHandle,
        annotation: Option<TypeAnnotationHandle>,
        initializer: Option<ExpressionHandle>,
    },
    Expression {
        expression: ExpressionHandle,
        has_semicolon: bool,
    },
    Definition,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct LocalBinding<'db> {
    pub(crate) name: Symbol<'db>,
    pub(crate) mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum TypeAnnotation<'db> {
    Path(Symbol<'db>),
    Hole,
}

impl<'db> TypeAnnotation<'db> {
    pub(crate) fn from_type_expression(
        db: &'db dyn crate::Db,
        type_expression: &ast::TypeExpression,
    ) -> Self {
        match type_expression.name() {
            Some(token) => Self::Path(Symbol::new(db, token.lexeme().to_string())),
            None => Self::Hole,
        }
    }

    pub(crate) fn to_ty(&self, db: &'db dyn crate::Db) -> Ty<'db> {
        match self {
            Self::Path(symbol) => {
                Ty::primitive(db, symbol.text(db)).unwrap_or_else(|| Ty::error(db))
            }
            Self::Hole => Ty::error(db),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum LoopSource {
    Loop,
    While,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum UnaryOperator {
    Neg,
    Not,
}

handle_impl!(pub(crate) StatementHandle);

handle_impl!(pub(crate) ExpressionHandle);

handle_impl!(pub(crate) TypeAnnotationHandle);

handle_impl!(pub(crate) LocalBindingHandle);

impl TryFrom<SyntaxKind> for BinaryOperator {
    type Error = SyntaxKind;

    fn try_from(kind: SyntaxKind) -> Result<Self, Self::Error> {
        Ok(match kind {
            SyntaxKind::Plus => Self::Add,
            SyntaxKind::Minus => Self::Sub,
            SyntaxKind::Star => Self::Mul,
            SyntaxKind::Slash => Self::Div,
            SyntaxKind::LessThan => Self::Lt,
            SyntaxKind::GreaterThan => Self::Gt,
            SyntaxKind::LessEqual => Self::Le,
            SyntaxKind::GreaterEqual => Self::Ge,
            SyntaxKind::EqualEqual => Self::Eq,
            SyntaxKind::NotEqual => Self::Ne,
            SyntaxKind::LogicalAnd => Self::And,
            SyntaxKind::LogicalOr => Self::Or,
            other => return Err(other),
        })
    }
}

impl TryFrom<SyntaxKind> for UnaryOperator {
    type Error = SyntaxKind;

    fn try_from(kind: SyntaxKind) -> Result<Self, Self::Error> {
        Ok(match kind {
            SyntaxKind::Minus => Self::Neg,
            SyntaxKind::LogicalNot => Self::Not,
            other => return Err(other),
        })
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::And => "&&",
            Self::Or => "||",
        };
        write!(f, "{s}")
    }
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Neg => "-",
            Self::Not => "not",
        };
        write!(f, "{s}")
    }
}
