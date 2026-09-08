//! YAML correctness checking with compiler-style diagnostics.
//!
//! This module answers one question about an input: *is it correct YAML that
//! yqr can faithfully process?* It is the verification half of the editing
//! loop — after a human or an agent edits a file, [`check_str`] delivers a
//! verdict and [`render`] turns each finding into a rustc-style diagnostic
//! (severity, stable code, `--> file:line:col`, source window with caret,
//! and a help line) that both can act on.
//!
//! Three checks always run:
//!
//! - **Syntax**: every document in the stream parses on noyalib's CST.
//! - **Stream integrity**: the parsed documents reproduce the input
//!   byte-for-byte, the same invariant the fidelity engine asserts before
//!   trusting any read.
//! - **Block value indentation**: a mapping value that sits on a later line
//!   is indented past its key. The engine accepts one that is not; the rest
//!   of the ecosystem refuses the file, so accepting it silently would make
//!   a clean verdict mean less than it says.
//!
//! Keys that collide after string conversion (like `1:` and `"1":`) are
//! refused by the parser itself, so they surface through the default checks
//! with their own code. Strict mode adds the finding ordinary reads accept
//! silently: duplicate mapping keys — including duplicate `<<` merge keys —
//! resolved last-wins by virtually every parser, so a bad edit silently
//! drops data. Duplicates are found by walking the lossless green tree, so
//! every occurrence is reported with a real source position.

// Feature f012: the validate subcommand (spec: editing-loop verification).

mod render;
pub(crate) mod scan;

pub use render::render;

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
    /// `Y003` — the input bytes are not valid UTF-8.
    Encoding,
    /// `Y101` — a mapping declares the same key twice (strict mode).
    DuplicateKey,
    /// `Y102` — two distinct keys collapse to the same string key.
    KeyCollision,
    /// `Y103` — a block mapping's value is not indented past its key.
    BlockValueIndent,
}

impl Code {
    /// The stable code string rendered in the diagnostic header.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Code::Syntax => "Y001",
            Code::StreamIntegrity => "Y002",
            Code::Encoding => "Y003",
            Code::DuplicateKey => "Y101",
            Code::KeyCollision => "Y102",
            Code::BlockValueIndent => "Y103",
        }
    }
}

/// One validation finding, ready to render.
///
/// `position` is a 1-based `(line, column)` in the checked source, present
/// when the finding has one. `note` adds context (such as where the first
/// occurrence of a duplicate key sits) and `help` suggests a fix when a
/// concrete one exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Which finding this is.
    pub code: Code,
    /// One-line description of the problem.
    pub message: String,
    /// 1-based `(line, column)` when the finding has a location.
    pub position: Option<(usize, usize)>,
    /// Additional context line, rendered as `= note: ...`.
    pub note: Option<String>,
    /// Suggested fix, rendered as `= help: ...`.
    pub help: Option<String>,
}

/// Check `source` and return every finding, in reporting order.
///
/// An empty result means the input is valid. The syntax check runs first
/// and short-circuits: an unparseable input yields exactly one finding,
/// because follow-on findings would describe a document that does not
/// exist. Strict findings are reported in source order and require the
/// stream-integrity check to hold (their positions are computed from the
/// document offsets that check certifies).
#[must_use]
pub fn check_str(source: &str, strict: bool) -> Vec<Diagnostic> {
    let docs =
        match ::noyalib::cst::parse_stream_with_config(source, &crate::fidelity::cst_config()) {
            Ok(docs) => docs,
            Err(err) => return vec![syntax_diagnostic(&err, source)],
        };

    let mut findings = Vec::new();
    if let Some(diag) = tiling_diagnostic(source, docs.iter().map(::noyalib::cst::Document::source))
    {
        findings.push(diag);
    } else {
        findings.extend(block_value_indent_findings(source, &docs));
        if strict {
            findings.extend(strict_findings(source, &docs));
        }
    }
    findings
}

/// Build the finding for input bytes that are not valid UTF-8.
///
/// `valid_prefix` is the longest UTF-8 prefix of the input; the finding
/// points one past its end — the first offending byte. Encoding problems
/// are validation findings (the file's owner must re-encode), not
/// environment errors, so they carry a code and exit 1 like any other
/// finding.
#[must_use]
pub fn encoding_diagnostic(valid_prefix: &str) -> Diagnostic {
    Diagnostic {
        code: Code::Encoding,
        message: "input is not valid UTF-8".into(),
        position: Some(render::position_of(valid_prefix, valid_prefix.len())),
        note: None,
        help: Some("yqr reads YAML as UTF-8; re-encode the file (e.g. iconv -t UTF-8)".into()),
    }
}

