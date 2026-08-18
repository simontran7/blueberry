use std::cell::Cell;

use crate::lexical_analysis::token_stream::TokenKind;
use crate::lexical_analysis::token_stream::TokenStream;
use crate::syntactic_analysis::cst::{SyntaxKind};
use crate::syntactic_analysis::parser_diagnostic::ParserDiagnostic;

struct Parser {
    cursor: TokenCursor,
    events: Vec<Event>,
    diagnostics: Vec<ParserDiagnostic>,
}

struct TokenCursor {
    source: TokenStream,
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

impl Parser {
    pub(crate) fn new(tokens: TokenStream) -> Self {
        Self {
            cursor: TokenCursor::new(tokens),
            events: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub(crate) fn parse(self) -> (Vec<Event>, Vec<ParserDiagnostic>) {
        todo!()
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
        // create a new event
        let marker = OpenMarker {
            index: self.events.len(),
            closed: false,
        };
        self.events.push(Event::OpenNode {
            kind: SyntaxKind::Tombstone,
            forward_parent: None,
        });

        // set the forward parent of `anchor` to the new event
        let anchor = &mut self.events[anchor.open_index];
        if let Event::OpenNode { forward_parent, .. } = anchor {
            *forward_parent = Some(marker.index);
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
        self.record_diagnostic(ParserDiagnostic::UnexpectedToken {
            span: None,
            expected: kind.to_string(),
            found: self.cursor.peek().to_string(),
        });
    }

    fn record_diagnostic(&mut self, diagnostic: ParserDiagnostic) {
        let index = self.diagnostics.len();
        self.diagnostics.push(diagnostic);
        self.events.push(Event::AddDiagnostic { index });
    }
}

impl TokenCursor {
    const MAX_FUEL: usize = 256;

    fn new(source: TokenStream) -> Self {
        let initial_index = source.next_significant(0);
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
            index = self.source.next_significant(index + 1);
        }
        self.source.kind_at(index).unwrap_or(TokenKind::Eof)
    }

    fn bump(&mut self) {
        assert!(!self.at_eof());
        self.fuel.set(Self::MAX_FUEL);
        self.index = self.source.next_significant(self.index + 1);
    }
}

impl Drop for OpenMarker {
    fn drop(&mut self) {
        if !self.closed {
            panic!("The event associated with this marker needs to be closed.")
        }
    }
}
