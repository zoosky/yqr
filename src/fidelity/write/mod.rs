//! The fidelity write tier: surgical, source-preserving edits.
//!
//! Where the read seam ([`super::FidelityEngine`]) slices original bytes to
//! *emit* an untouched node, this module *mutates* the source in place and
//! leaves every other byte identical — or refuses. It is the write-side
//! analogue of the read seam: a small [`FidelityWriter`] trait bounds the
//! engine's edit surface, and the concrete [`NoyalibWriter`] routes each edit
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
//! `to_noyalib_path` builder lowers it to a string path, and a mutator
//! addressed by that string applies the edit. Each mutator returns
//! `Result<()>`; that `Result` carries the structural-integrity guard — a
//! refused edit is reported (exit 5) and the document is left unchanged — to
//! the strength described above, which differs per mutator.
//!
//! ## Where things live
//!
//! The module is split by *what a piece decides*, not by which type owns it
//! (`yqr-f033`). [`apply`] is the only item anything outside reaches.
//!
//! - [`seam`] — the [`FidelityWriter`] trait and the two enums it speaks in.
//! - [`guards`] — everything deciding whether a write happens at all: the
//!   no-op guards, the type-change refusals, and the post-write integrity
//!   comparison. Three of these need the document and so are `NoyalibWriter`
//!   methods; they sit here by subject, because splitting the two
//!   type-change guards across files is what made them hard to follow.
//! - [`backend`] — `NoyalibWriter` and its implementation of the trait.
//! - `anchor`, `delete`, `reorder` — one edit family each, extending
//!   `NoyalibWriter` the same way.

// Feature f006 (see specs/features/): write tier v1 — value assignment.
// Feature f033: split from one file into this directory module.

use crate::Value;
use crate::ast::{FOOT_COMMENT_REFUSAL, Mutation, Target};
use crate::error::{Result, YqrError};
use crate::eval::{
    AssignTarget, eval_single, resolve_assign_target, resolve_rhs, resolve_target,
    resolve_update_target,
};
use crate::fidelity::noyalib::to_noyalib_path;
use crate::fidelity::{Path, PathSeg};

// Structural delete lives in a sub-module so the byte-arithmetic concern stays
// separate from the value-write trait. It extends `NoyalibWriter` with
// `delete_entry`, addressing the same private state through Rust's
// ancestor-module privacy.
mod anchor;
mod delete;

// Sequence reorder is one engine call per verb plus the index arithmetic and
// refusals yqr owns around it; the same sibling-module split `delete` uses.
mod reorder;

mod backend;
mod guards;
mod seam;

pub(crate) use backend::NoyalibWriter;
pub(crate) use seam::{CommentKind, FidelityWriter};

use guards::{append_item, set_comment_unless_unchanged, set_value_unless_unchanged};

