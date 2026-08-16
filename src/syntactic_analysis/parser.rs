use super::cst::NodeKind;

pub struct Parser {
    events: Vec<Event>,
}

enum Event {
    Open { kind: NodeKind },
    Close,
    Advance,
}
