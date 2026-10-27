use std::fmt;
use std::ops::Index;

use crate::core::common::handle_collections::handle_impl;
use crate::core::common::handle_collections::handle_map::{HandleMap, SideHandleMap};
use crate::core::common::runs::{Run, RunList};
use crate::core::common::span::{Span, TextSize};
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::keys::BlockKey;
use crate::core::syntactic_analysis::cst::{RedNodeTag, SyntaxKind};

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) enum Signature<'db> {
    Function(FunctionSignature<'db>),
    Constant(ConstantSignature<'db>),
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) struct FunctionSignature<'db> {
    pub(in crate::core) name: Symbol<'db>,
    pub(in crate::core) store: HirNodeStore<'db>,
    pub(in crate::core) parameter_annotation_handles: Box<[TypeExpressionHandle]>,
    pub(in crate::core) return_type_annotation: Option<TypeExpressionHandle>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) struct ConstantSignature<'db> {
    pub(in crate::core) name: Symbol<'db>,
    pub(in crate::core) store: HirNodeStore<'db>,
    pub(in crate::core) annotation: Option<TypeExpressionHandle>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) struct Body<'db> {
    pub(in crate::core) store: HirNodeStore<'db>,
    pub(in crate::core) root_handle: ExpressionHandle,
    pub(in crate::core) parameter_handles: Box<[LocalBindingHandle]>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) struct HirNodeStore<'db> {
    pub(in crate::core::semantic_analysis::ast_lowering) expressions:
        HandleMap<ExpressionHandle, Expression<'db>>,
    pub(in crate::core::semantic_analysis::ast_lowering) statements:
        HandleMap<StatementHandle, Statement>,
    pub(in crate::core::semantic_analysis::ast_lowering) local_bindings:
        HandleMap<LocalBindingHandle, LocalBinding<'db>>,
    pub(in crate::core::semantic_analysis::ast_lowering) type_expressions:
        HandleMap<TypeExpressionHandle, TypeExpression<'db>>,

    pub(in crate::core::semantic_analysis::ast_lowering) expression_children:
        RunList<ExpressionHandle>,
    pub(in crate::core::semantic_analysis::ast_lowering) statement_children:
        RunList<StatementHandle>,
    pub(in crate::core::semantic_analysis::ast_lowering) type_expression_children:
        RunList<TypeExpressionHandle>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) struct HirNodeSourceMap {
    pub(in crate::core::semantic_analysis::ast_lowering) expressions:
        SideHandleMap<ExpressionHandle, RedNodeTag>,
    pub(in crate::core::semantic_analysis::ast_lowering) statements:
        SideHandleMap<StatementHandle, RedNodeTag>,
    pub(in crate::core::semantic_analysis::ast_lowering) local_bindings:
        SideHandleMap<LocalBindingHandle, RedNodeTag>,
    pub(in crate::core::semantic_analysis::ast_lowering) type_expressions:
        SideHandleMap<TypeExpressionHandle, RedNodeTag>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) enum Expression<'db> {
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
pub(in crate::core) struct Path<'db> {
    #[returns(deref)]
    pub(in crate::core) segments: Box<[Symbol<'db>]>,
}

pub(in crate::core) struct TypeExpressionDisplay<'a, 'db> {
    store: &'a HirNodeStore<'db>,
    type_expression_handle: TypeExpressionHandle,
    db: &'db dyn crate::Db,
}

