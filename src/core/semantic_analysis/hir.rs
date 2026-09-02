use std::fmt;
use std::marker::PhantomData;

use handlemap::{HandleMap, SideHandleMap};

use crate::core::common::text_size::{TextRange, TextSize};
use crate::core::common::types::TypeId;
use crate::core::syntactic_analysis::cst::SyntaxKind;

/// Pure semantic data -- no spans anywhere in this type. This is what a
/// spans-free salsa query (nothing to invalidate on unrelated text shifts)
/// can return, mirroring rust-analyzer's `Body` (as opposed to
/// `(Body, BodySourceMap)`). See `HirSourceMaps` for the spans, and
/// `HirBuilder` for how the two get built together.
#[derive(Clone)]
pub(crate) struct Hir {
    pub(crate) source_file: SourceFileNode,
    pub(crate) definitions: HandleMap<DefinitionId, Definition>,
    pub(crate) definition_children_ids: Vec<DefinitionId>,
    pub(crate) definition_bindings: HandleMap<DefinitionBindingId, DefinitionBinding>,
    pub(crate) body: Body,
}

/// The span data stripped out of `Hir`, kept as a separate type so a query
/// that only needs semantics (not source locations) can depend on `Hir`
/// alone and skip whatever depends on this churning on every unrelated
/// edit -- mirroring rust-analyzer's `BodySourceMap`.
#[derive(Clone)]
pub(crate) struct HirSourceMaps {
    /// Absolute start offset of the top-level definition this `Hir`/`Body`
    /// was built for (set once via `HirBuilder::set_anchor`, before any
    /// definition/body node is added). Every span in `definition_spans` and
    /// `body_source_map` is stored relative to this anchor rather than
    /// absolute -- mirroring rust-analyzer's `Span`/`SpanAnchor`: an edit to
    /// an unrelated, earlier definition in the file shifts absolute
    /// offsets, but leaves this definition's own anchor-relative spans
    /// byte-identical.
    pub(crate) anchor: TextSize,

    /// Anchor-relative: every `Definition` here belongs to this `Hir`'s own
    /// focus definition (itself, or one of its nested local definitions),
    /// never a sibling top-level one.
    pub(crate) definition_spans: SideHandleMap<DefinitionId, TextRange>,

    /// Unlike `definition_spans`, this covers *every* top-level binding in
    /// the file (re-seeded on every `body_hir_of` call so name resolution
    /// can see sibling/forward references) -- so a `DuplicateDefinition`
    /// diagnostic compares one binding's span against a *different*
    /// definition's binding span. That cross-definition comparison needs
    /// file-absolute spans; they can't be relative to this `Hir`'s single
    /// `anchor`.
    pub(crate) definition_binding_spans: SideHandleMap<DefinitionBindingId, TextRange>,

    pub(crate) body_source_map: BodySourceMap,
}

impl HirSourceMaps {
    fn to_relative(&self, span: TextRange) -> TextRange {
        TextRange::new(span.start() - self.anchor, span.end() - self.anchor)
    }

    fn to_absolute(&self, span: TextRange) -> TextRange {
        TextRange::new(self.anchor + span.start(), self.anchor + span.end())
    }
}

#[derive(Clone)]
pub(crate) struct SourceFileNode {
    pub(crate) definition_id_span: DefinitionIdSpan,
}

/// The lowered statements, expressions, and local bindings belonging to a
/// single definition's body (a function's block, or a constant's
/// initializer). Ids here are only meaningful against this `Body`.
#[derive(Clone)]
pub(crate) struct Body {
    pub(crate) statements: HandleMap<StatementId, Statement>,
    pub(crate) expressions: HandleMap<ExpressionId, Expression>,
    pub(crate) local_bindings: HandleMap<LocalBindingId, LocalBinding>,

    pub(crate) statement_children_ids: Vec<StatementId>,
    pub(crate) expression_children_ids: Vec<ExpressionId>,
    pub(crate) parameter_children_ids: Vec<LocalBindingId>,
}

/// Anchor-relative spans (see `Hir::anchor`) for the nodes inside `body`,
/// kept separate from `Body` so the semantic nodes stay pure data.
#[derive(Clone)]
pub(crate) struct BodySourceMap {
    pub(crate) statement_spans: SideHandleMap<StatementId, TextRange>,
    pub(crate) expression_spans: SideHandleMap<ExpressionId, TextRange>,
    pub(crate) local_binding_spans: SideHandleMap<LocalBindingId, TextRange>,
}

