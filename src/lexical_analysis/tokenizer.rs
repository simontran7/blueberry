use std::str::{CharIndices};

use super::token_stream::TokenStream;
use super::token_stream::TokenKind;
use super::token_stream::TextWidth;

pub struct Tokenizer<'src> {
    cursor: CharIndices<'src>,
}

impl<'src> Tokenizer<'src> {
    const EOF: char = '\0';

    pub fn new(source: &'src str) -> Self {
        Tokenizer {
            cursor: source.char_indices(),
        }
    }

    pub fn tokenize(&mut self) -> TokenStream {
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
        let kind = match self.peek() {
            ';' => {
                self.advance();
                TokenKind::Semicolon
            }
            '(' => {
                self.advance();
                TokenKind::OpenParen
            },
            ')' => {
                self.advance();
                TokenKind::CloseParen
            },
            '{' => {
                self.advance();
                TokenKind::OpenBrace
            }
            '}' => {
                self.advance();
                TokenKind::CloseBrace
            }
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            ',' => {
                self.advance();
                TokenKind::Comma
            }
            '+' => {
                self.advance();
                TokenKind::Plus
            }
            '*' => {
                self.advance();
                TokenKind::Star
            }
            '/' => {
                self.advance();
                TokenKind::Slash
            }
            '-' => {
                self.advance();
                if self.peek() == '>' {
                    self.advance();
                    TokenKind::ThinArrow
                } else {
                    self.advance();
                    TokenKind::Minus
                }
            }
            '!' => {
                self.advance();
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::NotEqual
                } else {
                    TokenKind::Error
                }
            }
            '=' => {
                self.advance();
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::EqualEqual
                } else {
                    self.advance();
                    TokenKind::Equal
                }
            }
            '<' => {
                self.advance();
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::LessEqual
                } else {
                    self.advance();
                    TokenKind::LessThan
                }
            },
            '>' => {
                self.advance();
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::GreaterEqual
                } else {
                    self.advance();
                    TokenKind::GreaterThan
                }
            }
            c if c.is_alphabetic() || c == '_' => {
                self.eat_lexeme();
                let lexeme = &self.cursor.as_str()[start..self.position()];
                TokenKind::classify(lexeme)
            }
            '0'..='9' => {
                self.eat_integer();
                TokenKind::Integer
            }, 
            c if c.is_whitespace() => {
                self.eat_whitespace();
                TokenKind::Whitespace
            },
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

    fn eat_integer(&mut self) {
        self.advance_while(|c| c.is_ascii_hexdigit() || c == '_');
    }

    fn eat_lexeme(&mut self) {
        self.advance_while(|c| c.is_alphanumeric() || c == '_')
    }

    fn peek(&mut self) -> char {
        if let Some((_, chararacter)) = self.cursor.nth(self.cursor.offset()) {
            chararacter
        } else {
            Self::EOF
        }
    }

    fn at_eof(&self) -> bool {
        self.cursor.offset() == self.cursor.as_str().len()
    }

    fn position(&self) -> usize {
        self.cursor.offset()
    }

    fn advance(&mut self) {
        assert!(!self.at_eof());
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
    use std::fs;
    use super::*;

    #[test]
    fn test_tokenizer_output() {
        insta::glob!("snapshot_inputs/**/*.crw", |path| {
            let input = fs::read_to_string(path).unwrap();
            let mut tokenizer = Tokenizer::new(&input);
            let tokens = tokenizer.tokenize();
            insta::assert_snapshot!(tokens);
        })
    }
}