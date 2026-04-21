use ariadne::{Color, Label, Report, ReportKind, Source};

use super::diagnostic::{Severity, VerunError};

/// Convert a byte offset in `source` to a character (Unicode scalar) offset,
/// which is what ariadne expects for its span positions.
fn byte_to_char_offset(source: &str, byte_offset: usize) -> usize {
    let clamped = byte_offset.min(source.len());
    source[..clamped].chars().count()
}

/// Strip a leading "category: N:M: " location prefix from an error message so
/// ariadne's inline label doesn't duplicate the coordinates already shown in the gutter.
/// Matches patterns like "parse error: 3:10: ..." → "expected ..."
fn strip_location_prefix(msg: &str) -> &str {
    // Find "N:M: " after any "word... :" prefix
    // Walk past any non-digit prefix segments until we hit digits:digits: pattern
    let bytes = msg.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    // skip category prefix(es): anything up to and including the first ": " followed by digits
    while i < len {
        // look for ": " sequence
        if i + 1 < len && bytes[i] == b':' && bytes[i + 1] == b' ' {
            let rest = &msg[i + 2..];
            // check if rest starts with digits (line number)
            let digit_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            if digit_end > 0 {
                let after_digits = &rest[digit_end..];
                // expect ":digits: "
                if after_digits.starts_with(':') {
                    let after_colon = &after_digits[1..];
                    let col_end = after_colon.find(|c: char| !c.is_ascii_digit()).unwrap_or(after_colon.len());
                    if col_end > 0 {
                        let after_col = &after_colon[col_end..];
                        if after_col.starts_with(": ") {
                            return &after_col[2..];
                        }
                    }
                }
            }
        }
        i += 1;
    }
    msg
}

pub fn render_error(error: &VerunError, source: &str, filename: &str) -> String {
    let mut buf = Vec::new();

    let kind = match error.severity() {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
        Severity::Info => ReportKind::Advice,
    };

    let message = error.to_string();

    // For the inline label, strip any "category: N:M: " prefix so ariadne doesn't
    // duplicate the location that it already renders in the gutter.
    let label_message = strip_location_prefix(&message);

    if let Some(span) = error.span() {
        let start = byte_to_char_offset(source, span.start.min(source.len()));
        let end = byte_to_char_offset(source, span.end.min(source.len())).max(start + 1);

        Report::build(kind, filename, start)
            .with_message(&message)
            .with_label(
                Label::new((filename, start..end))
                    .with_message(label_message)
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
