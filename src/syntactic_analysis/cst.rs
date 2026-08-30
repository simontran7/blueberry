use smol_str::SmolStr;

use crate::common::text_width::TextWidth;
use crate::lexical_analysis::token_stream::TokenKind;

pub(crate) type GreenChild = NodeOrToken<GreenNode, GreenToken>;

pub(crate) struct GreenNode {
    kind: SyntaxKind,
    width: TextWidth,
    children: Vec<GreenChild>,
}

pub(crate) struct GreenToken {
    kind: SyntaxKind,
    text: SmolStr,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SyntaxKind {
    // --- token kinds ---

    // literals
    Identifier,
    Integer,

    // operators
    Equal,
    Plus,
    Minus,
    Star,
    Slash,
    LessThan,
    GreaterThan,
    LessEqual,
    GreaterEqual,
    EqualEqual,
    NotEqual,

    // punctuators
    Comma,
    Colon,
    ColonColon,
    Semicolon,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    ThinArrow,

    // keywords
    LogicalAnd,
    LogicalOr,
    LogicalNot,
    Let,
    Mut,
    Const,
    Func,
    If,
    Else,
    Return,
    True,
    False,
    While,
    Loop,
    Break,
    Continue,
    Import,

    // trivia
    Whitespace,
    InlineComment,

    // --- node kinds ---

    // top-level definitions
    FunctionDefinition,
    ConstantDefinition,

    // statements
    ExpressionStatement,
    DefinitionStatement,
    LetStatement,

    // expressions
    TypeExpression,
    ParenthesizedExpression,
    UnitLiteral,
    IntegerLiteral,
    BooleanLiteral,
    Variable,
    UnaryOperation,
    BinaryOperation,
    IfExpression,
    Block,
    ArgumentList,
    Argument,
    ReturnExpression,
    BreakExpression,
    ContinueExpression,
    Call,
    Assignment,
    WhileLoop,
    InfiniteLoop,

    // special
    File,
    Tombstone,
    Error,

    // miscellaneous
    ParameterList,
    Parameter,
}

pub(crate) enum NodeOrToken<N, T> {
    Node(N),
    Token(T),
}

impl GreenNode {
    pub(crate) fn new(kind: SyntaxKind) -> Self {
        Self {
            kind,
            width: TextWidth::new(0),
            children: Vec::new(),
        }
    }

    pub(crate) fn kind(&self) -> SyntaxKind {
        self.kind
    }

    pub(crate) fn width(&self) -> TextWidth {
        self.width
    }

    pub(crate) fn children(&self) -> &[GreenChild] {
        &self.children
    }

    pub(crate) fn add_child(&mut self, child: GreenChild) {
        let child_width = match &child {
            GreenChild::Node(node) => node.width(),
            GreenChild::Token(token) => token.width(),
        };
        self.width += child_width;
        self.children.push(child);
    }
}

impl GreenToken {
    pub(crate) fn new(kind: SyntaxKind, text: SmolStr) -> Self {
        Self {
            kind,
            text,
        }
    }

    pub(crate) fn kind(&self) -> SyntaxKind {
        self.kind
    }

    pub(crate) fn lexeme(&self) -> &str {
        &self.text
    }

    pub(crate) fn width(&self) -> TextWidth {
        TextWidth::new(self.text.len())
    }
}


impl From<TokenKind> for SyntaxKind {
    fn from(kind: TokenKind) -> Self {
        match kind {
            TokenKind::Identifier => Self::Identifier,
            TokenKind::Integer => Self::Integer,
            TokenKind::Equal => Self::Equal,
            TokenKind::Plus => Self::Plus,
            TokenKind::Minus => Self::Minus,
            TokenKind::Star => Self::Star,
            TokenKind::Slash => Self::Slash,
            TokenKind::LessThan => Self::LessThan,
            TokenKind::GreaterThan => Self::GreaterThan,
            TokenKind::LessEqual => Self::LessEqual,
            TokenKind::GreaterEqual => Self::GreaterEqual,
            TokenKind::EqualEqual => Self::EqualEqual,
            TokenKind::NotEqual => Self::NotEqual,
            TokenKind::Comma => Self::Comma,
            TokenKind::Colon => Self::Colon,
            TokenKind::ColonColon => Self::ColonColon,
            TokenKind::Semicolon => Self::Semicolon,
            TokenKind::OpenParen => Self::OpenParen,
            TokenKind::CloseParen => Self::CloseParen,
            TokenKind::OpenBrace => Self::OpenBrace,
            TokenKind::CloseBrace => Self::CloseBrace,
            TokenKind::ThinArrow => Self::ThinArrow,
            TokenKind::LogicalAnd => Self::LogicalAnd,
            TokenKind::LogicalOr => Self::LogicalOr,
            TokenKind::LogicalNot => Self::LogicalNot,
            TokenKind::Let => Self::Let,
            TokenKind::Mut => Self::Mut,
            TokenKind::Const => Self::Const,
            TokenKind::Func => Self::Func,
            TokenKind::If => Self::If,
            TokenKind::Else => Self::Else,
            TokenKind::Return => Self::Return,
            TokenKind::True => Self::True,
            TokenKind::False => Self::False,
            TokenKind::While => Self::While,
            TokenKind::Loop => Self::Loop,
            TokenKind::Break => Self::Break,
            TokenKind::Continue => Self::Continue,
            TokenKind::Import => Self::Import,
            TokenKind::Whitespace => Self::Whitespace,
            TokenKind::InlineComment => Self::InlineComment,
            TokenKind::Error => Self::Error,
            TokenKind::Eof => panic!("`eof` token is never pushed as a tree token"),
        }
    }
}
