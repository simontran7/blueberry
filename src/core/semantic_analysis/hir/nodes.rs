use std::fmt;

use crate::core::common::handle_collections::handle_impl;
use crate::core::common::handle_collections::handle_map::{HandleMap, SideHandleMap};
use crate::core::common::runs::{Run, RunList};
use crate::core::common::span::{Span, TextSize};
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ids::keys::BlockKey;
use crate::core::syntactic_analysis::cst::{RedNodeTag, SyntaxKind};

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum Signature<'db> {
    Function(FunctionSignature<'db>),
    Constant(ConstantSignature<'db>),
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct FunctionSignature<'db> {
    pub(crate) name: Symbol<'db>,
    pub(crate) store: HirNodeStore<'db>,
    pub(crate) parameter_annotation_handles: Box<[TypeExpressionHandle]>,
    pub(crate) return_type_annotation: Option<TypeExpressionHandle>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct ConstantSignature<'db> {
    pub(crate) name: Symbol<'db>,
    pub(crate) store: HirNodeStore<'db>,
    pub(crate) annotation: Option<TypeExpressionHandle>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct DefinitionBody<'db> {
    pub(crate) store: HirNodeStore<'db>,
    pub(crate) root_handle: ExpressionHandle,
    pub(crate) parameter_handles: Box<[LocalBindingHandle]>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct HirNodeStore<'db> {
    pub(crate) expressions: HandleMap<ExpressionHandle, Expression<'db>>,
    pub(crate) statements: HandleMap<StatementHandle, Statement>,
    pub(crate) local_bindings: HandleMap<LocalBindingHandle, LocalBinding<'db>>,
    pub(crate) type_expressions: HandleMap<TypeExpressionHandle, TypeExpression<'db>>,

    pub(crate) expression_children: RunList<ExpressionHandle>,
    pub(crate) statement_children: RunList<StatementHandle>,
    pub(crate) type_expression_children: RunList<TypeExpressionHandle>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct HirNodeSourceMap {
    pub(crate) expressions: SideHandleMap<ExpressionHandle, RedNodeTag>,
    pub(crate) statements: SideHandleMap<StatementHandle, RedNodeTag>,
    pub(crate) local_bindings: SideHandleMap<LocalBindingHandle, RedNodeTag>,
    pub(crate) type_expressions: SideHandleMap<TypeExpressionHandle, RedNodeTag>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum Expression<'db> {
    Integer(Option<u128>),
    Boolean(bool),
    Path(Path<'db>),
    Tuple {
        element_run: Run<ExpressionHandle>,
    },
    Field {
        receiver_handle: ExpressionHandle,
        name: Option<Symbol<'db>>,
    },
    If {
        condition_handle: ExpressionHandle,
        then_branch_handle: ExpressionHandle,
        else_branch_handle: Option<ExpressionHandle>,
    },
    Block {
        block_key: Option<BlockKey<'db>>, // `Some` only when this block has nested definitions inside it
        statement_run: Run<StatementHandle>,
        tail_handle: Option<ExpressionHandle>,
    },
    Loop {
        source: LoopSource,
        body_handle: ExpressionHandle,
    },
    Call {
        callee_handle: ExpressionHandle,
        argument_run: Run<ExpressionHandle>,
    },
    Continue,
    Break {
        value_handle: Option<ExpressionHandle>,
    },
    Return {
        value_handle: Option<ExpressionHandle>,
    },
    UnaryOperation {
        operator: UnaryOperator,
        operand_handle: ExpressionHandle,
    },
    BinaryOperation {
        lhs_handle: ExpressionHandle,
        operator: Option<BinaryOperator>,
        rhs_handle: ExpressionHandle,
    },
    Assignment {
        target_handle: ExpressionHandle,
        value_handle: ExpressionHandle,
    },
    Error,
}

#[salsa::interned(debug)]
pub(crate) struct Path<'db> {
    #[returns(deref)]
    pub(crate) segments: Box<[Symbol<'db>]>,
}

pub(crate) struct TypeExpressionDisplay<'a, 'db> {
    store: &'a HirNodeStore<'db>,
    type_expression_handle: TypeExpressionHandle,
    db: &'db dyn crate::Db,
}

pub(crate) struct PathDisplay<'db> {
    path: Path<'db>,
    db: &'db dyn crate::Db,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum Statement {
    Let {
        name_handle: LocalBindingHandle,
        annotation_handle: Option<TypeExpressionHandle>,
        initializer_handle: ExpressionHandle,
    },
    Expression {
        expression_handle: ExpressionHandle,
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
pub(crate) enum TypeExpression<'db> {
    Path(Path<'db>),
    Tuple(Run<TypeExpressionHandle>),
    Error,
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

handle_impl!(pub(crate) TypeExpressionHandle);

handle_impl!(pub(crate) LocalBindingHandle);

impl HirNodeSourceMap {
    pub(crate) fn expression_span(
        &self,
        expression_handle: ExpressionHandle,
        anchor: TextSize,
    ) -> Span {
        self.expressions[expression_handle].span.anchored_at(anchor)
    }

    pub(crate) fn type_expression_span(
        &self,
        type_expression_handle: TypeExpressionHandle,
        anchor: TextSize,
    ) -> Span {
        self.type_expressions[type_expression_handle]
            .span
            .anchored_at(anchor)
    }
}

impl<'db> Signature<'db> {
    pub(crate) fn name(&self) -> Symbol<'db> {
        match self {
            Self::Function(function) => function.name,
            Self::Constant(constant) => constant.name,
        }
    }

    pub(crate) fn store(&self) -> &HirNodeStore<'db> {
        match self {
            Self::Function(function) => &function.store,
            Self::Constant(constant) => &constant.store,
        }
    }

    pub(crate) fn annotation_handles(&self) -> impl Iterator<Item = TypeExpressionHandle> + '_ {
        let (parameter_annotation_handles, annotation_handle): (&[TypeExpressionHandle], _) =
            match self {
                Self::Function(function) => (
                    &function.parameter_annotation_handles,
                    function.return_type_annotation,
                ),
                Self::Constant(constant) => (&[], constant.annotation),
            };
        parameter_annotation_handles
            .iter()
            .copied()
            .chain(annotation_handle)
    }
}

impl<'db> HirNodeStore<'db> {
    pub(crate) fn display_type_expression<'a>(
        &'a self,
        type_expression_handle: TypeExpressionHandle,
        db: &'db dyn crate::Db,
    ) -> TypeExpressionDisplay<'a, 'db> {
        TypeExpressionDisplay {
            store: self,
            type_expression_handle,
            db,
        }
    }
}

impl<'db> Path<'db> {
    pub(crate) fn display(self, db: &'db dyn crate::Db) -> PathDisplay<'db> {
        PathDisplay { path: self, db }
    }
}

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

impl fmt::Display for TypeExpressionDisplay<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.store.type_expressions[self.type_expression_handle] {
            TypeExpression::Path(path) => write!(f, "{}", path.display(self.db)),
            TypeExpression::Tuple(element_run) => {
                let element_handles = &self.store.type_expression_children[*element_run];
                f.write_str("(")?;
                for (index, &element_handle) in element_handles.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(
                        f,
                        "{}",
                        self.store.display_type_expression(element_handle, self.db)
                    )?;
                }
                if element_handles.len() == 1 {
                    f.write_str(",")?;
                }
                f.write_str(")")
            }
            TypeExpression::Error => f.write_str("_"),
        }
    }
}

impl fmt::Display for PathDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, segment) in self.path.segments(self.db).iter().enumerate() {
            if index > 0 {
                f.write_str("::")?;
            }
            f.write_str(segment.text(self.db))?;
        }
        Ok(())
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

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Neg => "-",
            Self::Not => "not",
        };
        write!(f, "{s}")
    }
}
