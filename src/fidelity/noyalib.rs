//! Fidelity backend over noyalib's lossless CST.
//!
//! noyalib's `cst::Document` keeps each document's source bytes verbatim and
//! maps string paths to byte spans (`span_at`), which is exactly the engine
//! contract: this adapter parses the stream once, records each document's
//! byte offset, lowers noyalib's typed values into yqr's [`Value`] model, and
//! rebases document-relative spans onto the whole input.

// Feature f002 (see specs/features/): backend C of the fidelity seam.

use std::fmt::Write as _;

use rust_yaml::Value;

use crate::error::{Result, YqrError};
use crate::fidelity::{BackendId, FidelityEngine, Path, PathSeg, Resolved, Span, Unaddressable};

/// [`FidelityEngine`] implementation backed by `noyalib::cst`.
pub(crate) struct NoyalibEngine {
    /// The whole input, byte-for-byte.
    source: String,
    /// One lossless CST document per logical YAML document.
    docs: Vec<::noyalib::cst::Document>,
    /// Byte offset of each document's slice within `source`.
    offsets: Vec<usize>,
    /// Typed views, lowered once at open time from the same parse that owns
    /// the spans (the parse-once contract).
    values: Vec<Value>,
}

impl NoyalibEngine {
    /// Parse `input` into a lossless document stream.
    ///
    /// Defensively verifies that the per-document slices reproduce the input
    /// byte-for-byte before trusting any span from them.
    pub(crate) fn open(input: &str) -> Result<Self> {
        let docs = ::noyalib::cst::parse_stream(input)
            .map_err(|e| YqrError::io(format!("failed to parse YAML input: {e}")))?;

        let mut offsets = Vec::with_capacity(docs.len());
        let mut cursor = 0usize;
        for doc in &docs {
            offsets.push(cursor);
            cursor += doc.source().len();
        }
        if cursor != input.len() {
            return Err(YqrError::io(format!(
                "fidelity violation: parsed documents cover {cursor} of {} input bytes",
                input.len()
            )));
        }

        let values = docs.iter().map(|d| lower_value(&d.as_value())).collect();

        Ok(Self {
            source: input.to_string(),
            docs,
            offsets,
            values,
        })
    }

    /// Bounds-checked document accessor shared by the trait methods.
    fn check_doc(&self, doc: usize) -> Result<()> {
        if doc < self.docs.len() {
            Ok(())
        } else {
            Err(YqrError::eval(format!(
                "document index {doc} out of range ({} documents)",
                self.docs.len()
            )))
        }
    }
}

