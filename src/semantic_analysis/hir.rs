use std::fmt;
use std::marker::PhantomData;

use handlemap::HandleMap;

use crate::common::span::Span;
use crate::common::string_interner::Symbol;
use crate::common::types::TypeId;
use crate::front_end::syntactic_analysis::cst::operators::{BinOp, UnOp};

pub(crate) struct Hir {
    pub(crate) source_file: SourceFileNode,

    pub(crate) definitions: HandleMap<DefinitionId, Definition>,
    pub(crate) statements: HandleMap<StatementId, Statement>,
    pub(crate) expressions: HandleMap<ExpressionId, Expression>,

    pub(crate) definition_children_ids: Vec<DefinitionId>,
    pub(crate) statement_children_ids: Vec<StatementId>,
    pub(crate) expression_children_ids: Vec<ExpressionId>,
    pub(crate) parameter_children_ids: Vec<LocalBindingId>,

    pub(crate) local_bindings: HandleMap<LocalBindingId, LocalBinding>,
    pub(crate) definition_bindings: HandleMap<DefinitionBindingId, DefinitionBinding>,
}

pub(crate) struct SourceFileNode {
    pub(crate) definition_id_span: DefinitionIdSpan,
    pub(crate) span: Span,
}

#[derive(Debug)]
pub(crate) struct Definition {
    pub(crate) kind: DefinitionKind,
    pub(crate) span: Span,
}

#[derive(Debug)]
pub(crate) struct Statement {
    pub(crate) kind: StatementKind,
    pub(crate) span: Span,
}

#[derive(Debug)]
pub(crate) struct Expression {
    pub(crate) kind: ExpressionKind,
    pub(crate) ty: TypeId,
    pub(crate) span: Span,
}

#[derive(Debug)]
pub(crate) enum DefinitionKind {
    Function {
        definition_binding_id: DefinitionBindingId,
        parameter_id_span: ParameterIdSpan,
        body_id: ExpressionId,
    },
    Constant {
        definition_binding_id: DefinitionBindingId,
        initializer_id: ExpressionId,
    },
}

#[derive(Debug)]
pub(crate) enum StatementKind {
    Expression {
        expression_id: ExpressionId,
        has_semicolon: bool,
    },
    Let {
        pattern_id: LocalBindingId,
        value_id: Option<ExpressionId>,
    },
    Definition {
        definition_id: DefinitionId,
    },
}

#[derive(Debug)]
pub(crate) enum ExpressionKind {
    /// Produced in place of an expression the CST couldn't provide -- a
    /// `NodeKind::Error` node from parser recovery. Carries the error type,
    /// same as any other unresolved expression, so it unifies with anything
    /// without cascading further diagnostics.
    Missing,
    Unit,
    Integer(u128),
    Boolean(bool),
    Variable(BindingId),
    Unary {
        operator: UnOp,
        operand_id: ExpressionId,
    },
    Binary {
        operator: BinOp,
        lhs_id: ExpressionId,
        rhs_id: ExpressionId,
    },
    If {
        condition_id: ExpressionId,
        then_branch_id: ExpressionId,
        else_branch_id: Option<ExpressionId>,
    },
    Block {
        statement_id_span: StatementIdSpan,
        tail_id: Option<ExpressionId>,
    },
    Call {
        callee_id: ExpressionId,
        argument_id_span: ExpressionIdSpan,
    },
    Assign {
        target_id: ExpressionId,
        value_id: ExpressionId,
    },
    Return {
        value_id: Option<ExpressionId>,
    },
    Loop {
        body_id: ExpressionId,
        source: LoopSource,
    },
    Break {
        value_id: Option<ExpressionId>,
    },
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopSource {
    Loop,
    While,
}

impl LoopSource {
    pub(crate) const fn keyword(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::While => "while",
        }
    }

    pub(crate) const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::Loop => "infinite loop",
            Self::While => "while loop",
        }
    }
}

