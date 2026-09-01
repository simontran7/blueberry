use crate::syntactic_analysis::ast::support::{child, children, nth_child, token};
use crate::syntactic_analysis::cst::{RedChild, RedNode, RedToken, SyntaxKind};

pub(crate) trait AstNode {
    fn can_cast(kind: SyntaxKind) -> bool
    where
        Self: Sized;

    fn cast(red: RedNode) -> Option<Self>
    where
        Self: Sized;

    fn red(&self) -> &RedNode;
}

pub(crate) trait AstToken {
    fn can_cast(kind: SyntaxKind) -> bool
    where
        Self: Sized;

    fn cast(red: RedToken) -> Option<Self>
    where
        Self: Sized;

    fn red(&self) -> &RedToken;
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct File {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Definition {
    FunctionDefinition(FunctionDefinition),
    ConstantDefinition(ConstantDefinition),
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct FunctionDefinition {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ConstantDefinition {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Statement {
    LetStatement(LetStatement),
    DefinitionStatement(DefinitionStatement),
    ExpressionStatement(ExpressionStatement),
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct LetStatement {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DefinitionStatement {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ExpressionStatement {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ReturnExpression {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BreakExpression {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ContinueExpression {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Expression {
    IntegerLiteral(IntegerLiteral),
    BooleanLiteral(BooleanLiteral),
    UnitLiteral(UnitLiteral),
    Variable(Variable),
    ParenthesizedExpression(ParenthesizedExpression),
    UnaryOperation(UnaryOperation),
    BinaryOperation(BinaryOperation),
    Assignment(Assignment),
    IfExpression(IfExpression),
    WhileLoop(WhileLoop),
    InfiniteLoop(InfiniteLoop),
    Call(Call),
    Block(Block),
    ReturnExpression(ReturnExpression),
    BreakExpression(BreakExpression),
    ContinueExpression(ContinueExpression),
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct IfExpression {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum ElseBranch {
    Block(Block),
    IfExpression(IfExpression),
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WhileLoop {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct InfiniteLoop {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BinaryOperation {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct UnaryOperation {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Assignment {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Call {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ArgumentList {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Argument {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ParenthesizedExpression {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Variable {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct IntegerLiteral {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BooleanLiteral {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct UnitLiteral {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Block {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ParameterList {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Parameter {
    red: RedNode,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TypeExpression {
    red: RedNode,
}

impl File {
    pub(crate) fn definitions(&self) -> impl Iterator<Item = Definition> {
        children(self.red())
    }
}

impl FunctionDefinition {
    pub(crate) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }

    pub(crate) fn parameter_list(&self) -> Option<ParameterList> {
        child(self.red())
    }

    pub(crate) fn return_type(&self) -> Option<TypeExpression> {
        child(self.red())
    }

    pub(crate) fn body(&self) -> Option<Block> {
        child(self.red())
    }
}

impl ConstantDefinition {
    pub(crate) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }

    pub(crate) fn type_annotation(&self) -> Option<TypeExpression> {
        child(self.red())
    }

    pub(crate) fn value(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl LetStatement {
    pub(crate) fn is_mutable(&self) -> bool {
        token(self.red(), SyntaxKind::Mut).is_some()
    }

    pub(crate) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }

    pub(crate) fn type_annotation(&self) -> Option<TypeExpression> {
        child(self.red())
    }

    pub(crate) fn value(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl DefinitionStatement {
    pub(crate) fn definition(&self) -> Option<Definition> {
        child(self.red())
    }
}

impl ExpressionStatement {
    pub(crate) fn expression(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl ReturnExpression {
    pub(crate) fn value(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl BreakExpression {
    pub(crate) fn value(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl IfExpression {
    pub(crate) fn condition(&self) -> Option<Expression> {
        nth_child(self.red(), 0)
    }

    pub(crate) fn then_branch(&self) -> Option<Block> {
        nth_child(self.red(), 1)
    }

    pub(crate) fn else_branch(&self) -> Option<ElseBranch> {
        nth_child(self.red(), 2)
    }
}

impl WhileLoop {
    pub(crate) fn condition(&self) -> Option<Expression> {
        child(self.red())
    }

    pub(crate) fn body(&self) -> Option<Block> {
        child(self.red())
    }
}

impl InfiniteLoop {
    pub(crate) fn body(&self) -> Option<Block> {
        child(self.red())
    }
}

impl BinaryOperation {
    pub(crate) fn lhs(&self) -> Option<Expression> {
        children(self.red()).next()
    }

    pub(crate) fn operator(&self) -> Option<RedToken> {
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

    pub(crate) fn rhs(&self) -> Option<Expression> {
        children(self.red()).nth(1)
    }
}

impl UnaryOperation {
    pub(crate) fn operator(&self) -> Option<RedToken> {
        self.red().children().find_map(|child| match child {
            RedChild::Token(t) if matches!(t.kind(), SyntaxKind::Minus | SyntaxKind::LogicalNot) => Some(t),
            _ => None,
        })
    }

    pub(crate) fn operand(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl Assignment {
    pub(crate) fn target(&self) -> Option<Expression> {
        children(self.red()).next()
    }

    pub(crate) fn operator(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Equal)
    }

    pub(crate) fn value(&self) -> Option<Expression> {
        children(self.red()).nth(1)
    }
}

impl Call {
    pub(crate) fn callee(&self) -> Option<Expression> {
        child(self.red())
    }

    pub(crate) fn arguments(&self) -> Option<ArgumentList> {
        child(self.red())
    }
}

impl ArgumentList {
    pub(crate) fn arguments(&self) -> impl Iterator<Item = Argument> {
        children(self.red())
    }
}

impl Argument {
    pub(crate) fn value(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl ParenthesizedExpression {
    pub(crate) fn expression(&self) -> Option<Expression> {
        child(self.red())
    }
}

impl Variable {
    pub(crate) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }
}

impl IntegerLiteral {
    pub(crate) fn value(&self) -> Option<i64> {
        let integer_token = token(self.red(), SyntaxKind::Integer)?;
        let digits: String = integer_token.lexeme().chars().filter(|&c| c != '_').collect();
        digits.parse().ok()
    }
}

impl BooleanLiteral {
    pub(crate) fn value(&self) -> Option<bool> {
        self.red().children().find_map(|child| match child {
            RedChild::Token(t) if t.kind() == SyntaxKind::True => Some(true),
            RedChild::Token(t) if t.kind() == SyntaxKind::False => Some(false),
            _ => None,
        })
    }
}

impl Block {
    pub(crate) fn statements(&self) -> impl Iterator<Item = Statement> {
        children(self.red())
    }
}

impl ParameterList {
    pub(crate) fn parameters(&self) -> impl Iterator<Item = Parameter> {
        children(self.red())
    }
}

impl Parameter {
    pub(crate) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }

    pub(crate) fn type_annotation(&self) -> Option<TypeExpression> {
        child(self.red())
    }
}

impl TypeExpression {
    pub(crate) fn name(&self) -> Option<RedToken> {
        token(self.red(), SyntaxKind::Identifier)
    }
}

impl AstNode for File {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::File
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
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

impl AstNode for FunctionDefinition {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::FunctionDefinition
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for ConstantDefinition {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ConstantDefinition
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for Statement {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::LetStatement | SyntaxKind::DefinitionStatement | SyntaxKind::ExpressionStatement
        )
    }

    fn cast(red: RedNode) -> Option<Self> {
        let result = match red.kind() {
            SyntaxKind::LetStatement => Self::LetStatement(LetStatement { red }),
            SyntaxKind::DefinitionStatement => Self::DefinitionStatement(DefinitionStatement { red }),
            SyntaxKind::ExpressionStatement => Self::ExpressionStatement(ExpressionStatement { red }),
            _ => return None,
        };
        Some(result)
    }

    fn red(&self) -> &RedNode {
        match self {
            Self::LetStatement(n) => n.red(),
            Self::DefinitionStatement(n) => n.red(),
            Self::ExpressionStatement(n) => n.red(),
        }
    }
}

impl AstNode for LetStatement {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::LetStatement
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for DefinitionStatement {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::DefinitionStatement
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for ExpressionStatement {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ExpressionStatement
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for ReturnExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ReturnExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for BreakExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::BreakExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for ContinueExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ContinueExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
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
                | SyntaxKind::UnitLiteral
                | SyntaxKind::Variable
                | SyntaxKind::ParenthesizedExpression
                | SyntaxKind::UnaryOperation
                | SyntaxKind::BinaryOperation
                | SyntaxKind::Assignment
                | SyntaxKind::IfExpression
                | SyntaxKind::WhileLoop
                | SyntaxKind::InfiniteLoop
                | SyntaxKind::Call
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
            SyntaxKind::UnitLiteral => Self::UnitLiteral(UnitLiteral { red }),
            SyntaxKind::Variable => Self::Variable(Variable { red }),
            SyntaxKind::ParenthesizedExpression => {
                Self::ParenthesizedExpression(ParenthesizedExpression { red })
            }
            SyntaxKind::UnaryOperation => Self::UnaryOperation(UnaryOperation { red }),
            SyntaxKind::BinaryOperation => Self::BinaryOperation(BinaryOperation { red }),
            SyntaxKind::Assignment => Self::Assignment(Assignment { red }),
            SyntaxKind::IfExpression => Self::IfExpression(IfExpression { red }),
            SyntaxKind::WhileLoop => Self::WhileLoop(WhileLoop { red }),
            SyntaxKind::InfiniteLoop => Self::InfiniteLoop(InfiniteLoop { red }),
            SyntaxKind::Call => Self::Call(Call { red }),
            SyntaxKind::Block => Self::Block(Block { red }),
            SyntaxKind::ReturnExpression => Self::ReturnExpression(ReturnExpression { red }),
            SyntaxKind::BreakExpression => Self::BreakExpression(BreakExpression { red }),
            SyntaxKind::ContinueExpression => Self::ContinueExpression(ContinueExpression { red }),
            _ => return None,
        };
        Some(result)
    }

    fn red(&self) -> &RedNode {
        match self {
            Self::IntegerLiteral(n) => n.red(),
            Self::BooleanLiteral(n) => n.red(),
            Self::UnitLiteral(n) => n.red(),
            Self::Variable(n) => n.red(),
            Self::ParenthesizedExpression(n) => n.red(),
            Self::UnaryOperation(n) => n.red(),
            Self::BinaryOperation(n) => n.red(),
            Self::Assignment(n) => n.red(),
            Self::IfExpression(n) => n.red(),
            Self::WhileLoop(n) => n.red(),
            Self::InfiniteLoop(n) => n.red(),
            Self::Call(n) => n.red(),
            Self::Block(n) => n.red(),
            Self::ReturnExpression(n) => n.red(),
            Self::BreakExpression(n) => n.red(),
            Self::ContinueExpression(n) => n.red(),
        }
    }
}

impl AstNode for IfExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::IfExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
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

impl AstNode for WhileLoop {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::WhileLoop
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for InfiniteLoop {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::InfiniteLoop
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for BinaryOperation {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::BinaryOperation
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for UnaryOperation {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::UnaryOperation
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for Assignment {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Assignment
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for Call {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Call
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for ArgumentList {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ArgumentList
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for Argument {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Argument
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for ParenthesizedExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ParenthesizedExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for Variable {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Variable
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for IntegerLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::IntegerLiteral
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for BooleanLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::BooleanLiteral
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for UnitLiteral {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::UnitLiteral
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for Block {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Block
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for ParameterList {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::ParameterList
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for Parameter {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Parameter
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

impl AstNode for TypeExpression {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::TypeExpression
    }

    fn cast(red: RedNode) -> Option<Self> {
        Self::can_cast(red.kind()).then(|| Self { red })
    }

    fn red(&self) -> &RedNode {
        &self.red
    }
}

mod support {
    use super::{AstNode, RedNode, RedToken, SyntaxKind};
    use crate::syntactic_analysis::cst::RedChild;

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

    /// Gets the `index`-th *node* child (tokens don't count), then tries to cast it.
    /// Unlike `child`/`children`, this doesn't filter by castability first, so it stays
    /// correct even when a child's type also matches some other child's slot (e.g. an
    /// `if`'s condition and its `else if` branch are both potentially `Expression`).
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