impl FidelityEngine for NoyalibEngine {
    fn backend_id(&self) -> BackendId {
        BackendId::NoyalibCst
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn doc_count(&self) -> usize {
        self.docs.len()
    }

    fn doc_span(&self, doc: usize) -> Option<Span> {
        let start = *self.offsets.get(doc)?;
        Some(Span::new(start, start + self.docs[doc].source().len()))
    }

    fn value(&self, doc: usize) -> Result<Value> {
        self.check_doc(doc)?;
        Ok(self.values[doc].clone())
    }

    fn resolve(&self, doc: usize, path: &Path) -> Result<Resolved<'_>> {
        self.check_doc(doc)?;
        let doc_span = self.doc_span(doc).expect("checked above");

        if path.is_root() {
            return Ok(Resolved::Found {
                span: doc_span,
                bytes: doc_span.slice(&self.source),
            });
        }

        // noyalib addresses nodes through an unescaped string-path grammar;
        // a key it cannot express must fail loudly, never resolve wrongly.
        let Some(path_str) = to_noyalib_path(path) else {
            let offending = path
                .segments()
                .iter()
                .find_map(|seg| match seg {
                    PathSeg::Key(k) if !seg.is_plain() => Some(k.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            return Ok(Resolved::Unaddressable(Unaddressable::SpecialCharKey(
                offending,
            )));
        };

        let typed = walk_value(&self.values[doc], path);

        if let Some((start, end)) = self.docs[doc].span_at(&path_str) {
            let span = Span::new(doc_span.start + start, doc_span.start + end);
            if let Some(expected) = typed {
                if let Some(found) = self.verified_found(span, expected) {
                    return Ok(found);
                }
                // The slice disagrees with the value the evaluator selected
                // (duplicate keys resolve first-wins in the span layer but
                // last-wins in the typed view; implicit nulls yield indicator
                // bytes; keep-chomped block scalars lose kept blank lines;
                // aliases slice as dangling `*name`). Degrade visibly.
                return Ok(Resolved::Synthetic);
            }
        }

        // No span: the node either has no bytes of its own (implicit null,
        // merge-key entry, alias interior) or does not exist at all. The
        // typed value — from the same parse — disambiguates.
        if typed.is_some() {
            Ok(Resolved::Synthetic)
        } else {
            Ok(Resolved::Absent)
        }
    }
}

impl NoyalibEngine {
    /// Accept a resolved span only when its bytes demonstrably denote the
    /// value the evaluator selected: the slice (tried verbatim, then with its
    /// original leading columns restored, so block slices re-indent) must
    /// re-parse to `expected`. This is the wrong-node guard — without it a
    /// span could silently emit bytes of a different node than the typed view
    /// evaluated (e.g. under duplicate keys).
    fn verified_found(&self, span: Span, expected: &Value) -> Option<Resolved<'_>> {
        let bytes = span.slice(&self.source);
        if bytes.trim().is_empty() {
            // Degenerate spans (an implicit null's `:` neighborhood) carry no
            // content; the typed fallback renders `null` correctly.
            return None;
        }
        if reparses_to(bytes, expected) {
            return Some(Resolved::Found { span, bytes });
        }
        // A block-structured slice loses its first line's indentation (it
        // lives to the left of the span); restore the original leading
        // columns for the verification parse only.
        let line_start = self.source[..span.start].rfind('\n').map_or(0, |i| i + 1);
        let padded = format!("{}{bytes}", " ".repeat(span.start - line_start));
        if reparses_to(&padded, expected) {
            return Some(Resolved::Found { span, bytes });
        }
        None
    }
}

/// Whether `fragment` parses as a single YAML document whose lowered value
/// equals `expected`.
fn reparses_to(fragment: &str, expected: &Value) -> bool {
    ::noyalib::cst::parse_document(fragment)
        .map(|d| lower_value(&d.as_value()) == *expected)
        .unwrap_or(false)
}

/// Render a [`Path`] in noyalib's string-path grammar (`a.b[0].c`), or `None`
/// when a key cannot be expressed in it.
fn to_noyalib_path(path: &Path) -> Option<String> {
    let mut out = String::new();
    for seg in path.segments() {
        match seg {
            PathSeg::Key(k) => {
                if !seg.is_plain() {
                    return None;
                }
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(k);
            }
            PathSeg::Index(i) => {
                write!(out, "[{i}]").expect("writing to String cannot fail");
            }
        }
    }
    Some(out)
}

/// Walk yqr's typed value by path segments (used to tell "exists without
/// bytes" apart from "does not exist").
fn walk_value<'v>(value: &'v Value, path: &Path) -> Option<&'v Value> {
    let mut node = value;
    for seg in path.segments() {
        node = match (seg, node) {
            (PathSeg::Key(k), Value::Mapping(map)) => map.get(&Value::String(k.clone()))?,
            (PathSeg::Index(i), Value::Sequence(items)) => items.get(*i)?,
            _ => return None,
        };
    }
    Some(node)
}