#[derive(Clone, Debug)]
pub(crate) struct Definition {
    pub(crate) kind: DefinitionKind,
}

#[derive(Clone, Debug)]
pub(crate) struct Statement {
    pub(crate) kind: StatementKind,
}

#[derive(Clone, Debug)]
pub(crate) struct Expression {
    pub(crate) kind: ExpressionKind,
    pub(crate) ty: TypeId,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub(crate) enum ExpressionKind {
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
pub(crate) enum BinOp {
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

impl BinOp {
    pub(crate) fn from_syntax_kind(kind: SyntaxKind) -> Self {
        match kind {
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
            _ => unreachable!("not a binary operator token: {kind:?}"),
        }
    }
}

impl fmt::Display for BinOp {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnOp {
    Neg,
    Not,
}

impl UnOp {
    pub(crate) fn from_syntax_kind(kind: SyntaxKind) -> Self {
        match kind {
            SyntaxKind::Minus => Self::Neg,
            SyntaxKind::LogicalNot => Self::Not,
            _ => unreachable!("not a unary operator token: {kind:?}"),
        }
    }
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Neg => "-",
            Self::Not => "not",
        };
        write!(f, "{s}")
    }
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

// Opaque, 4-byte handles into the tables in `Hir`/`Body`.
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

#[derive(Clone, Debug)]
pub(crate) struct LocalBinding {
    pub(crate) name: String,
    pub(crate) mutable: bool,
    pub(crate) annotation: Option<TypeId>,
    pub(crate) ty: TypeId,
}

#[derive(Clone, Debug)]
pub(crate) struct DefinitionBinding {
    pub(crate) name: String,
    pub(crate) ty: TypeId,
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

// Every view holds a plain `&'a Hir` (always available) plus an optional
// `&'a HirSourceMaps` -- `Some` when constructed through a `HirBuilder`
// (during lowering, spans are available), `None` when constructed directly
// through a spans-free `Hir` (e.g. `HirDumper`, which never calls
// `text_range()`). `text_range()` panics if called on a `None` view --
// nothing does today, since span-consuming diagnostics are all built during
// lowering, while `HirBuilder` is still around.

pub(crate) struct DefinitionView<'a> {
    definition_id: DefinitionId,
    hir: &'a Hir,
    source_maps: Option<&'a HirSourceMaps>,
}

pub(crate) struct StatementView<'a> {
    statement_id: StatementId,
    hir: &'a Hir,
    source_maps: Option<&'a HirSourceMaps>,
}

pub(crate) struct ExpressionView<'a> {
    expression_id: ExpressionId,
    hir: &'a Hir,
    source_maps: Option<&'a HirSourceMaps>,
}

pub(crate) struct LocalBindingView<'a> {
    local_binding_id: LocalBindingId,
    hir: &'a Hir,
    source_maps: Option<&'a HirSourceMaps>,
}

pub(crate) struct DefinitionBindingView<'a> {
    definition_binding_id: DefinitionBindingId,
    hir: &'a Hir,
    source_maps: Option<&'a HirSourceMaps>,
}

/// Builds a `Hir` and its `HirSourceMaps` together (spans can only be
/// computed as nodes are lowered), then hands them back apart via
/// `finish`. Exposes the same `add_*`/`get_*` surface `Hir` used to have on
/// its own, so callers that only care about semantics (not the query
/// split) don't need to change.
pub(crate) struct HirBuilder {
    pub(crate) hir: Hir,
    pub(crate) source_maps: HirSourceMaps,
}

impl HirBuilder {
    pub(crate) fn new() -> Self {
        Self {
            hir: Hir {
                source_file: SourceFileNode {
                    definition_id_span: DefinitionIdSpan { start: 0, len: 0 },
                },
                definitions: HandleMap::new(),
                definition_children_ids: Vec::new(),
                definition_bindings: HandleMap::new(),
                body: Body {
                    statements: HandleMap::new(),
                    expressions: HandleMap::new(),
                    local_bindings: HandleMap::new(),
                    statement_children_ids: Vec::new(),
                    expression_children_ids: Vec::new(),
                    parameter_children_ids: Vec::new(),
                },
            },
            source_maps: HirSourceMaps {
                anchor: TextSize::new(0),
                definition_spans: SideHandleMap::new(),
                definition_binding_spans: SideHandleMap::new(),
                body_source_map: BodySourceMap {
                    statement_spans: SideHandleMap::new(),
                    expression_spans: SideHandleMap::new(),
                    local_binding_spans: SideHandleMap::new(),
                },
            },
        }
    }

