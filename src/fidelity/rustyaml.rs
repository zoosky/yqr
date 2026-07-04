//! Fidelity backend over the rust-yaml fork's `RoundTripDocument`.
//!
//! The fork keeps each document's source bytes verbatim and pairs them with a
//! node -> byte-span index built from the same scanner/parser pass that composes
//! the typed value, which is exactly the engine contract. This adapter parses
//! the stream once, records each document's byte offset, lowers the fork's typed
//! values into yqr's [`Value`] model, and rebases document-relative spans onto
//! the whole input.
//!
//! Compared with the CST backend, the fork's model is a closer fit for the
//! seam:
//!
//! - mapping keys keep their full typed value (not string-only), so distinct
//!   keys that share a spelling (`1` and `"1"`) never collide -- no
//!   entry-collision guard is needed;
//! - duplicate keys resolve **last-wins** in both the span index and the typed
//!   value, so a `Found` slice always denotes the value the evaluator selected;
//! - keys are addressed by their resolved scalar text, so special-character
//!   keys (`a.b`) are addressable rather than [`Unaddressable`].

// Feature f003 (see specs/features/): backend A of the fidelity seam.

use rust_yaml::Value;

use crate::error::{Result, YqrError};
use crate::fidelity::{BackendId, FidelityEngine, Path, PathSeg, Resolved, Span};

/// [`FidelityEngine`] implementation backed by `rust_yaml_rt::RoundTripDocument`.
pub(crate) struct RustYamlEngine {
    /// The whole input, byte-for-byte.
    source: String,
    /// One source-preserving round-trip document per logical YAML document.
    docs: Vec<::rust_yaml_rt::RoundTripDocument>,
    /// Byte offset of each document's slice within `source`.
    offsets: Vec<usize>,
    /// Typed views, lowered once at open time from the same parse that owns the
    /// spans (the parse-once contract).
    values: Vec<Value>,
}

impl RustYamlEngine {
    /// Parse `input` into a source-preserving document stream.
    ///
    /// Defensively verifies that the per-document slices reproduce the input
    /// byte-for-byte before trusting any span rebased onto them.
    pub(crate) fn open(input: &str) -> Result<Self> {
        let docs = ::rust_yaml_rt::RoundTripDocument::parse_all(input)
            .map_err(|e| YqrError::io(format!("failed to parse YAML input: {e}")))?;

        let mut offsets = Vec::with_capacity(docs.len());
        let mut cursor = 0usize;
        for doc in &docs {
            // Compare content, not just lengths: every span downstream is
            // rebased on these offsets, so a document whose slice diverged from
            // the input would silently mis-map every projection.
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

        let values: Vec<Value> = docs.iter().map(|d| lower_value(d.value())).collect();

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

    /// Accept a resolved span only when the bytes that will actually be emitted
    /// demonstrably denote the value the evaluator selected: the emitted slice
    /// must re-parse to `expected`. This is the wrong-node guard.
    ///
    /// A nested block collection's span starts at its first key, leaving the
    /// first line's indentation just left of the span while later lines keep
    /// theirs. Emitting that raw slice drops the first line's indentation, so a
    /// conformant downstream parser mis-nests it (`c: 1\n    d: 2` reads `d` as
    /// a sibling of the scalar `c`, or rejects it outright). When the bytes
    /// between the line start and the span are pure indentation, the span is
    /// therefore **extended to the line start** so the emitted slice is
    /// uniformly indented, and that extended form -- the one actually emitted --
    /// is what gets verified. The raw slice is used only for a node that already
    /// begins at its line's content (a scalar after `key: `, a flow collection,
    /// or a node at column 0): there is no dropped indentation to restore.
    ///
    /// Order matters: the raw slice is *not* tried first, because the fork's own
    /// loader is lenient enough to accept some first-line-dedented block slices,
    /// which would wave through a slice that stricter parsers reject.
    fn verified_found(&self, span: Span, expected: &Value) -> Option<Resolved<'_>> {
        let bytes = span.slice(&self.source);
        if bytes.trim().is_empty() {
            // Degenerate spans carry no content; the typed fallback renders the
            // value correctly.
            return None;
        }
        let line_start = self.source[..span.start].rfind('\n').map_or(0, |i| i + 1);
        let prefix = &self.source[line_start..span.start];
        if !prefix.is_empty() && prefix.bytes().all(|b| b == b' ') {
            // Block collection whose span starts after its line's indentation:
            // emit the uniformly-indented, line-start-extended slice, never the
            // mis-indented raw one.
            let extended = Span::new(line_start, span.end);
            let extended_bytes = extended.slice(&self.source);
            return reparses_to(extended_bytes, expected).then_some(Resolved::Found {
                span: extended,
                bytes: extended_bytes,
            });
        }
        // Scalar / flow / column-0 node: the raw slice already denotes it.
        reparses_to(bytes, expected).then_some(Resolved::Found { span, bytes })
    }
}

impl FidelityEngine for RustYamlEngine {
    fn backend_id(&self) -> BackendId {
        BackendId::RustYamlRoundTrip
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

        // The typed view -- from the same parse -- is authoritative for whether
        // the node exists at all. A path the evaluator produced a value for but
        // that is absent from this model (e.g. a string key spelled like a
        // non-string key) is jq `null`, not an error.
        let Some(expected) = walk_value(&self.values[doc], path) else {
            return Ok(Resolved::Absent);
        };

        let segments = to_rt_path(path);
        if let Some(rt_span) = self.docs[doc].span_of(&segments) {
            let span = Span::new(doc_span.start + rt_span.start, doc_span.start + rt_span.end);
            if let Some(found) = self.verified_found(span, expected) {
                return Ok(found);
            }
        }

        // No span (implicit null, merge-key entry, alias-expanded content) or a
        // span whose bytes disagree with the selected value: re-serialize from
        // the typed value, visibly, for this node only.
        Ok(Resolved::Synthetic)
    }
}

/// Whether `fragment` parses as a single YAML document whose lowered value
/// equals `expected`. Uses the fork's own loader so acceptance matches the pass
/// that produced the spans.
fn reparses_to(fragment: &str, expected: &Value) -> bool {
    ::rust_yaml_rt::RoundTripDocument::parse(fragment)
        .map(|d| lower_value(d.value()) == *expected)
        .unwrap_or(false)
}

/// Render a [`Path`] in the fork's `PathSegment` grammar. Keys are matched by
/// their resolved scalar text, so every key -- including special-character keys
/// -- is expressible; the segments borrow from `path`.
fn to_rt_path(path: &Path) -> Vec<::rust_yaml_rt::PathSegment<'_>> {
    path.segments()
        .iter()
        .map(|seg| match seg {
            PathSeg::Key(k) => ::rust_yaml_rt::PathSegment::Key(k.as_str()),
            PathSeg::Index(i) => ::rust_yaml_rt::PathSegment::Index(*i),
        })
        .collect()
}

