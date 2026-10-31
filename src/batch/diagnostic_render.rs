use ariadne::{Color, Label, Report, ReportKind, Source};

use crate::core::compiler::{Diagnostic, LabelKind};

pub(crate) fn render_diagnostic(diagnostic: Diagnostic, filename: &str, source: &str) {
    let mut report = Report::build(
        ReportKind::Error,
        filename,
        usize::from(diagnostic.range.start()),
    )
    .with_code(diagnostic.code)
    .with_message(diagnostic.message);

    for label in diagnostic.labels {
        let color = match label.kind {
            LabelKind::Primary => Color::Red,
            LabelKind::Secondary => Color::Blue,
        };
        let mut ariadne_label = Label::new((filename, label.range.into())).with_color(color);
        if let Some(message) = label.message {
            ariadne_label = ariadne_label.with_message(message);
        }
        report = report.with_label(ariadne_label);
    }

    // ariadne 0.4 keeps a single note and a single help per report
    if !diagnostic.notes.is_empty() {
        report = report.with_note(diagnostic.notes.join("\n"));
    }
    if !diagnostic.helps.is_empty() {
        report = report.with_help(diagnostic.helps.join("\n"));
    }

    report
        .finish()
        .eprint((filename, Source::from(source)))
        .unwrap();
}