pub(in crate::core) struct PathDisplay<'db> {
    path: Path<'db>,
    db: &'db dyn crate::Db,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) enum Statement {
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
pub(in crate::core) struct LocalBinding<'db> {
    pub(in crate::core) name: Symbol<'db>,
    pub(in crate::core) mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) enum TypeExpression<'db> {
    Path(Path<'db>),
    Tuple(Run<TypeExpressionHandle>),
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) enum LoopSource {
    Loop,
    While,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
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
pub(in crate::core) enum UnaryOperator {
    Negation,
    Not,
}

handle_impl!(pub(in crate::core) StatementHandle);

handle_impl!(pub(in crate::core) ExpressionHandle);

handle_impl!(pub(in crate::core) TypeExpressionHandle);

handle_impl!(pub(in crate::core) LocalBindingHandle);

impl HirNodeSourceMap {
    pub(in crate::core) fn expression_at(&self, offset: TextSize) -> Option<ExpressionHandle> {
        self.expressions
            .iter()
            .filter(|(_, tag)| tag.span.contains_offset(offset))
            .min_by_key(|(_, tag)| tag.span.len())
            .map(|(expression_handle, _)| expression_handle)
    }

    pub(in crate::core) fn local_binding_span(
        &self,
        binding_handle: LocalBindingHandle,
        anchor: TextSize,
    ) -> Span {
        self.local_bindings[binding_handle].span.anchored_at(anchor)
    }

    pub(in crate::core) fn expression_span(
        &self,
        expression_handle: ExpressionHandle,
        anchor: TextSize,
    ) -> Span {
        self.expressions[expression_handle].span.anchored_at(anchor)
    }

    pub(in crate::core) fn type_expression_span(
        &self,
        type_expression_handle: TypeExpressionHandle,
        anchor: TextSize,
    ) -> Span {
        self.type_expressions[type_expression_handle]
            .span
            .anchored_at(anchor)
    }
}

impl<'db> HirNodeStore<'db> {
    pub(in crate::core) fn expressions(
        &self,
    ) -> impl Iterator<Item = (ExpressionHandle, &Expression<'db>)> + '_ {
        self.expressions.iter()
    }
}

impl<'db> Signature<'db> {
    pub(in crate::core) fn name(&self) -> Symbol<'db> {
        match self {
            Self::Function(function) => function.name,
            Self::Constant(constant) => constant.name,
        }
    }

    pub(in crate::core) fn store(&self) -> &HirNodeStore<'db> {
        match self {
            Self::Function(function) => &function.store,
            Self::Constant(constant) => &constant.store,
        }
    }

    pub(in crate::core) fn annotation_handles(
        &self,
    ) -> impl Iterator<Item = TypeExpressionHandle> + '_ {
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
    pub(in crate::core) fn display_type_expression<'a>(
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
    pub(in crate::core) fn display(self, db: &'db dyn crate::Db) -> PathDisplay<'db> {
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
            SyntaxKind::Div => Self::Div,
            SyntaxKind::Mod => Self::Mod,
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

impl<'db> Index<ExpressionHandle> for HirNodeStore<'db> {
    type Output = Expression<'db>;

    fn index(&self, handle: ExpressionHandle) -> &Expression<'db> {
        &self.expressions[handle]
    }
}

impl<'db> Index<StatementHandle> for HirNodeStore<'db> {
    type Output = Statement;

    fn index(&self, handle: StatementHandle) -> &Statement {
        &self.statements[handle]
    }
}

impl<'db> Index<LocalBindingHandle> for HirNodeStore<'db> {
    type Output = LocalBinding<'db>;

    fn index(&self, handle: LocalBindingHandle) -> &LocalBinding<'db> {
        &self.local_bindings[handle]
    }
}

impl<'db> Index<TypeExpressionHandle> for HirNodeStore<'db> {
    type Output = TypeExpression<'db>;

    fn index(&self, handle: TypeExpressionHandle) -> &TypeExpression<'db> {
        &self.type_expressions[handle]
    }
}

impl<'db> Index<Run<ExpressionHandle>> for HirNodeStore<'db> {
    type Output = [ExpressionHandle];

    fn index(&self, run: Run<ExpressionHandle>) -> &[ExpressionHandle] {
        &self.expression_children[run]
    }
}

impl<'db> Index<Run<StatementHandle>> for HirNodeStore<'db> {
    type Output = [StatementHandle];

    fn index(&self, run: Run<StatementHandle>) -> &[StatementHandle] {
        &self.statement_children[run]
    }
}

impl<'db> Index<Run<TypeExpressionHandle>> for HirNodeStore<'db> {
    type Output = [TypeExpressionHandle];

    fn index(&self, run: Run<TypeExpressionHandle>) -> &[TypeExpressionHandle] {
        &self.type_expression_children[run]
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
            Self::Div => "div",
            Self::Mod => "mod",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::And => "and",
            Self::Or => "or",
        };
        write!(f, "{s}")
    }
}

impl TryFrom<SyntaxKind> for UnaryOperator {
    type Error = SyntaxKind;

    fn try_from(kind: SyntaxKind) -> Result<Self, Self::Error> {
        Ok(match kind {
            SyntaxKind::Minus => Self::Negation,
            SyntaxKind::LogicalNot => Self::Not,
            other => return Err(other),
        })
    }
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Negation => "-",
            Self::Not => "not",
        };
        write!(f, "{s}")
    }
}
