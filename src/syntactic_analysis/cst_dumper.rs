use crate::syntactic_analysis::cst::{GreenNode, NodeOrToken};

pub(crate) struct CstDumper;

impl CstDumper {
    pub(crate) fn dump(root: &GreenNode) -> String {
        let mut dump = String::new();
        let mut offset = 0;
        Self::dump_node(root, 0, &mut offset, &mut dump);
        dump
    }

    fn dump_node(node: &GreenNode, depth: usize, offset: &mut usize, dump: &mut String) {
        let start = *offset;
        let end = start + usize::from(node.width());
        dump.push_str(&"  ".repeat(depth));
        dump.push_str(&format!("{:?}@{}..{}\n", node.kind(), start, end));

        for child in node.children() {
            match child {
                NodeOrToken::Node(child_node) => Self::dump_node(child_node, depth + 1, offset, dump),
                NodeOrToken::Token(token) => {
                    let token_start = *offset;
                    let token_end = token_start + usize::from(token.width());
                    dump.push_str(&"  ".repeat(depth + 1));
                    dump.push_str(&format!(
                        "{:?}@{}..{} {:?}\n",
                        token.kind(),
                        token_start,
                        token_end,
                        token.text(),
                    ));
                    *offset = token_end;
                }
            }
        }
    }
}