    /// Splits the builder into its finished, independently-returnable
    /// halves -- called once, when a definition's body is fully lowered.
    pub(crate) fn finish(self) -> (Hir, HirSourceMaps) {
        (self.hir, self.source_maps)
    }

    /// Sets the absolute start offset that every subsequently-recorded
    /// definition/body span is stored relative to. Must be called once,
    /// before the first `add_definition`/`add_statement`/`add_expression`/
    /// `add_local_binding` call for this `Hir`'s focus definition --
    /// `add_definition_binding` is unaffected and may be called before or
    /// after (see the doc comment on `HirSourceMaps::definition_binding_spans`).
    pub(crate) fn set_anchor(&mut self, anchor: TextSize) {
        self.source_maps.anchor = anchor;
    }

    pub(crate) fn get_definition_ids(
        &self,
        definition_id_span: DefinitionIdSpan,
    ) -> &[DefinitionId] {
        self.hir.get_definition_ids(definition_id_span)
    }

    pub(crate) fn get_statement_ids(&self, statement_id_span: StatementIdSpan) -> &[StatementId] {
        self.hir.get_statement_ids(statement_id_span)
    }

    pub(crate) fn get_expression_ids(
        &self,
        expression_id_span: ExpressionIdSpan,
    ) -> &[ExpressionId] {
        self.hir.get_expression_ids(expression_id_span)
    }

    pub(crate) fn get_parameter_binding_ids(
        &self,
        parameter_id_span: ParameterIdSpan,
    ) -> &[LocalBindingId] {
        self.hir.get_parameter_binding_ids(parameter_id_span)
    }

