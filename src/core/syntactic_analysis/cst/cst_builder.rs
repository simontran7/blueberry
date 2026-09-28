use std::sync::Arc;

use crate::core::lexical_analysis::token_stream::{TokenKind, TokenStream};
use crate::core::syntactic_analysis::cst::GreenToken;
use crate::core::syntactic_analysis::cst::SyntaxKind;
use crate::core::syntactic_analysis::cst::{GreenChild, GreenNode};
use crate::core::syntactic_analysis::parser::sink::{Event, Sink};
use crate::core::syntactic_analysis::parser::syntax_diagnostic::SyntaxDiagnostic;
use std::ops::Range;

pub(crate) struct CstBuilder<'src> {
    source: &'src str,
    tokens: &'src TokenStream,
    sink: Sink,
    diagnostics: Vec<SyntaxDiagnostic>,
}

struct TokenCursor<'src> {
    source: &'src str,
    tokens: &'src TokenStream,
    index: usize,
}

impl<'src> CstBuilder<'src> {
    pub(crate) fn new(
        source: &'src str,
        tokens: &'src TokenStream,
        sink: Sink,
        diagnostics: Vec<SyntaxDiagnostic>,
    ) -> Self {
        Self {
            source,
            tokens,
            sink,
            diagnostics,
        }
    }

    pub(crate) fn build(mut self) -> (Arc<GreenNode>, Vec<SyntaxDiagnostic>) {
        let mut cursor = TokenCursor::new(self.source, self.tokens);
        let mut events = std::mem::replace(&mut self.sink, Sink::new()).into_events();
        let mut diagnostics = std::mem::take(&mut self.diagnostics);
        let mut stack: Vec<GreenNode> = Vec::new();

        assert!(matches!(events.pop(), Some(Event::CloseNode)));

        for i in 0..events.len() {
            match events[i] {
                Event::OpenNode {
                    kind: first_kind,
                    forward_parent: first_forward_parent,
                } => {
                    let mut nodes = vec![GreenNode::new(first_kind)];
                    let mut current_index = i;
                    let mut current_forward_parent = first_forward_parent;

                    // order the nodes
                    while let Some(cfp) = current_forward_parent {
                        current_index += cfp;
                        current_forward_parent = match std::mem::replace(
                            &mut events[current_index],
                            Event::OpenNode {
                                kind: SyntaxKind::Tombstone,
                                forward_parent: None,
                            },
                        ) {
                            Event::OpenNode {
                                kind: next_kind,
                                forward_parent: next_forward_parent,
                            } => {
                                nodes.push(GreenNode::new(next_kind));
                                next_forward_parent
                            }
                            _ => unreachable!(),
                        };
                    }

                    let nodes: Vec<GreenNode> = nodes
                        .into_iter()
                        .rev()
                        .filter(|node| node.kind() != SyntaxKind::Tombstone)
                        .collect();
                    // trivia before a node belongs to the enclosing node
                    if let (Some(parent), false) = (stack.last_mut(), nodes.is_empty()) {
                        cursor.eat_trivia(parent);
                    }
                    stack.extend(nodes);
                }
                Event::CloseNode => {
                    let node = stack.pop().unwrap();
                    let parent = stack.last_mut().unwrap();
                    parent.add_child(GreenChild::Node(Arc::new(node)));
                }
                Event::AddToken => {
                    // trivia before a token belongs to the node the token is added to
                    let parent = stack.last_mut().unwrap();
                    cursor.eat_trivia(parent);
                    cursor.add_token(parent);
                }
                Event::AddDiagnostic { index, token_index } => {
                    diagnostics[index].resolve(self.tokens.span_at(token_index));
                }
            }
        }

        // trailing trivia belongs to the root
        cursor.eat_trivia(stack.last_mut().unwrap());
        let root = stack.pop().unwrap();

        assert!(stack.is_empty());
        assert!(cursor.is_finished());

        (Arc::new(root), diagnostics)
    }
}

impl<'src> TokenCursor<'src> {
    fn new(source: &'src str, tokens: &'src TokenStream) -> Self {
        Self {
            source,
            tokens,
            index: 0,
        }
    }

    fn is_trivia_next(&self) -> bool {
        self.tokens
            .kind_at(self.index)
            .is_some_and(TokenKind::is_trivia)
    }

    fn eat_trivia(&mut self, parent: &mut GreenNode) {
        while self.is_trivia_next() {
            self.add_token(parent);
        }
    }

    fn add_token(&mut self, parent: &mut GreenNode) {
        let kind = self.tokens.kind_at(self.index).unwrap();
        let text = &self.source[Range::from(self.tokens.span_at(self.index))];
        parent.add_child(GreenChild::Token(Arc::new(GreenToken::new(
            kind.into(),
            text.into(),
        ))));
        self.index += 1;
    }

    fn is_finished(&self) -> bool {
        self.tokens.kind_at(self.index) == Some(TokenKind::Eof)
    }
}
