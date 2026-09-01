use std::sync::Arc;

use smol_str::SmolStr;

use crate::common::text_size::TextSize;
use crate::lexical_analysis::token_stream::TokenKind;

pub(crate) type GreenChild = NodeOrToken<Arc<GreenNode>, Arc<GreenToken>>;

pub(crate) type RedChild = NodeOrToken<RedNode, RedToken>;

pub(crate) struct GreenNode {
    kind: SyntaxKind,
    width: TextSize,
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

#[derive(Clone)]
pub(crate) struct RedNode {
    parent: Option<Arc<RedNode>>,
    green: Arc<GreenNode>,
    /// the node's index within `parent`'s children
    index: u32,
    /// absolute text offset in the whole file
    offset: TextSize,
}

#[derive(Clone)]
pub(crate) struct RedToken {
    parent: Arc<RedNode>,
    green: Arc<GreenToken>,
    /// the token's index within `parent`'s children
    index: u32,
    /// absolute text offset in the whole file
    offset: TextSize,
}

pub(crate) enum NodeOrToken<N, T> {
    Node(N),
    Token(T),
}

pub(crate) struct SiblingsIter {
    current: Option<RedChild>,
}

impl GreenNode {
    pub(crate) fn new(kind: SyntaxKind) -> Self {
        Self {
            kind,
            width: TextSize::new(0),
            children: Vec::new(),
        }
    }

    pub(crate) fn kind(&self) -> SyntaxKind {
        self.kind
    }

    pub(crate) fn width(&self) -> TextSize {
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
        Self { kind, text }
    }

    pub(crate) fn kind(&self) -> SyntaxKind {
        self.kind
    }

    pub(crate) fn lexeme(&self) -> &str {
        &self.text
    }

    pub(crate) fn width(&self) -> TextSize {
        TextSize::new(self.text.len())
    }
}

impl RedNode {
    pub(crate) fn new(green: Arc<GreenNode>) -> Self {
        Self {
            parent: None,
            green,
            index: 0,
            offset: TextSize::new(0),
        }
    }

    pub(crate) fn kind(&self) -> SyntaxKind {
        self.green.kind()
    }

    pub(crate) fn green(&self) -> &GreenNode {
        &self.green
    }

    pub(crate) fn offset(&self) -> TextSize {
        self.offset
    }

    pub(crate) fn parent(&self) -> Option<Arc<RedNode>> {
        self.parent.clone()
    }

    pub(crate) fn children(&self) -> impl Iterator<Item = RedChild> {
        let parent = Arc::new(self.clone());
        let mut offset = self.offset;
        self.green
            .children()
            .iter()
            .enumerate()
            .map(move |(index, child)| {
                let child_offset = offset;
                match child {
                    GreenChild::Node(node) => {
                        offset += node.width();
                        RedChild::Node(RedNode {
                            parent: Some(Arc::clone(&parent)),
                            green: Arc::clone(node),
                            index: index as u32,
                            offset: child_offset,
                        })
                    }
                    GreenChild::Token(token) => {
                        offset += token.width();
                        RedChild::Token(RedToken {
                            parent: Arc::clone(&parent),
                            green: Arc::clone(token),
                            index: index as u32,
                            offset: child_offset,
                        })
                    }
                }
            })
    }

    pub(crate) fn descendants(&self) -> impl Iterator<Item = RedNode> {
        let mut descendants = Vec::new();

        fn dfs(node: &RedNode, descendants: &mut Vec<RedNode>) {
            for child in node.children() {
                if let RedChild::Node(child) = child {
                    descendants.push(child.clone());
                    dfs(&child, descendants);
                }
            }
        }

        dfs(self, &mut descendants);

        descendants.into_iter()
    }

    pub(crate) fn siblings(&self) -> SiblingsIter {
        SiblingsIter {
            current: Some(RedChild::Node(self.clone())),
        }
    }

}

impl RedToken {
    pub(crate) fn kind(&self) -> SyntaxKind {
        self.green.kind()
    }

    pub(crate) fn lexeme(&self) -> &str {
        self.green.lexeme()
    }

    pub(crate) fn offset(&self) -> TextSize {
        self.offset
    }

    pub(crate) fn parent(&self) -> &RedNode {
        &self.parent
    }
}

impl PartialEq for RedNode {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset && Arc::ptr_eq(&self.green, &other.green)
    }
}

impl Eq for RedNode {}

impl PartialEq for RedToken {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset && Arc::ptr_eq(&self.green, &other.green)
    }
}


impl Iterator for SiblingsIter {
    type Item = RedChild;

    fn next(&mut self) -> Option<RedChild> {
        let current = self.current.take()?;

        let parent: Option<Arc<RedNode>> = match &current {
            RedChild::Node(node) => node.parent.clone(),
            RedChild::Token(token) => Some(Arc::clone(&token.parent)),
        };
        let (index, offset, width) = match &current {
            RedChild::Node(node) => (node.index, node.offset, node.green.width()),
            RedChild::Token(token) => (token.index, token.offset, token.green.width()),
        };

        self.current = parent.and_then(|parent| {
            let sibling_green = parent.green.children().get(index as usize + 1)?;
            let sibling_offset = offset + width;
            Some(match sibling_green {
                GreenChild::Node(node) => RedChild::Node(RedNode {
                    parent: Some(Arc::clone(&parent)),
                    green: Arc::clone(node),
                    index: index + 1,
                    offset: sibling_offset,
                }),
                GreenChild::Token(token) => RedChild::Token(RedToken {
                    parent: Arc::clone(&parent),
                    green: Arc::clone(token),
                    index: index + 1,
                    offset: sibling_offset,
                }),
            })
        });

        Some(current)
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
