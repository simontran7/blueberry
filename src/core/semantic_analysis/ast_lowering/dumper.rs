use std::fmt::Write;
use std::sync::Arc;

use crate::core::common::span::{Span, TextSize};
use crate::core::semantic_analysis::ast_lowering::nodes::{
    Body, Expression, ExpressionHandle, HirNodeSourceMap, Signature, Statement, StatementHandle,
};
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::semantic_diagnostic::SemanticDiagnostic;
use crate::core::semantic_analysis::type_checking::type_checker::BodySemantics;
use crate::core::semantic_analysis::{
    all_definitions_of, block_definitions_of, body_of, body_source_map_of,
    definition_diagnostics_of, import_diagnostics_of, semantics_of, signature_of,
    top_level_definitions_of,
};
use crate::core::source_file_key::SourceFileKey;

pub(in crate::core) struct HirBodyDumper<'db> {
    db: &'db dyn crate::Db,
    source: &'db str,
    with_types: bool,
}

struct BodyTypes<'db> {
    semantics: Arc<BodySemantics<'db>>,
    source_map: Arc<HirNodeSourceMap>,
    anchor: TextSize,
}

struct Node {
    label: String,
    children: Box<[Node]>,
}

impl<'db> HirBodyDumper<'db> {
    #[cfg(test)]
    pub(in crate::core) fn dump_file(db: &'db dyn crate::Db, file: SourceFileKey) -> String {
        Self::dump(db, file, false)
    }

    pub(in crate::core) fn dump_file_with_types(
        db: &'db dyn crate::Db,
        file: SourceFileKey,
    ) -> String {
        Self::dump(db, file, true)
    }

    fn dump(db: &'db dyn crate::Db, file: SourceFileKey, with_types: bool) -> String {
        let dumper = Self {
            db,
            source: file.contents(db),
            with_types,
        };
        let mut out = String::new();
        for (index, definition) in top_level_definitions_of(db, file)
            .definitions()
            .iter()
            .enumerate()
        {
            if index > 0 {
                out.push('\n');
            }
            dumper.definition(*definition).write(&mut out);
        }
        if with_types {
            out.push_str(&dumper.diagnostics(file));
        }
        out
    }

    fn diagnostics(&self, file: SourceFileKey) -> String {
        let mut diagnostics = import_diagnostics_of(self.db, file);
        diagnostics.extend(
            all_definitions_of(self.db, file)
                .into_iter()
                .flat_map(|definition| {
                    definition_diagnostics_of(self.db, definition)
                        .iter()
                        .cloned()
                }),
        );
        diagnostics.sort_by_key(|diagnostic| diagnostic.describe().span.start());
        let mut out = String::new();
        if !diagnostics.is_empty() {
            out.push_str("\ndiagnostics\n");
            for diagnostic in &diagnostics {
                out.push_str(&self.diagnostic(diagnostic));
            }
        }
        out
    }

    fn definition(&self, definition: Definition<'db>) -> Node {
        let signature = signature_of(self.db, definition);
        let body = body_of(self.db, definition);
        let types = self.with_types.then(|| BodyTypes {
            semantics: semantics_of(self.db, definition).clone(),
            source_map: body_source_map_of(self.db, definition),
            anchor: definition.anchor(self.db),
        });
        let types = types.as_ref();
        let store = signature.store();
        let name = signature.name().text(self.db);
        let label = match &**signature {
            Signature::Function(function) => {
                let parameters: Vec<String> = body
                    .parameter_handles
                    .iter()
                    .zip(&function.parameter_annotation_handles)
                    .map(|(binding_handle, annotation_handle)| {
                        format!(
                            "{}: {}",
                            body.store()
                                .local_binding(*binding_handle)
                                .name()
                                .text(self.db),
                            store.display_type_expression(*annotation_handle, self.db)
                        )
                    })
                    .collect();
                let returns =
                    function
                        .return_type_annotation
                        .map_or_else(String::new, |annotation_handle| {
                            format!(
                                " -> {}",
                                store.display_type_expression(annotation_handle, self.db)
                            )
                        });
                format!("function {name}({}){returns}", parameters.join(", "))
            }
            Signature::Constant(constant) => {
                let ty = constant
                    .annotation
                    .map_or_else(String::new, |annotation_handle| {
                        format!(
                            ": {}",
                            store.display_type_expression(annotation_handle, self.db)
                        )
                    });
                format!("constant {name}{ty}")
            }
        };
        Node::new(label, vec![self.expression(body, types, body.root_handle)])
    }

