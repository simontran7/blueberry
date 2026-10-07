use std::str::Chars;

use super::token_stream::TokenKind;
use super::token_stream::TokenStream;
use crate::core::common::span::{Span, TextSize};
use crate::core::lexical_analysis::lexical_diagnostic::LexicalDiagnostic;

pub(crate) struct Tokenizer<'src> {
    source: &'src str,
    cursor: Chars<'src>,
    tokens: TokenStream,
    diagnostics: Vec<LexicalDiagnostic>,
}

impl<'src> Tokenizer<'src> {
    pub(crate) fn new(source: &'src str) -> Self {
        Tokenizer {
            source,
            cursor: source.chars(),
            tokens: TokenStream::new(),
            diagnostics: Vec::new(),
        }
    }

    pub(crate) fn tokenize(mut self) -> (TokenStream, Vec<LexicalDiagnostic>) {
        while !self.at_eof() {
            let (kind, end) = self.tokenize_one();
            self.tokens.add(kind, end);
        }
        self.tokens
            .add(TokenKind::Eof, TextSize::new(self.offset()));

        (self.tokens, self.diagnostics)
    }

    fn peek(&self) -> Option<char> {
        self.cursor.clone().next()
    }

    fn at_eof(&self) -> bool {
        self.cursor.as_str().is_empty()
    }

    fn offset(&self) -> usize {
        self.source.len() - self.cursor.as_str().len()
    }

    fn tokenize_one(&mut self) -> (TokenKind, TextSize) {
        let start = self.offset();
        let current = self.peek().unwrap();
        self.advance();
        let kind = match current {
            ';' => TokenKind::Semicolon,
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            ':' => {
                if self.peek() == Some(':') {
                    self.advance();
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                }
            }
            ',' => TokenKind::Comma,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '/' => {
                if self.peek() == Some('/') {
                    self.eat_inline_comment();
                    TokenKind::InlineComment
                } else {
                    TokenKind::Slash
                }
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::ThinArrow
                } else {
                    TokenKind::Minus
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::NotEqual
                } else {
                    TokenKind::Error
                }
            }
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::EqualEqual
                } else {
                    TokenKind::Equal
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::LessEqual
                } else {
                    TokenKind::LessThan
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::GreaterThan
                }
            }
            c if c.is_alphabetic() || c == '_' => {
                self.eat_lexeme();
                let lexeme = &self.source[start..self.offset()];
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
            _ => {
                self.diagnostics.push(LexicalDiagnostic::UnknownToken {
                    character: current,
                    span: Span::new(TextSize::new(start), TextSize::new(self.offset())),
                });
                TokenKind::Error
            }
        };
        (kind, TextSize::new(self.offset()))
    }

    fn eat_whitespace(&mut self) {
        self.advance_while(|c| c.is_whitespace())
    }

    fn eat_inline_comment(&mut self) {
        self.advance_while(|c| c != '\n');
    }

    fn eat_integer(&mut self) {
        self.advance_while(|c| c.is_ascii_digit() || c == '_');
    }

    fn eat_lexeme(&mut self) {
        self.advance_while(|c| c.is_alphanumeric() || c == '_')
    }

    fn advance(&mut self) {
        self.cursor.next();
    }

    fn advance_while(&mut self, predicate: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&predicate) {
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
        insta::glob!("snapshot_inputs/**/*.bb", |path| {
            let input = fs::read_to_string(path).unwrap();
            let (tokens, diagnostics) = Tokenizer::new(&input).tokenize();

            let mut dump = TokenDumper::new(&input, tokens).dump();
            if !diagnostics.is_empty() {
                dump.push_str("\n--- diagnostics ---\n");
                for diagnostic in &diagnostics {
                    dump.push_str(&format!("{:?}\n", diagnostic));
                }
            }

            insta::assert_snapshot!(dump);
        })
    }
}
