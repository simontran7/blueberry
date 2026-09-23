use crate::core::semantic_analysis::hir::nodes::{
    DefinitionBody, Expression, ExpressionHandle, Statement, StatementHandle, TypeAnnotation,
};
use crate::core::semantic_analysis::name_resolution::definition_tree::Definition;
use crate::core::semantic_analysis::{
    block_scoped_definitions_of, constant_body_of, constant_signature_of,
    file_scoped_definitions_of, function_body_of, function_signature_of,
};
use crate::core::source_file_key::SourceFileKey;

struct Node {
    label: String,
    children: Vec<Node>,
}

impl Node {
    fn leaf(label: impl Into<String>) -> Self {
        Self::new(label, Vec::new())
    }

    fn new(label: impl Into<String>, children: Vec<Node>) -> Self {
        Self {
            label: label.into(),
            children,
        }
    }

    fn labeled(mut self, name: &str) -> Self {
        self.label = format!("{name}: {}", self.label);
        self
    }

    fn write(&self, out: &mut String) {
        out.push_str(&self.label);
        out.push('\n');
        self.write_children("", out);
    }

    fn write_children(&self, prefix: &str, out: &mut String) {
        let last = self.children.len().saturating_sub(1);
        for (index, child) in self.children.iter().enumerate() {
            let is_last = index == last;
            out.push_str(prefix);
            out.push_str(if is_last { "└─ " } else { "├─ " });
            out.push_str(&child.label);
            out.push('\n');
            let child_prefix = format!("{prefix}{}", if is_last { "   " } else { "│  " });
            child.write_children(&child_prefix, out);
        }
    }
}

pub(crate) struct HirDumper<'db> {
    db: &'db dyn crate::Db,
}

impl<'db> HirDumper<'db> {
    pub(crate) fn dump_file(db: &'db dyn crate::Db, file: SourceFileKey) -> String {
        let dumper = Self { db };
        let mut out = String::new();
        for (index, definition) in file_scoped_definitions_of(db, file)
            .definitions()
            .iter()
            .enumerate()
        {
            if index > 0 {
                out.push('\n');
            }
            dumper.definition(*definition).write(&mut out);
        }
        out
    }

    fn annotation_text(&self, annotation: &TypeAnnotation<'db>) -> String {
        match annotation {
            TypeAnnotation::Path(symbol) => symbol.text(self.db).to_string(),
            TypeAnnotation::Hole => "_".to_string(),
        }
    }

    fn definition(&self, definition: Definition<'db>) -> Node {
        match definition {
            Definition::Function(key) => {
                let signature = function_signature_of(self.db, key);
                let body = function_body_of(self.db, key);
                let parameters: Vec<String> = body.binding_children[body.parameters]
                    .iter()
                    .zip(&signature.parameters)
                    .map(|(binding, annotation)| {
                        format!(
                            "{}: {}",
                            body.local_bindings[*binding].name.text(self.db),
                            self.annotation_text(annotation)
                        )
                    })
                    .collect();
                let returns = signature
                    .return_type_annotation
                    .as_ref()
                    .map_or_else(String::new, |ty| {
                        format!(" -> {}", self.annotation_text(ty))
                    });
                Node::new(
                    format!(
                        "function {}({}){returns}",
                        signature.name.text(self.db),
                        parameters.join(", ")
                    ),
                    vec![self.expression(&body, body.root)],
                )
            }
            Definition::Constant(key) => {
                let signature = constant_signature_of(self.db, key);
                let body = constant_body_of(self.db, key);
                let ty = signature
                    .type_annotation
                    .as_ref()
                    .map_or_else(String::new, |ty| format!(": {}", self.annotation_text(ty)));
                Node::new(
                    format!("constant {}{ty}", signature.name.text(self.db)),
                    vec![self.expression(&body, body.root)],
                )
            }
        }
    }

