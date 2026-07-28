//! Rustc-style rendering of validation diagnostics, and the line model
//! shared by every position computation.
//!
//! YAML recognizes `\r\n`, `\n`, and a lone `\r` as line breaks; this
//! module's line model counts all three, and every rendered position is
//! derived from a byte offset through it. The parser's own line/column is
//! never trusted directly — it ignores lone-CR breaks, which would garble
//! windows for CR-only files (classic-Mac line endings, which the fidelity
//! engine explicitly supports).

// Feature f012: diagnostic rendering.

use super::Diagnostic;

/// Byte ranges `(start, end)` of every line's content, break excluded.
///
/// Splits on `\r\n`, `\n`, and lone `\r` — the YAML line-break set — so
/// line numbers agree with the parser's own accounting.
fn line_spans(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                spans.push((start, i));
                i += 1;
                start = i;
            }
            b'\r' => {
                spans.push((start, i));
                i += if bytes.get(i + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < bytes.len() {
        spans.push((start, bytes.len()));
    }
    spans
}

/// The 1-based line count of `source` under the YAML line-break set.
pub(crate) fn line_count(source: &str) -> usize {
    line_spans(source).len()
}

/// The content of 1-based `line`, break excluded. `None` when out of range.
pub(crate) fn line_text(source: &str, line: usize) -> Option<&str> {
    let (start, end) = *line_spans(source).get(line.checked_sub(1)?)?;
    Some(&source[start..end])
}

/// The 1-based `(line, column)` of byte offset `byte` in `source`.
///
/// The column counts characters from the line start. An offset at or past
/// the line's content end (pointing at the break, or at end of input)
/// resolves to one past the line's last character. Every position yqr
/// renders is derived here from a byte offset, so line numbers stay
/// consistent across `\n`, `\r\n`, and CR-only files regardless of which
/// line model the parser used internally.
pub(crate) fn position_of(source: &str, byte: usize) -> (usize, usize) {
    let spans = line_spans(source);
    let byte = byte.min(source.len());
    let line_idx = spans
        .iter()
        .rposition(|&(start, _)| start <= byte)
        .unwrap_or(0);
    let Some(&(start, end)) = spans.get(line_idx) else {
        return (1, 1);
    };
    let column = source[start..byte.clamp(start, end)].chars().count() + 1;
    (line_idx + 1, column)
}

/// Render `diag` as a rustc-style diagnostic block, ending in a newline.
///
/// `display_name` names the input (a path, or `<stdin>`); `source` is the
/// checked text, used to extract the offending line for the caret window.
/// A position past the last line (how the parser reports end-of-input
/// errors) is clamped to the end of the last line, so a truncated file
/// still gets its source window. Tabs in the offending line are expanded
/// to four spaces before the caret column is computed, keeping the caret
/// aligned. The output is stable and colour-free so it can be pinned by
/// golden tests and parsed by scripts.
#[must_use]
pub fn render(diag: &Diagnostic, display_name: &str, source: &str) -> String {
    let mut out = format!("error[{}]: {}\n", diag.code.as_str(), diag.message);

    match diag.position {
        Some((line, column)) => {
            let count = line_count(source);
            // End-of-input errors point one past the last line; clamp to
            // the end of the last line so the window still renders.
            let (line, column) = if count > 0 && line > count {
                let text = line_text(source, count).unwrap_or("");
                (count, text.chars().count() + 1)
            } else {
                (line, column)
            };
            out.push_str(&format!("  --> {display_name}:{line}:{column}\n"));
            if let Some(text) = line_text(source, line) {
                let display: String = text.replace('\t', "    ");
                let gutter = line.to_string();
                let pad = " ".repeat(gutter.len());
                out.push_str(&format!("{pad} |\n"));
                out.push_str(&format!("{gutter} | {display}\n"));
                let caret_cols: usize = text
                    .chars()
                    .take(column.saturating_sub(1))
                    .map(|c| if c == '\t' { 4 } else { 1 })
                    .sum();
                let caret_pad = " ".repeat(caret_cols.min(display.chars().count()));
                out.push_str(&format!("{pad} | {caret_pad}^\n"));
            }
        }
        None => out.push_str(&format!("  --> {display_name}\n")),
    }

    if let Some(note) = &diag.note {
        out.push_str(&format!("  = note: {note}\n"));
    }
    if let Some(help) = &diag.help {
        out.push_str(&format!("  = help: {help}\n"));
    }
    out
}
