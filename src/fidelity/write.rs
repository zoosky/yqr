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
use crate::ast::{FOOT_COMMENT_REFUSAL, Mutation, ReorderOp, Target};
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

/// Which comment attached to an entry a mutation addresses.
///
/// The two are separate operations upstream and separate selectors in the
/// filter grammar, but they share every pre-check, so the seam takes the kind
/// as a parameter rather than growing four methods.
// Feature f007.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommentKind {
    /// The `# ...` following the value on the entry's own line.
    Line,
    /// The run of comment lines immediately above the entry.
    Head,
}

impl CommentKind {
    /// The selector spelling, for diagnostics.
    fn word(self) -> &'static str {
        match self {
            CommentKind::Line => "line_comment",
            CommentKind::Head => "head_comment",
        }
    }
}

/// Why the node at a path is not the addressed entry's own.
///
/// Both arms mean the same thing to the no-op guard — do not skip — and
/// different things to a reader of the refusal, which is why they are kept
/// apart rather than collapsed to a bool at the source.
// Bugs b019, b020.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Borrowed {
    /// The **key** is not in the source. A `<<` merge or an alias expansion
    /// put it in the typed view, so the mapping has no entry to write at all.
    Key,
    /// The key is the mapping's own, but its **value** is an alias reference
    /// resolved through to the anchor. Writing there would splice the
    /// anchor's bytes, which name a different entry.
    Value,
}

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
    /// Errors when the path does not resolve to a scalar, the value is a
    /// collection, or the edit would re-parse differently.
    fn set_value(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()>;

    /// Whether the value at `path` lives elsewhere in the document rather than
    /// at the entry the path names: an alias reference resolved through to its
    /// anchor, or an entry a `<<` merge produced.
    ///
    /// The no-op guard consults this before it skips a write. A borrowed value
    /// compares equal to the literal it points at, so value-equality alone
    /// would call an edit a no-op when writing it would replace a reference
    /// with a literal — a real change, and one the writer refuses rather than
    /// performs.
    ///
    /// `false` means *not established*, not *proven own*: the caller's fallback
    /// is to attempt the write, where the writer's own refusals apply. That
    /// direction is the safe one, and it is why this reports a bool rather than
    /// an error — the diagnostic stays the writer's.
    ///
    /// # Errors
    ///
    /// Returns an error when `doc` is out of range.
    // Bug b019.
    fn value_is_borrowed(&self, doc: usize, path: &Path) -> Result<bool>;

    /// Insert a new `key: value` entry into the mapping at `parent`.
    ///
    /// The implementation places and spells `value`; callers pass a value, not
    /// a rendered fragment, so quoting and indentation are never theirs.
    ///
    /// # Errors
    ///
    /// Errors when the parent is not a non-empty block mapping. Also errors
    /// when the result would not **load back** as the pre-edit value with
    /// exactly this insertion applied — a stronger contract than "still
    /// parses", and the one an alternative implementation must meet.
    fn insert_key(&mut self, doc: usize, parent: &Path, key: &str, value: &Value) -> Result<()>;

    /// Append `value` as a new item to the block sequence at `path`.
    ///
    /// # Errors
    ///
    /// Errors when the path is not a non-empty block sequence, or the result
    /// would not load back as the pre-edit value with exactly this insertion
    /// applied (see [`Self::insert_key`]).
    fn append(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()>;

    /// Remove the block entry at `path`, whether single-line, multi-line, or a
    /// nested collection. The entry's own lines (its key/`-`, continuation, and
    /// any head comment documenting it) go; every surviving node stays
    /// byte-identical.
    ///
    /// Removing the last entry of a collection writes that collection out
    /// explicitly (`{}` / `[]`) rather than leaving a dangling key, which would
    /// re-parse as `null` — a type change rather than a removal.
    ///
    /// # Errors
    ///
    /// Errors when the edit would re-parse to a different document.
    fn delete(&mut self, doc: usize, path: &Path) -> Result<()>;

    /// Set (or replace) a comment attached to the entry at `path`.
    ///
    /// `kind` selects the inline comment on the entry's own line or the block
    /// of comment lines above it. `text` is the body without `#`; an empty
    /// body writes a bare `#` rather than removing anything (removal is
    /// [`remove_comment`](Self::remove_comment)).
    ///
    /// # Errors
    ///
    /// Errors when the entry cannot carry that kind of comment, or the block
    /// above it is not the entry's to rewrite.
    fn set_comment(&mut self, doc: usize, path: &Path, kind: CommentKind, text: &str)
    -> Result<()>;

    /// The body [`set_comment`](Self::set_comment) would be replacing at
    /// `path`, or `None` when there is none — or when the site is one this
    /// writer would refuse, so a caller cannot mistake a refusal for a match.
    ///
    /// The body is spelled as the read path reports it (no `#`, one leading
    /// space dropped), which is what makes reading a comment and writing it
    /// straight back a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error when `doc` is out of range.
    // Bug b019.
    fn current_comment(&self, doc: usize, path: &Path, kind: CommentKind)
    -> Result<Option<String>>;

    /// Remove a comment attached to the entry at `path`.
    ///
    /// Every refusal here is yqr's own. Upstream's two removers return
    /// `Ok(())` on an unresolved path, on a missing comment, and on every
    /// shape their setters reject, so delegating unchecked would turn a
    /// refusal into a silent no-op — which a mutation is never allowed to be.
    ///
    /// # Errors
    ///
    /// Errors wherever [`set_comment`](Self::set_comment) does.
    fn remove_comment(&mut self, doc: usize, path: &Path, kind: CommentKind) -> Result<()>;

    /// Rename the key of the mapping entry at `path`, leaving its value, its
    /// comments, and every other byte in the document identical.
    ///
    /// Only the key *token* is rewritten, so the entry keeps its position in
    /// the mapping — a rename is not a delete plus an insert.
    ///
    /// # Errors
    ///
    /// Errors when the path names a sequence item, when the rename would
    /// collide with an existing sibling, and when the entry has no key token
    /// of its own (a `<<` merge or an alias site).
    fn rename_key(&mut self, doc: usize, path: &Path, new_key: &str) -> Result<()>;

    /// Reorder the items of the block sequence at `path`.
    ///
    /// `from` and `to` are yqr indices, not the engine's: negative counts from
    /// the end, exactly as `.[-1]` does, and the implementation resolves them
    /// against the sequence before addressing anything. Whole entries move —
    /// each item's own comment lines travel with it — so a reorder never
    /// re-attributes the file's documentation to whatever landed in the slot.
    ///
    /// A **flow** sequence has no per-item lines, so its members exchange
    /// value spans instead; it is reordered rather than refused.
    ///
    /// # Errors
    ///
    /// Errors when the path does not name a sequence, or either index falls
    /// outside it, and when the engine refuses the splice
    /// or its result fails the integrity check (which leaves the document
    /// unchanged).
    fn reorder(&mut self, doc: usize, path: &Path, op: ReorderOp, from: i64, to: i64)
    -> Result<()>;

    /// Emit the whole document stream: edited documents reflect their edits,
    /// every other document is byte-identical to the input.
    fn emit(&self) -> String;
}

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

/// Append `item` to the sequence at `target`, refusing in yqr's own words when
/// it is not one.
///
/// **The engine's refusals here are not fit to show a user.** Every one of them
/// arrives as a *parse* error over a document that parsed fine, which sends the
/// reader to look at their file instead of their filter, and two of the three
/// name an internal: `swap_items` for a target that is not a sequence (a
/// primitive from the *reorder* path, shown to someone who asked to append) and
/// `push_back_value` for an empty one, whose advice is to *"use `set` with a
/// fragment"* — an API yqr does not expose. `set_value` already carries a guard
/// for exactly this reason; the append path never got one.
///
/// So yqr checks its own preconditions against the typed model first: the
/// target must be a sequence, and it must have an item to anchor indentation
/// on. What is left that can still fail is genuinely the engine's business —
/// flow layout — and its wording there is accurate and names nothing internal.
///
/// Each refusal names a remedy that **works**, which is `yqr-f025`'s rule, and
/// the remedy differs by what is actually there: `|=` computes a new value in
/// place of a scalar, and a mapping grows by assignment, not by append.
// Bug b024.
fn append_item(
    writer: &mut dyn FidelityWriter,
    doc: usize,
    target: &Path,
    current: &Value,
    item: &Value,
) -> Result<()> {
    let path_str = to_noyalib_path(target);
    let remedy = |v: &Value| match v {
        // Only the numeric arm carries an arithmetic example, because only
        // there does one work: `(. + 1)` over a string is a type error, and a
        // remedy that errors is worse than no remedy (`yqr-f025`).
        Value::Int(_) | Value::Float(_) => {
            format!("Use `|=` to compute a new value in place, as in `.{path_str} |= (. + 1)`")
        }
        // An implicit null has no value to compute from, so `|=` is not the
        // way in; `=` writes one (`yqr-b021`).
        Value::Null => format!("Use `=` to write a value there, as in `.{path_str} = 1`"),
        _ => "Use `|=` to compute a new value in place".to_string(),
    };
    match current {
        Value::Sequence(items) if !items.is_empty() => writer.append(doc, target, item),
        Value::Sequence(_) => Err(YqrError::eval(format!(
            "cannot append at {path_str:?}: the sequence is empty, and yqr places a new item by \
             following the indentation of the one above it. Write the first item into the file \
             yourself, and `+=` will extend it"
        ))),
        Value::Mapping(_) => Err(YqrError::eval(format!(
            "cannot append at {path_str:?}: `+=` appends an item to a sequence, and this is a \
             mapping. Assign the entry you want instead, as in `.{path_str}.<key> = <value>`"
        ))),
        scalar => Err(YqrError::eval(format!(
            "cannot append at {path_str:?}: `+=` appends an item to a sequence, and this is \
             {}. {}",
            type_name(scalar),
            remedy(scalar)
        ))),
    }
}

/// Write `new` at `path`, unless it is already what is there.
///
/// **A write that changes nothing must not rewrite anything.** `set_value`
/// re-emits a scalar from the typed model, and the model cannot carry a
/// number's spelling — so writing `0640` back as the same `Int` emits `640`,
/// and a `1.10` version pin comes back `1.1`. On an edit that changed nothing,
/// that is `yqr-a001` §1's own counter-example: *yqr never rewrites bytes it
/// did not change*.
///
/// The comparison is the typed model's, and that is **why** it works rather
/// than a limitation to work around: the model cannot tell `0640` from `640`,
/// so equal-by-value is exactly the set of cases where re-emitting would lose
/// a spelling. Where the model *can* tell two values apart, they are not
/// equal and the write proceeds.
///
/// A skip is a **success**. `.n = .n` is a no-op, not a refusal, matching the
/// absent-path rule.
///
/// That premise fails where the value is **borrowed** — an alias reference, or
/// an entry a `<<` merge produced. There the model compares equal to a literal
/// the entry only points at, so skipping would swallow a write that had real
/// work to do: `.b = 1` over `b: *x` leaves a reference behind, and a later
/// edit to the anchor moves `b` with it. So the borrowed check runs *first*
/// and a borrowed site falls through to the writer, which refuses it in its
/// own words rather than in a copy of them kept here.
///
/// Shared by `=` and `|=`, which reach it by different resolvers and would
/// otherwise carry a copy of this rule each — the shape that let the `=` half
/// go unguarded while `|=` was fixed (`yqr-b018`).
// Feature f006 / f008; bugs b018, b019.
fn set_value_unless_unchanged(
    writer: &mut dyn FidelityWriter,
    doc: usize,
    path: &Path,
    new: &Value,
    current: &Value,
) -> Result<()> {
    if new == current && !writer.value_is_borrowed(doc, path)? {
        return Ok(());
    }
    refuse_scalar_to_collection(path, new, current)?;
    writer.set_value(doc, path, new)
}

/// Refuse replacing a scalar with a collection, in yqr's words.
///
/// The engine writes a collection over a collection and a scalar over a
/// scalar; it has no typed route from one to the other, and says so by naming
/// `set` and "fragment", an API yqr does not expose.
///
/// Checked here rather than in the backend because this is the one caller
/// holding both the current value and the new one, and both are needed to
/// name the shape that is wrong.
///
/// The remedy differs by shape, and each is one this refusal was measured
/// running — `yqr-f025`'s rule, which a single remedy sentence broke in three
/// ways. Removing the entry and assigning it again works for a mapping entry
/// with a value, because a *new key* is the insertion path and that one spells
/// a collection. It does **not** work for a sequence item (`del` shifts the
/// items up, so the same path then names the next one and the refusal
/// repeats), and it does not work for an entry whose value is absent
/// altogether (`del` on an implicit null is itself refused). A sequence takes
/// a collection through `+=`; an implicit null has no route at all, so this
/// names none rather than one that fails.
// Feature f032; the per-shape remedies are the code review of that feature.
fn refuse_scalar_to_collection(path: &Path, new: &Value, current: &Value) -> Result<()> {
    let new_is_collection = matches!(new, Value::Sequence(_) | Value::Mapping(_));
    let current_is_collection = matches!(current, Value::Sequence(_) | Value::Mapping(_));
    if !new_is_collection || current_is_collection {
        return Ok(());
    }
    let path_str = to_noyalib_path(path);
    let remedy = match path.segments().last() {
        Some(PathSeg::Index(_)) => {
            let seq = to_noyalib_path(&rebased(path, None));
            format!(
                "A sequence takes a collection through `+=`, which appends it: `.{seq} += <path>`"
            )
        }
        // An entry left empty has no bytes, so `del` refuses it too; one
        // written `k: null` does not. The sentence has to hold for both,
        // because the typed value cannot tell them apart.
        _ if matches!(current, Value::Null) => format!(
            "A null takes no collection in place. If the entry is written `{}: null` rather \
             than left empty, `del(.{path_str})` removes it and assigning again writes the \
             collection at the end of its mapping",
            path.segments()
                .last()
                .and_then(|seg| match seg {
                    PathSeg::Key(k) => Some(k.as_str()),
                    PathSeg::Index(_) => None,
                })
                .unwrap_or("key")
        ),
        _ => format!(
            "Remove the entry and write it again, as in `del(.{path_str})` then \
             `.{path_str} = <path>`, which places it at the end of its mapping"
        ),
    };
    Err(YqrError::eval(format!(
        "cannot assign at {path_str:?}: the value there is {}, and yqr writes a collection \
         only where one already is. {remedy}",
        type_name(current)
    )))
}

/// Write the `kind` comment at `path`, unless it already says that.
///
/// The value guard's rule, on the other thing a write can re-spell. `#tight`
/// and `# tight` carry the same body, and the body is all a comment mutation
/// is given, so `set_comment` re-emits the canonical spacing and a write that
/// changed no content rewrites the line — `yqr-a001` §1 again, on a comment
/// instead of a scalar.
///
/// The comparison is the read path's spelling of the body, which is what makes
/// reading a comment and writing it straight back a no-op.
// Feature f007; bug b019.
fn set_comment_unless_unchanged(
    writer: &mut dyn FidelityWriter,
    doc: usize,
    path: &Path,
    kind: CommentKind,
    text: &str,
) -> Result<()> {
    if writer.current_comment(doc, path, kind)?.as_deref() == Some(text) {
        return Ok(());
    }
    writer.set_comment(doc, path, kind, text)
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
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Int(_) | Value::Float(_) => "a number",
        Value::String(_) => "a string",
        Value::Sequence(_) => "a sequence",
        Value::Mapping(_) => "a mapping",
    }
}