    fn expression(&self, body: &DefinitionBody<'db>, handle: ExpressionHandle) -> Node {
        match &body.expressions[handle] {
            Expression::Unit => Node::leaf("Unit"),
            Expression::Integer(value) => Node::leaf(format!("Integer {value}")),
            Expression::Boolean(value) => Node::leaf(format!("Boolean {value}")),
            Expression::Path(path) => {
                let segments: Vec<&str> = path
                    .segments(self.db)
                    .iter()
                    .map(|segment| segment.text(self.db))
                    .collect();
                Node::leaf(format!("Path {}", segments.join("::")))
            }
            Expression::Hole => Node::leaf("Hole"),
            Expression::If {
                condition_handle,
                then_branch_handle,
                else_branch_handle,
            } => {
                let mut children = vec![
                    self.expression(body, *condition_handle)
                        .labeled("condition"),
                    self.expression(body, *then_branch_handle).labeled("then"),
                ];
                if let Some(else_branch_handle) = else_branch_handle {
                    children.push(self.expression(body, *else_branch_handle).labeled("else"));
                }
                Node::new("If", children)
            }
            Expression::Block {
                block_key,
                statements,
                tail_handle,
            } => {
                let mut children: Vec<Node> = body.statement_children[*statements]
                    .iter()
                    .map(|statement| self.statement(body, *statement))
                    .collect();
                if let Some(tail_handle) = tail_handle {
                    children.push(self.expression(body, *tail_handle).labeled("tail"));
                }
                if let Some(block_key) = block_key {
                    children.push(Node::new(
                        "definitions",
                        block_scoped_definitions_of(self.db, *block_key)
                            .definitions()
                            .iter()
                            .map(|definition| self.definition(*definition))
                            .collect(),
                    ));
                }
                Node::new("Block", children)
            }
            Expression::Loop {
                source,
                body_handle: loop_body_handle,
            } => Node::new(
                format!("Loop {source:?}"),
                vec![self.expression(body, *loop_body_handle)],
            ),
            Expression::Call {
                callee_handle,
                arguments,
            } => {
                let mut children = vec![self.expression(body, *callee_handle).labeled("callee")];
                children.extend(
                    body.expression_children[*arguments]
                        .iter()
                        .map(|argument| self.expression(body, *argument).labeled("argument")),
                );
                Node::new("Call", children)
            }
            Expression::Continue => Node::leaf("Continue"),
            Expression::Break { value_handle } => Node::new(
                "Break",
                value_handle
                    .iter()
                    .map(|v| self.expression(body, *v))
                    .collect(),
            ),
            Expression::Return { value_handle } => Node::new(
                "Return",
                value_handle
                    .iter()
                    .map(|v| self.expression(body, *v))
                    .collect(),
            ),
            Expression::UnaryOperation {
                operand_handle,
                operator,
            } => Node::new(
                format!("Unary {operator}"),
                vec![self.expression(body, *operand_handle)],
            ),
            Expression::BinaryOperation {
                lhs_handle,
                operator,
                rhs_handle,
            } => {
                let operator = operator.map_or_else(|| "?".to_string(), |op| op.to_string());
                Node::new(
                    format!("Binary {operator}"),
                    vec![
                        self.expression(body, *lhs_handle),
                        self.expression(body, *rhs_handle),
                    ],
                )
            }
            Expression::Assignment {
                target_handle,
                value_handle,
            } => Node::new(
                "Assignment",
                vec![
                    self.expression(body, *target_handle),
                    self.expression(body, *value_handle),
                ],
            ),
        }
    }

    fn statement(&self, body: &DefinitionBody<'db>, handle: StatementHandle) -> Node {
        match &body.statements[handle] {
            Statement::Let {
                name_handle,
                annotation,
                initializer_handle,
            } => {
                let binding = &body.local_bindings[*name_handle];
                let mutability = if binding.mutable { "mut " } else { "" };
                let mut children = Vec::new();
                if let Some(annotation) = annotation {
                    let text = self.annotation_text(&body.type_annotations[*annotation]);
                    children.push(Node::leaf(format!("type: {text}")));
                }
                if let Some(initializer_handle) = initializer_handle {
                    children.push(
                        self.expression(body, *initializer_handle)
                            .labeled("initializer"),
                    );
                }
                Node::new(
                    format!("Let {mutability}{}", binding.name.text(self.db)),
                    children,
                )
            }
            Statement::Expression {
                expression_handle,
                has_semicolon,
            } => {
                let semicolon = if *has_semicolon { " ;" } else { "" };
                Node::new(
                    format!("Expression{semicolon}"),
                    vec![self.expression(body, *expression_handle)],
                )
            }
            Statement::Definition => Node::leaf("Definition"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::core::db::BlueberryDatabase;

    #[test]
    fn test_hir_dump() {
        insta::glob!(
            "../../syntactic_analysis/snapshot_inputs",
            "**/*.bb",
            |path| {
                let input = fs::read_to_string(path).unwrap();
                let db = BlueberryDatabase::default();
                let file = SourceFileKey::new(&db, path.to_path_buf(), input);
                insta::assert_snapshot!(HirDumper::dump_file(&db, file));
            }
        )
    }
}
