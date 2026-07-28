//! The fidelity write tier: surgical, source-preserving edits.
//!
//! Where the read seam ([`super::FidelityEngine`]) slices original bytes to
//! *emit* an untouched node, this module *mutates* the source in place and
//! leaves every other byte identical — or refuses. It is the write-side
//! analogue of the read seam: a small `FidelityWriter` trait bounds the
//! engine's edit surface, and the concrete `NoyalibWriter` routes each edit
//! through noyalib's first-class, re-parse-guarded mutators
//! (`set_value` / `insert_entry` / `push_back` / `remove`).
//!
//! The write path is the read path with the terminal call swapped: the
//! evaluator resolves a filter to a concrete [`Path`], the same
//! `to_noyalib_path` builder lowers it to a
//! string path, and a mutator addressed by that string applies the edit. Each
//! mutator returns `Result<()>`; that `Result` *is* the structural-integrity
//! guard — an edit whose result would re-parse differently is refused (exit 5)
//! and the document is left unchanged.

// Feature f006 (see specs/features/): write tier v1 — value assignment.

use crate::Value;
use crate::ast::Mutation;
use crate::error::{Result, YqrError};
use crate::eval::{AssignTarget, resolve_assign_target, resolve_rhs, resolve_target};
use crate::fidelity::{Path, PathSeg};

// The structural-delete fallback (multi-line / nested block entries) lives in a
// sub-module so the byte-arithmetic concern stays separate from the value-write
// trait. It extends `NoyalibWriter` with `delete_structural`, addressing the
// same private state through Rust's ancestor-module privacy.
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
    /// # Errors
    ///
    /// Errors when the path is unaddressable, does not resolve to a scalar, or
    /// the edit would re-parse differently.
    fn set_value(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()>;

    /// Insert a new `key: value` entry into the mapping at `parent`.
    ///
    /// # Errors
    ///
    /// Errors when the parent is unaddressable, is not a non-empty block
    /// mapping, or the edit would re-parse differently.
    fn insert_key(&mut self, doc: usize, parent: &Path, key: &str, value: &Value) -> Result<()>;

    /// Append `value` as a new item to the block sequence at `path`.
    ///
    /// # Errors
    ///
    /// Errors when the path is unaddressable, is not a non-empty block
    /// sequence, or the edit would re-parse differently.
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
        Ok(Self { docs })
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
        let ny = ::noyalib::Value::from(value);
        self.doc_mut(doc)?
            .set_value(&path_str, &ny)
            .map_err(|e| YqrError::eval(format!("cannot assign at {path_str:?}: {e}")))
    }

    fn insert_key(&mut self, doc: usize, parent: &Path, key: &str, value: &Value) -> Result<()> {
        // The new key itself must be plain — the string-path splice cannot
        // express an escaped key, the same honest gap the read path declares.
        if !PathSeg::key_is_plain(key) {
            return Err(YqrError::eval(format!(
                "cannot create key {key:?}: it uses characters the write path cannot express"
            )));
        }
        let parent_str = noyalib_path(parent)?;
        let fragment = value_fragment(value)?;
        self.doc_mut(doc)?
            .insert_entry(&parent_str, key, &fragment)
            .map_err(|e| YqrError::eval(format!("cannot insert key {key:?}: {e}")))
    }

    fn append(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()> {
        let path_str = noyalib_path(path)?;
        let fragment = value_fragment(value)?;
        self.doc_mut(doc)?
            .push_back(&path_str, &fragment)
            .map_err(|e| YqrError::eval(format!("cannot append to {path_str:?}: {e}")))
    }

    fn delete(&mut self, doc: usize, path: &Path) -> Result<()> {
        let path_str = noyalib_path(path)?;
        // noyalib 0.0.14's first-class `remove` handles only single-line block
        // entries; a multi-line / nested value makes it refuse (b004 2.4). On
        // refusal, fall through to the interim `replace_span` fallback, which
        // owns the byte arithmetic behind the same structural-integrity guard.
        // The refusal reason is carried into the fallback so a genuine noyalib
        // failure is surfaced rather than masked by a generic message.
        match self.doc_mut(doc)?.remove(&path_str) {
            Ok(()) => Ok(()),
            Err(remove_err) => self.delete_structural(doc, path, &remove_err.to_string()),
        }
    }

    fn emit(&self) -> String {
        self.docs.iter().map(ToString::to_string).collect()
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

/// Render a scalar [`Value`] to a single-line YAML fragment for the mutators
/// that take a raw string (`push_back` / `insert_entry`).
///
/// Routing through the same `Value -> noyalib` emission the classic pipeline
/// uses keeps quoting correct — a string needing quotes is quoted — so a
/// fragment is never a raw, unescaped user string (see the fidelity spec's
/// fragment-quoting note). A collection value has no single-line scalar form;
/// splicing its multi-line rendering would silently mis-shape the document, so
/// it is refused up front (v1 is scalar-only for `+=` / new-key inserts).
fn value_fragment(value: &Value) -> Result<String> {
    if matches!(value, Value::Sequence(_) | Value::Mapping(_)) {
        return Err(YqrError::eval(
            "the right-hand side of '+=' or a new-key assignment must be a scalar \
             (number, string, boolean, or null); collections are not yet supported"
                .to_string(),
        ));
    }
    let rendered = crate::render(std::slice::from_ref(value), false)?;
    Ok(rendered.trim_end().to_string())
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
        // Appending a collection has no single-line fragment form; it must be
        // refused with a clear message rather than splicing mis-shaped YAML.
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
