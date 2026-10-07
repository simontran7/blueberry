use crate::core::common::span::Span;
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::semantic_diagnostic::SemanticDiagnostic;
use crate::core::semantic_analysis::{
    all_definitions_of, checked_body_of, constant_signature_of, function_signature_of,
};
use crate::core::source_file_key::SourceFileKey;

/// Dumps the type of every expression and local binding of every definition in a file, then
/// the file's semantic errors. Each expression is printed as `range 'source text': type`.
pub(crate) struct TypeDumper<'db> {
    db: &'db dyn crate::Db,
    source: &'db str,
}

impl<'db> TypeDumper<'db> {
    pub(crate) fn dump_file(db: &'db dyn crate::Db, file: SourceFileKey) -> String {
        let dumper = Self {
            db,
            source: file.contents(db),
        };
        let mut out = String::new();
        for (index, definition) in all_definitions_of(db, file).into_iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            dumper.definition(definition, &mut out);
        }

        let diagnostics = crate::core::semantic_analysis::semantic_diagnostics_of(db, file);
        if !diagnostics.is_empty() {
            out.push_str("\ndiagnostics\n");
            for diagnostic in &diagnostics {
                out.push_str(&dumper.diagnostic(diagnostic));
            }
        }
        out
    }

    fn definition(&self, definition: Definition<'db>, out: &mut String) {
        let header = match definition {
            Definition::Function(key) => {
                format!(
                    "function {}",
                    function_signature_of(self.db, key).name.text(self.db)
                )
            }
            Definition::Constant(key) => {
                format!(
                    "constant {}",
                    constant_signature_of(self.db, key).name.text(self.db)
                )
            }
        };
        out.push_str(&header);
        out.push('\n');

        let (body, source_map, types) = checked_body_of(self.db, definition);

        for (binding_handle, ty) in types.local_bindings() {
            out.push_str(&format!(
                "  binding {}: {}\n",
                body.local_bindings[binding_handle].name.text(self.db),
                ty.display(self.db)
            ));
        }

        let mut expressions: Vec<_> = types
            .expressions()
            .map(|(expression_handle, ty)| (source_map.expressions[expression_handle].span, ty))
            .collect();
        expressions.sort_by_key(|(span, _)| (span.start(), span.end()));
        for (span, ty) in expressions {
            out.push_str(&format!(
                "  {} '{}': {}\n",
                self.range(span),
                self.text(span),
                ty.display(self.db)
            ));
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
            line.push_str(&format!(" ({message})"));
        }
        line.push('\n');
        // every other label, e.g. one pointing back at a signature
        for (index, label) in description.labels.iter().enumerate() {
            if Some(index) == main_label {
                continue;
            }
            if let Some(message) = &label.message {
                line.push_str(&format!(
                    "    {} '{}': {message}\n",
                    self.range(label.span),
                    self.text(label.span)
                ));
            }
        }
        for note in &description.notes {
            line.push_str(&format!("    note: {note}\n"));
        }
        for help in &description.helps {
            line.push_str(&format!("    help: {help}\n"));
        }
        line
    }

    fn range(&self, span: Span) -> String {
        format!("{}..{}", usize::from(span.start()), usize::from(span.end()))
    }

    /// Returns the source text at `span`, on one line and shortened.
    fn text(&self, span: Span) -> String {
        let text = self.source[std::ops::Range::from(span)]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        ellipsize(text, 15)
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
    fn test_type_dump() {
        insta::glob!("snapshot_inputs", "**/*.bb", |path| {
            let input = fs::read_to_string(path).unwrap();
            let db = BlueberryDatabase::default();
            let file = SourceFileKey::new(&db, path.to_path_buf(), input);
            insta::assert_snapshot!(TypeDumper::dump_file(&db, file));
        })
    }
}
