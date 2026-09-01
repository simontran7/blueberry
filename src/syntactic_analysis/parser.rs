use std::cell::Cell;

use crate::lexical_analysis::token_stream::TokenKind;
use crate::lexical_analysis::token_stream::TokenStream;
use crate::syntactic_analysis::cst::SyntaxKind;
use crate::syntactic_analysis::parser_diagnostic::ParserDiagnostic;

pub(crate) struct Parser<'a> {
    cursor: TokenStreamCursor<'a>,
    events: Vec<Event>,
    diagnostics: Vec<ParserDiagnostic>,
}

struct TokenStreamCursor<'a> {
    source: &'a TokenStream,
    index: usize,
    fuel: Cell<usize>,
}

pub(crate) enum Event {
    OpenNode {
        kind: SyntaxKind,
        forward_parent: Option<usize>,
    },
    CloseNode,
    AddToken,
    AddDiagnostic {
        index: usize,
    },
}

struct OpenMarker {
    index: usize,
    closed: bool,
}

struct ClosedMarker {
    open_index: usize,
}

impl<'a> Parser<'a> {
    const PARAMETER_LIST_RECOVERY: &'static [TokenKind] =
        &[TokenKind::ThinArrow, TokenKind::OpenBrace, TokenKind::Func];
    const EXPRESSION_STARTERS: &'static [TokenKind] = &[
        TokenKind::Integer,
        TokenKind::True,
        TokenKind::False,
        TokenKind::Identifier,
        TokenKind::OpenParen,
        TokenKind::Minus,
        TokenKind::LogicalNot,
        TokenKind::Loop,
        TokenKind::While,
        TokenKind::If,
        TokenKind::OpenBrace,
        TokenKind::Return,
        TokenKind::Break,
        TokenKind::Continue,
    ];
    const BLOCK_LIKE_EXPRESSION_STARTERS: &'static [TokenKind] = &[
        TokenKind::Loop,
        TokenKind::While,
        TokenKind::If,
        TokenKind::OpenBrace,
    ];
    const MIN_BINDING_POWER: u8 = 0;

    pub(crate) fn new(tokens: &'a TokenStream) -> Self {
        Self {
            cursor: TokenStreamCursor::new(tokens),
            events: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub(crate) fn parse(&mut self) -> (Vec<Event>, Vec<ParserDiagnostic>) {
        let marker = self.open();

        while !self.cursor.at_eof() {
            if self.cursor.at(TokenKind::Func) {
                self.parse_function_definition();
            } else if self.cursor.at(TokenKind::Const) {
                self.parse_constant_definition();
            } else {
                self.advance_with_error(ParserDiagnostic::new(
                    TokenKind::Func.to_string(),
                    self.cursor.peek().to_string(),
                ));
            }
        }

        self.close(marker, SyntaxKind::File);

        (
            std::mem::take(&mut self.events),
            std::mem::take(&mut self.diagnostics),
        )
    }

    fn parse_function_definition(&mut self) {
        assert!(self.cursor.at(TokenKind::Func));
        let marker = self.open();

        self.expect(TokenKind::Func);
        self.expect(TokenKind::Identifier);
        if self.cursor.at(TokenKind::OpenParen) {
            self.parse_parameter_list();
        }
        if self.eat(TokenKind::ThinArrow) {
            self.parse_type_expression();
        }
        if self.cursor.at(TokenKind::OpenBrace) {
            self.parse_block();
        }

        self.close(marker, SyntaxKind::FunctionDefinition);
    }

    fn parse_constant_definition(&mut self) {
        assert!(self.cursor.at(TokenKind::Const));
        let marker = self.open();

        self.expect(TokenKind::Const);
        self.expect(TokenKind::Identifier);
        self.expect(TokenKind::Colon);
        self.parse_type_expression();
        self.expect(TokenKind::Equal);
        let _ = self.parse_expression(Self::MIN_BINDING_POWER);
        self.expect(TokenKind::Semicolon);

        self.close(marker, SyntaxKind::ConstantDefinition);
    }

    fn parse_definition_statement(&mut self) {
        assert!(self.cursor.at_any(&[TokenKind::Func, TokenKind::Const]));
        let marker = self.open();

        if self.cursor.at(TokenKind::Func) {
            self.parse_function_definition();
        } else {
            self.parse_constant_definition();
        }

        self.close(marker, SyntaxKind::DefinitionStatement);
    }

    fn parse_parameter_list(&mut self) {
        assert!(self.cursor.at(TokenKind::OpenParen));
        let marker = self.open();

        self.expect(TokenKind::OpenParen);
        while !self.cursor.at(TokenKind::CloseParen) && !self.cursor.at_eof() {
            if self.cursor.at(TokenKind::Identifier) {
                self.parse_parameter();
            } else if self.cursor.at_any(Self::PARAMETER_LIST_RECOVERY) {
                break;
            } else {
                self.advance_with_error(ParserDiagnostic::new(
                    "parameter".to_string(),
                    self.cursor.peek().to_string(),
                ));
            }
        }
        self.expect(TokenKind::CloseParen);

        self.close(marker, SyntaxKind::ParameterList);
    }

    fn parse_parameter(&mut self) {
        assert!(self.cursor.at(TokenKind::Identifier));
        let marker = self.open();

        self.expect(TokenKind::Identifier);
        self.expect(TokenKind::Colon);
        self.parse_type_expression();
        if !self.cursor.at(TokenKind::CloseParen) {
            self.expect(TokenKind::Comma);
        }

        self.close(marker, SyntaxKind::Parameter);
    }

    fn parse_block(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::OpenBrace));
        let marker = self.open();

        self.expect(TokenKind::OpenBrace);
        while !self.cursor.at(TokenKind::CloseBrace) && !self.cursor.at_eof() {
            match self.cursor.peek() {
                TokenKind::Let => self.parse_let(),
                TokenKind::Func | TokenKind::Const => self.parse_definition_statement(),
                _ => {
                    if self.cursor.at_any(Self::EXPRESSION_STARTERS) {
                        self.parse_expression_statement()
                    } else {
                        self.advance_with_error(ParserDiagnostic::new(
                            "statement".to_string(),
                            self.cursor.peek().to_string(),
                        ));
                    }
                }
            }
        }
        self.expect(TokenKind::CloseBrace);

        self.close(marker, SyntaxKind::Block)
    }

    fn parse_let(&mut self) {
        assert!(self.cursor.at(TokenKind::Let));
        let marker = self.open();

        self.expect(TokenKind::Let);
        self.eat(TokenKind::Mut);
        self.expect(TokenKind::Identifier);
        if self.eat(TokenKind::Colon) {
            self.parse_type_expression();
        }
        self.expect(TokenKind::Equal);
        let _ = self.parse_expression(Self::MIN_BINDING_POWER);
        self.expect(TokenKind::Semicolon);

        self.close(marker, SyntaxKind::LetStatement);
    }

    fn parse_return(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::Return));
        let marker = self.open();

        self.expect(TokenKind::Return);
        let _ = self.parse_expression(Self::MIN_BINDING_POWER);

        self.close(marker, SyntaxKind::ReturnExpression)
    }

    fn parse_break(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::Break));
        let marker = self.open();

        self.expect(TokenKind::Break);
        if self.cursor.at_any(Self::EXPRESSION_STARTERS) {
            let _ = self.parse_expression(Self::MIN_BINDING_POWER);
        }

        self.close(marker, SyntaxKind::BreakExpression)
    }

    fn parse_continue(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::Continue));
        let marker = self.open();

        self.expect(TokenKind::Continue);

        self.close(marker, SyntaxKind::ContinueExpression)
    }

    fn parse_expression_statement(&mut self) {
        let marker = self.open();

        let is_block_like = self.cursor.at_any(Self::BLOCK_LIKE_EXPRESSION_STARTERS);
        // prevents parsing `if <cond> { ... } else { ... } <bin op> <operand>;` as a binary operation
        let _ = if is_block_like {
            self.nud()
        } else {
            self.parse_expression(Self::MIN_BINDING_POWER)
        };

        match self.cursor.peek() {
            TokenKind::Semicolon => self.advance(),
            TokenKind::CloseBrace => {} // no trailing `;` necessary for block-like expressions
            _ => {
                if !is_block_like {
                    self.expect(TokenKind::Semicolon);
                }
            }
        }

        self.close(marker, SyntaxKind::ExpressionStatement);
    }

    fn parse_expression(&mut self, min_bp: u8) -> Option<ClosedMarker> {
        let mut lhs = self.nud()?;

        while let Some((lbp, ())) = self.cursor.peek().postfix_binding_power()
            && min_bp <= lbp
        {
            lhs = self.led_postfix(lhs);
        }
        while let Some((lbp, rbp)) = self.cursor.peek().infix_binding_power()
            && min_bp <= lbp
        {
            lhs = self.led_infix(lhs, rbp);
        }

        Some(lhs)
    }

    fn nud(&mut self) -> Option<ClosedMarker> {
        match self.cursor.peek() {
            TokenKind::Integer => Some(self.parse_integer_literal()),
            TokenKind::True | TokenKind::False => Some(self.parse_boolean_literal()),
            TokenKind::Identifier => Some(self.parse_variable()),
            TokenKind::OpenParen if self.cursor.peek_ahead(1) == TokenKind::CloseParen => {
                Some(self.parse_unit_literal())
            }
            TokenKind::OpenParen => Some(self.parse_parenthesized_expression()),
            TokenKind::Loop => Some(self.parse_infinite_loop()),
            TokenKind::While => Some(self.parse_while_loop()),
            TokenKind::If => Some(self.parse_if()),
            TokenKind::OpenBrace => Some(self.parse_block()),
            TokenKind::Return => Some(self.parse_return()),
            TokenKind::Break => Some(self.parse_break()),
            TokenKind::Continue => Some(self.parse_continue()),
            kind if kind.prefix_binding_power().is_some() => Some(self.parse_unary_operation()),
            _ => {
                assert!(!self.cursor.at_any(Self::EXPRESSION_STARTERS));
                None
            }
        }
    }

    fn parse_integer_literal(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::Integer));
        let marker = self.open();
        self.advance();
        self.close(marker, SyntaxKind::IntegerLiteral)
    }

    fn parse_boolean_literal(&mut self) -> ClosedMarker {
        assert!(self.cursor.at_any(&[TokenKind::True, TokenKind::False]));
        let marker = self.open();
        self.advance();
        self.close(marker, SyntaxKind::BooleanLiteral)
    }

    fn parse_variable(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::Identifier));
        let marker = self.open();
        self.advance();
        self.close(marker, SyntaxKind::Variable)
    }

    fn parse_unit_literal(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::OpenParen));
        assert!(self.cursor.peek_ahead(1) == TokenKind::CloseParen);
        let marker = self.open();

        self.expect(TokenKind::OpenParen);
        self.expect(TokenKind::CloseParen);

        self.close(marker, SyntaxKind::UnitLiteral)
    }

    fn parse_parenthesized_expression(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::OpenParen));
        let marker = self.open();

        self.expect(TokenKind::OpenParen);
        let _ = self.parse_expression(Self::MIN_BINDING_POWER);
        self.expect(TokenKind::CloseParen);

        self.close(marker, SyntaxKind::ParenthesizedExpression)
    }

    fn parse_infinite_loop(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::Loop));
        let marker = self.open();

        self.expect(TokenKind::Loop);
        if self.cursor.at(TokenKind::OpenBrace) {
            self.parse_block();
        }

        self.close(marker, SyntaxKind::InfiniteLoop)
    }

    fn parse_while_loop(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::While));
        let marker = self.open();

        self.expect(TokenKind::While);
        let _ = self.parse_expression(Self::MIN_BINDING_POWER);
        if self.cursor.at(TokenKind::OpenBrace) {
            self.parse_block();
        }

        self.close(marker, SyntaxKind::WhileLoop)
    }

    fn parse_if(&mut self) -> ClosedMarker {
        assert!(self.cursor.at(TokenKind::If));
        let marker = self.open();

        self.expect(TokenKind::If);
        let _ = self.parse_expression(Self::MIN_BINDING_POWER);
        if self.cursor.at(TokenKind::OpenBrace) {
            self.parse_block();
        }
        if self.eat(TokenKind::Else) {
            if self.cursor.at(TokenKind::If) {
                self.parse_if();
            } else if self.cursor.at(TokenKind::OpenBrace) {
                self.parse_block();
            }
        }

        self.close(marker, SyntaxKind::IfExpression)
    }

    fn parse_unary_operation(&mut self) -> ClosedMarker {
        assert!(self.cursor.peek().prefix_binding_power().is_some());
        let marker = self.open();

        let ((), rbp) = self.cursor.peek().prefix_binding_power().unwrap();
        self.advance();
        let _ = self.parse_expression(rbp);

        self.close(marker, SyntaxKind::UnaryOperation)
    }

    fn led_postfix(&mut self, lhs: ClosedMarker) -> ClosedMarker {
        let marker = self.open_before(lhs);
        self.parse_argument_list();
        self.close(marker, SyntaxKind::Call)
    }

    fn led_infix(&mut self, lhs: ClosedMarker, rbp: u8) -> ClosedMarker {
        let marker = self.open_before(lhs);
        let op_kind = self.cursor.peek();
        self.advance();
        let _ = self.parse_expression(rbp);
        let kind = if op_kind == TokenKind::Equal {
            SyntaxKind::Assignment
        } else {
            SyntaxKind::BinaryOperation
        };
        self.close(marker, kind)
    }

    fn parse_argument_list(&mut self) {
        assert!(self.cursor.at(TokenKind::OpenParen));
        let marker = self.open();

        self.expect(TokenKind::OpenParen);
        while !self.cursor.at(TokenKind::CloseParen) && !self.cursor.at_eof() {
            if self.cursor.at_any(Self::EXPRESSION_STARTERS) {
                self.parse_argument();
            } else {
                break;
            }
        }
        self.expect(TokenKind::CloseParen);

        self.close(marker, SyntaxKind::ArgumentList);
    }

    fn parse_argument(&mut self) {
        let marker = self.open();

        let _ = self.parse_expression(Self::MIN_BINDING_POWER);
        if !self.cursor.at(TokenKind::CloseParen) {
            self.expect(TokenKind::Comma);
        }

        self.close(marker, SyntaxKind::Argument);
    }

    fn parse_type_expression(&mut self) {
        let marker = self.open();

        self.expect(TokenKind::Identifier);

        self.close(marker, SyntaxKind::TypeExpression);
    }

    fn open(&mut self) -> OpenMarker {
        let marker = OpenMarker {
            index: self.events.len(),
            closed: false,
        };
        self.events.push(Event::OpenNode {
            kind: SyntaxKind::Tombstone,
            forward_parent: None,
        });
        marker
    }

    fn open_before(&mut self, anchor: ClosedMarker) -> OpenMarker {
        let marker = OpenMarker {
            index: self.events.len(),
            closed: false,
        };
        self.events.push(Event::OpenNode {
            kind: SyntaxKind::Tombstone,
            forward_parent: None,
        });

        // set the forward parent of `anchor` to the new event
        if let Event::OpenNode { forward_parent, .. } = &mut self.events[anchor.open_index] {
            *forward_parent = Some(marker.index - anchor.open_index); // `CstBuilder::build` walks forward parents by offset
        } else {
            unreachable!()
        }

        marker
    }

    fn close(&mut self, mut marker: OpenMarker, kind: SyntaxKind) -> ClosedMarker {
        assert!(matches!(
            self.events[marker.index],
            Event::OpenNode {
                kind: SyntaxKind::Tombstone,
                ..
            }
        ));
        self.events[marker.index] = Event::OpenNode {
            kind,
            forward_parent: None,
        };
        self.events.push(Event::CloseNode);
        marker.closed = true;
        ClosedMarker {
            open_index: marker.index,
        }
    }

    fn advance(&mut self) {
        self.cursor.bump();
        self.events.push(Event::AddToken);
    }

    fn advance_with_error(&mut self, diagnostic: ParserDiagnostic) {
        let marker = self.open();
        self.record_diagnostic(diagnostic);
        self.advance();
        self.close(marker, SyntaxKind::Error);
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.cursor.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) {
        if self.eat(kind) {
            return;
        }
        self.record_diagnostic(ParserDiagnostic::new(
            kind.to_string(),
            self.cursor.peek().to_string(),
        ));
    }

    fn record_diagnostic(&mut self, diagnostic: ParserDiagnostic) {
        let index = self.diagnostics.len();
        self.diagnostics.push(diagnostic);
        self.events.push(Event::AddDiagnostic { index });
    }
}

