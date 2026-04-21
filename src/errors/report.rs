use ariadne::{Color, Label, Report, ReportKind, Source};

use super::diagnostic::{Severity, VerunError};

pub fn render_error(error: &VerunError, source: &str, filename: &str) -> String {
    let mut buf = Vec::new();

    let kind = match error.severity() {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
        Severity::Info => ReportKind::Advice,
    };

    let message = error.to_string();

    if let Some(span) = error.span() {
        let start = span.start.min(source.len());
        let end = span.end.min(source.len()).max(start);

        Report::build(kind, filename, start)
            .with_message(&message)
            .with_label(
                Label::new((filename, start..end))
                    .with_message(&message)
                    .with_color(Color::Red),
            )
            .finish()
            .write((filename, Source::from(source)), &mut buf)
            .ok();
    } else {
        Report::<(&str, std::ops::Range<usize>)>::build(kind, filename, 0)
            .with_message(&message)
            .finish()
            .write((filename, Source::from(source)), &mut buf)
            .ok();
    }

    String::from_utf8(buf).unwrap_or_else(|_| message)
}

pub fn render_errors(errors: &[VerunError], source: &str, filename: &str) -> String {
    errors
        .iter()
        .map(|e| render_error(e, source, filename))
        .collect::<Vec<_>>()
        .join("\n")
}