    fn expression(
        &self,
        body: &Body<'db>,
        types: Option<&BodyTypes<'db>>,
        expression_handle: ExpressionHandle,
    ) -> Node {
        let node = match body.store().expression(expression_handle) {
            Expression::Integer(Some(value)) => Node::leaf(format!("Integer {value}")),
            Expression::Integer(None) => Node::leaf("Integer (too large)"),
            Expression::Boolean(value) => Node::leaf(format!("Boolean {value}")),
            Expression::Path(path) => Node::leaf(format!("Path {}", path.display(self.db))),
            Expression::Tuple { element_run } => Node::new(
                "Tuple",
                body.store()
                    .expression_handles(*element_run)
                    .iter()
                    .map(|element_handle| self.expression(body, types, *element_handle))
                    .collect(),
            ),
            Expression::Field {
                receiver_handle,
                name,
            } => Node::new(
                format!("Field {}", name.map_or("?", |name| name.text(self.db))),
                vec![self.expression(body, types, *receiver_handle)],
            ),
            Expression::Error => Node::leaf("Error"),
            Expression::If {
                condition_handle,
                then_branch_handle,
                else_branch_handle,
            } => {
                let mut children = vec![
                    self.expression(body, types, *condition_handle)
                        .labeled("condition"),
                    self.expression(body, types, *then_branch_handle)
                        .labeled("then"),
                ];
                if let Some(else_branch_handle) = else_branch_handle {
                    children.push(
                        self.expression(body, types, *else_branch_handle)
                            .labeled("else"),
                    );
                }
                Node::new("If", children)
            }
            Expression::Block {
                block_key,
                statement_run,
                tail_handle,
            } => {
                let mut children: Vec<Node> = body
                    .store()
                    .statement_handles(*statement_run)
                    .iter()
                    .map(|statement_handle| self.statement(body, types, *statement_handle))
                    .collect();
                if let Some(tail_handle) = tail_handle {
                    children.push(self.expression(body, types, *tail_handle).labeled("tail"));
                }
                if let Some(block_key) = block_key {
                    let definitions = block_definitions_of(self.db, *block_key);
                    if !definitions.definitions().is_empty() {
                        children.push(Node::new(
                            "definitions",
                            definitions
                                .definitions()
                                .iter()
                                .map(|definition| self.definition(*definition))
                                .collect(),
                        ));
                    }
                }
                Node::new("Block", children)
            }
            Expression::Loop {
                source,
                body_handle: loop_body_handle,
            } => Node::new(
                format!("Loop {source:?}"),
                vec![self.expression(body, types, *loop_body_handle)],
            ),
            Expression::Call {
                callee_handle,
                argument_run,
            } => {
                let mut children = vec![
                    self.expression(body, types, *callee_handle)
                        .labeled("callee"),
                ];
                children.extend(body.store().expression_handles(*argument_run).iter().map(
                    |argument_handle| {
                        self.expression(body, types, *argument_handle)
                            .labeled("argument")
                    },
                ));
                Node::new("Call", children)
            }
            Expression::Continue => Node::leaf("Continue"),
            Expression::Break { value_handle } => Node::new(
                "Break",
                value_handle
                    .iter()
                    .map(|value_handle| self.expression(body, types, *value_handle))
                    .collect(),
            ),
            Expression::Return { value_handle } => Node::new(
                "Return",
                value_handle
                    .iter()
                    .map(|value_handle| self.expression(body, types, *value_handle))
                    .collect(),
            ),
            Expression::UnaryOperation {
                operand_handle,
                operator,
            } => Node::new(
                format!("Unary {operator}"),
                vec![self.expression(body, types, *operand_handle)],
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
                        self.expression(body, types, *lhs_handle),
                        self.expression(body, types, *rhs_handle),
                    ],
                )
            }
            Expression::Assignment {
                target_handle,
                value_handle,
            } => Node::new(
                "Assignment",
                vec![
                    self.expression(body, types, *target_handle),
                    self.expression(body, types, *value_handle),
                ],
            ),
        };
        match types {
            Some(types) => node
                .ranged(
                    &self.range(
                        types
                            .source_map
                            .expression_span(expression_handle, types.anchor),
                    ),
                )
                .typed(
                    &types
                        .semantics
                        .expression_type(expression_handle)
                        .display(self.db)
                        .to_string(),
                ),
            None => node,
        }
    }

    fn statement(
        &self,
        body: &Body<'db>,
        types: Option<&BodyTypes<'db>>,
        statement_handle: StatementHandle,
    ) -> Node {
        match body.store().statement(statement_handle) {
            Statement::Let {
                name_handle,
                annotation_handle,
                initializer_handle,
            } => {
                let binding = body.store().local_binding(*name_handle);
                let mutability = if binding.mutable { "mut " } else { "" };
                let mut children = Vec::new();
                if let Some(annotation_handle) = annotation_handle {
                    let text = body
                        .store
                        .display_type_expression(*annotation_handle, self.db);
                    children.push(Node::leaf(format!("type: {text}")));
                }
                children.push(
                    self.expression(body, types, *initializer_handle)
                        .labeled("initializer"),
                );
                let node = Node::new(
                    format!("Let {mutability}{}", binding.name.text(self.db)),
                    children,
                );
                match types {
                    Some(types) => node.typed(
                        &types
                            .semantics
                            .local_binding_type(*name_handle)
                            .display(self.db)
                            .to_string(),
                    ),
                    None => node,
                }
            }
            Statement::Expression {
                expression_handle,
                has_semicolon,
            } => {
                let semicolon = if *has_semicolon { " ;" } else { "" };
                Node::new(
                    format!("Expression{semicolon}"),
                    vec![self.expression(body, types, *expression_handle)],
                )
            }
            Statement::Definition => Node::leaf("Definition"),
        }
    }

    fn diagnostic(&self, diagnostic: &SemanticDiagnostic) -> String {
        let description = diagnostic.describe();
        let mut line = format!(
            "  {} {} '{}': {}",
            description.code,
            self.range(description.span),
            self.text(description.span),
            description.message
        );
        let main_label = description
            .labels
            .iter()
            .position(|label| label.span == description.span && label.message.is_some());
        if let Some(index) = main_label {
            let message = description.labels[index].message.as_ref().unwrap();
            write!(line, " ({message})").unwrap();
        }
        line.push('\n');
        for (index, label) in description.labels.iter().enumerate() {
            if Some(index) == main_label {
                continue;
            }
            if let Some(message) = &label.message {
                writeln!(
                    line,
                    "    {} '{}': {message}",
                    self.range(label.span),
                    self.text(label.span)
                )
                .unwrap();
            }
        }
        for note in &description.notes {
            writeln!(line, "    note: {note}").unwrap();
        }
        for help in &description.helps {
            writeln!(line, "    help: {help}").unwrap();
        }
        line
    }

    fn range(&self, span: Span) -> String {
        format!("{}..{}", usize::from(span.start()), usize::from(span.end()))
    }

    fn text(&self, span: Span) -> String {
        let text = self.source[std::ops::Range::from(span)]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        ellipsize(text, 15)
    }
}

