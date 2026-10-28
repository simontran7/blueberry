use std::fmt::Write;

use crate::core::syntactic_analysis::cst::{GreenChild, GreenNode};

pub(in crate::core) struct CstDumper<'cst> {
    root: &'cst GreenNode,
    dump: String,
    offset: usize,
}

impl<'cst> CstDumper<'cst> {
    pub(in crate::core) fn new(root: &'cst GreenNode) -> Self {
        Self {
            root,
            dump: String::new(),
            offset: 0,
        }
    }

    pub(in crate::core) fn dump(&mut self) -> String {
        writeln!(self.dump, "{:?}", self.root.kind()).unwrap();
        self.dump_children(self.root, "");
        writeln!(
            self.dump,
            "Span: [{}, {})",
            0,
            usize::from(self.root.width())
        )
        .unwrap();
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

                    writeln!(self.dump, "{indent}{connector}{:?}", child_node.kind()).unwrap();
                    self.dump_children(child_node, &child_indent);
                    writeln!(self.dump, "{child_indent}Span: [{start}, {end})").unwrap();
                }
                GreenChild::Token(token) => {
                    let start = self.offset;
                    let end = start + usize::from(token.width());
                    self.offset = end;

                    writeln!(self.dump, "{indent}{connector}{:?}", token.kind()).unwrap();
                    writeln!(self.dump, "{child_indent}  Lexeme: {:?}", token.lexeme()).unwrap();
                    writeln!(self.dump, "{child_indent}  Span: [{start}, {end})").unwrap();
                }
            }
        }
    }
}
