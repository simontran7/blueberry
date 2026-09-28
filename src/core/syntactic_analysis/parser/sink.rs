use std::ops::{Index, IndexMut};

use crate::core::syntactic_analysis::cst::SyntaxKind;

/// The parser's output: a flat stream of events, which the `CstBuilder` replays into a tree.
pub(crate) struct Sink {
    events: Vec<Event>,
}

pub(crate) enum Event {
    OpenNode {
        kind: SyntaxKind,
        forward_parent: Option<usize>,
    },
    CloseNode,
    AddToken,
    AddDiagnostic {
        /// Index into the parser's diagnostics.
        index: usize,
        /// Index into the `TokenStream` of the token the diagnostic points at (the `Eof` token if
        /// it points at the end of the file).
        token_index: usize,
    },
}

impl Sink {
    pub(crate) fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub(crate) fn push(&mut self, event: Event) {
        self.events.push(event);
    }

    pub(crate) fn len(&self) -> usize {
        self.events.len()
    }

    pub(crate) fn into_events(self) -> Vec<Event> {
        self.events
    }
}

impl Index<usize> for Sink {
    type Output = Event;

    fn index(&self, index: usize) -> &Event {
        &self.events[index]
    }
}

impl IndexMut<usize> for Sink {
    fn index_mut(&mut self, index: usize) -> &mut Event {
        &mut self.events[index]
    }
}