impl Node {
    fn leaf(label: impl Into<String>) -> Self {
        Self::new(label, Vec::new())
    }

    fn new(label: impl Into<String>, children: Vec<Node>) -> Self {
        Self {
            label: label.into(),
            children: children.into_boxed_slice(),
        }
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

    fn labeled(mut self, name: &str) -> Self {
        self.label = format!("{name}: {}", self.label);
        self
    }

    fn ranged(mut self, range: &str) -> Self {
        self.label = format!("{} {range}", self.label);
        self
    }

    fn typed(mut self, ty: &str) -> Self {
        self.label = format!("{}: {ty}", self.label);
        self
    }
}

fn ellipsize(mut text: String, max_len: usize) -> String {
    if text.len() <= max_len {
        return text;
    }
    let ellipsis = "...";
    let mut prefix_len = (max_len - ellipsis.len()) / 2;
    while !text.is_char_boundary(prefix_len) {
        prefix_len += 1;
    }
    let mut suffix_len = max_len - ellipsis.len() - prefix_len;
    while !text.is_char_boundary(text.len() - suffix_len) {
        suffix_len += 1;
    }
    text.replace_range(prefix_len..text.len() - suffix_len, ellipsis);
    text
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
                insta::assert_snapshot!(HirBodyDumper::dump_file(&db, file));
            }
        );
    }
}