/// Build the finding for a parse failure.
///
/// Almost every parse failure is a `Y001` syntax error, but a
/// stringified-key collision is refused by the parser itself — no yqr read
/// can process such a file — and gets its precise `Y102` here rather than
/// a generic syntax report. The parser locates the colliding key, so the
/// finding points at it and, in a multi-document stream, names the
/// document that holds it.
///
/// The bare message is taken from the error variant where possible (the
/// `Display` form embeds the location, which would duplicate the rendered
/// location line; for other located variants the embedded suffix is
/// stripped). When the file contains unresolved merge-conflict markers —
/// the most common way an edited file stops parsing — the diagnostic says
/// so directly and anchors itself at the first marker: the parser fails
/// somewhere past it, at a line that is a symptom rather than the cause.
/// In a stream, every finding names the document it is in.
fn syntax_diagnostic(err: &::noyalib::Error, source: &str) -> Diagnostic {
    // Every location the parser reports counts from the start of the
    // input, in a stream too (noyalib 0.0.36). Positions are derived from
    // that byte index through yqr's own line model rather than the
    // parser's line/column, which does not count lone-CR line breaks and
    // would garble CR-only files.
    let locate = |index: usize| render::position_of(source, index);
    let starts = document_starts(source);
    // The location-less collision is built only by noyalib's streaming
    // reader, which validate never uses; it is matched so a collision can
    // never degrade to a generic Y001.
    if let ::noyalib::Error::KeyCollision(key) | ::noyalib::Error::KeyCollisionAt { key, .. } = err
    {
        let position = err.location().map(|loc| locate(loc.index()));
        let note = err
            .location()
            .and_then(|loc| document_note(source, &starts, loc.index()));
        return key_collision_diagnostic(key, position, note);
    }
    let (message, help) = match err {
        ::noyalib::Error::Parse(m) | ::noyalib::Error::ParseWithLocation { message: m, .. } => {
            (m.clone(), None)
        }
        ::noyalib::Error::UnknownAnchorAt {
            name, suggestion, ..
        } => (
            format!("unknown anchor {name:?}"),
            suggestion.as_ref().map(|(s, at)| {
                let (line, _) = locate(at.index());
                // The parser suggests the alias's own name only when it
                // found that anchor in an earlier document of the stream.
                // It finds it by text, so the hint says "appears".
                if s == name {
                    format!(
                        "`&{s}` appears at line {line}, in an earlier document; \
                         anchors do not cross `---`"
                    )
                } else {
                    format!("a similar anchor &{s} is declared at line {line}")
                }
            }),
        ),
        other => {
            // Located variants embed " at line L, column C" in their
            // Display form; strip it so the location appears once, on the
            // rendered `-->` line.
            let mut m = other.to_string();
            if let Some(loc) = other.location() {
                m = m.replace(
                    &format!(" at line {}, column {}", loc.line(), loc.column()),
                    "",
                );
            }
            (m, None)
        }
    };
    // The finding is anchored at the first conflict marker when there is
    // one, else where the parser stopped.
    let (anchor, help) = match first_conflict_marker(source) {
        Some(marker) => {
            let (marker_line, _) = locate(marker);
            let help = format!(
                "the file contains unresolved merge-conflict markers (first at line \
                 {marker_line}); resolve the conflict"
            );
            (Some(marker), Some(help))
        }
        None => (err.location().map(|loc| loc.index()), help),
    };
    Diagnostic {
        code: Code::Syntax,
        message,
        position: anchor.map(locate),
        note: anchor.and_then(|index| document_note(source, &starts, index)),
        help,
    }
}