// Opaque, 4-byte handles into the tables in `Hir`.
handlemap::handle_impl!(pub(crate) DefinitionId);
handlemap::handle_impl!(pub(crate) StatementId);
handlemap::handle_impl!(pub(crate) ExpressionId);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DefinitionIdSpan {
    pub(crate) start: u32,
    pub(crate) len: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StatementIdSpan {
    pub(crate) start: u32,
    pub(crate) len: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExpressionIdSpan {
    pub(crate) start: u32,
    pub(crate) len: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ParameterIdSpan {
    pub(crate) start: u32,
    pub(crate) len: u32,
}

#[derive(Debug)]
pub(crate) struct LocalBinding {
    pub(crate) name: Symbol,
    pub(crate) mutable: bool,
    pub(crate) annotation: Option<TypeId>,
    pub(crate) ty: TypeId,
    pub(crate) span: Span,
}

#[derive(Debug)]
pub(crate) struct DefinitionBinding {
    pub(crate) name: Symbol,
    pub(crate) ty: TypeId,
    pub(crate) span: Span,
}

pub(crate) type LocalBindingId = TypedBindingId<LocalBinding, { BindingKind::Local as u8 }>;
pub(crate) type DefinitionBindingId =
    TypedBindingId<DefinitionBinding, { BindingKind::Definition as u8 }>;

// Clone/Copy/PartialEq/Eq/Handle are all manual (no derive) because derive
// adds unwanted bounds like `T: Clone`, but T is purely a phantom marker
// (the real data is just a `u32`).

pub(crate) struct TypedBindingId<T, const KIND: u8>(u32, PhantomData<T>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum BindingKind {
    Local = 0,
    Definition,
}

pub(crate) struct DefinitionView<'a> {
    definition_id: DefinitionId,
    hir: &'a Hir,
}

pub(crate) struct StatementView<'a> {
    statement_id: StatementId,
    hir: &'a Hir,
}

pub(crate) struct ExpressionView<'a> {
    expression_id: ExpressionId,
    hir: &'a Hir,
}

pub(crate) struct LocalBindingView<'a> {
    local_binding_id: LocalBindingId,
    hir: &'a Hir,
}

pub(crate) struct DefinitionBindingView<'a> {
    definition_binding_id: DefinitionBindingId,
    hir: &'a Hir,
}

impl Hir {
    pub(crate) fn new(source_size: usize) -> Self {
        Self {
            source_file: SourceFileNode {
                definition_id_span: DefinitionIdSpan { start: 0, len: 0 },
                span: Span::new(0_u32, source_size as u32),
            },
            definitions: HandleMap::new(),
            statements: HandleMap::new(),
            expressions: HandleMap::new(),
            definition_children_ids: Vec::new(),
            statement_children_ids: Vec::new(),
            expression_children_ids: Vec::new(),
            parameter_children_ids: Vec::new(),
            local_bindings: HandleMap::new(),
            definition_bindings: HandleMap::new(),
        }
    }

    pub(crate) fn get_definition_ids(
        &self,
        definition_id_span: DefinitionIdSpan,
    ) -> &[DefinitionId] {
        &self.definition_children_ids[definition_id_span.start as usize
            ..(definition_id_span.start + definition_id_span.len) as usize]
    }

    pub(crate) fn get_statement_ids(&self, statement_id_span: StatementIdSpan) -> &[StatementId] {
        &self.statement_children_ids[statement_id_span.start as usize
            ..(statement_id_span.start + statement_id_span.len) as usize]
    }

    pub(crate) fn get_expression_ids(
        &self,
        expression_id_span: ExpressionIdSpan,
    ) -> &[ExpressionId] {
        &self.expression_children_ids[expression_id_span.start as usize
            ..(expression_id_span.start + expression_id_span.len) as usize]
    }

    pub(crate) fn get_parameter_binding_ids(
        &self,
        parameter_id_span: ParameterIdSpan,
    ) -> &[LocalBindingId] {
        &self.parameter_children_ids[parameter_id_span.start as usize
            ..(parameter_id_span.start + parameter_id_span.len) as usize]
    }

