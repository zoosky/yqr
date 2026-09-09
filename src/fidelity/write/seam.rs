//! The write seam: the trait a backend implements, and the two enums it
//! speaks in.
//!
//! Kept apart from both the driver and the backend because it is the contract
//! between them. Every method's documentation states what it refuses and how
//! strongly it guards, which is the part a second implementation would have to
//! meet.

// Feature f006; moved here by f033.

use crate::Value;
use crate::ast::ReorderOp;
use crate::error::Result;
use crate::fidelity::Path;

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
    pub(super) fn word(self) -> &'static str {
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