    pub(crate) fn functions_ids(&self) -> impl Iterator<Item = DefinitionId> + '_ {
        self.hir.functions_ids()
    }

    pub(crate) fn add_definition(&mut self, kind: DefinitionKind, span: TextRange) -> DefinitionId {
        let definition_id = self.hir.definitions.add(Definition { kind });
        let relative = self.source_maps.to_relative(span);
        self.source_maps.definition_spans.add(definition_id, relative);
        definition_id
    }

    pub(crate) fn add_statement(&mut self, kind: StatementKind, span: TextRange) -> StatementId {
        let statement_id = self.hir.body.statements.add(Statement { kind });
        let relative = self.source_maps.to_relative(span);
        self.source_maps
            .body_source_map
            .statement_spans
            .add(statement_id, relative);
        statement_id
    }

    pub(crate) fn add_expression(
        &mut self,
        kind: ExpressionKind,
        ty: TypeId,
        span: TextRange,
    ) -> ExpressionId {
        let expression_id = self.hir.body.expressions.add(Expression { kind, ty });
        let relative = self.source_maps.to_relative(span);
        self.source_maps
            .body_source_map
            .expression_spans
            .add(expression_id, relative);
        expression_id
    }

    pub(crate) fn add_definition_ids(
        &mut self,
        definition_ids: &[DefinitionId],
    ) -> DefinitionIdSpan {
        let start = self.hir.definition_children_ids.len() as u32;
        self.hir
            .definition_children_ids
            .extend_from_slice(definition_ids);
        DefinitionIdSpan {
            start,
            len: definition_ids.len() as u32,
        }
    }

    pub(crate) fn add_statement_ids(&mut self, statement_ids: &[StatementId]) -> StatementIdSpan {
        let start = self.hir.body.statement_children_ids.len() as u32;
        self.hir
            .body
            .statement_children_ids
            .extend_from_slice(statement_ids);
        StatementIdSpan {
            start,
            len: statement_ids.len() as u32,
        }
    }

    pub(crate) fn add_expression_ids(
        &mut self,
        expression_ids: &[ExpressionId],
    ) -> ExpressionIdSpan {
        let start = self.hir.body.expression_children_ids.len() as u32;
        self.hir
            .body
            .expression_children_ids
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
        let start = self.hir.body.parameter_children_ids.len() as u32;
        self.hir
            .body
            .parameter_children_ids
            .extend_from_slice(parameter_ids);
        ParameterIdSpan {
            start,
            len: parameter_ids.len() as u32,
        }
    }

    pub(crate) fn add_local_binding(
        &mut self,
        name: String,
        mutable: bool,
        annotation: Option<TypeId>,
        ty: TypeId,
        span: TextRange,
    ) -> LocalBindingId {
        let local_binding_id = self.hir.body.local_bindings.add(LocalBinding {
            name,
            mutable,
            annotation,
            ty,
        });
        let relative = self.source_maps.to_relative(span);
        self.source_maps
            .body_source_map
            .local_binding_spans
            .add(local_binding_id, relative);
        local_binding_id
    }

    pub(crate) fn add_definition_binding(
        &mut self,
        name: String,
        ty: TypeId,
        span: TextRange,
    ) -> DefinitionBindingId {
        let definition_binding_id = self
            .hir
            .definition_bindings
            .add(DefinitionBinding { name, ty });
        // Absolute, not anchor-relative -- see the doc comment on
        // `HirSourceMaps::definition_binding_spans`.
        self.source_maps
            .definition_binding_spans
            .add(definition_binding_id, span);
        definition_binding_id
    }

    pub(crate) fn get_definition(&self, definition_id: DefinitionId) -> DefinitionView<'_> {
        let mut view = self.hir.get_definition(definition_id);
        view.source_maps = Some(&self.source_maps);
        view
    }

    pub(crate) fn get_statement(&self, statement_id: StatementId) -> StatementView<'_> {
        let mut view = self.hir.get_statement(statement_id);
        view.source_maps = Some(&self.source_maps);
        view
    }

    pub(crate) fn get_expression(&self, expression_id: ExpressionId) -> ExpressionView<'_> {
        let mut view = self.hir.get_expression(expression_id);
        view.source_maps = Some(&self.source_maps);
        view
    }

    pub(crate) fn get_local_binding(
        &self,
        local_binding_id: LocalBindingId,
    ) -> LocalBindingView<'_> {
        let mut view = self.hir.get_local_binding(local_binding_id);
        view.source_maps = Some(&self.source_maps);
        view
    }

    pub(crate) fn get_definition_binding(
        &self,
        definition_binding_id: DefinitionBindingId,
    ) -> DefinitionBindingView<'_> {
        let mut view = self.hir.get_definition_binding(definition_binding_id);
        view.source_maps = Some(&self.source_maps);
        view
    }
}

impl Hir {
    pub(crate) fn get_definition_ids(
        &self,
        definition_id_span: DefinitionIdSpan,
    ) -> &[DefinitionId] {
        &self.definition_children_ids[definition_id_span.start as usize
            ..(definition_id_span.start + definition_id_span.len) as usize]
    }

    pub(crate) fn get_statement_ids(&self, statement_id_span: StatementIdSpan) -> &[StatementId] {
        &self.body.statement_children_ids[statement_id_span.start as usize
            ..(statement_id_span.start + statement_id_span.len) as usize]
    }

    pub(crate) fn get_expression_ids(
        &self,
        expression_id_span: ExpressionIdSpan,
    ) -> &[ExpressionId] {
        &self.body.expression_children_ids[expression_id_span.start as usize
            ..(expression_id_span.start + expression_id_span.len) as usize]
    }

    pub(crate) fn get_parameter_binding_ids(
        &self,
        parameter_id_span: ParameterIdSpan,
    ) -> &[LocalBindingId] {
        &self.body.parameter_children_ids[parameter_id_span.start as usize
            ..(parameter_id_span.start + parameter_id_span.len) as usize]
    }

