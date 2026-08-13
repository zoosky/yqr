//! The fidelity write tier: surgical, source-preserving edits.
//!
//! Where the read seam ([`super::FidelityEngine`]) slices original bytes to
//! *emit* an untouched node, this module *mutates* the source in place and
//! leaves every other byte identical — or refuses. It is the write-side
//! analogue of the read seam: a small `FidelityWriter` trait bounds the
//! engine's edit surface, and the concrete `NoyalibWriter` routes each edit
//! through noyalib's *typed* mutators (`set_value` / `insert_entry_value` /
//! `push_back_value`) — never the fragment-taking ones, which splice a
//! caller-built string verbatim and whose guard rejects invalid YAML but not
//! valid-but-misinterpreted YAML. Delete is yqr's own, for the reason
//! `delete_entry` documents.
//!
//! The typed mutators do not all guard equally, which is worth knowing before
//! trusting one. The two *insertion* mutators carry a load-back oracle: after
//! the splice the document must load as the pre-edit value with exactly that
//! insertion applied, or the edit rolls back. `set_value` has no such oracle —
//! it formats for the site and splices, so it inherits only the re-parse
//! check. A spelling defect there produces a wrong value rather than a
//! refusal, which has happened, so a `set_value` case is worth an explicit
//! round-trip test rather than an assumption.
//!
//! The write path is the read path with the terminal call swapped: the
//! evaluator resolves a filter to a concrete [`Path`], the same
//! `to_noyalib_path` builder lowers it to a
//! string path, and a mutator addressed by that string applies the edit. Each
//! mutator returns `Result<()>`; that `Result` carries the structural-integrity
//! guard — a refused edit is reported (exit 5) and the document is left
//! unchanged — to the strength described above, which differs per mutator.

// Feature f006 (see specs/features/): write tier v1 — value assignment.

use crate::Value;
use crate::ast::Mutation;
use crate::error::{Result, YqrError};
use crate::eval::{AssignTarget, resolve_assign_target, resolve_rhs, resolve_target};
use crate::fidelity::{Path, PathSeg};

// Structural delete lives in a sub-module so the byte-arithmetic concern stays
// separate from the value-write trait. It extends `NoyalibWriter` with
// `delete_entry`, addressing the same private state through Rust's
// ancestor-module privacy.
mod delete;

/// A source-preserving *writer* over one parsed YAML input.
///
/// Implementations own an editable view of the document stream and apply
/// path-targeted mutations that change only the targeted node's bytes. Every
/// method that mutates is guarded: if the edit would restructure the document,
/// it returns an error and leaves the stream unchanged, so a failed edit can
/// never corrupt the output.
///
/// The trait is object-safe and drivable as `&mut dyn FidelityWriter`,
/// mirroring the read seam.
pub(crate) trait FidelityWriter {
    /// Number of logical YAML documents in the stream.
    fn doc_count(&self) -> usize;

    /// The typed value of document `doc`, in yqr's evaluation model — the same
    /// lossy view the read seam exposes, used to resolve a mutation target.
    ///
    /// # Errors
    ///
    /// Returns an error when `doc` is out of range.
    fn value(&self, doc: usize) -> Result<Value>;

