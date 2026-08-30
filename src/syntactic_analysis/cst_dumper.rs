use crate::syntactic_analysis::cst::{GreenChild, GreenNode};

pub(crate) struct CstDumper<'cst> {
    root: &'cst GreenNode,
    dump: String,
    offset: usize,
}

impl<'cst> CstDumper<'cst> {
    pub(crate) fn new(root: &'cst GreenNode) -> Self {
        Self {
            root,
            dump: String::new(),
            offset: 0,
        }
    }

    pub(crate) fn dump(&mut self) -> String {
        self.dump.push_str(&format!("{:?}\n", self.root.kind()));
        self.dump_children(self.root, "");
        self.dump.push_str(&format!(
            "Span: [{}, {})\n",
            0,
            usize::from(self.root.width())
        ));
        std::mem::take(&mut self.dump)
    }

    fn dump_children(&mut self, node: &GreenNode, indent: &str) {
        let Some(last_index) = node.children().len().checked_sub(1) else {
            return;
        };

        for (index, child) in node.children().iter().enumerate() {
            let is_last = index == last_index;
            let connector = if is_last { "└─" } else { "├─" };
            let continuation = if is_last { "  " } else { "│ " };
            let child_indent = format!("{indent}{continuation}");

            match child {
                GreenChild::Node(child_node) => {
                    let start = self.offset;
                    let end = start + usize::from(child_node.width());

                    self.dump
                        .push_str(&format!("{indent}{connector}{:?}\n", child_node.kind()));
                    self.dump_children(child_node, &child_indent);
                    self.dump
                        .push_str(&format!("{child_indent}Span: [{start}, {end})\n"));
                }
                GreenChild::Token(token) => {
                    let start = self.offset;
                    let end = start + usize::from(token.width());
                    self.offset = end;

                    self.dump
                        .push_str(&format!("{indent}{connector}{:?}\n", token.kind()));
                    self.dump
                        .push_str(&format!("{child_indent}  Lexeme: {:?}\n", token.lexeme()));
                    self.dump
                        .push_str(&format!("{child_indent}  Span: [{start}, {end})\n"));
                }
            }
        }
    }
}
