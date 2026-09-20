use std::sync::Arc;

use super::cst::{GreenChild, GreenNode};
use super::parser::Event;
use crate::core::common::span::{Span, TextSize};
use crate::core::lexical_analysis::token_stream::{TokenKind, TokenStream};
use crate::core::syntactic_analysis::cst::GreenToken;
use crate::core::syntactic_analysis::cst::SyntaxKind;
use crate::core::syntactic_analysis::syntax_diagnostic::SyntaxDiagnostic;

pub(crate) struct CstBuilder<'src> {
    source: &'src str,
    tokens: &'src TokenStream,
    events: Vec<Event>,
    diagnostics: Vec<SyntaxDiagnostic>,
}

impl<'src> CstBuilder<'src> {
    pub(crate) fn new(
        source: &'src str,
        tokens: &'src TokenStream,
        events: Vec<Event>,
        diagnostics: Vec<SyntaxDiagnostic>,
    ) -> Self {
        CstBuilder {
            source,
            tokens,
            events,
            diagnostics,
        }
    }

    pub(crate) fn build(self) -> (Arc<GreenNode>, Vec<SyntaxDiagnostic>) {
        let mut cursor = TokenCursor::new(self.source, self.tokens);
        let mut events = self.events;
        let mut diagnostics = self.diagnostics;
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
                Event::AddDiagnostic { index } => {
                    diagnostics[index].resolve(cursor.next_significant_span());
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

struct TokenCursor<'src> {
    source: &'src str,
    tokens: &'src TokenStream,
    index: usize,
    offset: usize,
}

impl<'src> TokenCursor<'src> {
    fn new(source: &'src str, tokens: &'src TokenStream) -> Self {
        Self {
            source,
            tokens,
            index: 0,
            offset: 0,
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
        let width = usize::from(self.tokens.width_at(self.index).unwrap());
        let text = &self.source[self.offset..self.offset + width];
        parent.add_child(GreenChild::Token(Arc::new(GreenToken::new(
            kind.into(),
            text.into(),
        ))));
        self.index += 1;
        self.offset += width;
    }

    /// The span of the next token that is not trivia, without consuming anything.
    fn next_significant_span(&self) -> Span {
        let mut index = self.index;
        let mut start = self.offset;
        while self
            .tokens
            .kind_at(index)
            .is_some_and(TokenKind::is_trivia)
        {
            start += usize::from(self.tokens.width_at(index).unwrap());
            index += 1;
        }
        let width = self
            .tokens
            .width_at(index)
            .map_or(0, usize::from);
        Span::new(TextSize::new(start), TextSize::new(start + width))
    }

    fn is_finished(&self) -> bool {
        self.tokens.kind_at(self.index).is_none()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::core::lexical_analysis::tokenizer::Tokenizer;
    use crate::core::syntactic_analysis::parser::Parser;

    fn is_trivia(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::Whitespace | SyntaxKind::InlineComment)
    }

    fn assert_no_trivia_at_edges(node: &GreenNode, is_root: bool, file: &Path) {
        if !is_root {
            let children = node.children();
            for edge in [children.first(), children.last()].into_iter().flatten() {
                if let GreenChild::Token(token) = edge {
                    assert!(
                        !is_trivia(token.kind()),
                        "{}: a {:?} node has trivia at its edge",
                        file.display(),
                        node.kind()
                    );
                }
            }
        }
        for child in node.children() {
            if let GreenChild::Node(child) = child {
                assert_no_trivia_at_edges(child, false, file);
            }
        }
    }

    #[test]
    fn trivia_is_never_at_the_edge_of_a_node() {
        let inputs = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core/syntactic_analysis/snapshot_inputs");
        let mut checked = 0;
        for entry in fs::read_dir(inputs).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|extension| extension != "bb") {
                continue;
            }
            let input = fs::read_to_string(&path).unwrap();
            let (tokens, _) = Tokenizer::new(&input).tokenize();
            let (events, diagnostics) = Parser::new(&tokens).parse();
            let (cst, _) = CstBuilder::new(&input, &tokens, events, diagnostics).build();
            assert_no_trivia_at_edges(&cst, true, &path);
            checked += 1;
        }
        assert!(checked > 0, "no snapshot inputs were checked");
    }
}
