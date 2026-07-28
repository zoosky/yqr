//! Fidelity backend over noyalib's lossless CST.
//!
//! noyalib's `cst::Document` keeps each document's source bytes verbatim and
//! maps string paths to byte spans (`span_at`), which is exactly the engine
//! contract: this adapter parses the stream once, records each document's
//! byte offset, lowers noyalib's typed values into yqr's [`Value`] model, and
//! rebases document-relative spans onto the whole input.

// Feature f002 (see specs/features/): backend C of the fidelity seam.

use std::fmt::Write as _;

// This module is named `noyalib`; reach the crate's `Value` through `crate::`
// (the engine crate is addressed as `::noyalib`).
use crate::Value;

use crate::error::{Result, YqrError};
use crate::fidelity::{FidelityEngine, Path, PathSeg, Resolved, Span, Unaddressable};

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

        // Every span downstream is rebased on these offsets, so a document whose
        // slice diverged from the input would silently mis-map every projection.
        let offsets = verify_stream_tiles_input(input, &docs)?;

        // The engine's value model has string-only mapping keys, so distinct
        // YAML keys that share a spelling (`1` and `"1"`) would collapse into
        // one entry — silent data loss. The fork's loader now raises
        // `Error::KeyCollision` for exactly that case (deficiency 2.5), so the
        // `parse_stream` call above already refused such an input loudly; no
        // cross-check against the classic loader is needed here.
        let values: Vec<Value> = docs.iter().map(|d| lower_value(&d.as_value())).collect();

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
            return Ok(Resolved::Unaddressable(Unaddressable::SpecialCharKey(
                offending_key(path),
            )));
        };

        let typed = walk_value(&self.values[doc], path.segments());

        if let Some((start, end)) = self.docs[doc].span_at(&path_str) {
            let span = Span::new(doc_span.start + start, doc_span.start + end);
            if let Some(expected) = typed {
                if let Some(found) = self.verified_found(span, expected) {
                    return Ok(found);
                }
                // The span exists but its bytes do not denote the value the
                // evaluator selected, so slicing them would emit the wrong
                // node. Degrade visibly. The fork closed the common causes at
                // the source (last-wins duplicate keys, keep-chomped block
                // scalars, alias resolve-through), so this is now a genuine
                // safety net for the residual long tail rather than a routine
                // path.
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
    /// Accept a resolved span only when the bytes that will actually be
    /// emitted demonstrably denote the value the evaluator selected: the
    /// emitted slice must re-parse to `expected`. This is the wrong-node
    /// guard — without it a span could silently emit bytes of a different
    /// node than the typed view evaluated. The fork closed the common
    /// mismatch sources upstream (keep-chomped block scalars, alias
    /// resolve-through, last-wins duplicate keys), so this now defends the
    /// residual long tail rather than routine cases.
    ///
    /// If a span's bytes do not re-parse, the extension below is retried: when
    /// the bytes between the line start and the span are pure indentation, the
    /// span is **extended to the line start** so the emitted slice is uniformly
    /// indented, still verbatim source, and verified in exactly the form it is
    /// emitted (a mis-indented slice would otherwise re-nest downstream). The
    /// fork's block-collection fix makes most such spans already line-start
    /// aligned, leaving this as a fallback for the cases it does not cover.
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
        let line_start = self.source[..span.start].rfind('\n').map_or(0, |i| i + 1);
        let prefix = &self.source[line_start..span.start];
        if !prefix.is_empty() && prefix.bytes().all(|b| b == b' ') {
            let extended = Span::new(line_start, span.end);
            let extended_bytes = extended.slice(&self.source);
            if reparses_to(extended_bytes, expected) {
                return Some(Resolved::Found {
                    span: extended,
                    bytes: extended_bytes,
                });
            }
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

/// Verify that concatenating each parsed document's source reproduces `input`
/// byte-for-byte, returning the byte offset of each document.
///
/// Shared by the read engine and the write adapter (`super::write`): both rebase
/// spans / emit slices against these offsets, so a document whose slice diverged
/// from the input would silently mis-map, and both must refuse it identically.
pub(super) fn verify_stream_tiles_input(
    input: &str,
    docs: &[::noyalib::cst::Document],
) -> Result<Vec<usize>> {
    let mut offsets = Vec::with_capacity(docs.len());
    let mut cursor = 0usize;
    for doc in docs {
        // Compare content, not just lengths.
        if !input[cursor..].starts_with(doc.source()) {
            return Err(YqrError::io(format!(
                "fidelity violation: document slice at byte {cursor} does not match the input"
            )));
        }
        offsets.push(cursor);
        cursor += doc.source().len();
    }
    if cursor != input.len() {
        return Err(YqrError::io(format!(
            "fidelity violation: parsed documents cover {cursor} of {} input bytes",
            input.len()
        )));
    }
    Ok(offsets)
}

/// The first non-plain key of `path`, for an "unaddressable" diagnostic (empty
/// when the path has no such key). Shared by the read `resolve` and the write
/// path-string builder so both name the offending key the same way.
pub(super) fn offending_key(path: &Path) -> String {
    path.segments()
        .iter()
        .find_map(|seg| match seg {
            PathSeg::Key(k) if !seg.is_plain() => Some(k.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Render a [`Path`] in noyalib's string-path grammar (`a.b[0].c`), or `None`
/// when a key cannot be expressed in it.
///
/// Shared with the write adapter (`super::write`): the read path resolves a
/// span from this string and the write path targets the same string with a
/// mutator, so both must address a node identically.
pub(super) fn to_noyalib_path(path: &Path) -> Option<String> {
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
///
/// Shared with the structural-delete fallback ([`super::write`]), which walks
/// the same typed model to resolve a delete target's parent and value.
pub(crate) fn walk_value<'v>(value: &'v Value, segs: &[PathSeg]) -> Option<&'v Value> {
    let mut node = value;
    for seg in segs {
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
    fn duplicate_keys_resolve_to_last_occurrence() {
        // noyalib 0.0.13 resolves duplicate keys last-wins in span_at, matching
        // the typed view (the fix folded in from noyalib#143). The guard now
        // verifies the last occurrence's real bytes instead of degrading.
        let e = engine("k: one\nk: two\n");
        let path = Path::root().child(PathSeg::Key("k".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "two"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_collection_keys_resolve_to_last_occurrence() {
        // The last `m`'s block value spans from its first line's indent (the
        // fork's block-collection line-start fix, deficiency 2.4), so the
        // emitted slice is uniformly indented, re-parses to `{a: 2}`, and is
        // emitted verbatim.
        let e = engine("m:\n  a: 1\nm:\n  a: 2\n");
        let path = Path::root().child(PathSeg::Key("m".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "  a: 2"),
            other => panic!("expected Found, got {other:?}"),
        }
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
    fn keep_chomped_block_scalar_keeps_its_trailing_blanks() {
        // The fork's keep-chomped span fix (deficiency 2.3) includes the kept
        // trailing blank lines of `|+` in the span, so the slice re-parses to
        // the full `"kept\n\n\n"` value and is emitted verbatim.
        let e = engine("key: |+\n  kept\n\n\n");
        let path = Path::root().child(PathSeg::Key("key".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "|+\n  kept\n\n\n"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn alias_reference_resolves_through_to_anchor_value() {
        // The fork resolves an alias reference through to the anchor value's
        // span (deficiency 2.6), so `*anc` emits the anchor's original bytes,
        // which re-parse to the same sequence the typed view holds.
        let e = engine("a: &anc [1, 2]\nb: *anc\n");
        let path = Path::root().child(PathSeg::Key("b".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "[1, 2]"),
            other => panic!("expected Found, got {other:?}"),
        }
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
    fn block_mapping_projection_extends_to_uniform_indentation() {
        // A block slice whose first line's indentation lies left of the raw
        // span is extended to the line start, so the EMITTED bytes are
        // uniformly indented, verbatim source, and re-parse to the selected
        // value (a mis-indented slice would silently re-nest downstream).
        let e = engine("config:\n  debug: true\n  level: info\nafter: 1\n");
        let path = Path::root().child(PathSeg::Key("config".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => {
                assert_eq!(bytes, "  debug: true\n  level: info");
                let reparsed = ::noyalib::cst::parse_document(bytes).expect("emitted parses");
                assert_eq!(
                    lower_value(&reparsed.as_value()),
                    e.value(0).unwrap().get_str("config").unwrap().clone(),
                    "emitted bytes must denote the selected value"
                );
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn block_sequence_projection_is_not_misleading() {
        // Regression: '- alpha\n    - beta' (first line dedented) silently
        // re-parses as ["alpha - beta"]; the extended span must prevent it.
        let e = engine("spec:\n  items:\n    - alpha\n    - beta\n");
        let path = Path::root()
            .child(PathSeg::Key("spec".into()))
            .child(PathSeg::Key("items".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => {
                assert_eq!(bytes, "    - alpha\n    - beta");
                let reparsed = ::noyalib::cst::parse_document(bytes).expect("emitted parses");
                let expected = Value::Sequence(vec![
                    Value::String("alpha".into()),
                    Value::String("beta".into()),
                ]);
                assert_eq!(lower_value(&reparsed.as_value()), expected);
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn colliding_keys_are_refused_loudly() {
        // `1` and `"1"` are distinct YAML keys but collide in the engine's
        // string-only key model; the fork's loader raises `KeyCollision`
        // (deficiency 2.5) so `open()` refuses the input instead of silently
        // dropping an entry.
        assert!(NoyalibEngine::open("1: a\n\"1\": b\n").is_err());
    }

    #[test]
    fn non_colliding_numeric_keys_still_load() {
        // A lone numeric key stringifies without losing entries; allowed
        // (documented divergence from the classic pipeline's typed keys).
        let e = engine("8080: service\n");
        assert_eq!(e.doc_count(), 1);
    }

    #[test]
    fn merge_keys_do_not_trip_the_collision_check() {
        let e = engine("defaults: &d\n  timeout: 30\nservice:\n  <<: *d\n  name: web\n");
        assert_eq!(e.doc_count(), 1);
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
        // The lone-CR case exercises the fork's classic-Mac line-break fix
        // (deficiency 2.7): a CR-only stream now scans as two lines and still
        // round-trips byte-for-byte.
        for input in ["\u{feff}a: 1\nb: 2\n", "a: 1\r\nb: 2\r\n", "a: 1\rb: 2\r"] {
            let e = engine(input);
            assert_eq!(e.source(), input);
            match e.resolve(0, &Path::root()).unwrap() {
                Resolved::Found { bytes, .. } => assert_eq!(bytes, input),
                other => panic!("expected Found, got {other:?}"),
            }
        }
    }
}
