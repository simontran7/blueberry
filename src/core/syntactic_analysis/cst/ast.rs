use crate::core::common::span::Span;
use crate::core::syntactic_analysis::cst::ast::support::{child, children, nth_child, token};
use crate::core::syntactic_analysis::cst::{RedChild, RedNode, RedToken, SyntaxKind};

pub(in crate::core) trait AstNode {
    fn can_cast(kind: SyntaxKind) -> bool
    where
        Self: Sized;

    fn cast(red: RedNode) -> Option<Self>
    where
        Self: Sized;

    fn red(&self) -> &RedNode;

    fn span(&self) -> Span {
        self.red().span()
    }
}

pub(in crate::core) trait AstToken {
    fn can_cast(kind: SyntaxKind) -> bool
    where
        Self: Sized;

    fn cast(red: RedToken) -> Option<Self>
    where
        Self: Sized;

    fn red(&self) -> &RedToken;
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct File {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) enum Definition {
    FunctionDefinition(FunctionDefinition),
    ConstantDefinition(ConstantDefinition),
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) enum Item {
    Definition(Definition),
    Import(ImportDeclaration),
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct ImportDeclaration {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct Path {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct PathSegment {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct FunctionDefinition {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct ConstantDefinition {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) enum Statement {
    Let(LetStatement),
    Definition(DefinitionStatement),
    Expression(ExpressionStatement),
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct LetStatement {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct DefinitionStatement {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct ExpressionStatement {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct Return {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct Break {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct Continue {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) enum Expression {
    IntegerLiteral(IntegerLiteral),
    BooleanLiteral(BooleanLiteral),
    PathExpression(PathExpression),
    ParenthesizedExpression(ParenthesizedExpression),
    TupleExpression(TupleExpression),
    UnaryOperation(UnaryOperation),
    BinaryOperation(BinaryOperation),
    Assignment(Assignment),
    IfExpression(IfExpression),
    WhileLoop(WhileLoop),
    InfiniteLoop(InfiniteLoop),
    Call(Call),
    FieldExpression(FieldExpression),
    Block(Block),
    Return(Return),
    Break(Break),
    Continue(Continue),
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct IfExpression {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) enum ElseBranch {
    Block(Block),
    IfExpression(IfExpression),
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct WhileLoop {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct InfiniteLoop {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct BinaryOperation {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct UnaryOperation {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct Assignment {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct Call {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct ArgumentList {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct Argument {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct FieldExpression {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct ParenthesizedExpression {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct TupleExpression {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct PathExpression {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct IntegerLiteral {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct BooleanLiteral {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct Block {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct ParameterList {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct Parameter {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) enum TypeExpression {
    PathType(PathType),
    TupleType(TupleType),
    ParenthesizedType(ParenthesizedType),
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct PathType {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct TupleType {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(in crate::core) struct ParenthesizedType {
    red: RedNode,
}

impl File {
    pub(in crate::core) fn items(&self) -> impl Iterator<Item = Item> {
        children(self.red())
    }
}

impl AstNode for File {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::File
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl Definition {
    pub(in crate::core) fn name(&self) -> Option<RedToken> {
        match self {
            Self::FunctionDefinition(n) => n.name(),
            Self::ConstantDefinition(n) => n.name(),
        }
    }
}

impl AstNode for Definition {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::FunctionDefinition | SyntaxKind::ConstantDefinition
        )
    }

    fn cast(red: RedNode) -> Option<Self> {
        let result = match red.kind() {
            SyntaxKind::FunctionDefinition => Self::FunctionDefinition(FunctionDefinition { red }),
            SyntaxKind::ConstantDefinition => Self::ConstantDefinition(ConstantDefinition { red }),
            _ => return None,
        };
        Some(result)
    }

    fn red(&self) -> &RedNode {
        match self {
            Self::FunctionDefinition(n) => n.red(),
            Self::ConstantDefinition(n) => n.red(),
        }
    }
}

impl AstNode for Item {
    fn can_cast(kind: SyntaxKind) -> bool {
        Definition::can_cast(kind) || kind == SyntaxKind::ImportDeclaration
    }

    fn cast(red: RedNode) -> Option<Self> {
        match red.kind() {
            SyntaxKind::ImportDeclaration => Some(Self::Import(ImportDeclaration { red })),
            _ => Definition::cast(red).map(Self::Definition),
        }
    }

    fn red(&self) -> &RedNode {
        match self {
            Self::Definition(n) => n.red(),
            Self::Import(n) => n.red(),
        }
    }
}

impl ImportDeclaration {
    pub(in crate::core) fn path(&self) -> Option<Path> {
        child(self.red())
    }
}

impl AstNode for ImportDeclaration {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ImportDeclaration
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl Path {
    pub(in crate::core) fn qualifier(&self) -> Option<Path> {
        child(self.red())
    }

    pub(in crate::core) fn segment(&self) -> Option<PathSegment> {
        child(self.red())
    }

    pub(in crate::core) fn segments(&self) -> Vec<PathSegment> {
        let mut segments = Vec::new();
        let mut path = Some(self.clone());
        while let Some(current) = path {
            segments.extend(current.segment());
            path = current.qualifier();
        }
        segments.reverse();
        segments
    }
}

impl AstNode for Path {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Path
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl PathSegment {
    pub(in crate::core) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }
}

impl AstNode for PathSegment {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PathSegment
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl FunctionDefinition {
    pub(in crate::core) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }

    pub(in crate::core) fn parameter_list(&self) -> Option<ParameterList> {
        child(self.red())
    }

    pub(in crate::core) fn return_type_annotation(&self) -> Option<TypeExpression> {
        child(self.red())
    }

    pub(in crate::core) fn body(&self) -> Option<Block> {
        child(self.red())
    }
}

impl AstNode for FunctionDefinition {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::FunctionDefinition
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl ConstantDefinition {
    pub(in crate::core) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }

    pub(in crate::core) fn type_annotation(&self) -> Option<TypeExpression> {
        child(self.red())
    }

    pub(in crate::core) fn value(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl AstNode for ConstantDefinition {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ConstantDefinition
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for Statement {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::LetStatement
                | SyntaxKind::DefinitionStatement
                | SyntaxKind::ExpressionStatement
        )
    }

    fn cast(red: RedNode) -> Option<Self> {
        let result = match red.kind() {
            SyntaxKind::LetStatement => Self::Let(LetStatement { red }),
            SyntaxKind::DefinitionStatement => Self::Definition(DefinitionStatement { red }),
            SyntaxKind::ExpressionStatement => Self::Expression(ExpressionStatement { red }),
            _ => return None,
        };
        Some(result)
    }

    fn red(&self) -> &RedNode {
        match self {
            Self::Let(n) => n.red(),
            Self::Definition(n) => n.red(),
            Self::Expression(n) => n.red(),
        }
    }
}

impl LetStatement {
    pub(in crate::core) fn is_mutable(&self) -> bool {
        token(self.red(), SyntaxKind::Mut).is_some()
    }

    pub(in crate::core) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }

    pub(in crate::core) fn type_annotation(&self) -> Option<TypeExpression> {
        child(self.red())
    }

    pub(in crate::core) fn value(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl AstNode for LetStatement {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::LetStatement
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl DefinitionStatement {
    pub(in crate::core) fn definition(&self) -> Option<Definition> {
        child(self.red())
    }
}

impl AstNode for DefinitionStatement {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::DefinitionStatement
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl ExpressionStatement {
    pub(in crate::core) fn expression(&self) -> Option<Expression> {
        child(self.red())
    }

    pub(in crate::core) fn has_semicolon(&self) -> bool {
        token(self.red(), SyntaxKind::Semicolon).is_some()
    }
}

impl AstNode for ExpressionStatement {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ExpressionStatement
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl Return {
    pub(in crate::core) fn value(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl AstNode for Return {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ReturnExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl Break {
    pub(in crate::core) fn value(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl AstNode for Break {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::BreakExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for Continue {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ContinueExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for Expression {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::IntegerLiteral
                | SyntaxKind::BooleanLiteral
                | SyntaxKind::PathExpression
                | SyntaxKind::ParenthesizedExpression
                | SyntaxKind::TupleExpression
                | SyntaxKind::UnaryOperation
                | SyntaxKind::BinaryOperation
                | SyntaxKind::Assignment
                | SyntaxKind::IfExpression
                | SyntaxKind::WhileLoop
                | SyntaxKind::InfiniteLoop
                | SyntaxKind::Call
                | SyntaxKind::FieldExpression
                | SyntaxKind::Block
                | SyntaxKind::ReturnExpression
                | SyntaxKind::BreakExpression
                | SyntaxKind::ContinueExpression
        )
    }

    fn cast(red: RedNode) -> Option<Self> {
        let result = match red.kind() {
            SyntaxKind::IntegerLiteral => Self::IntegerLiteral(IntegerLiteral { red }),
            SyntaxKind::BooleanLiteral => Self::BooleanLiteral(BooleanLiteral { red }),
            SyntaxKind::PathExpression => Self::PathExpression(PathExpression { red }),
            SyntaxKind::ParenthesizedExpression => {
                Self::ParenthesizedExpression(ParenthesizedExpression { red })
            }
            SyntaxKind::TupleExpression => Self::TupleExpression(TupleExpression { red }),
            SyntaxKind::UnaryOperation => Self::UnaryOperation(UnaryOperation { red }),
            SyntaxKind::BinaryOperation => Self::BinaryOperation(BinaryOperation { red }),
            SyntaxKind::Assignment => Self::Assignment(Assignment { red }),
            SyntaxKind::IfExpression => Self::IfExpression(IfExpression { red }),
            SyntaxKind::WhileLoop => Self::WhileLoop(WhileLoop { red }),
            SyntaxKind::InfiniteLoop => Self::InfiniteLoop(InfiniteLoop { red }),
            SyntaxKind::Call => Self::Call(Call { red }),
            SyntaxKind::FieldExpression => Self::FieldExpression(FieldExpression { red }),
            SyntaxKind::Block => Self::Block(Block { red }),
            SyntaxKind::ReturnExpression => Self::Return(Return { red }),
            SyntaxKind::BreakExpression => Self::Break(Break { red }),
            SyntaxKind::ContinueExpression => Self::Continue(Continue { red }),
            _ => return None,
        };
        Some(result)
    }

    fn red(&self) -> &RedNode {
        match self {
            Self::IntegerLiteral(n) => n.red(),
            Self::BooleanLiteral(n) => n.red(),
            Self::PathExpression(n) => n.red(),
            Self::ParenthesizedExpression(n) => n.red(),
            Self::TupleExpression(n) => n.red(),
            Self::UnaryOperation(n) => n.red(),
            Self::BinaryOperation(n) => n.red(),
            Self::Assignment(n) => n.red(),
            Self::IfExpression(n) => n.red(),
            Self::WhileLoop(n) => n.red(),
            Self::InfiniteLoop(n) => n.red(),
            Self::Call(n) => n.red(),
            Self::FieldExpression(n) => n.red(),
            Self::Block(n) => n.red(),
            Self::Return(n) => n.red(),
            Self::Break(n) => n.red(),
            Self::Continue(n) => n.red(),
        }
    }
}

impl IfExpression {
    pub(in crate::core) fn condition(&self) -> Option<Expression> {
        nth_child(self.red(), 0)
    }

    pub(in crate::core) fn then_branch(&self) -> Option<Block> {
        nth_child(self.red(), 1)
    }

    pub(in crate::core) fn else_branch(&self) -> Option<ElseBranch> {
        nth_child(self.red(), 2)
    }
}

impl AstNode for IfExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::IfExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for ElseBranch {
    fn can_cast(kind: SyntaxKind) -> bool {
        Block::can_cast(kind) || IfExpression::can_cast(kind)
    }

    fn cast(red: RedNode) -> Option<Self> {
        if Block::can_cast(red.kind()) {
            Block::cast(red).map(Self::Block)
        } else {
            IfExpression::cast(red).map(Self::IfExpression)
        }
    }

    fn red(&self) -> &RedNode {
        match self {
            Self::Block(n) => n.red(),
            Self::IfExpression(n) => n.red(),
        }
    }
}

impl WhileLoop {
    pub(in crate::core) fn condition(&self) -> Option<Expression> {
        child(self.red())
    }

    pub(in crate::core) fn body(&self) -> Option<Block> {
        child(self.red())
    }
}

impl AstNode for WhileLoop {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::WhileLoop
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl InfiniteLoop {
    pub(in crate::core) fn body(&self) -> Option<Block> {
        child(self.red())
    }
}

impl AstNode for InfiniteLoop {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::InfiniteLoop
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl BinaryOperation {
    pub(in crate::core) fn lhs(&self) -> Option<Expression> {
        children(self.red()).next()
    }

    pub(in crate::core) fn operator(&self) -> Option<RedToken> {
        self.red().children().find_map(|child| match child {
            RedChild::Token(t)
                if matches!(
                    t.kind(),
                    SyntaxKind::Plus
                        | SyntaxKind::Minus
                        | SyntaxKind::Star
                        | SyntaxKind::Slash
                        | SyntaxKind::LessThan
                        | SyntaxKind::GreaterThan
                        | SyntaxKind::LessEqual
                        | SyntaxKind::GreaterEqual
                        | SyntaxKind::EqualEqual
                        | SyntaxKind::NotEqual
                        | SyntaxKind::LogicalAnd
                        | SyntaxKind::LogicalOr
                ) =>
            {
                Some(t)
            }
            _ => None,
        })
    }

    pub(in crate::core) fn rhs(&self) -> Option<Expression> {
        children(self.red()).nth(1)
    }
}

impl AstNode for BinaryOperation {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::BinaryOperation
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl UnaryOperation {
    pub(in crate::core) fn operator(&self) -> Option<RedToken> {
        self.red().children().find_map(|child| match child {
            RedChild::Token(t)
                if matches!(t.kind(), SyntaxKind::Minus | SyntaxKind::LogicalNot) =>
            {
                Some(t)
            }
            _ => None,
        })
    }

    pub(in crate::core) fn operand(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl AstNode for UnaryOperation {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::UnaryOperation
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl Assignment {
    pub(in crate::core) fn target(&self) -> Option<Expression> {
        children(self.red()).next()
    }

    pub(in crate::core) fn operator(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Equal)
    }

    pub(in crate::core) fn value(&self) -> Option<Expression> {
        children(self.red()).nth(1)
    }
}

impl AstNode for Assignment {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Assignment
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl Call {
    pub(in crate::core) fn callee(&self) -> Option<Expression> {
        child(self.red())
    }

    pub(in crate::core) fn arguments(&self) -> Option<ArgumentList> {
        child(self.red())
    }
}

impl AstNode for Call {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Call
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl ArgumentList {
    pub(in crate::core) fn arguments(&self) -> impl Iterator<Item = Argument> {
        children(self.red())
    }
}

impl AstNode for ArgumentList {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ArgumentList
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl Argument {
    pub(in crate::core) fn value(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl AstNode for Argument {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Argument
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl FieldExpression {
    pub(in crate::core) fn receiver(&self) -> Option<Expression> {
        child(self.red())
    }

    pub(in crate::core) fn field(&self) -> Option<RedToken> {
        self.red().children().find_map(|child| match child {
            RedChild::Token(t)
                if matches!(t.kind(), SyntaxKind::Integer | SyntaxKind::Identifier) =>
            {
                Some(t)
            }
            _ => None,
        })
    }
}

impl AstNode for FieldExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::FieldExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl ParenthesizedExpression {
    pub(in crate::core) fn expression(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl AstNode for ParenthesizedExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ParenthesizedExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl TupleExpression {
    pub(in crate::core) fn elements(&self) -> impl Iterator<Item = Expression> {
        children(self.red())
    }
}

impl AstNode for TupleExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::TupleExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl PathExpression {
    pub(in crate::core) fn path(&self) -> Option<Path> {
        child(self.red())
    }
}

impl AstNode for PathExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PathExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl IntegerLiteral {
    pub(in crate::core) fn token(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Integer)
    }

    pub(in crate::core) fn value(&self) -> Result<u128, std::num::ParseIntError> {
        let token = self.token().unwrap();

        let cleaned: String = token.lexeme().chars().filter(|&c| c != '_').collect();

        let (digits, radix) = if let Some(rest) = cleaned.strip_prefix("0x") {
            (rest, 16)
        } else if let Some(rest) = cleaned.strip_prefix("0b") {
            (rest, 2)
        } else if let Some(rest) = cleaned.strip_prefix("0o") {
            (rest, 8)
        } else {
            (cleaned.as_str(), 10)
        };

        u128::from_str_radix(digits, radix)
    }
}

impl AstNode for IntegerLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::IntegerLiteral
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl BooleanLiteral {
    pub(in crate::core) fn value(&self) -> Option<bool> {
        self.red().children().find_map(|child| match child {
            RedChild::Token(t) if t.kind() == SyntaxKind::True => Some(true),
            RedChild::Token(t) if t.kind() == SyntaxKind::False => Some(false),
            _ => None,
        })
    }
}

impl AstNode for BooleanLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::BooleanLiteral
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl Block {
    pub(in crate::core) fn statements(&self) -> impl Iterator<Item = Statement> {
        children(self.red())
    }

    pub(in crate::core) fn definitions(&self) -> impl Iterator<Item = Definition> {
        self.statements().filter_map(|stmt| match stmt {
            Statement::Definition(stmt) => stmt.definition(),
            Statement::Let(_) | Statement::Expression(_) => None,
        })
    }
}

impl AstNode for Block {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Block
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl ParameterList {
    pub(in crate::core) fn parameters(&self) -> impl Iterator<Item = Parameter> {
        children(self.red())
    }
}

impl AstNode for ParameterList {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ParameterList
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl Parameter {
    pub(in crate::core) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }

    pub(in crate::core) fn type_annotation(&self) -> Option<TypeExpression> {
        child(self.red())
    }
}

impl AstNode for Parameter {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Parameter
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for TypeExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::PathType | SyntaxKind::TupleType | SyntaxKind::ParenthesizedType
        )
    }

    fn cast(red: RedNode) -> Option<Self> {
        let result = match red.kind() {
            SyntaxKind::PathType => Self::PathType(PathType { red }),
            SyntaxKind::TupleType => Self::TupleType(TupleType { red }),
            SyntaxKind::ParenthesizedType => Self::ParenthesizedType(ParenthesizedType { red }),
            _ => return None,
        };
        Some(result)
    }

    fn red(&self) -> &RedNode {
        match self {
            Self::PathType(n) => n.red(),
            Self::TupleType(n) => n.red(),
            Self::ParenthesizedType(n) => n.red(),
        }
    }
}

impl PathType {
    pub(in crate::core) fn path(&self) -> Option<Path> {
        child(self.red())
    }
}

impl AstNode for PathType {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::PathType
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl TupleType {
    pub(in crate::core) fn elements(&self) -> impl Iterator<Item = TypeExpression> {
        children(self.red())
    }
}

impl AstNode for TupleType {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::TupleType
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl ParenthesizedType {
    pub(in crate::core) fn inner(&self) -> Option<TypeExpression> {
        child(self.red())
    }
}

impl AstNode for ParenthesizedType {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ParenthesizedType
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then_some(Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

mod support {
    use super::{AstNode, RedNode, RedToken, SyntaxKind};
    use crate::core::syntactic_analysis::cst::RedChild;

    pub(super) fn child<N: AstNode>(parent: &RedNode) -> Option<N> {
        parent.children().find_map(|child| match child {
            RedChild::Node(node) => N::cast(node),
            RedChild::Token(_) => None,
        })
    }

    pub(super) fn children<N: AstNode>(parent: &RedNode) -> impl Iterator<Item = N> {
        parent.children().filter_map(|child| match child {
            RedChild::Node(node) => N::cast(node),
            RedChild::Token(_) => None,
        })
    }

    pub(super) fn token(parent: &RedNode, kind: SyntaxKind) -> Option<RedToken> {
        parent.children().find_map(|child| match child {
            RedChild::Token(token) if token.kind() == kind => Some(token),
            _ => None,
        })
    }

    pub(super) fn nth_child<N: AstNode>(parent: &RedNode, index: usize) -> Option<N> {
        parent
            .children()
            .filter_map(|child| match child {
                RedChild::Node(node) => Some(node),
                RedChild::Token(_) => None,
            })
            .nth(index)
            .and_then(N::cast)
    }
}
