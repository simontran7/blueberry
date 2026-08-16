use core::fmt;

#[derive(Default)]
pub(super) struct TokenStream {
    kinds: Vec<TokenKind>,
    widths: Vec<TextWidth>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TokenKind {
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

    // trivia
    Whitespace,

    // special
    Eof,
    Error,
}

#[derive(Clone, Copy)]
pub(super) struct TextWidth(u32);

impl TokenStream {
    pub(super) fn add(&mut self, kind: TokenKind, width: TextWidth) {
        self.kinds.push(kind);
        self.widths.push(width);
    }

    fn count(&self) -> usize {
        self.kinds.len()
    }

    pub(super) fn kinds(&self) -> impl Iterator<Item = TokenKind> {
        self.kinds.iter().copied()
    }

    pub(super) fn widths(&self) -> impl Iterator<Item = TextWidth> {
        self.widths.iter().copied()
    }
}

impl TokenKind {
    pub(super) fn classify(lexeme: &str) -> Self {
        match lexeme {
            "let" => Self::Let,
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
            _ => Self::Identifier,
        }
    }

    pub(super) fn has_lexeme(&self) -> bool {
        matches!(self, TokenKind::Identifier | TokenKind::Integer)
    }
}

impl TextWidth {
    pub(super) fn new(width: usize) -> Self {
        TextWidth(width as u32)
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

            Self::Whitespace => "whitespace",

            Self::Eof => "end of file",
            Self::Error => "error token",
        };
        write!(f, "{}", output)
    }
}

impl From<TextWidth> for usize {
    fn from(value: TextWidth) -> Self {
        value.0 as usize
    }
}