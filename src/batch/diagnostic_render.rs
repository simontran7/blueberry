use ariadne::{Color, Label, Report, ReportKind, Source};

use crate::core::common::diagnostic::{Diagnostic, LabelSeverity};

pub(crate) fn render_diagnostic(diagnostic: &Diagnostic, filename: &str, source: &str) {
    let description = diagnostic.describe();

    let mut report = Report::build(
        ReportKind::Error,
        filename,
        usize::from(description.span.start()),
    )
    .with_code(description.code)
    .with_message(description.message);

    for label in description.labels {
        let color = match label.severity {
            LabelSeverity::Primary => Color::Red,
            LabelSeverity::Secondary => Color::Blue,
        };
        let mut ariadne_label = Label::new((filename, label.span.into())).with_color(color);
        if let Some(message) = label.message {
            ariadne_label = ariadne_label.with_message(message);
        }
        report = report.with_label(ariadne_label);
    }

    report
        .finish()
        .eprint((filename, Source::from(source)))
        .unwrap();
}
