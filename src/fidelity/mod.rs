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
//! Backends implement the engine over different YAML libraries; they are
//! selected at runtime via [`BackendId`] and [`open`]. Backends may be
//! feature-gated so the default build stays dependency-minimal.

// Feature f002 (see specs/features/): fidelity read floor — seam + driver.

use rust_yaml::Value;

use crate::error::Result;

#[cfg(feature = "backend-noyalib")]
mod noyalib;

#[cfg(feature = "backend-rust-yaml")]
mod rustyaml;

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
            PathSeg::Key(k) => !k.is_empty() && !k.contains(['.', '[', ']', '*']),
        }
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

/// Stable identifier of a fidelity backend, used by [`open`] and in
/// diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendId {
    /// noyalib's lossless CST (feature `backend-noyalib`).
    NoyalibCst,
    /// The rust-yaml fork's source-preserving `RoundTripDocument` (feature
    /// `backend-rust-yaml`).
    RustYamlRoundTrip,
}

impl BackendId {
    /// Every backend yqr knows about, whether or not it is compiled in.
    pub const ALL: &'static [BackendId] = &[BackendId::NoyalibCst, BackendId::RustYamlRoundTrip];

    /// The name used on the command line and in messages.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            BackendId::NoyalibCst => "noyalib",
            BackendId::RustYamlRoundTrip => "rust-yaml",
        }
    }

    /// Look up a backend by its command-line name. This is the single place
    /// engine names are interpreted, so the CLI, error messages, and the
    /// [`open`] dispatch cannot drift apart.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|b| b.as_str() == name)
    }

    /// Comma-separated list of all engine names, for error messages.
    #[must_use]
    pub fn known_names() -> String {
        Self::ALL
            .iter()
            .map(|b| b.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A source-preserving view over one parsed YAML input (read path).
///
/// Implementations pair the verbatim input bytes with a structural span
/// index derived from the same parse as the typed value, so a path that is
/// valid against [`value`](Self::value) never resolves to the wrong bytes —
/// at worst it resolves to no bytes and the caller falls back visibly.
///
/// The trait is object-safe; yqr selects a backend at startup and drives it
/// through `Box<dyn FidelityEngine>`.
pub trait FidelityEngine {
    /// Which backend this is.
    fn backend_id(&self) -> BackendId;

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

/// Parse `input` with the chosen backend, keeping its bytes verbatim.
///
/// Backends may be feature-gated; requesting one that is not compiled into
/// this build is an error that names the missing feature.
///
/// # Errors
///
/// Returns an error when the backend is unavailable in this build or the
/// input is not valid YAML for that backend.
pub fn open(backend: BackendId, input: &str) -> Result<Box<dyn FidelityEngine>> {
    match backend {
        BackendId::NoyalibCst => {
            #[cfg(feature = "backend-noyalib")]
            {
                Ok(Box::new(noyalib::NoyalibEngine::open(input)?))
            }
            #[cfg(not(feature = "backend-noyalib"))]
            {
                let _ = input;
                Err(crate::error::YqrError::io(
                    "engine 'noyalib' is not available in this build \
                     (rebuild with: cargo build --features backend-noyalib)",
                ))
            }
        }
        BackendId::RustYamlRoundTrip => {
            #[cfg(feature = "backend-rust-yaml")]
            {
                Ok(Box::new(rustyaml::RustYamlEngine::open(input)?))
            }
            #[cfg(not(feature = "backend-rust-yaml"))]
            {
                let _ = input;
                Err(crate::error::YqrError::io(
                    "engine 'rust-yaml' is not available in this build \
                     (rebuild with: cargo build --features backend-rust-yaml)",
                ))
            }
        }
    }
}

/// Evaluate `filter` over `input` with a fidelity backend and render the
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
/// Returns an error when the filter does not parse, the backend rejects the
/// input, or evaluation fails.
pub fn run(backend: BackendId, filter: &str, input: &str, raw: bool) -> Result<String> {
    let ast = crate::parser::parse(filter)?;
    let engine = open(backend, input)?;
    let mut out = String::new();

    for doc in 0..engine.doc_count() {
        let value = engine.value(doc)?;
        let results = crate::eval::eval_traced(&ast, &value, Some(&Path::root()))?;
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

    #[cfg(not(feature = "backend-noyalib"))]
    #[test]
    fn open_reports_missing_backend() {
        match open(BackendId::NoyalibCst, "a: 1\n") {
            Err(err) => assert!(err.to_string().contains("backend-noyalib")),
            Ok(_) => panic!("expected the backend to be unavailable"),
        }
    }

    #[cfg(not(feature = "backend-rust-yaml"))]
    #[test]
    fn open_reports_missing_rust_yaml_backend() {
        match open(BackendId::RustYamlRoundTrip, "a: 1\n") {
            Err(err) => assert!(err.to_string().contains("backend-rust-yaml")),
            Ok(_) => panic!("expected the backend to be unavailable"),
        }
    }
}