/// Walk yqr's typed value by path segments (used to tell "exists without bytes"
/// apart from "does not exist").
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

/// Lower a fork value into yqr's evaluation model (`rust_yaml::Value`).
///
/// The fork and the shipped engine are the same YAML library, so the two
/// `Value` enums have identical shape; this is a structural one-to-one map.
/// Mapping keys keep their full typed value (non-string keys are preserved).
/// The typed view is intentionally lossy for formatting; fidelity is never
/// derived from it.
fn lower_value(value: &::rust_yaml_rt::Value) -> Value {
    match value {
        ::rust_yaml_rt::Value::Null => Value::Null,
        ::rust_yaml_rt::Value::Bool(b) => Value::Bool(*b),
        ::rust_yaml_rt::Value::Int(i) => Value::Int(*i),
        ::rust_yaml_rt::Value::Float(f) => Value::Float(*f),
        ::rust_yaml_rt::Value::String(s) => Value::String(s.clone()),
        ::rust_yaml_rt::Value::Sequence(items) => {
            Value::Sequence(items.iter().map(lower_value).collect())
        }
        ::rust_yaml_rt::Value::Mapping(map) => Value::Mapping(
            map.iter()
                .map(|(k, v)| (lower_value(k), lower_value(v)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(input: &str) -> RustYamlEngine {
        RustYamlEngine::open(input).expect("valid input")
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
    fn special_char_key_is_addressable() {
        // Unlike the CST backend, the fork addresses keys by scalar text, so a
        // dotted key resolves to its original bytes rather than degrading.
        let e = engine("'a.b': 'quoted value'\n");
        let path = Path::root().child(PathSeg::Key("a.b".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "'quoted value'"),
            other => panic!("expected Found, got {other:?}"),
        }
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
        // The span index and the typed value both keep the last occurrence, so
        // the slice legitimately denotes the selected value.
        let e = engine("k: one\nk: two\n");
        let path = Path::root().child(PathSeg::Key("k".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "two"),
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
    fn merge_key_entry_degrades_to_typed_expansion() {
        // `.service.timeout` exists only through the merge key: present in the
        // typed value, but with no bytes inside `service`.
        let e = engine("defaults: &d\n  timeout: 30\nservice:\n  <<: *d\n  name: web\n");
        let path = Path::root()
            .child(PathSeg::Key("service".into()))
            .child(PathSeg::Key("timeout".into()));
        assert!(matches!(e.resolve(0, &path).unwrap(), Resolved::Synthetic));
    }

    #[test]
    fn alias_reference_degrades_to_typed_expansion() {
        let e = engine("a: &anc [1, 2]\nb: *anc\n");
        let path = Path::root().child(PathSeg::Key("b".into()));
        assert!(matches!(e.resolve(0, &path).unwrap(), Resolved::Synthetic));
    }

    #[test]
    fn anchored_scalar_projects_its_value_without_the_anchor_property() {
        // The fork indexes the scalar token (`1`) separately from its `&x`
        // anchor property, so a projection emits the value bytes. The anchor is
        // a label, not part of the value, and the emitted bytes still denote
        // the selected value (a documented divergence from the CST backend,
        // which keeps the property bytes in the slice).
        let e = engine("a: &x 1\n");
        let path = Path::root().child(PathSeg::Key("a".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "1"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn non_string_keys_are_preserved_not_stringified() {
        // The fork keeps full typed keys, so a boolean key stays Bool(true);
        // the string key `.true` therefore does not resolve (matches jq).
        let e = engine("true: yes\n");
        let path = Path::root().child(PathSeg::Key("true".into()));
        assert!(matches!(e.resolve(0, &path).unwrap(), Resolved::Absent));
        let Value::Mapping(map) = &e.value(0).unwrap() else {
            panic!("expected mapping");
        };
        assert!(map.contains_key(&Value::Bool(true)));
    }

    #[test]
    fn distinct_stringlike_keys_do_not_collide() {
        // `1` and `"1"` are distinct keys the CST backend refuses; the fork
        // keeps both because keys carry their full typed value.
        let e = engine("1: a\n\"1\": b\n");
        let Value::Mapping(map) = &e.value(0).unwrap() else {
            panic!("expected mapping");
        };
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn block_mapping_projection_extends_to_uniform_indentation() {
        let e = engine("config:\n  debug: true\n  level: info\nafter: 1\n");
        let path = Path::root().child(PathSeg::Key("config".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => {
                assert_eq!(bytes, "  debug: true\n  level: info");
                let reparsed =
                    ::rust_yaml_rt::RoundTripDocument::parse(bytes).expect("emitted parses");
                assert_eq!(
                    lower_value(reparsed.value()),
                    *e.value(0).unwrap().get_str("config").unwrap(),
                    "emitted bytes must denote the selected value"
                );
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn block_sequence_projection_is_not_misleading() {
        let e = engine("spec:\n  items:\n    - alpha\n    - beta\n");
        let path = Path::root()
            .child(PathSeg::Key("spec".into()))
            .child(PathSeg::Key("items".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => {
                assert_eq!(bytes, "    - alpha\n    - beta");
                let reparsed =
                    ::rust_yaml_rt::RoundTripDocument::parse(bytes).expect("emitted parses");
                let expected = Value::Sequence(vec![
                    Value::String("alpha".into()),
                    Value::String("beta".into()),
                ]);
                assert_eq!(lower_value(reparsed.value()), expected);
            }
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

    #[test]
    fn deeply_nested_block_mapping_projection_is_uniformly_indented() {
        // Regression (adversarial review): the raw span for `.a.b` starts at the
        // first key `c` (column 4) while sibling `d` keeps its 4-space indent, so
        // the un-extended slice "c: 1\n    d: 2" drops the first line's indent and
        // mis-nests under a conformant parser (the fork's own lenient loader
        // would accept it, so the raw slice must never be tried first). The guard
        // must extend to the line start and emit "    c: 1\n    d: 2".
        let e = engine("a:\n  b:\n    c: 1\n    d: 2\n");
        let path = Path::root()
            .child(PathSeg::Key("a".into()))
            .child(PathSeg::Key("b".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => {
                assert_eq!(bytes, "    c: 1\n    d: 2");
                for line in bytes.lines() {
                    assert!(
                        line.starts_with("    "),
                        "line not uniformly indented: {line:?}"
                    );
                }
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn int_first_two_space_block_mapping_extends_to_line_start() {
        // With an integer first value the fork's lenient loader accepts the
        // dedented "a: 1\n  b: 2" that a stricter parser rejects; the guard must
        // still extend to the uniformly-indented "  a: 1\n  b: 2".
        let e = engine("m:\n  a: 1\n  b: 2\n");
        let path = Path::root().child(PathSeg::Key("m".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "  a: 1\n  b: 2"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn anchored_block_mapping_projection_is_uniformly_indented() {
        // An anchored mapping's span starts at its first key; the projection must
        // restore the first line's indentation like any other block collection.
        let e = engine("a: &x\n  p: 1\n  q: 2\nb: *x\n");
        let path = Path::root().child(PathSeg::Key("a".into()));
        match e.resolve(0, &path).unwrap() {
            Resolved::Found { bytes, .. } => assert_eq!(bytes, "  p: 1\n  q: 2"),
            other => panic!("expected Found, got {other:?}"),
        }
    }
}