impl<'a> TokenStreamCursor<'a> {
    const MAX_FUEL: usize = 256;

    fn new(source: &'a TokenStream) -> Self {
        let initial_index = source.next_non_trivia(0);
        Self {
            source,
            index: initial_index,
            fuel: Cell::new(Self::MAX_FUEL),
        }
    }

    fn at_eof(&self) -> bool {
        self.index == self.source.count()
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.peek() == kind
    }

    fn at_any(&self, kinds: &[TokenKind]) -> bool {
        kinds.contains(&self.peek())
    }

    fn peek(&self) -> TokenKind {
        self.peek_ahead(0)
    }

    fn peek_ahead(&self, n: usize) -> TokenKind {
        if self.fuel.get() == 0 {
            panic!("parser is stuck");
        }
        self.fuel.set(self.fuel.get() - 1);
        let mut index = self.index;
        for _ in 0..n {
            index = self.source.next_non_trivia(index + 1);
        }
        self.source.kind_at(index).unwrap_or(TokenKind::Eof)
    }

    fn bump(&mut self) {
        assert!(!self.at_eof());
        self.fuel.set(Self::MAX_FUEL);
        self.index = self.source.next_non_trivia(self.index + 1);
    }
}

impl Drop for OpenMarker {
    fn drop(&mut self) {
        if !self.closed {
            panic!("The event associated with this marker needs to be closed.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexical_analysis::tokenizer::Tokenizer;
    use crate::syntactic_analysis::cst_builder::CstBuilder;
    use crate::syntactic_analysis::cst_dumper::CstDumper;
    use std::fs;

    #[test]
    fn test_parser_output() {
        insta::glob!("snapshot_inputs/**/*.bb", |path| {
            let input = fs::read_to_string(path).unwrap();
            let tokens = Tokenizer::new(&input).tokenize();

            let (events, diagnostics) = Parser::new(&tokens).parse();
            let (cst, diagnostics) = CstBuilder::new(&input, &tokens, events, diagnostics).build();

            let mut dump = CstDumper::new(&cst).dump();
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