/// [`FidelityWriter`] backed by noyalib's editable `cst::Document` stream.
/// The structural facts [`NoyalibWriter::check_integrity`] compares on either
/// side of a write. Counts rather than positions: a write is refused when it
/// *adds* a violation, so a document that already had one stays editable.
// Bugs b029, b030.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Integrity {
    /// Block mapping values sitting at or before their key's column (`Y103`).
    under_indented: usize,
    /// `\r\n` pairs.
    crlf: usize,
    /// Line feeds with no carriage return before them.
    bare_lf: usize,
}

/// `bytes` with any leading `&anchor` and `!tag` properties removed.
///
/// A node's properties precede its value, so a flow collection carrying one
/// does not start with `[` or `{` — and reading it as a block collection is
/// how the `yqr-b029` guard came to refuse `.k = 5` over `k: &an {a: 1}`, an
/// edit that works and that `write::anchor` has its own accurate message for.
/// Both spellings are skipped, in either order, since a node may carry both.
// Bug b029, found in code review.
fn after_properties(bytes: &str) -> &str {
    let mut rest = bytes.trim_start();
    while rest.starts_with(['&', '!']) {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        rest = rest[end..].trim_start();
    }
    rest
}

/// Line feeds in `source` that no carriage return precedes.
// Bug b030.
fn bare_line_feeds(source: &str) -> usize {
    let bytes = source.as_bytes();
    bytes
        .iter()
        .enumerate()
        .filter(|(i, b)| **b == b'\n' && (*i == 0 || bytes[i - 1] != b'\r'))
        .count()
}

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
        let docs = super::noyalib::parse_lossless_stream(input)?;
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

    /// Whether the entry at `path_str` currently carries a comment of `kind`
    /// that yqr considers its own.
    ///
    /// Used twice by a removal: once to refuse "there is nothing here", and
    /// once afterwards to refuse an upstream `Ok` that removed nothing.
    // Feature f007.
    fn comment_present(&self, doc: usize, path_str: &str, kind: CommentKind) -> Result<bool> {
        let d = self.doc_ref(doc)?;
        Ok(match kind {
            CommentKind::Line => d.comments_at(path_str).inline.is_some(),
            CommentKind::Head => super::noyalib::attached_head_len(d, path_str) > 0,
        })
    }

    /// The pre-checks a comment mutation runs before it reaches upstream.
    ///
    /// These exist because upstream's guards answer a *different* question
    /// from the one the filter asked, in two measured ways. Both are silent
    /// wrong results rather than refusals, so nothing downstream would catch
    /// them.
    // Feature f007; the two cases are catalogued in yqr-a002 §4.1.
    fn check_comment_site(&self, doc: usize, path_str: &str, kind: CommentKind) -> Result<()> {
        let d = self.doc_ref(doc)?;
        if d.span_at(path_str).is_none() {
            return Err(YqrError::eval(format!(
                "cannot address {}({path_str}): the path does not resolve to a node",
                kind.word()
            )));
        }
        match kind {
            // An entry whose value begins on the next line has no line of its
            // own to comment. Upstream's guard looks at the value span, which
            // is single-line here, so it writes the comment onto the child's
            // line instead — and removal deletes the child's comment.
            CommentKind::Line => {
                if !super::noyalib::value_starts_on_key_line(d, path_str) {
                    return Err(YqrError::eval(format!(
                        "cannot address line_comment({path_str}): its value starts on the \
                         next line, so the entry has no line of its own to comment; \
                         comment one of its entries instead"
                    )));
                }
            }
            // Upstream's leading mutators rewrite whatever `comments_at`
            // reports above the entry, and that walk crosses blank lines. yqr
            // owns only the contiguous run, so a mismatch means the edit would
            // reach a comment that documents whatever came before.
            CommentKind::Head => {
                let owned = super::noyalib::attached_head_len(d, path_str);
                let upstream = d.comments_at(path_str).before.len();
                if owned != upstream {
                    // Deliberately generic. A blank line is the common cause
                    // and the one §4.1.1 describes, but not the only one — a
                    // differently-indented comment and an alias-valued entry
                    // both land here too, and naming a cause the check has not
                    // established would be a confident wrong answer.
                    return Err(YqrError::eval(format!(
                        "cannot address head_comment({path_str}): the comment block the YAML \
                         engine would rewrite is larger than the run directly above this \
                         entry — typically because a blank line separates part of it, so it \
                         documents what precedes the entry rather than the entry itself. \
                         Editing it here would rewrite bytes the path does not name"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Establish whether the node at `path` is the addressed entry's own, and
    /// if not, which way it is borrowed.
    ///
    /// Two source-level facts settle it, and neither needs to reason about
    /// where the anchor is. Upstream decides the same question in
    /// `Document::write_span`, but that is private, so this establishes it
    /// from the public span API instead of guessing — and answers `None`
    /// wherever it cannot, which leaves the verdict to the write itself.
    // Bugs b019, b020.
    fn borrowed_site(&self, doc: usize, path: &Path) -> Result<Option<Borrowed>> {
        let d = self.doc_ref(doc)?;
        let path_str = to_noyalib_path(path);
        // A key the source does not contain is not the mapping's own — a `<<`
        // merge or an alias expansion produced it. Restricting this to paths
        // ending in a key is what keeps a sequence item and the root out: both
        // legitimately have no key. An implicit null keeps its key and so
        // stays out too, which matters — `a:` has no value span either, and
        // refusing `.a = null` there would be the very re-spell the no-op
        // guard exists to prevent.
        if matches!(path.segments().last(), Some(PathSeg::Key(_)))
            && d.key_span(&path_str).is_none()
        {
            return Ok(Some(Borrowed::Key));
        }
        // Everything below asks one question: do the resolved bytes start
        // before the earliest point this node's own bytes could? YAML requires
        // an anchor to precede every alias to it, so bytes ahead of that floor
        // are some other node's, reached by resolving an alias through.
        let Some((value_start, _)) = d.span_at(&path_str) else {
            return Ok(None);
        };
        let floor = match path.segments().last() {
            // A mapping entry's value cannot precede its own key.
            Some(PathSeg::Key(_)) => d.key_span(&path_str).map(|(start, _)| start),
            // A sequence item has no key, so the floor is the end of the item
            // before it — or, for the first, where the sequence itself starts.
            // Both are the item's own container, which is why an anchor
            // *outside* the sequence and one in an earlier sibling are caught
            // by the same comparison. An anchor reached through the sequence's
            // own parent is not, and cannot be: see `value_is_borrowed`.
            Some(PathSeg::Index(i)) => {
                let neighbour = match i.checked_sub(1) {
                    Some(prev) => rebased(path, Some(PathSeg::Index(prev))),
                    None => rebased(path, None),
                };
                let neighbour = to_noyalib_path(&neighbour);
                d.span_at(&neighbour).map(|(start, end)| {
                    if i.checked_sub(1).is_some() {
                        end
                    } else {
                        start
                    }
                })
            }
            // The root has nothing to precede.
            None => None,
        };
        Ok(floor
            .filter(|floor| value_start < *floor)
            .map(|_| Borrowed::Value))
    }

    /// Bounds-checked shared document accessor (used by the structural-delete
    /// fallback, which reads spans and source bytes before it mutates).
    /// Refuse replacing a *block* collection with a scalar, in yqr's words.
    ///
    /// The engine writes the scalar where the collection's first line began,
    /// which is the key's own column, so `k:` gains a value line no other
    /// implementation accepts. Its own guard cannot see it (the result parses,
    /// for this parser) and its message where it does refuse blames the input
    /// for an "inconsistent indentation" it does not have — the `yqr-b024`
    /// shape, on a document that parsed fine.
    ///
    /// Two collections are deliberately left alone, because neither has a
    /// mapping value to under-indent: a **flow** collection sits on the key's
    /// line, and a **sequence item** is not a mapping entry at all. Both were
    /// measured writing correctly.
    ///
    /// [`Self::check_integrity`] would catch the same class afterwards; this
    /// exists so the message is yqr's for every shape rather than for some.
    // Bug b029.
    fn refuse_block_collection_to_scalar(
        &self,
        doc: usize,
        path: &Path,
        path_str: &str,
        new: &Value,
    ) -> Result<()> {
        if matches!(new, Value::Sequence(_) | Value::Mapping(_)) {
            return Ok(());
        }
        if !matches!(path.segments().last(), Some(PathSeg::Key(_))) {
            return Ok(());
        }
        let current = self.value(doc)?;
        if !matches!(
            super::noyalib::walk_value(&current, path.segments()),
            Some(Value::Sequence(_) | Value::Mapping(_))
        ) {
            return Ok(());
        }
        let d = self.doc_ref(doc)?;
        if d.get(path_str)
            .is_some_and(|bytes| after_properties(bytes).starts_with(['[', '{']))
        {
            return Ok(());
        }
        Err(YqrError::eval(format!(
            "cannot assign at {path_str:?}: the value there is a block collection, and writing a \
             scalar over one would leave it at its key's own column, which this engine reads \
             back but other YAML parsers reject. Remove the entry and write it again, as in \
             `del(.{path_str})` then `.{path_str} = <value>`, which places it at the end of its \
             mapping"
        )))
    }

    /// The structural properties an edit must not make worse.
    ///
    /// Both are things the engine's own re-parse guard cannot see, because
    /// both produce a document *it* reads back: the parser accepts a block
    /// mapping value at its key's own column (`yqr-b014`'s class, and what
    /// `validate` reports as `Y103`), and a line ending is not structure at
    /// all. Measuring them on either side of a write turns each into a
    /// refusal rather than a file yqr quietly damaged.
    // Bugs b029, b030.
    fn integrity(&self, doc: usize) -> Result<Integrity> {
        let d = self.doc_ref(doc)?;
        Ok(Integrity {
            under_indented: crate::validate::scan::under_indented_values(d, 0).len(),
            crlf: d.source().matches("\r\n").count(),
            bare_lf: bare_line_feeds(d.source()),
        })
    }

    /// Refuse the edit just made when it broke either property of
    /// [`Self::integrity`].
    ///
    /// The caller has already mutated the document; returning `Err` is what
    /// undoes it, because [`apply`] emits nothing unless every document
    /// succeeded.
    // Bugs b029, b030.
    fn check_integrity(&self, doc: usize, before: Integrity, path_str: &str) -> Result<()> {
        let after = self.integrity(doc)?;
        if after.under_indented > before.under_indented {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: the result would leave a block mapping's value at \
                 its key's own column, which this engine reads back but other YAML parsers \
                 reject. Replacing a block collection with a scalar is what does this; remove the \
                 entry and write it again, as in `del(.{path_str})` then `.{path_str} = <value>`, \
                 which places it at the end of its mapping"
            )));
        }
        // Only a document that is wholly CRLF is held to it. One already
        // mixing the two is not made worse by this rule, and an all-LF
        // document gains a bare line feed per added line, legitimately.
        if before.crlf > 0 && before.bare_lf == 0 && after.bare_lf > 0 {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: this file's lines end with CRLF, and the \
                 replacement's own lines would end with LF, leaving the file with mixed line \
                 endings. yqr refuses rather than change bytes the edit does not name"
            )));
        }
        Ok(())
    }

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
        let path_str = to_noyalib_path(path);
        // A merged-in key is refused either way; what yqr owns here is the
        // *reason*. Only the anchor route is named, because it is the only one
        // that works: writing an *overriding* entry here is this very check,
        // and inserting a sibling is refused too unless the mapping already
        // owns one (both measured). Naming a remedy the tool declines would be
        // worse than naming none. Creating the override is `yqr-f025`. Upstream's resolver returns the same `None` for a key a
        // merge produced as for one that does not exist, so `set_value`
        // reports `path not found` for a path yqr had just read a value from —
        // one tool contradicting itself. Its own `rename_key` words this
        // correctly, so the wording is borrowed from there rather than
        // invented. The alias arm is *not* intercepted: upstream's message for
        // it is already accurate and names the way out.
        if let (Some(PathSeg::Key(key)), Some(Borrowed::Key)) =
            (path.segments().last(), self.borrowed_site(doc, path)?)
        {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: the mapping has no {key:?} entry of its own \
                 to write; it is merged in from elsewhere, through a `<<` merge key or an \
                 alias. Assign where the key is defined instead"
            )));
        }
        let ny = insertable(value);
        // A value led by a `&anchor` or `!tag` property is not `set_value`'s
        // to rewrite: its span starts at the property, so the edit would
        // delete the definition. Bug b026; handled by the guarded span
        // surgery in `write::anchor`, which also decides the tagged case.
        self.refuse_block_collection_to_scalar(doc, path, &path_str, value)?;
        let before = self.integrity(doc)?;
        if self.value_has_leading_property(doc, &path_str)? {
            self.assign_at_definition(doc, path, &path_str, value, &ny)?;
            return self.check_integrity(doc, before, &path_str);
        }
        match self.doc_mut(doc)?.set_value(&path_str, &ny) {
            Ok(()) => self.check_integrity(doc, before, &path_str),
            // Since noyalib 0.0.29 (#338) every mutator refuses a write into
            // a value that live alias sites share — the anchor's own
            // definition included, which is the one remedy the merged-key
            // refusal above names. The refusal is `Error::Parse` with no
            // variant of its own, so it is recognized by the API it points
            // at; a test pins the marker. The definition write goes through
            // the guarded span surgery instead.
            Err(e) if e.to_string().contains("materialise_aliases_of") => {
                self.assign_at_definition(doc, path, &path_str, value, &ny)?;
                self.check_integrity(doc, before, &path_str)
            }
            Err(e) => Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: {e}"
            ))),
        }
    }

    fn value_is_borrowed(&self, doc: usize, path: &Path) -> Result<bool> {
        // A node inside bytes that are not their owner's is not its own
        // either, so every ancestor counts, not just the addressed node.
        // `.b[0]` over `b: *x` is the case that needs it: the item clears its
        // own floor because the floor is measured inside the anchor's
        // sequence, and it is the *sequence* that is borrowed.
        let mut probe = path.clone();
        loop {
            if self.borrowed_site(doc, &probe)?.is_some() {
                return Ok(true);
            }
            if probe.is_root() {
                return Ok(false);
            }
            probe = rebased(&probe, None);
        }
    }

    fn insert_key(&mut self, doc: usize, parent: &Path, key: &str, value: &Value) -> Result<()> {
        let parent_str = to_noyalib_path(parent);
        let ny = insertable(value);
        self.doc_mut(doc)?
            .insert_entry_value(&parent_str, key, &ny)
            .map_err(|e| YqrError::eval(format!("cannot insert key {key:?}: {e}")))
    }

    fn append(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()> {
        let path_str = to_noyalib_path(path);
        let ny = insertable(value);
        self.doc_mut(doc)?
            .push_back_value(&path_str, &ny)
            .map_err(|e| YqrError::eval(format!("cannot append at {path_str:?}: {e}")))
    }

    fn delete(&mut self, doc: usize, path: &Path) -> Result<()> {
        // Block entries are deliberately not routed to noyalib's `remove`
        // (an item of a *flow* collection is — see `delete_entry`), and the gap
        // has closed rather than narrowed: measured on 0.0.22, upstream agrees with this path on
        // every case the b006 tests pin, differing only in how it words the
        // flow-item refusal. Keeping this path is therefore not a claim that
        // upstream is behind. It is that two implementations are what make
        // either one checkable — this one is how upstream's trivia divergences
        // were found and fixed (noyalib#225/#226) — and that swapping
        // implementations is a different trade from deleting a redundant pass
        // over the engine's own output, which is what f015 removed. Settled;
        // reopen on a new argument, not on upstream improving further.
        self.delete_entry(doc, path)
    }

    fn set_comment(
        &mut self,
        doc: usize,
        path: &Path,
        kind: CommentKind,
        text: &str,
    ) -> Result<()> {
        let path_str = to_noyalib_path(path);
        self.check_comment_site(doc, &path_str, kind)?;
        let d = self.doc_mut(doc)?;
        match kind {
            CommentKind::Line => d.set_inline_comment(&path_str, text),
            CommentKind::Head => d.set_leading_comment(&path_str, text),
        }
        .map_err(|e| YqrError::eval(format!("cannot set {}({path_str}): {e}", kind.word())))
    }

    fn current_comment(
        &self,
        doc: usize,
        path: &Path,
        kind: CommentKind,
    ) -> Result<Option<String>> {
        self.doc_ref(doc)?;
        let path_str = to_noyalib_path(path);
        // Report nothing at a site `set_comment` would refuse, so an equal
        // body can never stand in for a refusal — the lesson `value_is_borrowed`
        // records on the value side.
        if self.check_comment_site(doc, &path_str, kind).is_err() {
            return Ok(None);
        }
        let bundle = self.doc_ref(doc)?.comments_at(&path_str);
        Ok(match kind {
            CommentKind::Line => bundle
                .inline
                .map(|c| super::noyalib::comment_body(&c.text).to_string()),
            // `check_comment_site` passed, which for a head comment means the
            // run yqr owns *is* `before` — so no tail-slicing is needed here,
            // unlike on the read side where the two can disagree.
            CommentKind::Head if !bundle.before.is_empty() => Some(
                bundle
                    .before
                    .iter()
                    .map(|c| super::noyalib::comment_body(&c.text))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            CommentKind::Head => None,
        })
    }

    fn remove_comment(&mut self, doc: usize, path: &Path, kind: CommentKind) -> Result<()> {
        let path_str = to_noyalib_path(path);
        self.check_comment_site(doc, &path_str, kind)?;
        // Upstream's removers refuse nothing, so "there is no comment here" has
        // to be yqr's own check: `del` is a mutation, and a mutation refuses
        // rather than no-ops (`yqr-a002` §4.4).
        if !self.comment_present(doc, &path_str, kind)? {
            return Err(YqrError::eval(format!(
                "cannot remove {}({path_str}): the entry has no {} to remove",
                kind.word(),
                match kind {
                    CommentKind::Line => "comment on its own line",
                    CommentKind::Head => "comment block above it",
                }
            )));
        }
        let d = self.doc_mut(doc)?;
        match kind {
            CommentKind::Line => d.remove_inline_comment(&path_str),
            CommentKind::Head => d.remove_leading_comment(&path_str),
        }
        .map_err(|e| YqrError::eval(format!("cannot remove {}({path_str}): {e}", kind.word())))?;

        // Upstream's removers report `Ok(())` for shapes they do not handle —
        // a leading block on a sequence item is the measured case — so `Ok` is
        // not evidence that anything happened. Checking the comment is
        // actually gone turns that into the refusal `yqr-a002` §4.4 requires,
        // and does it for any such shape rather than for an enumerated list.
        // The document is already unchanged in exactly this case, so there is
        // nothing to roll back.
        if self.comment_present(doc, &path_str, kind)? {
            return Err(YqrError::eval(format!(
                "cannot remove {}({path_str}): the YAML engine does not support removing \
                 it from this kind of entry, and reported success without removing it",
                kind.word()
            )));
        }
        Ok(())
    }

    fn rename_key(&mut self, doc: usize, path: &Path, new_key: &str) -> Result<()> {
        // The addressable set is closed under rename without a check here:
        // every string is a key some filter can name, the empty one included
        // (`.[""]`), so no rename can produce an entry yqr cannot reach again.
        // Upstream still refuses `<<` and a non-printable character, and yqr
        // forwards both.
        let path_str = to_noyalib_path(path);
        self.doc_mut(doc)?
            .rename_key(&path_str, new_key)
            .map_err(|e| YqrError::eval(format!("cannot rename key at {path_str:?}: {e}")))
    }

    fn reorder(
        &mut self,
        doc: usize,
        path: &Path,
        op: ReorderOp,
        from: i64,
        to: i64,
    ) -> Result<()> {
        self.reorder_items(doc, path, op, from, to)
    }

    /// Concatenate the document stream, byte-for-byte as each document now
    /// stands.
    ///
    /// Nothing is post-processed here. yqr used to re-terminate the lines an
    /// edit added, because the mutators ended an inserted line with `\n`
    /// whatever the document used; noyalib 0.0.22 derives the terminator from
    /// the document the same way it already derived the indentation, so the
    /// bytes arrive correct and second-guessing the engine's line endings would
    /// only be a rewrite waiting to disagree with it.
    fn emit(&self) -> String {
        self.docs.iter().map(ToString::to_string).collect()
    }
}

/// `path` with its last segment replaced by `seg`, or dropped when `seg` is
/// `None` — the parent path and the sibling path, which [`Path`] has no
/// accessors for because nothing before the borrowed-site check needed to walk
/// *upwards*.
///
/// The root has no last segment, so both forms return it unchanged.
// Bug b019.
fn rebased(path: &Path, seg: Option<PathSeg>) -> Path {
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
fn insertable(value: &Value) -> ::noyalib::Value {
    ::noyalib::Value::from(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Rhs;

    fn assign(path: &str, rhs: Rhs) -> Mutation {
        Mutation::Assign {
            target: Target::Value(crate::parser::parse(path).expect("valid path")),
            rhs,
        }
    }

    /// Run `key(<path>) = <new>` over `input`.
    // Feature f007: key rename.
    fn rename(path: &str, new_key: &str, input: &str) -> Result<String> {
        apply(
            &Mutation::Assign {
                target: Target::Key(crate::parser::parse(path).expect("valid path")),
                rhs: Rhs::Literal(Value::String(new_key.to_string())),
            },
            input,
        )
    }

    /// Run `line_comment(<path>) = <text>` / `head_comment(...)` over `input`.
    fn set_comment_on(kind: Target, text: &str, input: &str) -> Result<String> {
        apply(
            &Mutation::Assign {
                target: kind,
                rhs: Rhs::Literal(Value::String(text.to_string())),
            },
            input,
        )
    }

    fn line_of(path: &str) -> Target {
        Target::LineComment(crate::parser::parse(path).expect("valid path"))
    }
    fn head_of(path: &str) -> Target {
        Target::HeadComment(crate::parser::parse(path).expect("valid path"))
    }
    fn del_comment(target: Target, input: &str) -> Result<String> {
        apply(&Mutation::Delete { target }, input)
    }

    // -- Feature f007: comment editing (a002 slice 2) --------------------------

    #[test]
    fn sets_and_changes_an_inline_comment_byte_exactly() {
        let input = "# header\nspec:\n  replicas: 3\n  image: web\n";
        let once = set_comment_on(line_of(".spec.replicas"), "tuned", input).unwrap();
        assert_eq!(
            once,
            "# header\nspec:\n  replicas: 3  # tuned\n  image: web\n"
        );
        // Changing replaces the body in place, keeping the separator.
        let twice = set_comment_on(line_of(".spec.replicas"), "again", &once).unwrap();
        assert_eq!(
            twice,
            "# header\nspec:\n  replicas: 3  # again\n  image: web\n"
        );
    }

    #[test]
    fn an_empty_body_writes_a_bare_hash_rather_than_removing() {
        // a002 §4.2: upstream distinguishes the two and yqr already owns an
        // unambiguous removal spelling, so conflating them would make one of
        // them unreachable.
        let out = set_comment_on(line_of(".a"), "", "a: 1  # note\n").unwrap();
        assert_eq!(out, "a: 1  #\n");
        assert_eq!(
            del_comment(line_of(".a"), "a: 1  # note\n").unwrap(),
            "a: 1\n"
        );
    }

    #[test]
    fn head_comment_lands_above_the_entry_at_its_own_indent() {
        let out =
            set_comment_on(head_of(".spec.replicas"), "why", "spec:\n  replicas: 3\n").unwrap();
        assert_eq!(out, "spec:\n  # why\n  replicas: 3\n");
    }

    #[test]
    fn a_multi_line_head_comment_is_one_line_per_segment() {
        let out = set_comment_on(head_of(".a"), "one\ntwo", "a: 1\n").unwrap();
        assert_eq!(out, "# one\n# two\na: 1\n");
    }

    #[test]
    fn a_crlf_document_gets_crlf_comment_lines() {
        let out = set_comment_on(head_of(".a"), "why", "a: 1\r\nb: 2\r\n").unwrap();
        assert_eq!(out, "# why\r\na: 1\r\nb: 2\r\n");
    }

    #[test]
    fn an_entry_whose_value_starts_on_the_next_line_is_refused_both_ways() {
        // a002 §4.1.2. Upstream's guard looks at the value span, which is
        // single-line here, so it writes onto the *child's* line — and the
        // remover deletes the child's comment. Neither may happen.
        let input = "a:\n  b: 1  # child\nc: 2\n";
        for err in [
            set_comment_on(line_of(".a"), "x", input).unwrap_err(),
            del_comment(line_of(".a"), input).unwrap_err(),
        ] {
            assert!(
                format!("{err}").contains("no line of its own"),
                "got: {err}"
            );
        }
    }

    #[test]
    fn a_sequence_item_whose_value_starts_below_the_dash_is_refused_too() {
        // The §4.1.2 guard originally asked only "does this entry have a key
        // token on the value's line", and answered `true` for every sequence
        // item on the reasoning that an item always sits on its own line. A
        // bare `-` with the value below it is the counterexample: the shape is
        // the mapping case exactly, and the comment lands on the child.
        let input = "xs:\n  -\n    a: 1  # child\n";
        for err in [
            set_comment_on(line_of(".xs[0]"), "mine", input).unwrap_err(),
            del_comment(line_of(".xs[0]"), input).unwrap_err(),
        ] {
            assert!(
                format!("{err}").contains("no line of its own"),
                "got: {err}"
            );
        }
    }

    #[test]
    fn an_ordinary_sequence_item_still_takes_an_inline_comment() {
        // The guard above must not refuse the common shape.
        let out = set_comment_on(line_of(".xs[0]"), "first", "xs:\n  - one\n").unwrap();
        assert_eq!(out, "xs:\n  - one  # first\n");
    }

    #[test]
    fn a_blank_detached_block_is_never_rewritten() {
        // a002 §4.1.1: `comments_at().before` walks past blank lines, so
        // delegating would replace (or delete) a comment documenting whatever
        // came before the entry.
        let input = "# detached\n\na: 1\n";
        for err in [
            set_comment_on(head_of(".a"), "new", input).unwrap_err(),
            del_comment(head_of(".a"), input).unwrap_err(),
        ] {
            assert!(
                format!("{err}").contains("rewrite bytes the path does not name"),
                "got: {err}"
            );
        }
    }

    #[test]
    fn a_partially_detached_block_is_refused_too() {
        // Upstream reports both comments as the entry's; yqr owns only the
        // contiguous tail. Editing here would reach `# far`, which the path
        // does not name.
        let input = "# far\n\n# near\na: 1\n";
        assert!(set_comment_on(head_of(".a"), "new", input).is_err());
        assert!(del_comment(head_of(".a"), input).is_err());
    }

    #[test]
    fn removing_a_comment_that_is_not_there_refuses() {
        // a002 §4.4: `del` is a mutation, and a mutation refuses rather than
        // no-ops. Upstream's removers return Ok for this.
        assert!(del_comment(line_of(".a"), "a: 1\n").is_err());
        assert!(del_comment(head_of(".a"), "a: 1\n").is_err());
    }

    #[test]
    fn removing_a_sequence_items_head_comment_refuses_instead_of_no_opping() {
        // Upstream returns Ok having done nothing, which is the shape the
        // post-removal check exists to catch.
        let input = "xs:\n  # about one\n  - one\n  - two\n";
        let err = del_comment(head_of(".xs[0]"), input).unwrap_err();
        assert!(
            format!("{err}").contains("reported success without removing it"),
            "got: {err}"
        );
    }

    #[test]
    fn a_comment_on_an_absent_path_is_a_skip_not_an_error() {
        let input = "a: 1\n";
        assert_eq!(set_comment_on(line_of(".nope"), "x", input).unwrap(), input);
        assert_eq!(del_comment(line_of(".nope"), input).unwrap(), input);
    }

    #[test]
    fn a_non_string_comment_body_is_refused() {
        let err = apply(
            &Mutation::Assign {
                target: line_of(".a"),
                rhs: Rhs::Literal(Value::Int(5)),
            },
            "a: 1\n",
        )
        .unwrap_err();
        assert!(format!("{err}").contains("must be a string"), "got: {err}");
    }

    // -- Feature f007: key rename ---------------------------------------------

    #[test]
    fn rename_rewrites_the_key_token_and_nothing_else() {
        // The whole property in one case: the value keeps its spelling, the
        // inline comment keeps its column, the head comment stays above, the
        // sibling and the document header are untouched, and the entry keeps
        // its position in the mapping.
        let input = "# header\nmetadata:\n  # names the app\n  name:   app   # why\n  tier: web\n";
        let out = rename(".metadata.name", "title", input).unwrap();
        assert_eq!(
            out,
            "# header\nmetadata:\n  # names the app\n  title:   app   # why\n  tier: web\n"
        );
    }

    #[test]
    fn rename_keeps_key_order() {
        // A rename is not a delete plus an insert; `b` must not move to the end.
        let out = rename(".b", "z", "a: 1\nb: 2\nc: 3\n").unwrap();
        assert_eq!(out, "a: 1\nz: 2\nc: 3\n");
    }

    #[test]
    fn rename_matches_the_neighbouring_quote_style() {
        let out = rename(".b", "new", "\"a\": 1\n\"b\": 2\n").unwrap();
        assert_eq!(out, "\"a\": 1\n\"new\": 2\n");
    }

    #[test]
    fn rename_preserves_a_multi_line_value_verbatim() {
        let input = "notes: |\n    line one\n    line two\nafter: 1\n";
        let out = rename(".notes", "text", input).unwrap();
        assert_eq!(out, "text: |\n    line one\n    line two\nafter: 1\n");
    }

    #[test]
    fn rename_of_an_absent_path_is_a_noop_not_a_new_key() {
        // `resolve_assign_target`'s create-a-key branch would be the wrong
        // answer here: there is no key to rename, so the document is left
        // alone rather than growing an entry the filter never named.
        let input = "a: 1\n";
        assert_eq!(rename(".absent", "x", input).unwrap(), input);
    }

    #[test]
    fn rename_skips_documents_where_the_path_is_absent() {
        let input = "a: 1\n---\nb: 2\n";
        assert_eq!(rename(".a", "z", input).unwrap(), "z: 1\n---\nb: 2\n");
    }

    #[test]
    fn rename_refuses_a_sequence_item() {
        let input = "xs:\n  - one\n";
        let err = rename(".xs[0]", "k", input).unwrap_err();
        assert!(
            format!("{err}").contains("sequence item"),
            "message should name the reason, got: {err}"
        );
    }

    #[test]
    fn rename_refuses_a_sibling_collision() {
        let err = rename(".a", "b", "a: 1\nb: 2\n").unwrap_err();
        assert!(
            format!("{err}").contains("already has an entry"),
            "message should name the collision, got: {err}"
        );
    }

    #[test]
    fn rename_refuses_a_merge_key() {
        let input = "base: &b\n  x: 1\nuse:\n  <<: *b\n  y: 2\n";
        let err = rename(".use.x", "z", input).unwrap_err();
        assert!(
            format!("{err}").contains("merge key"),
            "message should name the merge, got: {err}"
        );
    }

    #[test]
    fn rename_refuses_a_non_string_key() {
        let err = apply(
            &Mutation::Assign {
                target: Target::Key(crate::parser::parse(".a").expect("valid")),
                rhs: Rhs::Literal(Value::Int(5)),
            },
            "a: 1\n",
        )
        .unwrap_err();
        assert!(format!("{err}").contains("must be a string"), "got: {err}");
    }

    #[test]
    fn a_refused_rename_leaves_the_document_untouched() {
        // The `-i` contract: a refusal must not have half-applied anything.
        let input = "a: 1\nb: 2\n";
        assert!(rename(".a", "b", input).is_err());
        // Re-running a *valid* rename over the same input still produces the
        // clean result, so nothing was mutated in place on the way out.
        assert_eq!(rename(".a", "z", input).unwrap(), "z: 1\nb: 2\n");
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

    // -- Feature f032: collection right-hand sides ------------------------

    /// Run `<path> = <rhs path>` over `input`.
    fn assign_path(path: &str, rhs: &str, input: &str) -> Result<String> {
        apply(
            &assign(
                path,
                Rhs::Path(crate::parser::parse(rhs).expect("valid rhs")),
            ),
            input,
        )
    }

    #[test]
    fn a_collection_becomes_a_new_key() {
        let out = assign_path(".m.new", ".src", "m:\n  a: 1\nsrc:\n  x: 1\n  y: two\n").unwrap();
        assert_eq!(
            out,
            "m:\n  a: 1\n  new:\n    x: 1\n    y: two\nsrc:\n  x: 1\n  y: two\n"
        );
    }

    #[test]
    fn a_sequence_becomes_a_new_key() {
        let out = assign_path(".m.new", ".src", "m:\n  a: 1\nsrc:\n  - one\n  - two\n").unwrap();
        assert_eq!(
            out,
            "m:\n  a: 1\n  new:\n    - one\n    - two\nsrc:\n  - one\n  - two\n"
        );
    }

    #[test]
    fn a_collection_is_appended_as_a_sequence_item() {
        let out = apply(
            &Mutation::Append {
                path: crate::parser::parse(".xs").expect("valid"),
                rhs: Rhs::Path(crate::parser::parse(".src").expect("valid")),
            },
            "xs:\n  - a: 1\nsrc:\n  a: 2\n  b: 3\n",
        )
        .unwrap();
        assert_eq!(
            out,
            "xs:\n  - a: 1\n  - a: 2\n    b: 3\nsrc:\n  a: 2\n  b: 3\n"
        );
    }

    #[test]
    fn a_collection_replaces_a_collection_at_the_sites_own_indent() {
        let out = assign_path(
            ".top.k",
            ".src",
            "top:\n  k:\n    old: 1\n  after: keep   # tail\nsrc:\n  new: 2\n",
        )
        .unwrap();
        assert_eq!(
            out,
            "top:\n  k:\n    new: 2\n  after: keep   # tail\nsrc:\n  new: 2\n"
        );
    }

    #[test]
    fn writing_a_collection_over_an_equal_one_is_a_no_op() {
        // The no-op guard compares values, so it holds for a collection the
        // same way it holds for a scalar: nothing is re-spelled.
        let input = "k:\n  a: 1      # kept\n  b: 'two'\n";
        assert_eq!(assign_path(".k", ".k", input).unwrap(), input);
    }

    #[test]
    fn a_collection_over_a_scalar_names_a_remedy_that_works() {
        let input = "c: 1\nsrc:\n  a: 1\n";
        let err = assign_path(".c", ".src", input).unwrap_err();
        let text = format!("{err}");
        assert!(
            text.contains("yqr writes a collection only where one already is"),
            "got: {text}"
        );
        assert!(text.contains("del(.c)"), "no remedy named: {text}");
        // The remedy has to work (`yqr-f025`).
        let removed = apply(
            &Mutation::Delete {
                target: Target::Value(crate::parser::parse(".c").expect("valid")),
            },
            input,
        )
        .unwrap();
        assert_eq!(
            assign_path(".c", ".src", &removed).unwrap(),
            "src:\n  a: 1\nc:\n  a: 1\n"
        );
    }

    // -- Bug b029: a scalar must not be written over a block collection -----

    #[test]
    fn a_scalar_over_a_block_collection_is_refused() {
        // Upstream writes the scalar at column 0 (`k:` then `5`), which its
        // own parser reads back and PyYAML and Psych reject, so neither the
        // re-parse guard nor upstream's own guard sees it.
        for (path, input) in [
            (".k", "k:\n  a: 1\nafter: 1\n"),
            (".k", "k:\n  - 1\n  - 2\nafter: 1\n"),
            (".top.k", "top:\n  k:\n    a: 1\n  after: 2\n"),
        ] {
            let err = apply(&assign(path, Rhs::Literal(Value::Int(5))), input).unwrap_err();
            assert!(
                format!("{err}").contains("at its key's own column"),
                "{input:?} was not refused in yqr's words: {err}"
            );
        }
    }

    #[test]
    fn a_scalar_over_a_flow_collection_or_a_sequence_item_still_writes() {
        // The guard is the Y103 property, not a ban on the type change: a
        // flow value has no line of its own to under-indent, and a sequence
        // item is not a mapping entry.
        assert_eq!(
            apply(
                &assign(".k", Rhs::Literal(Value::Int(5))),
                "k: {a: 1}\nafter: 1\n"
            )
            .unwrap(),
            "k: 5\nafter: 1\n"
        );
        assert_eq!(
            apply(
                &assign(".xs[0]", Rhs::Literal(Value::Int(5))),
                "xs:\n  - a: 1\n  - b: 2\n"
            )
            .unwrap(),
            "xs:\n  - 5\n  - b: 2\n"
        );
    }

    #[test]
    fn an_absent_right_hand_path_does_not_flatten_a_block() {
        // `.k = .missing` resolves to null, a scalar, so this is the b029
        // shape reached without naming a scalar at all.
        let err = assign_path(".k", ".missing", "k:\n  a: 1\n").unwrap_err();
        assert!(format!("{err}").contains("block collection"), "{err}");
    }

    // Found in code review of f032: a flow collection carrying a property
    // does not start with `[`, so the b029 guard read it as a block one and
    // refused an edit that works.
    #[test]
    fn a_scalar_over_a_flow_collection_with_a_property_still_writes() {
        assert_eq!(
            apply(
                &assign(".k", Rhs::Literal(Value::Int(5))),
                "k: &an {a: 1}\nafter: 1\n"
            )
            .unwrap(),
            "k: &an 5\nafter: 1\n"
        );
        // A tag is refused, but by the guard that knows why: replacing the
        // scalar under a tag can change what the tag makes of it.
        let err = apply(
            &assign(".k", Rhs::Literal(Value::Int(5))),
            "k: !!map {a: 1}\nafter: 1\n",
        )
        .unwrap_err();
        let text = format!("{err}");
        assert!(text.contains("carries the tag"), "{text}");
        assert!(
            !text.contains("block collection"),
            "wrong diagnosis: {text}"
        );
    }

    #[test]
    fn an_anchored_block_collection_is_still_refused() {
        // The property skip must not turn the guard off for a block value.
        let err = apply(
            &assign(".k", Rhs::Literal(Value::Int(5))),
            "k: &an\n  a: 1\nafter: 1\n",
        )
        .unwrap_err();
        assert!(format!("{err}").contains("block collection"), "{err}");
    }

    // Each refusal names the remedy for its own shape, and the remedy is run
    // here rather than asserted to exist (`yqr-f025`). One sentence for all
    // three was wrong for two of them.
    #[test]
    fn the_collection_over_a_scalar_remedy_fits_the_shape() {
        // A sequence item: `del` would shift the items up, so the remedy is
        // `+=`, which appends.
        let seq = "xs:\n  - 1\n  - 2\nsrc:\n  a: 1\n";
        let err = assign_path(".xs[0]", ".src", seq).unwrap_err();
        let text = format!("{err}");
        assert!(text.contains("`.xs += <path>`"), "{text}");
        assert!(
            !text.contains("end of its mapping"),
            "no mapping here: {text}"
        );
        assert_eq!(
            apply(
                &Mutation::Append {
                    path: crate::parser::parse(".xs").expect("valid"),
                    rhs: Rhs::Path(crate::parser::parse(".src").expect("valid")),
                },
                seq,
            )
            .unwrap(),
            "xs:\n  - 1\n  - 2\n  - a: 1\nsrc:\n  a: 1\n"
        );

        // An entry left empty has no bytes, so `del` refuses it too; the
        // message says which null it is talking about rather than promising.
        let err = assign_path(".k", ".src", "k:\nsrc:\n  a: 1\n").unwrap_err();
        assert!(format!("{err}").contains("rather than left empty"), "{err}");

        // Written `k: null`, the entry has bytes and the remedy works.
        let removed = apply(
            &Mutation::Delete {
                target: Target::Value(crate::parser::parse(".k").expect("valid")),
            },
            "k: null\nsrc:\n  a: 1\n",
        )
        .unwrap();
        assert_eq!(
            assign_path(".k", ".src", &removed).unwrap(),
            "src:\n  a: 1\nk:\n  a: 1\n"
        );
    }

    // -- Bug b030: a write must not give a CRLF file mixed line endings -----

    #[test]
    fn a_multi_line_write_into_a_crlf_document_is_refused() {
        // The engine joins the replacement's own lines with LF while the
        // file's are CRLF. Shipped since the multi-line string write existed;
        // the collection arm would have inherited it.
        let err = apply(
            &assign(".a", Rhs::Literal(Value::String("one\ntwo".into()))),
            "a: 1\r\nb: 2\r\n",
        )
        .unwrap_err();
        assert!(format!("{err}").contains("mixed line endings"), "{err}");
    }

    #[test]
    fn the_crlf_guard_leaves_the_paths_that_are_correct_alone() {
        // The insertion mutators derive the terminator from the document
        // (`yqr-b009`, fixed upstream), so they are untouched by the guard —
        // and a single-line write never trips it.
        assert_eq!(
            assign_path(".m.new", ".src", "m:\r\n  a: 1\r\nsrc:\r\n  x: 1\r\n").unwrap(),
            "m:\r\n  a: 1\r\n  new:\r\n    x: 1\r\nsrc:\r\n  x: 1\r\n"
        );
        assert_eq!(
            apply(
                &assign(".a", Rhs::Literal(Value::Int(9))),
                "a: 1\r\nb: 2\r\n"
            )
            .unwrap(),
            "a: 9\r\nb: 2\r\n"
        );
    }

    #[test]
    fn an_all_lf_document_may_still_grow_lines() {
        // The rule keys on a document that is wholly CRLF, so an LF file
        // gaining a line per inserted entry is not a violation.
        let out = assign_path(".m.new", ".src", "m:\n  a: 1\nsrc:\n  x: 1\n  y: 2\n").unwrap();
        assert!(!out.contains('\r'), "{out:?}");
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
    fn a_borrowed_site_says_which_way_it_is_borrowed() {
        // The two arms drive different refusals -- `Key` is yqr's own message
        // (`yqr-b020`), `Value` falls through to upstream's -- so collapsing
        // them to a bool at the source would lose the distinction the
        // diagnostics depend on.
        let cases: &[(&str, &str, Option<Borrowed>)] = &[
            ("m: &m\n  k: 1\nc:\n  <<: *m\n", ".c.k", Some(Borrowed::Key)),
            ("m: &m\n  k: 1\nc: *m\n", ".c.k", Some(Borrowed::Key)),
            ("a: &x 1\nb: *x\n", ".b", Some(Borrowed::Value)),
            ("a: &x 1\nb:\n  c: *x\n", ".b.c", Some(Borrowed::Value)),
            // A sequence item measured against its floor rather than a key.
            ("b:\n  - &x 1\n  - *x\n", ".b[1]", Some(Borrowed::Value)),
            ("a:\n  - &x 1\nb:\n  - *x\n", ".b[0]", Some(Borrowed::Value)),
            ("a:\n  - &x 1\n  - 2\n", ".a[0]", None),
            ("a:\n  - 1\n  - 2\n", ".a[1]", None),
            ("n: 0640\n", ".n", None),
            ("a:\nb: 1\n", ".a", None),
        ];
        for (src, filter, want) in cases {
            let writer = NoyalibWriter::open(src).expect("valid YAML");
            let value = writer.value(0).expect("one document");
            let ast = crate::parser::parse(filter).expect("a valid path");
            let path = crate::eval::resolve_target(&ast, &value)
                .expect("resolves")
                .expect("to a node");
            assert_eq!(
                writer.borrowed_site(0, &path).expect("in range"),
                *want,
                "{filter} over {src:?}"
            );
        }
    }

    #[test]
    fn borrowed_is_the_value_living_somewhere_else() {
        // The predicate the no-op guard consults. Its exactness is what keeps
        // `yqr-b018`'s skip intact while restoring `yqr-b019`'s refusal, so
        // both directions are pinned: a false positive re-spells a scalar that
        // needed no write, a false negative swallows a refusal.
        let borrowed: &[(&str, &str, &str)] = &[
            ("an alias-valued entry", "a: &x 1\nb: *x\n", ".b"),
            ("nested under one", "a: &x 1\nb:\n  c: *x\n", ".b.c"),
            (
                "a key a merge produced",
                "m: &m\n  k: 1\nc:\n  <<: *m\n",
                ".c.k",
            ),
            ("a key an alias expanded", "m: &m\n  k: 1\nc: *m\n", ".c.k"),
            // Not borrowed by its *own* measurement — it clears the floor,
            // because the floor is inside the anchor's sequence. This is the
            // case the ancestor walk exists for: the borrowing is at `b`.
            (
                "an item of an aliased sequence",
                "a: &x\n  - 1\nb: *x\n",
                ".b[0]",
            ),
        ];
        let own: &[(&str, &str, &str)] = &[
            ("a plain scalar", "n: 0640\n", ".n"),
            // The anchor is where the value actually lives.
            ("the anchor itself", "a: &x 1\nb: *x\n", ".a"),
            // No value span, but the key is the mapping's own -- refusing
            // here would re-spell `a:` on `.a = null`.
            ("an implicit null", "a:\nb: 1\n", ".a"),
            // No key span, and legitimately so.
            ("a sequence item", "a:\n  - 1\n", ".a[0]"),
            ("the document root", "1\n", "."),
            (
                "a merge's own sibling",
                "m: &m\n  k: 1\nc:\n  <<: *m\n  z: 2\n",
                ".c.z",
            ),
        ];
        for (what, src, filter) in borrowed.iter().chain(own) {
            let writer = NoyalibWriter::open(src).expect("valid YAML");
            let value = writer.value(0).expect("one document");
            let ast = crate::parser::parse(filter).expect("a valid path");
            let path = crate::eval::resolve_target(&ast, &value)
                .expect("resolves")
                .expect("to a node");
            assert_eq!(
                writer.value_is_borrowed(0, &path).expect("in range"),
                borrowed.iter().any(|(w, _, _)| w == what),
                "{what}"
            );
        }
    }

    // Feature f030: a dotted key is addressed through a bracket-quoted
    // segment, so the write lands on it like on any other key.
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