/// The byte offset of the first merge-conflict marker line in `source`,
/// if any.
///
/// Recognizes the three git marker shapes at the start of a line:
/// `<<<<<<< `, `=======`, and `>>>>>>> `. Checked against the whole file —
/// a conflict block breaks the parse somewhere *else*, at a line that is
/// a symptom, so inspecting only the error line would miss it.
fn first_conflict_marker(source: &str) -> Option<usize> {
    render::line_spans(source)
        .into_iter()
        .map(|(start, end)| (start, &source[start..end]))
        .find(|(_, l)| {
            l.starts_with("<<<<<<<") || l.starts_with(">>>>>>>") || l.trim_end() == "======="
        })
        .map(|(start, _)| start)
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

/// Collect every `Y103` finding: a block mapping's value on a later line that
/// is not indented past its key.
///
/// This runs in **default** mode, not under `--strict`, because it is not a
/// question of taste or of a policy other tools apply differently: the
/// document is invalid, and implementations outside noyalib refuse to read it
/// (`yqr-b014`). noyalib's parser accepts the shape, so this scan is the only
/// place the loop can notice — the fidelity engine's re-parse guard, which
/// re-parses with the same engine, cannot.
fn block_value_indent_findings(source: &str, docs: &[::noyalib::cst::Document]) -> Vec<Diagnostic> {
    let mut findings = Vec::new();
    let mut base = 0usize;
    for doc in docs {
        for hit in scan::under_indented_values(doc, base) {
            let (line, column) = render::position_of(source, hit.value);
            let (key_line, key_column) = render::position_of(source, hit.key);
            findings.push(Diagnostic {
                code: Code::BlockValueIndent,
                message: "block mapping value is not indented past its key".to_string(),
                position: Some((line, column)),
                note: Some(format!(
                    "its key is at line {key_line}, column {key_column}, so the value must \
                     start at column {} or deeper",
                    key_column + 1
                )),
                help: Some(
                    "indent the value, or write it on the key's own line; noyalib reads this \
                     file but other YAML implementations reject it"
                        .to_string(),
                ),
            });
        }
        base += doc.source().len();
    }
    findings
}

/// Collect every duplicate-mapping-key finding across the stream.
///
/// The green-tree scan (see [`scan`]) reports **all** duplicates —
/// including duplicate `<<` merge keys, which the value layer's
/// duplicate-key policy exempts — each with the byte offsets of both
/// occurrences, converted here to 1-based positions in the whole file.
fn strict_findings(source: &str, docs: &[::noyalib::cst::Document]) -> Vec<Diagnostic> {
    let mut findings = Vec::new();
    let mut base = 0usize;
    for doc in docs {
        for dup in scan::duplicate_keys(doc, base) {
            let (line, column) = render::position_of(source, dup.second);
            let (first_line, first_column) = render::position_of(source, dup.first);
            let (message, help) = if dup.key == "<<" {
                (
                    "duplicate merge key \"<<\"".to_string(),
                    "only one merge survives; combine the aliases into a single \
                     '<<: [*a, *b]' sequence"
                        .to_string(),
                )
            } else {
                (
                    format!("duplicate mapping key {:?}", dup.key),
                    "later occurrences silently override earlier ones; remove or rename one"
                        .to_string(),
                )
            };
            findings.push(Diagnostic {
                code: Code::DuplicateKey,
                message,
                position: Some((line, column)),
                note: Some(format!(
                    "first occurrence at line {first_line}, column {first_column}"
                )),
                help: Some(help),
            });
        }
        base += doc.source().len();
    }
    findings
}

/// Build the `Y102` finding for a stringified-key collision on `key`,
/// pointing at the colliding key when the parser located it.
fn key_collision_diagnostic(
    key: &str,
    position: Option<(usize, usize)>,
    note: Option<String>,
) -> Diagnostic {
    Diagnostic {
        code: Code::KeyCollision,
        message: format!("distinct mapping keys collide after string conversion: {key:?}"),
        position,
        note,
        help: Some(
            "yqr matches keys by spelling; rename one so the keys \
             stay distinct as strings"
                .into(),
        ),
    }
}

/// Name the document of a multi-document stream that holds byte `index`,
/// given the document starts from [`document_starts`]. `None` for a
/// single-document source, where the note would only repeat the
/// position.
fn document_note(source: &str, starts: &[usize], index: usize) -> Option<String> {
    if starts.len() < 2 {
        return None;
    }
    let document = document_index(starts, index);
    let (line, _) = render::position_of(source, starts[document]);
    Some(format!(
        "in document {} (starting at line {line})",
        document + 1
    ))
}

/// Zero-based index of the document holding byte `index`, given the
/// document starts from [`document_starts`].
fn document_index(starts: &[usize], index: usize) -> usize {
    starts
        .iter()
        .rposition(|&start| start <= index)
        .unwrap_or(0)
}

/// Byte offsets at which the documents of `source` start, split the way
/// the CST parser splits a stream: a `---` marker line opens a new
/// document only after some content, so directives, comments and blank
/// lines ahead of the first `---` are that document's prologue rather
/// than a document of their own; a `...` marker line closes the document
/// at the end of its line, and the next document starts right after it
/// once anything but a trailing comment follows, whether or not a `---`
/// opens it. A marker opens its line and is followed by whitespace or the
/// end of the line. The suite holds this to the lengths of the documents
/// the parser returns.
fn document_starts(source: &str) -> Vec<usize> {
    let spans = render::line_spans(source);
    let mut starts = vec![0];
    let mut has_content = false;
    // A `...` closed the document: the next `---` opens no further one.
    let mut closed = false;
    // Where the document after a `...` starts, once something follows it.
    let mut pending = None;
    for (i, &(start, end)) in spans.iter().enumerate() {
        let text = &source[start..end];
        let text = if start == 0 {
            text.strip_prefix('\u{FEFF}').unwrap_or(text)
        } else {
            text
        };
        if is_marker(text, "---") {
            if let Some(p) = pending.take() {
                starts.push(p);
            } else if has_content && !closed {
                starts.push(start);
            }
            has_content = true;
            closed = false;
        } else if is_marker(text, "...") {
            if has_content {
                let next = spans.get(i + 1).map_or(source.len(), |&(s, _)| s);
                pending = (next < source.len()).then_some(next);
                has_content = false;
            }
            closed = true;
        } else if !is_prologue(text) {
            if let Some(p) = pending.take() {
                starts.push(p);
            }
            has_content = true;
            closed = false;
        }
    }
    starts
}

/// Whether a line is the document marker `marker`: the marker at the
/// start of the line, followed by whitespace or the end of the line.
fn is_marker(line: &str, marker: &str) -> bool {
    line.strip_prefix(marker)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t']))
}

/// Whether a line carries no content for the parser: blank, a comment,
/// or a directive.
fn is_prologue(line: &str) -> bool {
    let trimmed = line.trim_start_matches([' ', '\t']);
    trimmed.is_empty() || trimmed.starts_with('#') || line.starts_with('%')
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

    // Bug b014: the shapes noyalib's parser lets through. Every input below
    // was checked against PyYAML and Ruby's Psych — the ones flagged here are
    // rejected by both, the ones asserted clean are accepted by both.

    #[test]
    fn under_indented_block_value_is_a_located_y103() {
        // The shape upstream's sole-entry `remove` writes, and the reason
        // yqr does not delegate that class (`yqr-f018` §4).
        let findings = check_str("on:\n[]\njobs: {}\n", false);
        assert_eq!(findings.len(), 1);
        let d = &findings[0];
        assert_eq!(d.code, Code::BlockValueIndent);
        assert_eq!(d.position, Some((2, 1)));
        assert!(
            d.note
                .as_ref()
                .is_some_and(|n| n.contains("column 2 or deeper"))
        );
    }

    #[test]
    fn y103_fires_in_default_mode_not_only_under_strict() {
        // The document is invalid, not merely questionable, so it is not a
        // strict-mode opinion — that distinction is the whole point of the
        // finding.
        assert_eq!(check_str("on:\nfoo\nb: 1\n", false).len(), 1);
        assert_eq!(check_str("on:\nfoo\nb: 1\n", true).len(), 1);
    }

    #[test]
    fn y103_is_reported_in_a_nested_mapping_and_a_later_document() {
        let nested = check_str("steps:\n  on:\n  []\nx: 1\n", false);
        assert_eq!(nested.len(), 1);
        assert_eq!(nested[0].position, Some((3, 3)));
        // Positions are absolute in the file, so the second document's
        // offsets have to carry the first document's length. (The trailing
        // `jobs:` entry is load-bearing: noyalib rejects the same under-indented
        // value outright when it is the mapping's only entry, and accepts it
        // when a sibling follows — the leniency is narrower than the shape.)
        let multi = check_str("a: 1\n---\non:\n[]\njobs: {}\n", false);
        assert_eq!(multi.len(), 1);
        assert_eq!(multi[0].code, Code::BlockValueIndent);
        assert_eq!(multi[0].position, Some((4, 1)));
    }

    #[test]
    fn a_block_sequence_at_its_keys_column_is_not_a_finding() {
        // The GitHub Actions / Ansible idiom. Valid YAML, and by far the most
        // common way a value sits at its key's column — a check that flagged
        // it would be worse than no check.
        assert!(check_str("on:\n- push\n- pull_request\njobs: {}\n", false).is_empty());
        assert!(check_str("jobs:\n  build:\n    steps:\n    - run: make\n", false).is_empty());
        assert!(check_str("on:\r\n- push\r\n", false).is_empty());
    }

    #[test]
    fn the_other_layouts_that_look_under_indented_are_not_findings() {
        // A block scalar's own content sets its indentation, so its header
        // may sit at the key's column.
        assert!(check_str("a:\n|\n  x\n", false).is_empty());
        // `a:` with no value, followed by a sibling — the sibling is not the
        // value, and the tree says so.
        assert!(check_str("a:\nb: 1\n", false).is_empty());
        assert!(check_str("a:\n  b:\n  c: 1\n", false).is_empty());
        // An explicit key measures its value against the `?`, under a rule
        // this scan does not claim to know.
        assert!(check_str("? a\n: b\n", false).is_empty());
        // An anchored value on its own line, properly indented.
        assert!(check_str("a:\n  &x 1\nb: *x\n", false).is_empty());
    }

    #[test]
    fn syntax_error_is_a_located_y001() {
        let findings = check_str("a: 1\n---\nb: [1,\n", false);
        assert_eq!(findings.len(), 1);
        let d = &findings[0];
        assert_eq!(d.code, Code::Syntax);
        // The position counts from the start of the stream, not of the
        // document the parser was on: end of input, one past the last
        // character, exactly where a single document reports it. Bug b028:
        // until noyalib 0.0.36 the parser counted from the document and the
        // unmapped index landed on the `[`.
        assert_eq!(d.position, Some((3, 7)));
        assert_eq!(
            d.note.as_deref(),
            Some("in document 2 (starting at line 2)")
        );
    }

    #[test]
    fn full_merge_conflict_block_gets_help_and_a_position() {
        // A complete three-marker git conflict: the parser fails past the
        // markers (at the `>>>>>>>` line since noyalib 0.0.36, unlocated
        // before), and the diagnostic anchors at the first marker either
        // way, where the cause is.
        let source = "a: 1\n<<<<<<< HEAD\nb: 2\n=======\nb: 3\n>>>>>>> feature\n";
        let findings = check_str(source, false);
        assert_eq!(findings.len(), 1, "findings: {findings:?}");
        let d = &findings[0];
        assert_eq!(d.code, Code::Syntax);
        assert!(
            d.help
                .as_deref()
                .is_some_and(|h| h.contains("merge-conflict")),
            "help: {:?}",
            d.help
        );
        assert_eq!(d.position, Some((2, 1)), "anchors at the first marker");
    }

    #[test]
    fn partial_conflict_marker_still_gets_help() {
        let findings = check_str("a: 1\n<<<<<<< HEAD\nb: 2\n", false);
        assert!(
            findings.iter().any(|d| d.code == Code::Syntax
                && d.help
                    .as_deref()
                    .is_some_and(|h| h.contains("merge-conflict"))),
            "findings: {findings:?}"
        );
    }

    #[test]
    fn unknown_anchor_message_carries_no_embedded_location() {
        let findings = check_str("a: *undef\n", false);
        assert_eq!(findings.len(), 1);
        let d = &findings[0];
        assert!(!d.message.contains("at line"), "message: {}", d.message);
        assert!(d.position.is_some());
    }

    #[test]
    fn duplicate_keys_are_strict_y101_with_positions_all_reported() {
        let source = "a: 1\nb: 2\na: 9\nb: 9\n";
        assert!(check_str(source, false).is_empty(), "default mode accepts");
        let findings = check_str(source, true);
        assert_eq!(findings.len(), 2, "both duplicates reported: {findings:?}");
        assert_eq!(findings[0].code, Code::DuplicateKey);
        assert_eq!(findings[0].position, Some((3, 1)));
        assert_eq!(
            findings[0].note.as_deref(),
            Some("first occurrence at line 1, column 1")
        );
        assert!(findings[1].message.contains("\"b\""));
        assert_eq!(findings[1].position, Some((4, 1)));
    }

    #[test]
    fn duplicate_merge_keys_are_reported() {
        let source = "x: &a\n  k: 1\ny: &b\n  k: 2\nz:\n  <<: *a\n  <<: *b\n";
        assert!(check_str(source, false).is_empty());
        let findings = check_str(source, true);
        assert_eq!(findings.len(), 1, "findings: {findings:?}");
        assert_eq!(findings[0].code, Code::DuplicateKey);
        assert!(findings[0].message.contains("merge key"));
        assert_eq!(findings[0].position, Some((7, 3)));
    }

    #[test]
    fn quoted_and_plain_spellings_of_the_same_key_are_duplicates() {
        // `a`, `'a'`, and `"a"` denote the same YAML string key.
        let findings = check_str("a: 1\n\"a\": 2\n", true);
        assert_eq!(findings.len(), 1, "findings: {findings:?}");
        assert_eq!(findings[0].code, Code::DuplicateKey);
    }

    #[test]
    fn nested_and_flow_duplicates_are_found() {
        let nested = check_str("m:\n  x: 1\n  x: 2\n", true);
        assert_eq!(nested.len(), 1, "nested: {nested:?}");
        let flow = check_str("m: {a: 1, a: 2}\n", true);
        assert_eq!(flow.len(), 1, "flow: {flow:?}");
    }

    #[test]
    fn key_collision_is_a_y102_by_default_with_document_note() {
        // The parser refuses collisions outright — no yqr read can process
        // such a file — so the finding needs no --strict. It points at the
        // colliding key (noyalib 0.0.33 locates it); in a stream the
        // affected document is named as well.
        let findings = check_str("1: a\n\"1\": b\n", false);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, Code::KeyCollision);
        assert_eq!(findings[0].position, Some((2, 1)));
        assert!(findings[0].note.is_none());
        assert!(findings[0].help.is_some());

        let streamed = check_str("a: 1\n---\nb: 2\n---\n1: x\n\"1\": y\n", false);
        assert_eq!(streamed.len(), 1);
        assert_eq!(streamed[0].position, Some((6, 1)));
        let note = streamed[0].note.as_deref().expect("document note");
        assert_eq!(note, "in document 3 (starting at line 4)");
    }

    #[test]
    fn located_errors_in_a_stream_count_from_the_stream() {
        // The parser locates an error in the stream (noyalib 0.0.36, bug
        // b028); the finding renders that position through yqr's line model.
        let findings = check_str("a: 1\n---\nb: 2\n---\nc: *nope\n", false);
        assert_eq!(findings.len(), 1, "findings: {findings:?}");
        assert_eq!(findings[0].code, Code::Syntax);
        assert_eq!(findings[0].position, Some((5, 4)));
        // The similar-anchor hint names the anchor's line in the stream.
        let hinted = check_str("a: 1\n---\nb: &nope 1\nc: *nop\n", false);
        assert_eq!(hinted.len(), 1, "findings: {hinted:?}");
        assert_eq!(hinted[0].position, Some((4, 4)));
        assert_eq!(
            hinted[0].help.as_deref(),
            Some("a similar anchor &nope is declared at line 3")
        );
        // An alias to an anchor of an earlier document is suggested under
        // its own name; the hint says why it does not resolve, whether the
        // boundary is a `---` or a `...`.
        let hint = "`&x` appears at line 1, in an earlier document; anchors do not cross `---`";
        for src in ["a: &x 1\n---\nb: *x\n", "a: &x 1\n...\nb: *x\n"] {
            let crossed = check_str(src, false);
            assert_eq!(crossed.len(), 1, "findings: {crossed:?}");
            assert_eq!(crossed[0].position, Some((3, 4)), "{src:?}");
            assert_eq!(crossed[0].help.as_deref(), Some(hint), "{src:?}");
        }
        // The parser finds that earlier `&x` by text, so the hint does not
        // claim it is an anchor: here it is a comment.
        let commented = check_str("a: 1 # see &x\n---\nb: *x\n", false);
        assert_eq!(commented[0].help.as_deref(), Some(hint));
    }

    #[test]
    fn document_starts_match_the_parsers_split() {
        // Held to the lengths of the documents the parser returns, so the
        // prologue and `...` rules cannot drift from it unnoticed.
        let streams = [
            "a: 1\n",
            "---\na: 1\n---\nb: 2\n",
            "%YAML 1.2\n---\na: 1\n",
            "# lead\n\n---\na: 1\n---\nb: 2\n",
            "a: 1\n...\nb: 2\n",
            "a: 1\n...\n---\nb: 2\n",
            "a: 1\n...\nb: 1\n---\nc: 2\n",
            "...\na: 1\n",
            "a: 1\n...\n...\nb: 2\n",
            "a: 1\n...\n# trailing\n",
            "a: 1\n...\n",
            "a: 1\r\n---\r\nb: 2\r\n",
            "\u{FEFF}a: 1\n---\nb: 2\n",
            "a: 1\n--- # c\nb: 2\n",
            "a: |\n  ---\n---\nb: 2\n",
            "--- a\n--- b\n",
            "---\n---\n",
        ];
        for src in streams {
            let docs =
                ::noyalib::cst::parse_stream_with_config(src, &crate::fidelity::cst_config())
                    .unwrap_or_else(|e| panic!("{src:?}: {e}"));
            let mut expected = vec![0];
            let mut offset = 0;
            for doc in &docs[..docs.len() - 1] {
                offset += doc.source().len();
                expected.push(offset);
            }
            assert_eq!(document_starts(src), expected, "stream {src:?}");
        }
        // The marker itself opens a line and is followed by whitespace or
        // the end of the line; a lone CR is a line break too.
        assert_eq!(
            document_starts("a: ---\n--- # doc\nb: 1\r---"),
            vec![0, 7, 22]
        );
        assert_eq!(document_starts("a: 1\n----\n"), vec![0]);
    }

    #[test]
    fn encoding_diagnostic_points_past_the_valid_prefix() {
        let d = encoding_diagnostic("a: 1\nb: ");
        assert_eq!(d.code, Code::Encoding);
        assert_eq!(d.position, Some((2, 4)));
        assert!(d.help.is_some());
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
    fn cr_only_line_breaks_position_and_render_correctly() {
        let source = "a: 1\rb: [1,\r";
        let findings = check_str(source, false);
        assert_eq!(findings.len(), 1);
        let rendered = render(&findings[0], "<stdin>", source);
        assert!(rendered.contains("| b: [1,"), "rendered:\n{rendered}");
        assert!(
            !rendered.contains("| a: 1\rb"),
            "no whole-file line:\n{rendered}"
        );
    }

    #[test]
    fn renders_located_diagnostic_rustc_style() {
        let source = "a: 1\n---\nb: [1,\n";
        let findings = check_str(source, false);
        let rendered = render(&findings[0], "deploy.yaml", source);
        let expected = "\
error[Y001]: expected a node but found StreamEnd
  --> deploy.yaml:3:7
  |
3 | b: [1,
  |       ^
  = note: in document 2 (starting at line 2)
";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn renders_end_of_input_error_clamped_to_the_last_line() {
        let source = "a: [1,\n";
        let findings = check_str(source, false);
        let rendered = render(&findings[0], "<stdin>", source);
        let expected = "\
error[Y001]: expected a node but found StreamEnd
  --> <stdin>:1:7
  |
1 | a: [1,
  |       ^
";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn renders_duplicate_key_with_window_note_and_help() {
        let source = "a: 1\na: 2\n";
        let findings = check_str(source, true);
        let rendered = render(&findings[0], "<stdin>", source);
        let expected = "\
error[Y101]: duplicate mapping key \"a\"
  --> <stdin>:2:1
  |
2 | a: 2
  | ^
  = note: first occurrence at line 1, column 1
  = help: later occurrences silently override earlier ones; remove or rename one
";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn tabs_in_the_source_line_keep_the_caret_aligned() {
        // The displayed line expands tabs to four spaces and the caret
        // padding counts the same expansion.
        let d = Diagnostic {
            code: Code::Syntax,
            message: "m".into(),
            position: Some((1, 3)),
            note: None,
            help: None,
        };
        let rendered = render(&d, "f", "\ta: 1\n");
        assert!(rendered.contains("1 |     a: 1\n"), "rendered:\n{rendered}");
        assert!(rendered.contains("  |      ^\n"), "rendered:\n{rendered}");
    }
}