    pub(crate) fn functions_ids(&self) -> impl Iterator<Item = DefinitionId> + '_ {
        self.definitions
            .iter()
            .filter(|(_, definition)| matches!(definition.kind, DefinitionKind::Function { .. }))
            .map(|(definition_id, _)| definition_id)
    }

    pub(crate) fn add_definition(&mut self, kind: DefinitionKind, span: Span) -> DefinitionId {
        self.definitions.add(Definition { kind, span })
    }

    pub(crate) fn add_statement(&mut self, kind: StatementKind, span: Span) -> StatementId {
        self.statements.add(Statement { kind, span })
    }

    pub(crate) fn add_expression(
        &mut self,
        kind: ExpressionKind,
        ty: TypeId,
        span: Span,
    ) -> ExpressionId {
        self.expressions.add(Expression { kind, ty, span })
    }

    pub(crate) fn add_definition_ids(
        &mut self,
        definition_ids: &[DefinitionId],
    ) -> DefinitionIdSpan {
        let start = self.definition_children_ids.len() as u32;
        self.definition_children_ids
            .extend_from_slice(definition_ids);
        DefinitionIdSpan {
            start,
            len: definition_ids.len() as u32,
        }
    }

    pub(crate) fn add_statement_ids(&mut self, statement_ids: &[StatementId]) -> StatementIdSpan {
        let start = self.statement_children_ids.len() as u32;
        self.statement_children_ids.extend_from_slice(statement_ids);
        StatementIdSpan {
            start,
            len: statement_ids.len() as u32,
        }
    }

    pub(crate) fn add_expression_ids(
        &mut self,
        expression_ids: &[ExpressionId],
    ) -> ExpressionIdSpan {
        let start = self.expression_children_ids.len() as u32;
        self.expression_children_ids
            .extend_from_slice(expression_ids);
        ExpressionIdSpan {
            start,
            len: expression_ids.len() as u32,
        }
    }

    pub(crate) fn add_parameter_ids(
        &mut self,
        parameter_ids: &[LocalBindingId],
    ) -> ParameterIdSpan {
        let start = self.parameter_children_ids.len() as u32;
        self.parameter_children_ids.extend_from_slice(parameter_ids);
        ParameterIdSpan {
            start,
            len: parameter_ids.len() as u32,
        }
    }

    pub(crate) fn add_local_binding(
        &mut self,
        name: Symbol,
        mutable: bool,
        annotation: Option<TypeId>,
        ty: TypeId,
        span: Span,
    ) -> LocalBindingId {
        self.local_bindings.add(LocalBinding {
            name,
            mutable,
            annotation,
            ty,
            span,
        })
    }

    pub(crate) fn add_definition_binding(
        &mut self,
        name: Symbol,
        ty: TypeId,
        span: Span,
    ) -> DefinitionBindingId {
        self.definition_bindings
            .add(DefinitionBinding { name, ty, span })
    }

    pub(crate) fn get_definition(&self, definition_id: DefinitionId) -> DefinitionView<'_> {
        DefinitionView {
            definition_id,
            hir: self,
        }
    }

    pub(crate) fn get_statement(&self, statement_id: StatementId) -> StatementView<'_> {
        StatementView {
            statement_id,
            hir: self,
        }
    }

    pub(crate) fn get_expression(&self, expression_id: ExpressionId) -> ExpressionView<'_> {
        ExpressionView {
            expression_id,
            hir: self,
        }
    }

    pub(crate) fn get_local_binding(
        &self,
        local_binding_id: LocalBindingId,
    ) -> LocalBindingView<'_> {
        LocalBindingView {
            local_binding_id,
            hir: self,
        }
    }

    pub(crate) fn get_definition_binding(
        &self,
        definition_binding_id: DefinitionBindingId,
    ) -> DefinitionBindingView<'_> {
        DefinitionBindingView {
            definition_binding_id,
            hir: self,
        }
    }
}

impl<'a> DefinitionView<'a> {
    pub(crate) fn id(&self) -> DefinitionId {
        self.definition_id
    }

    pub(crate) fn kind(&self) -> &'a DefinitionKind {
        &self.hir.definitions[self.definition_id].kind
    }

    pub(crate) fn span(&self) -> Span {
        self.hir.definitions[self.definition_id].span
    }
}

impl<'a> StatementView<'a> {
    pub(crate) fn id(&self) -> StatementId {
        self.statement_id
    }

    pub(crate) fn kind(&self) -> &'a StatementKind {
        &self.hir.statements[self.statement_id].kind
    }

    pub(crate) fn span(&self) -> Span {
        self.hir.statements[self.statement_id].span
    }
}

impl<'a> ExpressionView<'a> {
    pub(crate) fn id(&self) -> ExpressionId {
        self.expression_id
    }

    pub(crate) fn kind(&self) -> &'a ExpressionKind {
        &self.hir.expressions[self.expression_id].kind
    }

    pub(crate) fn ty(&self) -> TypeId {
        self.hir.expressions[self.expression_id].ty
    }

    pub(crate) fn span(&self) -> Span {
        self.hir.expressions[self.expression_id].span
    }
}

impl<'a> LocalBindingView<'a> {
    pub(crate) fn id(&self) -> LocalBindingId {
        self.local_binding_id
    }

    pub(crate) fn name(&self) -> Symbol {
        self.hir.local_bindings[self.local_binding_id].name
    }