/// Apply a single [`Mutation`] to `input` and return the whole emitted stream.
///
/// The mutation is applied to every document whose target resolves; documents
/// where the path is absent are emitted byte-identically.
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
/// ambiguous, or an edit is refused by the re-parse guard.
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
        Mutation::Assign {
            target: Target::Value(path),
            rhs,
        } => {
            let Some(target) = resolve_assign_target(path, value)? else {
                return Ok(());
            };
            let rhs_value = resolve_rhs(rhs, value)?;
            match target {
                AssignTarget::Existing { path, current } => {
                    set_value_unless_unchanged(writer, doc, &path, &rhs_value, &current)
                }
                // Nothing to compare against: the key is not there yet.
                AssignTarget::NewKey { parent, key } => {
                    writer.insert_key(doc, &parent, &key, &rhs_value)
                }
            }
        }
        // A rename addresses an entry that must already exist: there is no
        // key to rename otherwise, and `resolve_assign_target`'s create-a-key
        // branch would be the wrong answer (`key(.absent) = "x"` must not
        // invent an entry). So this uses the plain resolver and skips the
        // document when the path is absent, exactly as `del` does.
        Mutation::Assign {
            target: Target::Key(path),
            rhs,
        } => {
            let Some(target) = resolve_target(path, value)? else {
                return Ok(());
            };
            let new_key = key_name(&resolve_rhs(rhs, value)?)?;
            writer.rename_key(doc, &target, &new_key)
        }
        // A comment is text, so the RHS must be a string. It reaches upstream
        // as the body without `#`; an empty body is a bare `#`, not a removal
        // (`yqr-a002` §4.2) — upstream already spells it that way.
        Mutation::Assign {
            target: target @ (Target::LineComment(path) | Target::HeadComment(path)),
            rhs,
        } => {
            let Some(resolved) = resolve_target(path, value)? else {
                return Ok(());
            };
            let text = comment_text(&resolve_rhs(rhs, value)?)?;
            set_comment_unless_unchanged(writer, doc, &resolved, comment_kind(target), &text)
        }
        Mutation::Delete {
            target: target @ (Target::LineComment(path) | Target::HeadComment(path)),
        } => {
            let Some(resolved) = resolve_target(path, value)? else {
                return Ok(());
            };
            writer.remove_comment(doc, &resolved, comment_kind(target))
        }
        // Refused when the target is built (`parse_target`), so reaching here
        // would mean the parser and this dispatch had drifted apart.
        Mutation::Assign {
            target: Target::FootComment(_),
            ..
        }
        | Mutation::Delete {
            target: Target::FootComment(_),
        } => Err(YqrError::eval(FOOT_COMMENT_REFUSAL.to_string())),
        // An ordering is not a node, so there is no target to build — the
        // path names the *sequence* whose items move, and the indices name
        // positions within it. A document where that path is absent is skipped
        // like every other mutation's.
        Mutation::Reorder { path, op, from, to } => {
            let Some(target) = resolve_target(path, value)? else {
                return Ok(());
            };
            writer.reorder(doc, &target, *op, *from, *to)
        }
        // `|=` differs from `=` in one word: the right-hand filter runs
        // against the **node**, not the document. Everything after that is
        // `=`'s path — the same guarded `set_value`, the same single-node
        // contract, the same skip when the path is absent in this document.
        // Feature f008.
        Mutation::Update { path, rhs } => {
            let Some((target, current)) = resolve_update_target(path, value)? else {
                return Ok(());
            };
            let updated = eval_single(rhs, &current, "the update filter")?;
            set_value_unless_unchanged(writer, doc, &target, &updated, &current)
        }
        Mutation::Append { path, rhs } => {
            // `resolve_update_target` rather than `resolve_target`: identical
            // contract, and it hands back the current value, which is what the
            // precondition check below needs (`yqr-b024`).
            let Some((target, current)) = resolve_update_target(path, value)? else {
                return Ok(());
            };
            let item = resolve_rhs(rhs, value)?;
            append_item(writer, doc, &target, &current, &item)
        }
        Mutation::Delete {
            target: Target::Value(path),
        } => match resolve_target(path, value)? {
            Some(target) => writer.delete(doc, &target),
            None => Ok(()),
        },
        // Refused at parse (`parse_del`), so reaching here would mean the
        // parser and this dispatch had drifted apart.
        Mutation::Delete {
            target: Target::Key(_),
        } => Err(YqrError::eval(
            "del(key(...)) is not an edit: a key cannot outlive its entry".to_string(),
        )),
    }
}

/// The [`CommentKind`] a comment target selects.
///
/// Total on the two comment variants; every other target is routed before
/// this is reached.
fn comment_kind(target: &Target) -> CommentKind {
    match target {
        Target::HeadComment(_) => CommentKind::Head,
        _ => CommentKind::Line,
    }
}

/// The body a `line_comment(...) = <rhs>` / `head_comment(...) = <rhs>` writes.
///
/// A comment is text. A number or boolean would have to be rendered to write
/// it, and yqr would then be choosing a spelling the user did not — refused
/// here, where the message can name what was given.
fn comment_text(value: &Value) -> Result<String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        other => Err(YqrError::eval(format!(
            "the right-hand side of a comment assignment must be a string, but found {}",
            type_name(other)
        ))),
    }
}

/// The new key a `key(<path>) = <rhs>` rename writes.
///
/// yqr's path model addresses mapping keys as strings (`PathSeg::Key`), so a
/// rename target has to be one. A number or boolean would produce an entry the
/// typed view could hold but no filter could name — refused here, where the
/// message can name what was given.
fn key_name(value: &Value) -> Result<String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        other => Err(YqrError::eval(format!(
            "the right-hand side of a key rename must be a string, but found {}",
            type_name(other)
        ))),
    }
}

/// The user-facing name of a value's type, for diagnostics.
pub(super) fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Int(_) | Value::Float(_) => "a number",
        Value::String(_) => "a string",
        Value::Sequence(_) => "a sequence",
        Value::Mapping(_) => "a mapping",
    }
}

