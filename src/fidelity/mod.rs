//! The fidelity engine seam: byte-preserving reads over YAML sources.
//!
//! yqr's default pipeline (`load_str` -> [`Value`] -> `dump_str`) is a
//! *semantic* round trip: it re-serializes values, normalizing comments,
//! quoting, indentation, and formatting away. This module provides the
//! alternative contract — **never rewrite bytes the filter did not change** —
//! through a backend-agnostic engine interface:
//!
//! - [`FidelityEngine`] pairs a verbatim copy of the input with a
//!   path -> byte-span index, so a selected node is emitted by **slicing its
//!   original bytes** instead of re-serializing it.
//! - The evaluator threads a concrete [`Path`] alongside each value it
//!   produces; [`run`] resolves that path per result and chooses between a
//!   verbatim slice ([`Resolved::Found`]) and a visible, per-node fallback to
//!   typed rendering ([`Resolved::Synthetic`] / [`Resolved::Unaddressable`]).
//! - Multi-document streams are first-class: the filter runs against every
//!   document, and identity output concatenates the original document slices
//!   byte-for-byte.
//!
//! noyalib's lossless CST is yqr's one and only engine. The [`FidelityEngine`]
//! trait remains as the boundary between the driver and the engine's API
//! surface, not as a runtime choice point.

// Feature f002 (see specs/features/): fidelity read floor — seam + driver.

// `noyalib` below is the local backend module; reach the crate's `Value`
// re-export through `crate::` to avoid shadowing.
use crate::Value;
use crate::ast::Ast;
use crate::error::Result;

mod noyalib;
pub mod write;

/// A half-open byte range `[start, end)` into [`FidelityEngine::source`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

impl Span {
    /// Create a span from start/end byte offsets.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Slice this span out of `source`, yielding the verbatim original bytes.
    ///
    /// This is the read path's entire emit primitive: a faithfully located
    /// node is printed from this slice with no re-serialization.
    #[must_use]
    pub fn slice<'s>(&self, source: &'s str) -> &'s str {
        &source[self.start..self.end]
    }
}

/// One step of a concrete, fully resolved access path.
///
/// The evaluator desugars a filter into these *after* resolving dynamic forms
/// (negative indices, iteration) against the typed value, so a segment is
/// always directly addressable — the engine never interprets filter
/// semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSeg {
    /// A mapping key, stored decoded (no quotes, escapes resolved) — the same
    /// string the typed [`Value`] uses.
    Key(String),
    /// A zero-based, already-resolved sequence index.
    Index(usize),
}

impl PathSeg {
    /// Whether this segment is expressible through a backend that only speaks
    /// a plain dotted/bracketed string path with no key escaping. `Index` is
    /// always plain; a key is plain unless it is empty or contains one of the
    /// path metacharacters. A string-path backend uses this to report
    /// [`Unaddressable::SpecialCharKey`] deterministically instead of
    /// silently resolving the wrong node.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        match self {
            PathSeg::Index(_) => true,
            PathSeg::Key(k) => Self::key_is_plain(k),
        }
    }

    /// Whether a mapping key string is expressible in a plain dotted/bracketed
    /// string path. Lets a caller test a bare `&str` key (a not-yet-inserted
    /// key) without constructing a [`PathSeg`].
    #[must_use]
    pub fn key_is_plain(key: &str) -> bool {
        !key.is_empty() && !key.contains(['.', '[', ']', '*'])
    }
}

/// A concrete path from a document root to a single node.
///
/// The empty path denotes the document root, which is how the identity filter
/// round-trips: its span is the whole document slice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Path(Vec<PathSeg>);

impl Path {
    /// The root path (selects the whole document).
    #[must_use]
    pub const fn root() -> Self {
        Path(Vec::new())
    }

    /// Whether this path selects the document root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// A new path extended by one segment. The evaluator threads a path
    /// alongside each value it produces, branching on iteration.
    #[must_use]
    pub fn child(&self, seg: PathSeg) -> Self {
        let mut next = self.0.clone();
        next.push(seg);
        Path(next)
    }

    /// The ordered segments of this path.
    #[must_use]
    pub fn segments(&self) -> &[PathSeg] {
        &self.0
    }
}

/// Why a node that exists in the typed value cannot be sliced from the
/// original source on this backend.
///
/// This is deliberately distinct from absence (which is jq `null`): it means
/// "no faithful span available here", and the caller degrades to lossy
/// re-serialization *visibly*, for this node only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unaddressable {
    /// The key uses characters the backend's path layer cannot express.
    SpecialCharKey(String),
}

/// Outcome of resolving a concrete path against one document.
///
/// The four arms are mutually exclusive and each drives a distinct emit
/// choice: verbatim bytes on `Found`, jq `null` on `Absent`, and a visible
/// fallback to typed rendering on `Synthetic`/`Unaddressable`.
#[derive(Debug)]
pub enum Resolved<'a> {
    /// Node found with original source bytes; the read path emits `bytes`
    /// verbatim. Backends must only return this when the bytes demonstrably
    /// denote the value the evaluator selected (the wrong-node guard) —
    /// otherwise they degrade to [`Resolved::Synthetic`].
    Found {
        /// Byte range of the node within [`FidelityEngine::source`].
        span: Span,
        /// The node's exact original bytes.
        bytes: &'a str,
    },
    /// The path is valid but selects a node with no source bytes of its own
    /// (an implicit null, a merge-key entry, alias-expanded content). The
    /// caller re-serializes from the typed value.
    Synthetic,
    /// The path does not resolve in this document: jq `null`, not an error.
    Absent,
    /// The node exists but this backend cannot address it faithfully.
    Unaddressable(Unaddressable),
}

