//! YAML correctness checking with compiler-style diagnostics.
//!
//! This module answers one question about an input: *is it correct YAML that
//! yqr can faithfully process?* It is the verification half of the editing
//! loop — after a human or an agent edits a file, [`check_str`] delivers a
//! verdict and [`render`] turns each finding into a rustc-style diagnostic
//! (severity, stable code, `--> file:line:col`, source window with caret,
//! and a help line) that both can act on.
//!
//! Two checks always run:
//!
//! - **Syntax**: every document in the stream parses on noyalib's CST.
//! - **Stream integrity**: the parsed documents reproduce the input
//!   byte-for-byte, the same invariant the fidelity engine asserts before
//!   trusting any read.
//!
//! Keys that collide after string conversion (like `1:` and `"1":`) are
//! refused by the parser itself, so they surface through the default checks
//! with their own code. Strict mode adds the finding ordinary reads accept
//! silently: duplicate mapping keys, resolved last-wins by virtually every
//! parser, so a bad edit silently drops data.
//!
//! Diagnostics carry a location only when the underlying error reports one;
//! duplicate-key and collision errors name the offending key instead, since
//! the parser does not expose key spans.

// Feature f012: the validate subcommand (spec: editing-loop verification).

/// Stable identifier of a validation finding.
///
/// Codes are part of yqr's CLI contract: scripts may match on them, so they
/// are never renumbered or reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// `Y001` — the input is not well-formed YAML.
    Syntax,
    /// `Y002` — the parsed documents do not reproduce the input
    /// byte-for-byte (the fidelity invariant does not hold).
    StreamIntegrity,
    /// `Y101` — a mapping declares the same key twice (strict mode).
    DuplicateKey,
    /// `Y102` — two distinct keys collapse to the same string key.
    KeyCollision,
}

impl Code {
    /// The stable code string rendered in the diagnostic header.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Code::Syntax => "Y001",
            Code::StreamIntegrity => "Y002",
            Code::DuplicateKey => "Y101",
            Code::KeyCollision => "Y102",
        }
    }
}

/// One validation finding, ready to render.
///
/// `line` and `column` are 1-based positions in the checked source; they are
/// present only when the underlying parser reported a location. `note` adds
/// context (such as which document of a multi-document stream is affected)
/// and `help` suggests a fix when a concrete one exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Which finding this is.
    pub code: Code,
    /// One-line description of the problem.
    pub message: String,
    /// 1-based `(line, column)` when the parser reported a location.
    pub position: Option<(usize, usize)>,
    /// Additional context line, rendered as `= note: ...`.
    pub note: Option<String>,
    /// Suggested fix, rendered as `= help: ...`.
    pub help: Option<String>,
}

/// Check `source` and return every finding, in reporting order.
///
/// An empty result means the input is valid. The syntax check runs first
/// and short-circuits: an unparseable input yields exactly one `Y001`
/// finding, because follow-on findings would describe a document that does
/// not exist. Strict findings are reported per document, in document order.
#[must_use]
pub fn check_str(source: &str, strict: bool) -> Vec<Diagnostic> {
    let docs = match ::noyalib::cst::parse_stream(source) {
        Ok(docs) => docs,
        Err(err) => return vec![syntax_diagnostic(&err, source)],
    };

    let mut findings = Vec::new();
    if let Some(diag) = tiling_diagnostic(source, docs.iter().map(::noyalib::cst::Document::source))
    {
        findings.push(diag);
    }
    if strict {
        findings.extend(strict_findings(&docs));
    }
    findings
}

