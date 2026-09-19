use std::fmt;

use crate::core::common::handlemap::{self, HandleMap, HandleRange};
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::red_node_directory::RedNodeId;
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

pub(crate) struct FunctionSignature<'db> {
    name: Symbol<'db>,
    parameters: Vec<Parameter<'db>>,
    return_type_annotation: Option<TypeAnnotation<'db>>,
}
pub(crate) struct ConstantSignature<'db> {
    name: Symbol<'db>,
    type_annotation: Option<TypeAnnotation<'db>>,
}
pub(crate) struct Parameter<'db> {
    name: Symbol<'db>,
    type_annotation: Option<TypeAnnotation<'db>>,
}
pub(crate) struct DefinitionBody<'db> {
    root: ExpressionHandle,
    expressions: HandleMap<ExpressionHandle, Expression<'db>>,
    statements: HandleMap<StatementHandle, Statement>,
    local_bindings: HandleMap<LocalBindingHandle, LocalBinding<'db>>,
    type_annotations: HandleMap<TypeAnnotationHandle, TypeAnnotation<'db>>,
}

enum Expression<'db> {
    Unit,
    Integer(u128),
    Boolean(bool),
    Path(Symbol<'db>),
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
        operand: ExpressionHandle,
        operator: UnaryOperator,
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

enum Statement {
    Let {
        name: LocalBindingHandle,
        annotation: Option<TypeAnnotationHandle>,
        initializer: Option<ExpressionHandle>,
    },
    Expression {
        expression: ExpressionHandle,
        has_semi: bool,
    },
    Definition,
}

struct LocalBinding<'db> {
    name: Symbol<'db>,
    mutable: bool,
}

enum TypeAnnotation<'db> {
    Path(Symbol<'db>),
    Hole,
}

enum LoopSource {
    Loop,
    While,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOperator {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnaryOperator {
    Neg,
    Not,
}

handlemap::handle_impl!(pub(crate) StatementHandle);

handlemap::handle_impl!(pub(crate) ExpressionHandle);

handlemap::handle_impl!(pub(crate) TypeAnnotationHandle);

handlemap::handle_impl!(pub(crate) LocalBindingHandle);

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