/// `path` with its last segment replaced by `seg`, or dropped when `seg` is
/// `None` — the parent path and the sibling path, which [`Path`] has no
/// accessors for because nothing before the borrowed-site check needed to walk
/// *upwards*.
///
/// The root has no last segment, so both forms return it unchanged.
// Bug b019.
pub(super) fn rebased(path: &Path, seg: Option<PathSeg>) -> Path {
    let segments = path.segments();
    let keep = segments.len().saturating_sub(1);
    let mut out = Path::root();
    for s in &segments[..keep] {
        out = out.child(s.clone());
    }
    match seg {
        Some(s) => out.child(s),
        None => out,
    }
}

/// Lower a [`Value`] to the noyalib value the typed mutators take.
///
/// Passing a value rather than a rendered fragment is what lets the engine
/// place and spell it: quoting follows the edit site, indentation follows the
/// document, and a collection is written as a block or a flow member
/// according to where it lands. Hand-building the fragment instead put yqr on
/// the wrong side of the guard, which is why this tier exists at all
/// (`yqr-b008`).
///
/// Collections pass through. They were refused here until `yqr-f032` — a
/// scope limit rather than a backend one, since the typed tier has spelled a
/// nested collection since `yqr-b008`. What the sites still refuse is the one
/// shape the engine has no route for, a scalar replaced by a collection, and
/// that check lives with the current value it needs
/// ([`set_value_unless_unchanged`]).
// Feature f032.
pub(super) fn insertable(value: &Value) -> ::noyalib::Value {
    ::noyalib::Value::from(value)
}

/// Helpers shared by the test modules of this directory. One definition, so a
/// change to how a mutation is built cannot land differently in two files.
#[cfg(test)]
pub(super) mod testutil {
    use super::*;
    use crate::ast::Rhs;

    pub(super) fn assign(path: &str, rhs: Rhs) -> Mutation {
        Mutation::Assign {
            target: Target::Value(crate::parser::parse(path).expect("valid path")),
            rhs,
        }
    }

    /// Run `key(<path>) = <new>` over `input`.
    // Feature f007: key rename.
    pub(super) fn rename(path: &str, new_key: &str, input: &str) -> Result<String> {
        apply(
            &Mutation::Assign {
                target: Target::Key(crate::parser::parse(path).expect("valid path")),
                rhs: Rhs::Literal(Value::String(new_key.to_string())),
            },
            input,
        )
    }

    /// Run `line_comment(<path>) = <text>` / `head_comment(...)` over `input`.
    pub(super) fn set_comment_on(kind: Target, text: &str, input: &str) -> Result<String> {
        apply(
            &Mutation::Assign {
                target: kind,
                rhs: Rhs::Literal(Value::String(text.to_string())),
            },
            input,
        )
    }

    pub(super) fn line_of(path: &str) -> Target {
        Target::LineComment(crate::parser::parse(path).expect("valid path"))
    }
    pub(super) fn head_of(path: &str) -> Target {
        Target::HeadComment(crate::parser::parse(path).expect("valid path"))
    }
    pub(super) fn del_comment(target: Target, input: &str) -> Result<String> {
        apply(&Mutation::Delete { target }, input)
    }