    /// Replace the scalar at `path` with `value`, matching the neighbouring
    /// quoting style.
    ///
    /// Unlike the insertion methods, this carries only a re-parse check, not a
    /// load-back oracle: a mis-spelled value can be committed rather than
    /// refused, so round-trip coverage is the caller's responsibility.
    ///
    /// # Errors
    ///
    /// Errors when the path is unaddressable, does not resolve to a scalar, the
    /// value is a collection, or the edit would re-parse differently.
    fn set_value(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()>;

    /// Insert a new `key: value` entry into the mapping at `parent`.
    ///
    /// The implementation places and spells `value`; callers pass a value, not
    /// a rendered fragment, so quoting and indentation are never theirs.
    ///
    /// # Errors
    ///
    /// Errors when the parent is unaddressable, is not a non-empty block
    /// mapping, or `key` cannot be addressed once written. Also errors when
    /// the result would not **load back** as the pre-edit value with exactly
    /// this insertion applied — a stronger contract than "still parses", and
    /// the one an alternative implementation must meet.
    fn insert_key(&mut self, doc: usize, parent: &Path, key: &str, value: &Value) -> Result<()>;

    /// Append `value` as a new item to the block sequence at `path`.
    ///
    /// # Errors
    ///
    /// Errors when the path is unaddressable, is not a non-empty block
    /// sequence, or the result would not load back as the pre-edit value with
    /// exactly this insertion applied (see [`Self::insert_key`]).
    fn append(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()>;

    /// Remove the block entry at `path`, whether single-line, multi-line, or a
    /// nested collection. The entry's own lines (its key/`-`, continuation, and
    /// any head comment documenting it) go; every surviving node stays
    /// byte-identical.
    ///
    /// # Errors
    ///
    /// Errors when the path is unaddressable, is the sole entry of its block,
    /// is an item of a flow collection, or the edit would re-parse to a
    /// different document.
    fn delete(&mut self, doc: usize, path: &Path) -> Result<()>;

    /// Emit the whole document stream: edited documents reflect their edits,
    /// every other document is byte-identical to the input.
    fn emit(&self) -> String;
}

/// Apply a single [`Mutation`] to `input` and return the whole emitted stream.
///
/// The mutation is applied to every document whose target resolves; documents
/// where the path is absent are emitted byte-identically. It is an error for a
/// mutation to match no document at all.
///
/// This function performs no file I/O — the caller decides whether to print the
/// result or write it back (see the `-i` handling in `main`).
///
/// A mutation that matches no document is a successful **no-op**: the input is
/// returned unchanged (jq/yq semantics), so `del(.x)` over a batch of files
/// leaves files that lack `.x` untouched instead of failing them.
///
/// # Errors
///
/// Returns an error when the input is not valid YAML, the target is
/// ambiguous/unaddressable, or an edit is refused by the re-parse guard.
pub fn apply(mutation: &Mutation, input: &str) -> Result<String> {
    let mut writer = NoyalibWriter::open(input)?;
    for doc in 0..writer.doc_count() {
        let value = writer.value(doc)?;
        apply_to_doc(&mut writer, doc, mutation, &value)?;
    }
    Ok(writer.emit())
}

/// Apply `mutation` to a single document.
///
/// A document whose target does not resolve is left untouched (a no-op, not an
/// error); an `Err` means the edit was attempted and refused by the re-parse
/// guard.
fn apply_to_doc(
    writer: &mut dyn FidelityWriter,
    doc: usize,
    mutation: &Mutation,
    value: &Value,
) -> Result<()> {
    // Resolve the target (and decide whether to skip this document) *before*
    // evaluating the RHS: a document whose target does not resolve is left
    // untouched, so a path RHS that happens to be absent in that document must
    // not be evaluated (and must not turn a skip into a hard error).
    match mutation {
        Mutation::Assign { path, rhs } => {
            let Some(target) = resolve_assign_target(path, value)? else {
                return Ok(());
            };
            let rhs_value = resolve_rhs(rhs, value)?;
            match target {
                AssignTarget::Existing(target) => writer.set_value(doc, &target, &rhs_value),
                AssignTarget::NewKey { parent, key } => {
                    writer.insert_key(doc, &parent, &key, &rhs_value)
                }
            }
        }
        Mutation::Append { path, rhs } => {
            let Some(target) = resolve_target(path, value)? else {
                return Ok(());
            };
            let item = resolve_rhs(rhs, value)?;
            writer.append(doc, &target, &item)
        }
        Mutation::Delete { path } => match resolve_target(path, value)? {
            Some(target) => writer.delete(doc, &target),
            None => Ok(()),
        },
    }
}

/// [`FidelityWriter`] backed by noyalib's editable `cst::Document` stream.
pub(crate) struct NoyalibWriter {
    /// One editable CST document per logical YAML document.
    docs: Vec<::noyalib::cst::Document>,
    /// Per document: was its source wholly CRLF-terminated at open time? The
    /// mutators end an inserted line with `\n` whatever the document uses, so
    /// this records what to restore. See [`Self::emit`].
    crlf: Vec<bool>,
}

impl NoyalibWriter {
    /// Parse `input` into an editable document stream.
    ///
    /// Defensively verifies that concatenating the per-document sources
    /// reproduces the input byte-for-byte before trusting any edit — the same
    /// fidelity invariant the read engine asserts at open time.
    pub(crate) fn open(input: &str) -> Result<Self> {
        let docs = ::noyalib::cst::parse_stream(input)
            .map_err(|e| YqrError::io(format!("failed to parse YAML input: {e}")))?;
        // Same fidelity invariant the read engine asserts at open time: emit
        // concatenates each document, so a slice that diverged from the input
        // would corrupt an untouched document.
        super::noyalib::verify_stream_tiles_input(input, &docs)?;
        let crlf = docs.iter().map(|d| is_all_crlf(&d.to_string())).collect();
        Ok(Self { docs, crlf })
    }

    /// Bounds-checked mutable document accessor.
    fn doc_mut(&mut self, doc: usize) -> Result<&mut ::noyalib::cst::Document> {
        let len = self.docs.len();
        self.docs
            .get_mut(doc)
            .ok_or_else(|| YqrError::eval(format!("document index {doc} out of range ({len})")))
    }

    /// Bounds-checked shared document accessor (used by the structural-delete
    /// fallback, which reads spans and source bytes before it mutates).
    fn doc_ref(&self, doc: usize) -> Result<&::noyalib::cst::Document> {
        let len = self.docs.len();
        self.docs
            .get(doc)
            .ok_or_else(|| YqrError::eval(format!("document index {doc} out of range ({len})")))
    }
}

impl FidelityWriter for NoyalibWriter {
    fn doc_count(&self) -> usize {
        self.docs.len()
    }

    fn value(&self, doc: usize) -> Result<Value> {
        let len = self.docs.len();
        let doc = self
            .docs
            .get(doc)
            .ok_or_else(|| YqrError::eval(format!("document index {doc} out of range ({len})")))?;
        Ok(Value::from(&*doc.as_value()))
    }

    fn set_value(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()> {
        let path_str = noyalib_path(path)?;
        // Same scalar-only limit as the insert paths, checked here rather than
        // left to the engine: `set_value`'s own refusal names `set` and
        // fragments, APIs yqr never exposes, and reports a *parse* error for
        // input that parses fine.
        let ny = insertable(value)?;
        self.doc_mut(doc)?
            .set_value(&path_str, &ny)
            .map_err(|e| YqrError::eval(format!("cannot assign at {path_str:?}: {e}")))
    }

    fn insert_key(&mut self, doc: usize, parent: &Path, key: &str, value: &Value) -> Result<()> {
        // A key holding `.` or `[` composes into a path meaning something else,
        // so it cannot be *addressed*. Creating one is still refused here even
        // though the typed tier can splice it (it only needs a path to replace
        // an existing key), because a key yqr can write but not read back is
        // its own trap. Lifting this is tracked as structural-edit work.
        if !PathSeg::key_is_plain(key) {
            return Err(YqrError::eval(format!(
                "cannot create key {key:?}: it uses characters the write path cannot express"
            )));
        }
        let parent_str = noyalib_path(parent)?;
        let ny = insertable(value)?;
        self.doc_mut(doc)?
            .insert_entry_value(&parent_str, key, &ny)
            .map_err(|e| YqrError::eval(format!("cannot insert key {key:?}: {e}")))
    }

    fn append(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()> {
        let path_str = noyalib_path(path)?;
        let ny = insertable(value)?;
        self.doc_mut(doc)?
            .push_back_value(&path_str, &ny)
            .map_err(|e| YqrError::eval(format!("cannot append to {path_str:?}: {e}")))
    }

    fn delete(&mut self, doc: usize, path: &Path) -> Result<()> {
        // Deliberately not noyalib's `remove`, though the gap has narrowed: it
        // accepts the same shapes since 0.0.18, and since 0.0.19 it folds the
        // same entry-owned trivia this path does (that fix was yqr's, filed as
        // the b006 rules diverging from it). What keeps the two separate now is
        // churn, not semantics — 0.0.21 fixed a `remove` that destroyed a whole
        // flow collection while reporting success, and this path carries the
        // b006 rules next to the tests that pin them. Revisit deliberately.
        self.delete_entry(doc, path)
    }

    fn emit(&self) -> String {
        self.docs
            .iter()
            .zip(&self.crlf)
            .map(|(doc, &was_crlf)| {
                let out = doc.to_string();
                // An inserted line is terminated with `\n` regardless of the
                // document's convention, so a CRLF file silently gained mixed
                // endings. A document that was wholly CRLF has no bare `\n` of
                // its own, which makes the restore exact rather than a guess:
                // every bare `\n` left in the output is one this edit added.
                // A mixed-ending document is left alone — there is no
                // convention to restore, and guessing one would be the same
                // class of unasked-for rewrite.
                if was_crlf && out.contains('\n') {
                    restore_crlf(&out)
                } else {
                    out
                }
            })
            .collect()
    }
}

/// Lower a [`Path`] to noyalib's string-path grammar, erroring when a key is
/// not expressible in it (the same "unaddressable" gap the read path reports).
fn noyalib_path(path: &Path) -> Result<String> {
    super::noyalib::to_noyalib_path(path).ok_or_else(|| {
        let key = super::noyalib::offending_key(path);
        YqrError::eval(format!(
            "cannot address key {key:?}: it uses characters the write path cannot express"
        ))
    })
}

/// Does `s` use CRLF for every line break it has, with no bare `\n`?
///
/// False for an LF document, for a mixed one, and for a document with no line
/// break at all — in each case there is no single convention an inserted line
/// should adopt.
fn is_all_crlf(s: &str) -> bool {
    let mut saw_crlf = false;
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            if i == 0 || bytes[i - 1] != b'\r' {
                return false;
            }
            saw_crlf = true;
        }
    }
    saw_crlf
}

/// Re-terminate every bare `\n` in `s` as `\r\n`, leaving existing `\r\n` alone.
///
/// Only ever applied to a document that was wholly CRLF before the edit, so the
/// bare breaks it finds are exactly the ones an inserted line brought.
fn restore_crlf(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    let mut prev = '\0';
    for c in s.chars() {
        if c == '\n' && prev != '\r' {
            out.push('\r');
        }
        out.push(c);
        prev = c;
    }
    out
}

/// Lower a scalar [`Value`] to the noyalib value the typed mutators take.
///
/// Passing a value rather than a rendered fragment is what lets the engine
/// place and spell it. Hand-building the fragment instead put yqr on the wrong
/// side of the guard: a string containing a newline renders to a block scalar,
/// which the fragment-taking mutators splice without re-indenting its
/// continuation lines, silently producing a wrong value or unparseable output.
///
/// A collection value stays refused. The typed tier can express one, so this is
/// now a scope limit on the mutating filters rather than a backend constraint —
/// lifting it is structural-edit work, not a bug fix.
// Bug b008: the fragment-splice corruption this refusal and the typed lowering
// together replace.
fn insertable(value: &Value) -> Result<::noyalib::Value> {
    if matches!(value, Value::Sequence(_) | Value::Mapping(_)) {
        return Err(YqrError::eval(
            "the right-hand side of '+=' or a new-key assignment must be a scalar \
             (number, string, boolean, or null); collections are not yet supported"
                .to_string(),
        ));
    }
    Ok(::noyalib::Value::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Rhs;

    fn assign(path: &str, rhs: Rhs) -> Mutation {
        Mutation::Assign {
            path: crate::parser::parse(path).expect("valid path"),
            rhs,
        }
    }

    #[test]
    fn set_value_replaces_only_the_target_scalar() {
        let out = apply(
            &assign(".spec.replicas", Rhs::Literal(Value::Int(5))),
            "spec:\n  replicas: 3  # keep me\n  image: web\n",
        )
        .unwrap();
        assert_eq!(out, "spec:\n  replicas: 5  # keep me\n  image: web\n");
    }

    #[test]
    fn set_value_matches_neighbouring_quote_style() {
        // `name` is single-quoted; the replacement keeps that style.
        let out = apply(
            &assign(".name", Rhs::Literal(Value::String("web2".into()))),
            "name: 'web'\n",
        )
        .unwrap();
        assert_eq!(out, "name: 'web2'\n");
    }

    #[test]
    fn new_key_is_inserted_under_existing_mapping() {
        let out = apply(
            &assign(".metadata.env", Rhs::Literal(Value::String("prod".into()))),
            "metadata:\n  name: app\n",
        )
        .unwrap();
        assert_eq!(out, "metadata:\n  name: app\n  env: prod\n");
    }

    #[test]
    fn append_pushes_a_block_sequence_item() {
        let out = apply(
            &Mutation::Append {
                path: crate::parser::parse(".spec.ports").expect("valid"),
                rhs: Rhs::Literal(Value::Int(9090)),
            },
            "spec:\n  ports:\n    - 8080\n",
        )
        .unwrap();
        assert_eq!(out, "spec:\n  ports:\n    - 8080\n    - 9090\n");
    }

    // Bug b008: a multi-line string used to be hand-rendered to a block scalar
    // and spliced verbatim, so its continuation lines kept the *rendering's*
    // indentation rather than the insertion site's. The typed tier owns the
    // indent now, and its oracle refuses anything that would load differently.

    #[test]
    fn appended_multiline_string_is_indented_for_its_insertion_site() {
        let out = apply(
            &Mutation::Append {
                path: crate::parser::parse(".s").expect("valid"),
                rhs: Rhs::Literal(Value::String("v\nqq: 7".into())),
            },
            "keep: 0\ns:\n  - one\n",
        )
        .unwrap();
        assert_eq!(out, "keep: 0\ns:\n  - one\n  - |-\n      v\n      qq: 7\n");
        // The decisive property: it loads back as the string it was given, and
        // `qq` did not become a node of its own.
        let reparsed = crate::eval_str(".s[1]", &out).unwrap();
        assert_eq!(reparsed, vec![Value::String("v\nqq: 7".into())]);
    }

    #[test]
    fn inserted_multiline_string_is_indented_for_its_insertion_site() {
        let out = apply(
            &assign(".m.b", Rhs::Literal(Value::String("v\nqq: 7".into()))),
            "keep: 0\nm:\n  a: 1\n",
        )
        .unwrap();
        assert_eq!(out, "keep: 0\nm:\n  a: 1\n  b: |-\n    v\n    qq: 7\n");
        let reparsed = crate::eval_str(".m.b", &out).unwrap();
        assert_eq!(reparsed, vec![Value::String("v\nqq: 7".into())]);
    }

    #[test]
    fn inserted_string_is_quoted_when_its_plain_spelling_would_change_type() {
        // `8080` plain would load as an integer; the typed tier quotes it.
        let out = apply(
            &assign(
                ".labels.version",
                Rhs::Literal(Value::String("8080".into())),
            ),
            "labels:\n  app: yqr\n",
        )
        .unwrap();
        assert_eq!(out, "labels:\n  app: yqr\n  version: \"8080\"\n");
        let reparsed = crate::eval_str(".labels.version", &out).unwrap();
        assert_eq!(reparsed, vec![Value::String("8080".into())]);
    }

    // A CRLF document must stay CRLF. The mutators terminate an inserted line
    // with `\n` whatever the file uses, so before the restore in `emit` these
    // produced mixed endings at exit 0 — with `-i`, written straight to disk.

    #[test]
    fn inserting_a_key_keeps_a_crlf_document_crlf() {
        let out = apply(
            &assign(".m.b", Rhs::Literal(Value::Int(2))),
            "m:\r\n  a: 1\r\n",
        )
        .unwrap();
        assert_eq!(out, "m:\r\n  a: 1\r\n  b: 2\r\n");
    }

    #[test]
    fn appending_an_item_keeps_a_crlf_document_crlf() {
        let out = apply(
            &Mutation::Append {
                path: crate::parser::parse(".s").expect("valid"),
                rhs: Rhs::Literal(Value::Int(3)),
            },
            "s:\r\n  - 1\r\n",
        )
        .unwrap();
        assert_eq!(out, "s:\r\n  - 1\r\n  - 3\r\n");
    }

    #[test]
    fn a_multiline_insert_into_a_crlf_document_uses_crlf_throughout() {
        let out = apply(
            &assign(".m.b", Rhs::Literal(Value::String("x\ny".into()))),
            "m:\r\n  a: 1\r\n",
        )
        .unwrap();
        assert_eq!(out, "m:\r\n  a: 1\r\n  b: |-\r\n    x\r\n    y\r\n");
        assert_eq!(
            crate::eval_str(".m.b", &out).unwrap(),
            vec![Value::String("x\ny".into())]
        );
    }

    #[test]
    fn an_lf_document_is_untouched_by_the_crlf_restore() {
        let out = apply(&assign(".m.b", Rhs::Literal(Value::Int(2))), "m:\n  a: 1\n").unwrap();
        assert_eq!(out, "m:\n  a: 1\n  b: 2\n");
    }

    #[test]
    fn a_mixed_ending_document_is_left_alone() {
        // No convention to restore; inventing one would be its own unasked-for
        // rewrite, so only the inserted line's own ending is in play.
        let out = apply(
            &assign(".m.b", Rhs::Literal(Value::Int(2))),
            "m:\r\n  a: 1\n",
        )
        .unwrap();
        assert_eq!(out, "m:\r\n  a: 1\n  b: 2\n");
    }

    // `set_value` carries no load-back oracle (see the module doc), so the
    // spellings it gets wrong surface as wrong values rather than refusals.
    // These two pin the cases that did: a string ending in `:` was rejected as
    // invalid, and a lone newline was written as an empty block scalar that
    // read back as "|". Both are engine fixes, which is exactly why yqr needs
    // its own assertion — nothing else here would catch their return.

    #[test]
    fn assigned_string_ending_in_a_colon_round_trips() {
        let out = apply(
            &assign(".k", Rhs::Literal(Value::String("a:".into()))),
            "k: 1\n",
        )
        .unwrap();
        assert_eq!(
            crate::eval_str(".k", &out).unwrap(),
            vec![Value::String("a:".into())]
        );
    }

    #[test]
    fn assigned_lone_newline_round_trips() {
        let out = apply(
            &assign(".k", Rhs::Literal(Value::String("\n".into()))),
            "k: 1\n",
        )
        .unwrap();
        assert_eq!(
            crate::eval_str(".k", &out).unwrap(),
            vec![Value::String("\n".into())]
        );
    }

    #[test]
    fn collection_rhs_is_refused_the_same_way_on_an_existing_key() {
        // The scalar-only limit is yqr's, so all three write paths must report
        // it in yqr's words. Left to the engine, this one named `set` and
        // "fragment" — APIs yqr does not expose — and called it a parse error.
        let err = apply(
            &assign(
                ".a",
                Rhs::Path(crate::parser::parse(".src").expect("valid")),
            ),
            "a: 1\nsrc:\n  k: 1\n",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("must be a scalar"),
            "engine wording leaked: {err}"
        );
    }

    #[test]
    fn delete_removes_a_single_line_entry() {
        let out = apply(
            &Mutation::Delete {
                path: crate::parser::parse(".metadata.labels").expect("valid"),
            },
            "metadata:\n  name: app\n  labels: prod\n",
        )
        .unwrap();
        assert_eq!(out, "metadata:\n  name: app\n");
    }

    #[test]
    fn path_rhs_copies_another_value() {
        let out = apply(
            &assign(
                ".dst",
                Rhs::Path(crate::parser::parse(".src").expect("valid")),
            ),
            "src: 42\ndst: 0\n",
        )
        .unwrap();
        assert_eq!(out, "src: 42\ndst: 42\n");
    }

    #[test]
    fn idempotent_assignment_is_a_byte_level_no_op() {
        let input = "a: 1\n";
        let out = apply(&assign(".a", Rhs::Literal(Value::Int(1))), input).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn missing_target_is_a_noop() {
        // `.a.b` cannot resolve (a is absent) and cannot be created, so the
        // input is returned unchanged (jq/yq no-op semantics), not an error.
        let input = "z: 1\n";
        let out = apply(&assign(".a.b", Rhs::Literal(Value::Int(1))), input).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn delete_of_absent_path_is_a_noop() {
        // `del(.deprecated)` over a document that lacks the key succeeds and
        // leaves the input unchanged, so a batch cleanup does not fail files
        // that never had the field.
        let input = "kept: 1\n";
        let out = apply(
            &Mutation::Delete {
                path: crate::parser::parse(".deprecated").expect("valid"),
            },
            input,
        )
        .unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn non_scalar_rhs_is_rejected() {
        // A collection RHS is refused by scope, not by capability — the typed
        // tier could spell one. What this pins is that the refusal is yqr's
        // own message rather than an engine error naming engine APIs.
        let err = apply(
            &Mutation::Append {
                path: crate::parser::parse(".list").expect("valid"),
                rhs: Rhs::Path(crate::parser::parse(".src").expect("valid")),
            },
            "list:\n  - 1\nsrc:\n  a: 1\n",
        )
        .unwrap_err();
        assert!(matches!(err, YqrError::Eval(ref m) if m.contains("must be a scalar")));
    }

    #[test]
    fn multi_document_edits_only_the_matching_document() {
        // A nested edit applies to the document whose parent path exists; the
        // document without `.spec` is left byte-identical (the realistic
        // multi-manifest case: a Deployment has `.spec.replicas`, a Service
        // does not).
        let out = apply(
            &assign(".spec.replicas", Rhs::Literal(Value::Int(9))),
            "spec:\n  replicas: 1\n---\nkind: Service\n",
        )
        .unwrap();
        assert_eq!(out, "spec:\n  replicas: 9\n---\nkind: Service\n");
    }

    #[test]
    fn path_rhs_absent_in_skipped_document_does_not_error() {
        // The second document lacks both `.spec` (target skipped) and `.src`
        // (the RHS source). Because the target does not resolve there, the RHS
        // must not be evaluated — an absent path RHS in a skipped document must
        // not turn a skip into a hard error.
        let out = apply(
            &assign(
                ".spec.replicas",
                Rhs::Path(crate::parser::parse(".src").expect("valid")),
            ),
            "spec:\n  replicas: 1\nsrc: 7\n---\nkind: Service\n",
        )
        .unwrap();
        assert_eq!(out, "spec:\n  replicas: 7\nsrc: 7\n---\nkind: Service\n");
    }

    #[test]
    fn new_key_assignment_fans_out_like_jq() {
        // A top-level new key is created in every document (each document is
        // filtered independently, matching jq/yq). Nested edits do not fan out
        // because the absent parent gates them (see the test above).
        let out = apply(
            &assign(".added", Rhs::Literal(Value::Int(1))),
            "a: 1\n---\nb: 2\n",
        )
        .unwrap();
        assert_eq!(out, "a: 1\nadded: 1\n---\nb: 2\nadded: 1\n");
    }

    #[test]
    fn multi_line_delete_removes_the_nested_entry() {
        // A nested/multi-line entry is deleted by the structural fallback,
        // closing up its owned lines and leaving the sibling byte-identical.
        let out = apply(
            &Mutation::Delete {
                path: crate::parser::parse(".outer").expect("valid"),
            },
            "outer:\n  inner: 1\nother: 2\n",
        )
        .unwrap();
        assert_eq!(out, "other: 2\n");
    }

    #[test]
    fn sole_entry_delete_is_refused() {
        // Removing the only entry of a block would leave an empty collection
        // (a structural change); it is refused, not silently emptied.
        let err = apply(
            &Mutation::Delete {
                path: crate::parser::parse(".only").expect("valid"),
            },
            "only:\n  a: 1\n  b: 2\n",
        )
        .unwrap_err();
        assert!(matches!(err, YqrError::Eval(ref m) if m.contains("only entry")));
    }

    #[test]
    fn unaddressable_key_is_reported() {
        // A dotted key cannot be expressed in the string-path grammar.
        let err = apply(
            &Mutation::Assign {
                path: crate::parser::parse(r#".["a.b"]"#).expect("valid"),
                rhs: Rhs::Literal(Value::Int(1)),
            },
            "'a.b': 1\n",
        )
        .unwrap_err();
        assert!(matches!(err, YqrError::Eval(ref m) if m.contains("cannot address")));
    }
}
