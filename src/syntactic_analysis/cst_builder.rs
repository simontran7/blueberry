use super::cst::{GreenChild, GreenNode};
use super::parser::Event;
use crate::lexical_analysis::token_stream::TokenStream;
use crate::syntactic_analysis::cst::GreenToken;
use crate::syntactic_analysis::cst::SyntaxKind;
use crate::syntactic_analysis::parser_diagnostic::ParserDiagnostic;

pub(crate) struct CstBuilder<'src> {
    source: &'src str,
    tokens: TokenStream,
    events: Vec<Event>,
    diagnostics: Vec<ParserDiagnostic>,
}

impl<'src> CstBuilder<'src> {
    pub(crate) fn new(
        source: &'src str,
        tokens: TokenStream,
        events: Vec<Event>,
        diagnostics: Vec<ParserDiagnostic>,
    ) -> Self {
        CstBuilder { source, tokens, events, diagnostics }
    }

    pub(crate) fn build(self) -> (GreenNode, Vec<ParserDiagnostic>) {
        let source = self.source;
        let mut raw_tokens = self.tokens.kinds().zip(self.tokens.widths()).peekable();
        let mut events = self.events;
        let mut diagnostics = self.diagnostics;
        let mut stack: Vec<GreenNode> = Vec::new();
        let mut offset = 0;

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
                        current_forward_parent = match std::mem::replace(&mut events[current_index], Event::OpenNode { kind: SyntaxKind::Tombstone, forward_parent: None }) {
                            Event::OpenNode { kind: next_kind,forward_parent: next_forward_parent } => { nodes.push(GreenNode::new(next_kind)); next_forward_parent }
                            _ => unreachable!(),
                        };
                    }

                    // push the nodes onto the stack
                    for node in nodes.into_iter().rev() {
                        if node.kind() != SyntaxKind::Tombstone {
                            stack.push(node);
                        }
                    }
                },
                Event::CloseNode => {
                    let node = stack.pop().unwrap();
                    let parent = stack.last_mut().unwrap();
                    parent.add_child(GreenChild::Node(node));
                }
                Event::AddToken => {
                    let (kind, width) = raw_tokens.next().unwrap();
                    let text = &source[offset..offset + usize::from(width)];
                    let parent = stack.last_mut().unwrap();
                    parent.add_child(GreenChild::Token(GreenToken::new(kind.into(), text.into())));
                    offset += usize::from(width);
                }
                Event::AddDiagnostic { index } => {
                    let end = offset + raw_tokens.peek().map_or(0, |(_, width)| usize::from(*width));
                    diagnostics[index].resolve(offset..end);
                }
            }

            while raw_tokens.peek().is_some_and(|(kind, _)| kind.is_trivia()) {
                let (kind, width) = raw_tokens.next().unwrap();
                let text = &source[offset..offset + usize::from(width)];
                let parent = stack.last_mut().unwrap();
                parent.add_child(GreenChild::Token(GreenToken::new(kind.into(), text.into())));
                offset += usize::from(width);
            }
        }

        let root = stack.pop().unwrap();

        assert!(stack.is_empty());
        assert!(raw_tokens.next().is_none());

        (root, diagnostics)
    }
}