/// Build the finding for a parse failure.
///
/// Almost every parse failure is a `Y001` syntax error, but a
/// stringified-key collision is refused by the parser itself — no yqr read
/// can process such a file — and gets its precise `Y102` here rather than
/// a generic syntax report.
///
/// The bare message is taken from the error variant (the `Display` form
/// prefixes it with "YAML parse error at ...", which would duplicate the
/// rendered location line). A location is attached when the parser reported
/// one, and an unresolved merge-conflict marker on the offending line gets a
/// dedicated help text — the most common way an edited file stops parsing.
fn syntax_diagnostic(err: &::noyalib::Error, source: &str) -> Diagnostic {
    if let ::noyalib::Error::KeyCollision(key) = err {
        return key_collision_diagnostic(key, None);
    }
    let message = match err {
        ::noyalib::Error::Parse(m) | ::noyalib::Error::ParseWithLocation { message: m, .. } => {
            m.clone()
        }
        other => other.to_string(),
    };
    let position = err.location().map(|loc| (loc.line(), loc.column()));
    let help = position
        .and_then(|(line, _)| source.lines().nth(line.saturating_sub(1)))
        .filter(|l| {
            l.starts_with("<<<<<<<") || l.starts_with("=======") || l.starts_with(">>>>>>>")
        })
        .map(|_| "this line is an unresolved merge-conflict marker; resolve the conflict".into());
    Diagnostic {
        code: Code::Syntax,
        message,
        position,
        note: None,
        help,
    }
}

/// Build the `Y002` finding when the parsed documents do not tile `source`.
///
/// This is the validate-side statement of the fidelity invariant: the
/// concatenated per-document sources must equal the input byte-for-byte.
/// It cannot fire for any input the parser handles correctly — a finding
/// here means the engine could silently corrupt untouched bytes, which is
/// why validate refuses instead of shrugging.
fn tiling_diagnostic<'a, I>(source: &str, doc_sources: I) -> Option<Diagnostic>
where
    I: Iterator<Item = &'a str>,
{
    let rebuilt: String = doc_sources.collect();
    (rebuilt != source).then(|| Diagnostic {
        code: Code::StreamIntegrity,
        message: "parsed documents do not reproduce the input byte-for-byte".into(),
        position: None,
        note: Some(format!(
            "reassembled {} bytes from the parse, but the input has {}",
            rebuilt.len(),
            source.len()
        )),
        help: Some("the file exercises a parser defect; report it with this input attached".into()),
    })
}

/// Run the strict checks over every document and collect their findings.
///
/// Each document is re-loaded through the value layer with the
/// duplicate-key policy set to error, which surfaces both strict findings
/// with the offending key's name: a genuine duplicate (`Y101`) or a
/// stringified-key collision (`Y102`). Errors of any other kind are not
/// strict findings and are ignored — the default checks already accepted
/// the document. The parser reports no key spans, so the diagnostic names
/// the key and (in a multi-document stream) the affected document instead
/// of a source position.
fn strict_findings(docs: &[::noyalib::cst::Document]) -> Vec<Diagnostic> {
    let mut cfg = ::noyalib::ParserConfig::default();
    cfg.duplicate_key_policy = ::noyalib::DuplicateKeyPolicy::Error;

    let mut findings = Vec::new();
    let mut start_line = 1usize;
    for (index, doc) in docs.iter().enumerate() {
        let doc_note = (docs.len() > 1)
            .then(|| format!("in document {} (starting at line {start_line})", index + 1));
        match ::noyalib::from_str_with_config::<::noyalib::Value>(doc.source(), &cfg) {
            Err(::noyalib::Error::DuplicateKey(key)) => findings.push(Diagnostic {
                code: Code::DuplicateKey,
                message: format!("duplicate mapping key {key:?}"),
                position: None,
                note: doc_note,
                help: Some(
                    "later occurrences silently override earlier ones; \
                     remove or rename one"
                        .into(),
                ),
            }),
            // Collisions are normally refused by the CST parse itself
            // (surfacing through the default checks); this arm covers any
            // that only materialize at the value layer, such as through
            // merge-key expansion.
            Err(::noyalib::Error::KeyCollision(key)) => {
                findings.push(key_collision_diagnostic(&key, doc_note));
            }
            _ => {}
        }
        start_line += doc.source().matches('\n').count();
    }
    findings
}

/// Build the `Y102` finding for a stringified-key collision on `key`.
fn key_collision_diagnostic(key: &str, note: Option<String>) -> Diagnostic {
    Diagnostic {
        code: Code::KeyCollision,
        message: format!("distinct mapping keys collide after string conversion: {key:?}"),
        position: None,
        note,
        help: Some(
            "yqr matches keys by spelling; rename one so the keys \
             stay distinct as strings"
                .into(),
        ),
    }
}

