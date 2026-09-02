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
mod scan;

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
    let docs = match ::noyalib::cst::parse_stream(source) {
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
/// a generic syntax report.
///
/// The bare message is taken from the error variant where possible (the
/// `Display` form embeds the location, which would duplicate the rendered
/// location line; for other located variants the embedded suffix is
/// stripped). When the file contains unresolved merge-conflict markers —
/// the most common way an edited file stops parsing, and one the parser
/// often reports as an unlocated indentation error — the diagnostic says
/// so directly and anchors itself at the first marker if the parser gave
/// no location.
fn syntax_diagnostic(err: &::noyalib::Error, source: &str) -> Diagnostic {
    if let ::noyalib::Error::KeyCollision(key) = err {
        return key_collision_diagnostic(key, collision_document_note(source, key));
    }
    let (message, help) = match err {
        ::noyalib::Error::Parse(m) | ::noyalib::Error::ParseWithLocation { message: m, .. } => {
            (m.clone(), None)
        }
        // Bug b025: the trip is a parser resource heuristic, not a YAML
        // syntax rule, and heavy anchor reuse in ordinary values files
        // sets it off — say so instead of implying a syntax defect.
        ::noyalib::Error::Budget(::noyalib::BudgetBreach::AliasAnchorRatio { .. }) => (
            err.to_string(),
            Some(
                "this is a parser resource heuristic tripped by heavy anchor reuse, \
                 not a YAML syntax rule"
                    .into(),
            ),
        ),
        ::noyalib::Error::UnknownAnchorAt {
            name, suggestion, ..
        } => (
            format!("unknown anchor {name:?}"),
            suggestion.as_ref().map(|(s, loc)| {
                let (line, _) = render::position_of(source, loc.index());
                format!("a similar anchor &{s} is declared at line {line}")
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
    // Positions are derived from the error's byte index through yqr's own
    // line model rather than the parser's line/column, which does not
    // count lone-CR line breaks and would garble CR-only files.
    let mut position = err
        .location()
        .map(|loc| render::position_of(source, loc.index()));
    let mut help = help;
    if let Some(marker_line) = first_conflict_marker_line(source) {
        help = Some(format!(
            "the file contains unresolved merge-conflict markers (first at line \
             {marker_line}); resolve the conflict"
        ));
        if position.is_none() {
            position = Some((marker_line, 1));
        }
    }
    Diagnostic {
        code: Code::Syntax,
        message,
        position,
        note: None,
        help,
    }
}

/// The 1-based line of the first merge-conflict marker in `source`, if any.
///
/// Recognizes the three git marker shapes at the start of a line:
/// `<<<<<<< `, `=======`, and `>>>>>>> `. Checked against the whole file —
/// a conflict block usually breaks the parse somewhere *else* (the parser
/// frequently reports an unlocated indentation error), so inspecting only
/// the error line would miss it.
fn first_conflict_marker_line(source: &str) -> Option<usize> {
    (1..=render::line_count(source)).find(|&n| {
        render::line_text(source, n).is_some_and(|l| {
            l.starts_with("<<<<<<<") || l.starts_with(">>>>>>>") || l.trim_end() == "======="
        })
    })
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

/// Locate which document of a multi-document stream holds the collision.
///
/// The collision aborts the whole stream parse, so no per-document
/// structure exists; the stream is re-split lexically at `---` document
/// markers and each chunk re-parsed. The note is attached only when
/// exactly one chunk reproduces a collision on the same key — anything
/// ambiguous stays note-less rather than risking a wrong pointer.
fn collision_document_note(source: &str, key: &str) -> Option<String> {
    let count = render::line_count(source);
    let mut chunk_starts = vec![1usize];
    for n in 2..=count {
        if render::line_text(source, n).is_some_and(|l| {
            l == "---"
                || l.strip_prefix("---")
                    .is_some_and(|r| r.starts_with([' ', '\t']))
        }) {
            chunk_starts.push(n);
        }
    }
    if chunk_starts.len() < 2 {
        return None;
    }
    let mut matches = Vec::new();
    for (index, &start) in chunk_starts.iter().enumerate() {
        let end = chunk_starts.get(index + 1).copied().unwrap_or(count + 1);
        let chunk: String = (start..end)
            .filter_map(|n| render::line_text(source, n))
            .fold(String::new(), |mut acc, l| {
                acc.push_str(l);
                acc.push('\n');
                acc
            });
        if let Err(::noyalib::Error::KeyCollision(k)) =
            ::noyalib::from_str::<::noyalib::Value>(&chunk)
            && k == key
        {
            matches.push((index + 1, start));
        }
    }
    match matches.as_slice() {
        [(index, start)] => Some(format!("in document {index} (starting at line {start})")),
        _ => None,
    }
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
        // The location is absolute in the file, not relative to a document.
        assert_eq!(d.position, Some((3, 3)));
    }

    #[test]
    fn full_merge_conflict_block_gets_help_and_a_position() {
        // A complete three-marker git conflict: the parser reports an
        // unlocated indentation error, so the diagnostic must anchor at
        // the first marker itself.
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
        assert!(d.position.is_some(), "must anchor at a marker line");
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
        // such a file — so the finding needs no --strict; in a stream the
        // affected document is named.
        let findings = check_str("1: a\n\"1\": b\n", false);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, Code::KeyCollision);
        assert!(findings[0].help.is_some());

        let streamed = check_str("a: 1\n---\nb: 2\n---\n1: x\n\"1\": y\n", false);
        assert_eq!(streamed.len(), 1);
        let note = streamed[0].note.as_deref().expect("document note");
        assert!(note.contains("document 3"), "note: {note}");
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
  --> deploy.yaml:3:3
  |
3 | b: [1,
  |   ^
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