    /// Run `<path> = <rhs path>` over `input`.
    pub(super) fn assign_path(path: &str, rhs: &str, input: &str) -> Result<String> {
        apply(
            &assign(
                path,
                Rhs::Path(crate::parser::parse(rhs).expect("valid rhs")),
            ),
            input,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::*;
    use super::*;
    use crate::ast::Rhs;

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

    // A CRLF document must stay CRLF. These five once pinned a yqr-side pass
    // over the emitted string, added because the mutators terminated an
    // inserted line with `\n` whatever the file used and so produced mixed
    // endings at exit 0 — with `-i`, written straight to disk. The engine owns
    // the terminator as of 0.0.22, so they now pin *its* behaviour, and they
    // are the only thing here that would catch its return: the property is
    // invisible to the corpus and fidelity harnesses, which never edit a CRLF
    // document.

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
    fn an_lf_document_stays_lf() {
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
    fn delete_removes_a_single_line_entry() {
        let out = apply(
            &Mutation::Delete {
                target: Target::Value(crate::parser::parse(".metadata.labels").expect("valid")),
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
                target: Target::Value(crate::parser::parse(".deprecated").expect("valid")),
            },
            input,
        )
        .unwrap();
        assert_eq!(out, input);
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
                target: Target::Value(crate::parser::parse(".outer").expect("valid")),
            },
            "outer:\n  inner: 1\nother: 2\n",
        )
        .unwrap();
        assert_eq!(out, "other: 2\n");
    }

    #[test]
    fn sole_entry_delete_empties_the_collection() {
        // Deleting the last entry of a block writes the collection out
        // explicitly: removing the bytes would leave `only:`, which re-parses
        // as null — a type change rather than a removal. Routed through the
        // full mutation surface here, not just `delete_entry`.
        let out = apply(
            &Mutation::Delete {
                target: Target::Value(crate::parser::parse(".only.a").expect("valid")),
            },
            "only:\n  a: 1\nother: 2\n",
        )
        .unwrap();
        assert_eq!(out, "only:\n  {}\nother: 2\n");
    }

    #[test]
    fn a_no_op_write_is_skipped_before_the_key_is_addressed() {
        // The `yqr-b018` guard runs before the writer, so an assignment that
        // changes nothing succeeds even at a key the write path cannot
        // express. That is deliberate: nothing needed writing, so no
        // limitation was reached. The sibling test below keeps the refusal
        // pinned for the case where something *does* need writing.
        let out = apply(
            &Mutation::Assign {
                target: Target::Value(crate::parser::parse(r#".["a.b"]"#).expect("valid")),
                rhs: Rhs::Literal(Value::Int(1)),
            },
            "'a.b': 1\n",
        )
        .expect("a write that changes nothing cannot fail");
        assert_eq!(out, "'a.b': 1\n", "and it must not touch the bytes");
    }

    #[test]
    fn a_dotted_key_is_assigned_in_place() {
        let out = apply(
            &Mutation::Assign {
                target: Target::Value(crate::parser::parse(r#".["a.b"]"#).expect("valid")),
                rhs: Rhs::Literal(Value::Int(2)),
            },
            "'a.b': 1  # kept\nc: 3\n",
        )
        .expect("writes");
        assert_eq!(out, "'a.b': 2  # kept\nc: 3\n");
    }

    #[test]
    fn a_dotted_key_is_created_beside_dotted_keys() {
        let out = apply(
            &Mutation::Assign {
                target: Target::Value(
                    crate::parser::parse(r#".labels["app.kubernetes.io/version"]"#).expect("valid"),
                ),
                rhs: Rhs::Literal(Value::String("1.4.2".into())),
            },
            "labels:\n  app.kubernetes.io/name: web\n",
        )
        .expect("writes");
        assert_eq!(
            out,
            "labels:\n  app.kubernetes.io/name: web\n  app.kubernetes.io/version: 1.4.2\n"
        );
    }

    #[test]
    fn a_rename_to_a_dotted_key_is_addressable_again() {
        let out = rename(
            ".labels.name",
            "app.kubernetes.io/name",
            "labels:\n  name: web\n",
        )
        .expect("renames");
        assert_eq!(out, "labels:\n  app.kubernetes.io/name: web\n");
        let again = apply(
            &Mutation::Assign {
                target: Target::Value(
                    crate::parser::parse(r#".labels."app.kubernetes.io/name""#).expect("valid"),
                ),
                rhs: Rhs::Literal(Value::String("api".into())),
            },
            &out,
        )
        .expect("the renamed entry is reachable");
        assert_eq!(again, "labels:\n  app.kubernetes.io/name: api\n");
    }

    #[test]
    fn a_rename_to_the_empty_key_is_addressable_again() {
        // The empty key used to be refused because no filter could name the
        // result. `.[""]` names it, so the addressable set stays closed under
        // rename without the refusal.
        let out = rename(".a", "", "a: 1\nb: 2\n").expect("renames");
        let again = apply(
            &Mutation::Assign {
                target: Target::Value(crate::parser::parse(r#".[""]"#).expect("valid")),
                rhs: Rhs::Literal(Value::Int(3)),
            },
            &out,
        )
        .expect("the renamed entry is reachable");
        assert_eq!(again.lines().count(), 2);
        assert!(again.ends_with("b: 2\n"), "{again:?}");
        assert_eq!(
            crate::eval_str(r#".[""]"#, &again).expect("evaluates"),
            vec![Value::Int(3)]
        );
    }
}
