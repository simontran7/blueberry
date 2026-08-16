use std::iter::Peekable;
use std::str::CharIndices;

use super::token_stream::TextWidth;
use super::token_stream::TokenKind;
use super::token_stream::TokenStream;

pub(super) struct Tokenizer<'src> {
    source: &'src str,
    cursor: Peekable<CharIndices<'src>>,
}

impl<'src> Tokenizer<'src> {
    const EOF: char = '\0';

    pub(super) fn new(source: &'src str) -> Self {
        Tokenizer {
            source,
            cursor: source.char_indices().peekable(),
        }
    }

    pub(super) fn tokenize(&mut self) -> TokenStream {
        let mut tokens = TokenStream::default();

        loop {
            let (kind, width) = self.tokenize_one();
            if kind == TokenKind::Eof {
                break;
            }
            tokens.add(kind, width);
        }

        tokens
    }

    fn tokenize_one(&mut self) -> (TokenKind, TextWidth) {
        let start = self.position();
        let first = self.peek();
        self.advance();
        let kind = match first {
            ';' => TokenKind::Semicolon,
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '/' => {
                if self.peek() == '/' {
                    self.eat_inline_comment();
                    TokenKind::InlineComment
                } else {
                    TokenKind::Slash
                }
            },
            '-' => {
                if self.peek() == '>' {
                    self.advance();
                    TokenKind::ThinArrow
                } else {
                    TokenKind::Minus
                }
            }
            '!' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::NotEqual
                } else {
                    TokenKind::Error
                }
            }
            '=' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::EqualEqual
                } else {
                    TokenKind::Equal
                }
            }
            '<' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::LessEqual
                } else {
                    TokenKind::LessThan
                }
            }
            '>' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::GreaterThan
                }
            }
            c if c.is_alphabetic() || c == '_' => {
                self.eat_lexeme();
                let lexeme = &self.source[start..self.position()];
                TokenKind::classify(lexeme)
            }
            '0'..='9' => {
                self.eat_integer();
                TokenKind::Integer
            }
            c if c.is_whitespace() => {
                self.eat_whitespace();
                TokenKind::Whitespace
            }
            Self::EOF => TokenKind::Eof,
            _ => TokenKind::Error,
        };
        let end = self.position();
        let width = TextWidth::new(end - start);
        (kind, width)
    }

    fn eat_whitespace(&mut self) {
        self.advance_while(|c| c.is_whitespace())
    }

    fn eat_inline_comment(&mut self) {
        self.advance_while(|c| c != '\n');
    }

    fn eat_integer(&mut self) {
        self.advance_while(|c| c.is_ascii_hexdigit() || c == '_');
    }

    fn eat_lexeme(&mut self) {
        self.advance_while(|c| c.is_alphanumeric() || c == '_')
    }

    fn peek(&mut self) -> char {
        if let Some((_, character)) = self.cursor.peek() {
            *character
        } else {
            Self::EOF
        }
    }

    fn at_eof(&mut self) -> bool {
        self.position() == self.source.len()
    }

    fn position(&mut self) -> usize {
        if let Some((position, _)) = self.cursor.peek() {
            *position
        } else {
            self.source.len()
        }
    }

    fn advance(&mut self) {
        self.cursor.next();
    }

    fn advance_while(&mut self, predicate: impl Fn(char) -> bool) {
        while predicate(self.peek()) && !self.at_eof() {
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::token_stream_dumper::TokenDumper;
    use super::*;
    use std::fs;

    #[test]
    fn test_tokenizer_output() {
        insta::glob!("snapshot_inputs/**/*.crw", |path| {
            let input = fs::read_to_string(path).unwrap();
            let mut tokenizer = Tokenizer::new(&input);
            let tokens = tokenizer.tokenize();
            insta::assert_snapshot!(TokenDumper::new(&input, tokens).dump());
        })
    }
}