/// Render `diag` as a rustc-style diagnostic block, ending in a newline.
///
/// `display_name` names the input (a path, or `<stdin>`); `source` is the
/// checked text, used to extract the offending line for the caret window.
/// The output is stable and colour-free so it can be pinned by golden tests
/// and parsed by scripts.
#[must_use]
pub fn render(diag: &Diagnostic, display_name: &str, source: &str) -> String {
    let mut out = format!("error[{}]: {}\n", diag.code.as_str(), diag.message);

    match diag.position {
        Some((line, column)) => {
            out.push_str(&format!("  --> {display_name}:{line}:{column}\n"));
            if let Some(text) = source.lines().nth(line.saturating_sub(1)) {
                let gutter = line.to_string();
                let pad = " ".repeat(gutter.len());
                out.push_str(&format!("{pad} |\n"));
                out.push_str(&format!("{gutter} | {text}\n"));
                let caret_pad = " ".repeat(column.saturating_sub(1).min(text.len()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_input_has_no_findings() {
        assert!(check_str("a: 1\nb:\n  - x\n", false).is_empty());
        assert!(check_str("", false).is_empty());
        assert!(check_str("a: 1\n---\nb: 2\n", true).is_empty());
    }

    #[test]
    fn syntax_error_is_a_located_y001() {
        let findings = check_str("a: 1\n---\nb: [1,\n", false);
        assert_eq!(findings.len(), 1);
        let d = &findings[0];
        assert_eq!(d.code, Code::Syntax);
        // The location is absolute in the file, not relative to a document.
        assert_eq!(d.position, Some((3, 1)));
    }

    #[test]
    fn merge_conflict_marker_gets_help() {
        let source = "a: 1\n<<<<<<< HEAD\nb: 2\n";
        let findings = check_str(source, false);
        let with_marker_help = findings.iter().any(|d| {
            d.code == Code::Syntax
                && d.help
                    .as_deref()
                    .is_some_and(|h| h.contains("merge-conflict"))
        });
        assert!(with_marker_help, "findings: {findings:?}");
    }

    #[test]
    fn duplicate_key_is_a_strict_y101() {
        let source = "a: 1\nb: 2\na: 3\n";
        assert!(check_str(source, false).is_empty(), "default mode accepts");
        let findings = check_str(source, true);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, Code::DuplicateKey);
        assert!(findings[0].message.contains("\"a\""));
    }

    #[test]
    fn key_collision_is_a_y102_by_default() {
        // The parser refuses collisions outright — no yqr read can process
        // such a file — so the finding needs no --strict.
        let source = "1: a\n\"1\": b\n";
        let findings = check_str(source, false);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, Code::KeyCollision);
        assert!(findings[0].help.is_some());
    }

    #[test]
    fn strict_findings_name_the_document_in_a_stream() {
        let findings = check_str("a: 1\n---\nb: 2\nb: 3\n", true);
        assert_eq!(findings.len(), 1);
        let note = findings[0].note.as_deref().expect("multi-doc note");
        assert!(note.contains("document 2"), "note: {note}");
    }

    #[test]
    fn tiling_mismatch_is_a_y002() {
        // The invariant cannot be broken through the real parser, so the
        // check is exercised directly with a fabricated mismatch.
        let diag = tiling_diagnostic("a: 1\nX", ["a: 1\n"].into_iter());
        let diag = diag.expect("mismatch must be reported");
        assert_eq!(diag.code, Code::StreamIntegrity);
        assert!(tiling_diagnostic("a: 1\n", ["a: 1\n"].into_iter()).is_none());
    }

    #[test]
    fn renders_located_diagnostic_rustc_style() {
        let source = "a: 1\n---\nb: [1,\n";
        let findings = check_str(source, false);
        let rendered = render(&findings[0], "deploy.yaml", source);
        let expected = "\
error[Y001]: expected a node but found StreamEnd
  --> deploy.yaml:3:1
  |
3 | b: [1,
  | ^
";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn renders_unlocated_diagnostic_with_note_and_help() {
        let findings = check_str("a: 1\na: 2\n", true);
        let rendered = render(&findings[0], "<stdin>", "a: 1\na: 2\n");
        let expected = "\
error[Y101]: duplicate mapping key \"a\"
  --> <stdin>
  = help: later occurrences silently override earlier ones; remove or rename one
";
        assert_eq!(rendered, expected);
    }
}