    pub(crate) fn functions_ids(&self) -> impl Iterator<Item = DefinitionId> + '_ {
        self.definitions
            .iter()
            .filter(|(_, definition)| matches!(definition.kind, DefinitionKind::Function { .. }))
            .map(|(definition_id, _)| definition_id)
    }

    /// Structural-only view (`source_maps: None`) -- `.text_range()` on it
    /// panics. Use `HirBuilder::get_definition` during lowering, when spans
    /// are available.
    pub(crate) fn get_definition(&self, definition_id: DefinitionId) -> DefinitionView<'_> {
        DefinitionView {
            definition_id,
            hir: self,
            source_maps: None,
        }
    }

    pub(crate) fn get_statement(&self, statement_id: StatementId) -> StatementView<'_> {
        StatementView {
            statement_id,
            hir: self,
            source_maps: None,
        }
    }

    pub(crate) fn get_expression(&self, expression_id: ExpressionId) -> ExpressionView<'_> {
        ExpressionView {
            expression_id,
            hir: self,
            source_maps: None,
        }
    }

    pub(crate) fn get_local_binding(
        &self,
        local_binding_id: LocalBindingId,
    ) -> LocalBindingView<'_> {
        LocalBindingView {
            local_binding_id,
            hir: self,
            source_maps: None,
        }
    }

    pub(crate) fn get_definition_binding(
        &self,
        definition_binding_id: DefinitionBindingId,
    ) -> DefinitionBindingView<'_> {
        DefinitionBindingView {
            definition_binding_id,
            hir: self,
            source_maps: None,
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

    pub(crate) fn text_range(&self) -> TextRange {
        let source_maps = self
            .source_maps
            .expect("text_range() called on a spans-free Hir view");
        source_maps.to_absolute(source_maps.definition_spans[self.definition_id])
    }
}

impl<'a> StatementView<'a> {
    pub(crate) fn id(&self) -> StatementId {
        self.statement_id
    }

    pub(crate) fn kind(&self) -> &'a StatementKind {
        &self.hir.body.statements[self.statement_id].kind
    }

    pub(crate) fn text_range(&self) -> TextRange {
        let source_maps = self
            .source_maps
            .expect("text_range() called on a spans-free Hir view");
        source_maps.to_absolute(source_maps.body_source_map.statement_spans[self.statement_id])
    }
}

impl<'a> ExpressionView<'a> {
    pub(crate) fn id(&self) -> ExpressionId {
        self.expression_id
    }

    pub(crate) fn kind(&self) -> &'a ExpressionKind {
        &self.hir.body.expressions[self.expression_id].kind
    }

    pub(crate) fn ty(&self) -> TypeId {
        self.hir.body.expressions[self.expression_id].ty
    }

    pub(crate) fn text_range(&self) -> TextRange {
        let source_maps = self
            .source_maps
            .expect("text_range() called on a spans-free Hir view");
        source_maps.to_absolute(source_maps.body_source_map.expression_spans[self.expression_id])
    }
}

impl<'a> LocalBindingView<'a> {
    pub(crate) fn id(&self) -> LocalBindingId {
        self.local_binding_id
    }

    pub(crate) fn name(&self) -> &'a str {
        &self.hir.body.local_bindings[self.local_binding_id].name
    }

    pub(crate) fn mutable(&self) -> bool {
        self.hir.body.local_bindings[self.local_binding_id].mutable
    }

    pub(crate) fn annotation(&self) -> Option<TypeId> {
        self.hir.body.local_bindings[self.local_binding_id].annotation
    }

    pub(crate) fn ty(&self) -> TypeId {
        self.hir.body.local_bindings[self.local_binding_id].ty
    }

    pub(crate) fn text_range(&self) -> TextRange {
        let source_maps = self
            .source_maps
            .expect("text_range() called on a spans-free Hir view");
        source_maps
            .to_absolute(source_maps.body_source_map.local_binding_spans[self.local_binding_id])
    }
}

impl<'a> DefinitionBindingView<'a> {
    pub(crate) fn id(&self) -> DefinitionBindingId {
        self.definition_binding_id
    }

    pub(crate) fn name(&self) -> &'a str {
        &self.hir.definition_bindings[self.definition_binding_id].name
    }

    pub(crate) fn ty(&self) -> TypeId {
        self.hir.definition_bindings[self.definition_binding_id].ty
    }

    pub(crate) fn text_range(&self) -> TextRange {
        // Absolute, not anchor-relative -- see the doc comment on
        // `HirSourceMaps::definition_binding_spans`.
        let source_maps = self
            .source_maps
            .expect("text_range() called on a spans-free Hir view");
        source_maps.definition_binding_spans[self.definition_binding_id]
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

impl<T, const KIND: u8> handlemap::Handle for TypedBindingId<T, KIND> {
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