    pub(crate) fn mutable(&self) -> bool {
        self.hir.local_bindings[self.local_binding_id].mutable
    }

    pub(crate) fn annotation(&self) -> Option<TypeId> {
        self.hir.local_bindings[self.local_binding_id].annotation
    }

    pub(crate) fn ty(&self) -> TypeId {
        self.hir.local_bindings[self.local_binding_id].ty
    }

    pub(crate) fn span(&self) -> Span {
        self.hir.local_bindings[self.local_binding_id].span
    }
}

impl<'a> DefinitionBindingView<'a> {
    pub(crate) fn id(&self) -> DefinitionBindingId {
        self.definition_binding_id
    }

    pub(crate) fn name(&self) -> Symbol {
        self.hir.definition_bindings[self.definition_binding_id].name
    }

    pub(crate) fn ty(&self) -> TypeId {
        self.hir.definition_bindings[self.definition_binding_id].ty
    }

    pub(crate) fn span(&self) -> Span {
        self.hir.definition_bindings[self.definition_binding_id].span
    }
}

impl<T, const KIND: u8> TypedBindingId<T, KIND> {
    pub(crate) const ERROR: Self = Self(u32::MAX, PhantomData);

    pub(crate) const fn new(index: usize) -> Self {
        Self(index as u32, PhantomData)
    }

    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) const fn is_error(self) -> bool {
        self.0 == u32::MAX
    }
}

impl<T, const KIND: u8> handlemap::handle_map::Handle for TypedBindingId<T, KIND> {
    fn new(index: usize) -> Self {
        Self(index as u32, PhantomData)
    }
    fn index(&self) -> usize {
        self.0 as usize
    }
}

impl<T, const KIND: u8> Clone for TypedBindingId<T, KIND> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, const KIND: u8> Copy for TypedBindingId<T, KIND> {}
impl<T, const KIND: u8> PartialEq for TypedBindingId<T, KIND> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T, const KIND: u8> Eq for TypedBindingId<T, KIND> {}
impl<T, const KIND: u8> std::hash::Hash for TypedBindingId<T, KIND> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl<T, const KIND: u8> From<usize> for TypedBindingId<T, KIND> {
    fn from(index: usize) -> Self {
        Self::new(index)
    }
}
impl<T, const KIND: u8> From<TypedBindingId<T, KIND>> for usize {
    fn from(id: TypedBindingId<T, KIND>) -> Self {
        id.index()
    }
}
impl<T, const KIND: u8> fmt::Debug for TypedBindingId<T, KIND> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BindingId({})", self.0)
    }
}

impl BindingId {
    const INDEX_BITS: u32 = 31;
    const KIND_MASK: u32 = 0b1 << Self::INDEX_BITS;
    const INDEX_MASK: u32 = (1 << Self::INDEX_BITS) - 1;

    pub(crate) const ERROR: Self = Self(u32::MAX);

    pub(crate) const fn is_error(self) -> bool {
        self.0 == u32::MAX
    }

    pub(crate) fn kind(self) -> BindingKind {
        assert!(!self.is_error(), "called `kind()` on an error BindingId");
        match (self.0 & Self::KIND_MASK) >> Self::INDEX_BITS {
            0 => BindingKind::Local,
            _ => BindingKind::Definition,
        }
    }

    pub(crate) fn index(self) -> usize {
        assert!(!self.is_error(), "called `index()` on an error BindingId");
        (self.0 & Self::INDEX_MASK) as usize
    }

    pub(crate) fn as_local(self) -> Option<LocalBindingId> {
        if !self.is_error() && self.kind() == BindingKind::Local {
            Some(LocalBindingId::new(self.index()))
        } else {
            None
        }
    }

    pub(crate) fn as_definition(self) -> Option<DefinitionBindingId> {
        if !self.is_error() && self.kind() == BindingKind::Definition {
            Some(DefinitionBindingId::new(self.index()))
        } else {
            None
        }
    }

    fn new(kind: u8, index: usize) -> Self {
        assert!(
            index <= Self::INDEX_MASK as usize,
            "Index too large for 31-bit storage"
        );
        Self(u32::from(kind) << Self::INDEX_BITS | index as u32)
    }
}

impl<T, const KIND: u8> From<TypedBindingId<T, KIND>> for BindingId {
    fn from(typed: TypedBindingId<T, KIND>) -> Self {
        Self::new(KIND, typed.index())
    }
}
