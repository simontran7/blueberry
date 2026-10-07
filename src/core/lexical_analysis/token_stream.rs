use core::fmt;

use crate::core::common::span::{Span, TextSize};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TokenStream {
    kinds: Vec<TokenKind>,
    starts: Vec<TextSize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenKind {
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

    // special
    Eof,
    Error,
}

impl TokenStream {
    pub(crate) fn new() -> Self {
        Self {
            kinds: Vec::new(),
            starts: vec![TextSize::new(0)],
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.kinds.len()
    }

    pub(crate) fn kind_at(&self, index: usize) -> Option<TokenKind> {
        self.kinds.get(index).copied()
    }

    pub(crate) fn span_at(&self, index: usize) -> Span {
        Span::new(self.starts[index], self.starts[index + 1])
    }

    pub(crate) fn kinds(&self) -> impl Iterator<Item = TokenKind> {
        self.kinds.iter().copied()
    }

    pub(crate) fn spans(&self) -> impl Iterator<Item = Span> {
        (0..self.count()).map(|index| self.span_at(index))
    }

    pub(crate) fn add(&mut self, kind: TokenKind, end: TextSize) {
        assert!(
            end >= *self.starts.last().unwrap(),
            "a token can't end before it starts"
        );
        self.kinds.push(kind);
        self.starts.push(end);
    }
}

impl TokenKind {
    pub(crate) fn classify(lexeme: &str) -> Self {
        match lexeme {
            "let" => Self::Let,
            "mut" => Self::Mut,
            "const" => Self::Const,
            "func" => Self::Func,
            "if" => Self::If,
            "else" => Self::Else,
            "not" => Self::LogicalNot,
            "and" => Self::LogicalAnd,
            "or" => Self::LogicalOr,
            "return" => Self::Return,
            "true" => Self::True,
            "false" => Self::False,
            "while" => Self::While,
            "loop" => Self::Loop,
            "break" => Self::Break,
            "continue" => Self::Continue,
            "import" => Self::Import,
            _ => Self::Identifier,
        }
    }

    pub(crate) fn has_lexeme(self) -> bool {
        matches!(self, TokenKind::Identifier | TokenKind::Integer)
    }

    pub(crate) fn is_trivia(self) -> bool {
        matches!(self, TokenKind::Whitespace | TokenKind::InlineComment)
    }

    pub(crate) const fn postfix_binding_power(self) -> Option<(u8, ())> {
        match self {
            Self::OpenParen => Some((15, ())),
            _ => None,
        }
    }

    pub(crate) const fn infix_binding_power(self) -> Option<(u8, u8)> {
        match self {
            Self::Equal => Some((2, 1)),
            Self::LogicalOr => Some((3, 4)),
            Self::LogicalAnd => Some((5, 6)),
            Self::EqualEqual | Self::NotEqual => Some((7, 8)),
            Self::LessThan | Self::GreaterThan | Self::LessEqual | Self::GreaterEqual => {
                Some((9, 10))
            }
            Self::Plus | Self::Minus => Some((11, 12)),
            Self::Star | Self::Slash => Some((13, 14)),
            _ => None,
        }
    }

    pub(crate) const fn prefix_binding_power(self) -> Option<((), u8)> {
        match self {
            Self::LogicalNot | Self::Minus => Some(((), 15)),
            _ => None,
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = match self {
            Self::Identifier => "identifier",
            Self::Integer => "integer",

            Self::Equal => "=",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::LessThan => "<",
            Self::GreaterThan => ">",
            Self::LessEqual => "<=",
            Self::GreaterEqual => ">=",
            Self::EqualEqual => "==",
            Self::NotEqual => "!=",

            Self::Comma => ",",
            Self::Colon => ":",
            Self::ColonColon => "::",
            Self::Semicolon => ";",
            Self::OpenParen => "(",
            Self::CloseParen => ")",
            Self::OpenBrace => "{",
            Self::CloseBrace => "}",
            Self::ThinArrow => "->",

            Self::LogicalAnd => "and",
            Self::LogicalOr => "or",
            Self::LogicalNot => "not",
            Self::Let => "let",
            Self::Mut => "mut",
            Self::Const => "const",
            Self::Func => "func",
            Self::If => "if",
            Self::Else => "else",
            Self::True => "true",
            Self::False => "false",
            Self::Return => "return",
            Self::While => "while",
            Self::Loop => "loop",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Import => "import",

            Self::Whitespace => "whitespace",
            Self::InlineComment => "inline comment",

            Self::Eof => "end of file",
            Self::Error => "error token",
        };
        write!(f, "{}", output)
    }
}