/// Lower a noyalib value into yqr's evaluation model (`rust_yaml::Value`).
///
/// The typed view is intentionally lossy: tags are dropped to their inner
/// value, and numbers use noyalib's parsing (an unsigned integer above
/// `i64::MAX` degrades to a float). Fidelity is never derived from this view.
fn lower_value(value: &::noyalib::Value) -> Value {
    match value {
        ::noyalib::Value::Null => Value::Null,
        ::noyalib::Value::Bool(b) => Value::Bool(*b),
        ::noyalib::Value::Number(n) => n
            .as_i64()
            .map_or_else(|| Value::Float(n.as_f64()), Value::Int),
        ::noyalib::Value::String(s) => Value::String(s.clone()),
        ::noyalib::Value::Sequence(items) => {
            Value::Sequence(items.iter().map(lower_value).collect())
        }
        ::noyalib::Value::Mapping(map) => Value::Mapping(
            map.iter()
                .map(|(k, v)| (Value::String(k.clone()), lower_value(v)))
                .collect(),
        ),
        ::noyalib::Value::Tagged(tagged) => lower_value(tagged.value()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(input: &str) -> NoyalibEngine {
        NoyalibEngine::open(input).expect("valid input")
    }

    #[test]
    fn source_is_verbatim() {
        let input = "# c\na: 1\n\nb: 'x'\n";
        assert_eq!(engine(input).source(), input);
    }

    #[test]
    fn multi_doc_spans_tile_the_source() {
        let input = "---\na: 1\n---\nb: 2\n";
        let e = engine(input);
        assert_eq!(e.doc_count(), 2);
        let joined: String = (0..e.doc_count())
            .map(|i| e.doc_span(i).unwrap().slice(input).to_string())
            .collect();
        assert_eq!(joined, input);
    }

    #[test]
    fn resolve_root_is_whole_document() {
        let input = "# header\na: 1\n";
        let e = engine(input);
        match e.resolve(0, &Path::root()).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, input),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn resolve_scalar_keeps_original_spelling() {
        let e = engine("zip: 007\nquoted: 'hi'\n");
        let path = Path::root().child(PathSeg::Key("zip".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "007"),
            other => panic!("expected Found, got {other:?}"),
        }
        let quoted = Path::root().child(PathSeg::Key("quoted".into()));
        match e.resolve(0, &quoted).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "'hi'"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn resolve_missing_is_absent() {
        let e = engine("a: 1\n");
        let path = Path::root().child(PathSeg::Key("nope".into()));
        assert!(matches!(e.resolve(0, &path).unwrap(), Resolved::Absent));
    }

    #[test]
    fn resolve_special_char_key_is_unaddressable() {
        let e = engine("'a.b': 1\n");
        let path = Path::root().child(PathSeg::Key("a.b".into()));
        assert!(matches!(
            e.resolve(0, &path).unwrap(),
            Resolved::Unaddressable(Unaddressable::SpecialCharKey(k)) if k == "a.b"
        ));
    }

    #[test]
    fn top_level_sequence_index_resolves() {
        let e = engine("- alpha\n- beta\n");
        let path = Path::root().child(PathSeg::Index(1));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "beta"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn lowering_preserves_types_and_order() {
        let e = engine("z: 1\na: 1.5\nflag: true\nlist: [1, x]\n");
        let v = e.value(0).unwrap();
        let Value::Mapping(map) = &v else {
            panic!("expected mapping");
        };
        let keys: Vec<_> = map.keys().cloned().collect();
        assert_eq!(
            keys,
            vec![
                Value::String("z".into()),
                Value::String("a".into()),
                Value::String("flag".into()),
                Value::String("list".into()),
            ]
        );
        assert_eq!(map.get(&Value::String("z".into())), Some(&Value::Int(1)));
        assert_eq!(
            map.get(&Value::String("a".into())),
            Some(&Value::Float(1.5))
        );
    }

    #[test]
    fn duplicate_keys_never_yield_first_occurrence_bytes() {
        // span_at resolves duplicates first-wins while the typed view is
        // last-wins; the verification guard must refuse the stale slice.
        let e = engine("k: one\nk: two\n");
        let path = Path::root().child(PathSeg::Key("k".into()));
        assert!(
            matches!(e.resolve(0, &path).unwrap(), Resolved::Synthetic),
            "duplicate-key slice must degrade, never emit the wrong node"
        );
    }

    #[test]
    fn duplicate_collection_keys_degrade_too() {
        let e = engine("m:\n  a: 1\nm:\n  a: 2\n");
        let path = Path::root().child(PathSeg::Key("m".into()));
        assert!(matches!(e.resolve(0, &path).unwrap(), Resolved::Synthetic));
    }

    #[test]
    fn implicit_null_is_synthetic_not_indicator_bytes() {
        let e = engine("c:\nother: 1\n");
        let path = Path::root().child(PathSeg::Key("c".into()));
        assert!(
            matches!(e.resolve(0, &path).unwrap(), Resolved::Synthetic),
            "an implicit null must not slice the ':' indicator"
        );
    }

    #[test]
    fn keep_chomped_block_scalar_degrades_rather_than_losing_blanks() {
        // noyalib's span excludes the kept trailing blank lines of `|+`, so
        // the slice denotes a DIFFERENT value; the guard must catch it.
        let e = engine("key: |+\n  kept\n\n\n");
        let path = Path::root().child(PathSeg::Key("key".into()));
        assert!(matches!(e.resolve(0, &path).unwrap(), Resolved::Synthetic));
    }

    #[test]
    fn alias_reference_degrades_to_typed_expansion() {
        // A dangling `*name` slice is not the node's value; per the seam
        // contract alias-expanded content re-serializes from the typed view.
        let e = engine("a: &anc [1, 2]\nb: *anc\n");
        let path = Path::root().child(PathSeg::Key("b".into()));
        assert!(matches!(e.resolve(0, &path).unwrap(), Resolved::Synthetic));
    }

    #[test]
    fn anchored_scalar_keeps_its_property_bytes() {
        // `&x 1` re-parses to the same value, so the anchor definition site
        // legitimately keeps its original bytes.
        let e = engine("a: &x 1\n");
        let path = Path::root().child(PathSeg::Key("a".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "&x 1"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn block_mapping_projection_survives_verification() {
        // The padded re-parse must accept block slices whose first line's
        // indentation lives outside the span.
        let e = engine("config:\n  debug: true\n  level: info\nafter: 1\n");
        let path = Path::root().child(PathSeg::Key("config".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => {
                assert!(bytes.starts_with("debug: true"));
                assert!(bytes.contains("level: info"));
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn multiline_plain_scalar_survives_verification() {
        let e = engine("k: one\n  two\nz: 3\n");
        let path = Path::root().child(PathSeg::Key("k".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "one\n  two"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn bom_and_crlf_inputs_stay_byte_exact() {
        for input in ["\u{feff}a: 1\nb: 2\n", "a: 1\r\nb: 2\r\n"] {
            let e = engine(input);
            assert_eq!(e.source(), input);
            match e.resolve(0, &Path::root()).unwrap() {
                Resolved::Found { bytes, .. } => assert_eq!(bytes, input),
                other => panic!("expected Found, got {other:?}"),
            }
        }
    }
}