/// A source-preserving view over one parsed YAML input (read path).
///
/// Implementations pair the verbatim input bytes with a structural span
/// index derived from the same parse as the typed value, so a path that is
/// valid against [`value`](Self::value) never resolves to the wrong bytes —
/// at worst it resolves to no bytes and the caller falls back visibly.
///
/// The trait is object-safe; yqr opens the engine at startup and drives it
/// through `Box<dyn FidelityEngine>`.
pub trait FidelityEngine {
    /// The entire original input, byte-for-byte (BOM, CRLF, comments,
    /// `---`/`...` markers, trailing whitespace and all).
    fn source(&self) -> &str;

    /// Number of logical YAML documents in the stream (0 for an empty
    /// stream).
    fn doc_count(&self) -> usize;

    /// Byte span of document `doc` within [`source`](Self::source).
    /// Concatenating every document span reproduces the source. `None` when
    /// `doc` is out of range.
    fn doc_span(&self, doc: usize) -> Option<Span>;

    /// The typed value of document `doc`, in yqr's evaluation model. This
    /// view is intentionally lossy (formatting and scalar spellings are
    /// normalized); fidelity is recovered through
    /// [`resolve`](Self::resolve).
    ///
    /// # Errors
    ///
    /// Returns an error when `doc` is out of range.
    fn value(&self, doc: usize) -> Result<Value>;

    /// Resolve a concrete [`Path`] against document `doc`.
    ///
    /// The root path resolves to `Found` over the whole document slice. A
    /// missing key/index is [`Resolved::Absent`] (jq `null`), not an error.
    ///
    /// # Errors
    ///
    /// Returns an error when `doc` is out of range.
    fn resolve(&self, doc: usize, path: &Path) -> Result<Resolved<'_>>;
}

/// Parse `input` with the noyalib engine, keeping its bytes verbatim.
///
/// # Errors
///
/// Returns an error when the input is not valid YAML.
pub fn open(input: &str) -> Result<Box<dyn FidelityEngine>> {
    Ok(Box::new(noyalib::NoyalibEngine::open(input)?))
}

/// Evaluate `filter` over `input` with the fidelity engine and render the
/// results, slicing original bytes wherever the result is an untouched node.
///
/// Semantics per result:
/// - the identity/root result emits its document slice **verbatim** (so the
///   identity filter reproduces the input byte-for-byte, including
///   multi-document streams),
/// - other path-derived results emit their original bytes,
///   newline-terminated,
/// - computed, absent, and unaddressable results fall back to yqr's regular
///   typed rendering (`null` for absent paths, matching jq),
/// - `raw` keeps its usual meaning: top-level string results print their
///   value without quoting.
///
/// # Errors
///
/// Returns an error when the filter does not parse, the engine rejects the
/// input, or evaluation fails.
pub fn run(filter: &str, input: &str, raw: bool) -> Result<String> {
    let ast = crate::parser::parse(filter)?;
    run_ast(&ast, input, raw)
}

/// Like [`run`], but over an already-compiled read-only [`Ast`], so a caller
/// that has already parsed the filter (the binary's dispatch) does not lex and
/// parse it a second time.
///
/// # Errors
///
/// Returns an error when the engine rejects the input or evaluation fails.
pub fn run_ast(ast: &Ast, input: &str, raw: bool) -> Result<String> {
    let engine = open(input)?;
    let mut out = String::new();

    for doc in 0..engine.doc_count() {
        let value = engine.value(doc)?;
        let results = crate::eval::eval_traced(ast, &value, Some(&Path::root()))?;
        for (value, path) in results {
            // jq's --raw-output prints a top-level string's *value*; the
            // typed view is authoritative for it, span or no span.
            if raw && matches!(value, Value::String(_)) {
                out.push_str(&crate::render(&[value], true)?);
                continue;
            }
            match path {
                Some(p) => match engine.resolve(doc, &p)? {
                    Resolved::Found { bytes, .. } => {
                        out.push_str(bytes);
                        // The root slice is already byte-exact (the cat
                        // case); sub-document slices get the customary
                        // one-result-per-line newline.
                        if !p.is_root() && !bytes.ends_with('\n') {
                            out.push('\n');
                        }
                    }
                    Resolved::Absent => out.push_str(&crate::render(&[Value::Null], raw)?),
                    Resolved::Synthetic | Resolved::Unaddressable(_) => {
                        out.push_str(&crate::render(&[value], raw)?);
                    }
                },
                None => out.push_str(&crate::render(&[value], raw)?),
            }
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_root_and_child() {
        let root = Path::root();
        assert!(root.is_root());
        let child = root.child(PathSeg::Key("a".into()));
        assert!(!child.is_root());
        assert_eq!(
            child.child(PathSeg::Index(2)).segments(),
            &[PathSeg::Key("a".into()), PathSeg::Index(2)]
        );
    }

    #[test]
    fn plain_segments() {
        assert!(PathSeg::Key("normal_key".into()).is_plain());
        assert!(PathSeg::Index(0).is_plain());
        assert!(!PathSeg::Key("dotted.key".into()).is_plain());
        assert!(!PathSeg::Key(String::new()).is_plain());
        assert!(!PathSeg::Key("a[0]".into()).is_plain());
    }

    #[test]
    fn span_slices_bytes() {
        let src = "a: 1\n";
        assert_eq!(Span::new(3, 4).slice(src), "1");
    }

    #[test]
    fn open_accepts_valid_yaml() {
        assert!(open("a: 1\n").is_ok());
    }

    #[test]
    fn open_rejects_invalid_yaml() {
        assert!(open("items: [1, 2").is_err());
    }
}
