//! Creating an explicit entry that shadows a merged-in key.
//!
//! A mapping that inherits `k` through a `<<` merge has no `k` entry of
//! its own, so `.c.k = 9` cannot be an assignment to existing bytes.
//! What it can be — and what every YAML reader agrees it means — is a
//! new explicit entry in `c`, which shadows the inherited value for
//! this mapping alone while the definition and every other inheritor
//! keep theirs. The path is the choice of edit: `.c.k = 9` changes
//! exactly `c.k`, where assigning at the definition changes every
//! mapping that merges it.
//!
//! Two routes, by what the engine can do:
//!
//! - A mapping with at least one entry of its own takes the engine's
//!   typed insertion, which places the new entry where every other new
//!   key goes and owns its spelling.
//! - A merge-only mapping is refused by that insertion (no own entry to
//!   anchor against) and gets yqr's own splice: one placeholder line
//!   after the mapping's last line, committed only when the re-parsed
//!   document is the original with exactly that shadow added. The real
//!   value then goes through the ordinary assignment, so the engine —
//!   not yqr — spells it, the same division of labor the
//!   scalar-over-block collapse uses. The same route serves a brand-new
//!   key (`insert_key` lands here too) and a mapping inside a value an
//!   alias shares, where the proof accepts the insertion reflected at
//!   the alias sites — the anchor rule, mirroring structural delete.
//!
//! A parent whose value *is* an alias (`c: *m`) stays refused: an
//! explicit entry cannot exist in it without rewriting the alias into a
//! block, which is a restructuring the user must spell out.

// Feature f025: override a merged-in key by creating an explicit entry.

use super::seam::FidelityWriter;
use super::{NoyalibWriter, insertable, rebased, to_noyalib_path};
use crate::Value;
use crate::error::{Result, YqrError};
use crate::fidelity::noyalib::walk_value;
use crate::fidelity::{Path, PathSeg};

impl NoyalibWriter {
    /// Create the explicit entry `key` in the mapping holding `path`,
    /// shadowing the value a `<<` merge gave it.
    ///
    /// # Errors
    ///
    /// Errors when the mapping's bytes cannot be located, the entry
    /// cannot be spelled on one line, or the splice would change
    /// anything beyond adding the shadow.
    pub(super) fn insert_override(
        &mut self,
        doc: usize,
        path: &Path,
        path_str: &str,
        key: &str,
        value: &Value,
    ) -> Result<()> {
        let parent = rebased(path, None);
        let parent_str = to_noyalib_path(&parent);
        let ny = insertable(value);
        let before = self.integrity(doc)?;
        match self.doc_mut(doc)?.insert_entry_value(&parent_str, key, &ny) {
            Ok(()) => self.check_integrity(doc, before, path_str),
            // A merge-only mapping has no entry of its own for the
            // engine's insertion to anchor against; the splice below is
            // yqr's answer, the same fork structural delete took.
            // Two refusals route to yqr's own splice. A merge-only
            // mapping has no entry for the engine's insertion to anchor
            // against — the fork structural delete took. A mapping inside
            // a value an alias shares is refused by the engine wholesale;
            // the anchor rule (`yqr-b026`, `yqr-f038`) says the edit lands
            // at the definition and every alias site shows it, and the
            // splice's proof accepts exactly that reflection.
            Err(e) if is_merge_only_refusal(&e) || super::anchor::is_shared_value_refusal(&e) => {
                self.splice_shadow(doc, path, &parent_str, path_str, key, value)?;
                // The entry now owns bytes, so the ordinary assignment
                // applies and the engine spells the value.
                self.set_value(doc, path, value)
            }
            Err(e) => Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: {e}"
            ))),
        }
    }

    /// Splice the placeholder entry `key: null` (or `{}` / `[]` for a
    /// collection value, so the follow-up write replaces like with
    /// like) after the merge-only mapping's last line.
    fn splice_shadow(
        &mut self,
        doc: usize,
        path: &Path,
        parent_str: &str,
        path_str: &str,
        key: &str,
        value: &Value,
    ) -> Result<()> {
        let placeholder = match value {
            Value::Mapping(_) => Value::Mapping(crate::value::Mapping::new()),
            Value::Sequence(_) => Value::Sequence(Vec::new()),
            _ => Value::Null,
        };
        // The engine's emitter spells the entry, so a key that needs
        // quoting gets it; yqr only prepends the indentation.
        let mut entry = crate::value::Mapping::new();
        entry.insert(Value::String(key.to_string()), placeholder.clone());
        let spelled = ::noyalib::to_string_value(&insertable(&Value::Mapping(entry)))
            .map_err(|e| YqrError::eval(format!("cannot assign at {path_str:?}: {e}")))?;
        let entry_line = spelled.trim_end_matches(['\r', '\n']);
        if entry_line.contains('\n') {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: the key does not spell on one line, so an \
                 override entry cannot be written into a merge-only mapping"
            )));
        }

        let original = self.value(doc)?;

        let (at, fragment, new_source) = {
            let d = self.doc_ref(doc)?;
            let src = d.source();
            let (span_start, span_end) = d.span_at(parent_str).ok_or_else(|| {
                YqrError::eval(format!(
                    "cannot assign at {path_str:?}: the mapping's bytes cannot be located"
                ))
            })?;
            // The shadow goes below the mapping's last line, at the
            // column of its first content byte — `- ` item markers
            // consumed, so a merge-only mapping that is a sequence item
            // (`- <<: *m`) continues at the column its keys would. A
            // mapping that does not open its line past markers is
            // flow-shaped (`c: {<<: *m}`); a line splice cannot express
            // it, so it is refused.
            let line_start = src[..span_start].rfind(['\n', '\r']).map_or(0, |n| n + 1);
            let bytes = src.as_bytes();
            let mut content = line_start;
            loop {
                match bytes.get(content) {
                    Some(b' ') => content += 1,
                    Some(b'-') if matches!(bytes.get(content + 1), Some(b' ')) => content += 2,
                    _ => break,
                }
            }
            if span_start > content {
                return Err(YqrError::eval(format!(
                    "cannot assign at {path_str:?}: the mapping's layout does not take a \
                     spliced entry line"
                )));
            }
            let indent = " ".repeat(content - line_start);
            let indent = indent.as_str();
            let value_line_end = src[span_end..]
                .find(['\n', '\r'])
                .map_or(src.len(), |n| span_end + n);
            let nl = if src[value_line_end..].starts_with("\r\n") {
                "\r\n"
            } else if src[value_line_end..].starts_with('\n') {
                "\n"
            } else if src[value_line_end..].starts_with('\r') {
                "\r"
            } else {
                ""
            };
            let (at, fragment) = if nl.is_empty() {
                // The mapping's last line ends the file without a
                // terminator; the shadow brings its own line break — the
                // nearest preceding line's, so a mixed-ending file is not
                // made worse at the edit site — and the file still ends
                // without one.
                let own = match src.rfind(['\n', '\r']) {
                    Some(p) if bytes[p] == b'\n' && p > 0 && bytes[p - 1] == b'\r' => "\r\n",
                    Some(p) if bytes[p] == b'\n' => "\n",
                    Some(_) => "\r",
                    None => "\n",
                };
                (src.len(), format!("{own}{indent}{entry_line}"))
            } else {
                let at = value_line_end + nl.len();
                (at, format!("{indent}{entry_line}{nl}"))
            };
            let mut new_source = String::with_capacity(src.len() + fragment.len());
            new_source.push_str(&src[..at]);
            new_source.push_str(&fragment);
            new_source.push_str(&src[at..]);
            (at, fragment, new_source)
        };

        // Committed only when the re-parse is the original document with
        // exactly the shadow added — the same proof-before-commit the
        // structural delete runs.
        let candidate =
            ::noyalib::cst::parse_document_with_config(&new_source, &crate::fidelity::cst_config())
                .map_err(|e| {
                    YqrError::eval(format!(
                        "cannot assign at {path_str:?}: the override does not re-parse ({e})"
                    ))
                })?;
        let got = Value::from(&*candidate.as_value());
        let (last, parents) = path
            .segments()
            .split_last()
            .expect("insert_override rejected the root");
        debug_assert!(matches!(last, PathSeg::Key(_)));
        let key_value = Value::String(key.to_string());
        if !is_the_shadow(&original, &got, parents, &key_value, &placeholder) {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: the edit would change the document structure \
                 and was refused"
            )));
        }
        self.doc_mut(doc)?
            .replace_span(at, at, &fragment)
            .map_err(|e| YqrError::eval(format!("cannot assign at {path_str:?}: {e}")))
    }
}

/// Proof that `got` is `original` with exactly the shadow added.
///
/// At the parent path, the mapping must carry `key` holding the
/// placeholder, with every other entry unchanged **in order** — the
/// shadow key's own position is the engine's (merge expansion lists own
/// entries before merged ones in the typed view, so it is not yqr's to
/// predict). Anywhere else, a divergent subtree is accepted only as a
/// copy of some prefix of the parent path showing the same insertion —
/// an alias site of an anchor covering the parent, the anchor rule's
/// reflection and the insertion mirror of the `f038` deletion rule. The
/// path anchoring is what refuses a shadow at the wrong level: a line
/// landing one block out changes a sibling's entry count, which no arm
/// here excuses.
fn is_the_shadow(
    original: &Value,
    got: &Value,
    parents: &[PathSeg],
    key: &Value,
    placeholder: &Value,
) -> bool {
    let mut prefixes = Vec::with_capacity(parents.len() + 1);
    for depth in 0..=parents.len() {
        match walk_value(original, &parents[..depth]) {
            Some(value) => prefixes.push((value, &parents[depth..])),
            None => return false,
        }
    }
    let ctx = ShadowCtx {
        prefixes,
        key,
        placeholder,
    };
    along(original, got, parents, &ctx)
}

/// What the proof walks with: each prefix of the parent path paired with
/// the suffix leading from it to the parent, and the shadow itself.
struct ShadowCtx<'a> {
    prefixes: Vec<(&'a Value, &'a [PathSeg])>,
    key: &'a Value,
    placeholder: &'a Value,
}

/// `got` equals `expected` except along `rel`, at whose end the shadow
/// sits; every sibling off the path satisfies [`tree_ok`].
fn along(expected: &Value, got: &Value, rel: &[PathSeg], ctx: &ShadowCtx<'_>) -> bool {
    let Some((seg, rest)) = rel.split_first() else {
        return shadowed(expected, got, ctx);
    };
    match (seg, expected, got) {
        (PathSeg::Key(k), Value::Mapping(a), Value::Mapping(b)) => {
            let k = Value::String(k.clone());
            a.len() == b.len()
                && a.iter().zip(b.iter()).all(|((ka, va), (kb, vb))| {
                    ka == kb
                        && if *ka == k {
                            along(va, vb, rest, ctx)
                        } else {
                            tree_ok(va, vb, ctx)
                        }
                })
        }
        (PathSeg::Index(i), Value::Sequence(a), Value::Sequence(b)) => {
            a.len() == b.len()
                && a.iter().zip(b.iter()).enumerate().all(|(j, (va, vb))| {
                    if j == *i {
                        along(va, vb, rest, ctx)
                    } else {
                        tree_ok(va, vb, ctx)
                    }
                })
        }
        _ => false,
    }
}

/// A subtree off the parent path: unchanged, or a reflection — a copy of
/// a prefix of the parent path showing the same insertion.
fn tree_ok(expected: &Value, got: &Value, ctx: &ShadowCtx<'_>) -> bool {
    if equal_in_order(expected, got) {
        return true;
    }
    if ctx
        .prefixes
        .iter()
        .any(|(value, suffix)| equal_in_order(expected, value) && along(value, got, suffix, ctx))
    {
        return true;
    }
    match (expected, got) {
        (Value::Mapping(a), Value::Mapping(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b.iter())
                    .all(|((ka, va), (kb, vb))| ka == kb && tree_ok(va, vb, ctx))
        }
        (Value::Sequence(a), Value::Sequence(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(va, vb)| tree_ok(va, vb, ctx))
        }
        _ => false,
    }
}

/// The parent mapping with the shadow in: `key` holds the placeholder,
/// and with it set aside both sides match in order.
fn shadowed(expected: &Value, got: &Value, ctx: &ShadowCtx<'_>) -> bool {
    let (Value::Mapping(a), Value::Mapping(b)) = (expected, got) else {
        return false;
    };
    if b.get(ctx.key) != Some(ctx.placeholder) {
        return false;
    }
    let a_rest = a.iter().filter(|(ka, _)| *ka != ctx.key);
    let b_rest: Vec<_> = b.iter().filter(|(kb, _)| *kb != ctx.key).collect();
    a.iter().filter(|(ka, _)| *ka != ctx.key).count() == b_rest.len()
        && a_rest
            .zip(b_rest)
            .all(|((ka, va), (kb, vb))| ka == kb && tree_ok(va, vb, ctx))
}

/// Structural equality that also holds mapping key **order** — the
/// `IndexMap` equality behind `Value` ignores it, which would let a
/// splice that reordered siblings pass the proof-before-commit.
fn equal_in_order(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Mapping(x), Value::Mapping(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|((ka, va), (kb, vb))| ka == kb && equal_in_order(va, vb))
        }
        (Value::Sequence(x), Value::Sequence(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|(va, vb)| equal_in_order(va, vb))
        }
        _ => a == b,
    }
}

/// Whether the engine's insertion refusal is the merge-only one: no
/// entry of the mapping has source bytes of its own to anchor against.
/// Recognized by its wording, like the shared-value refusal; a test
/// pins the marker so an upstream rewording fails loudly.
fn is_merge_only_refusal(e: &::noyalib::Error) -> bool {
    e.to_string()
        .contains("has source bytes of its own to anchor")
}

#[cfg(test)]
mod tests {
    use super::super::apply;
    use crate::ast::Program;
    use crate::error::YqrError;

    /// Run the mutating `filter` over `input` on the default backend.
    fn assign(filter: &str, input: &str) -> Result<String, YqrError> {
        match crate::parser::parse_program(filter).expect("valid filter") {
            Program::Mutate(m) => apply(&m, input),
            Program::Query(_) => panic!("not a mutation: {filter}"),
        }
    }

    fn int(out: &str, path: &str) -> crate::Value {
        crate::eval_str(path, out).unwrap().remove(0)
    }

    #[test]
    fn overrides_a_merged_key_beside_own_entries() {
        let out = assign(".c.k = 9", "m: &m\n  k: 1\nc:\n  <<: *m\n  own: 2\n").unwrap();
        assert_eq!(out, "m: &m\n  k: 1\nc:\n  <<: *m\n  own: 2\n  k: 9\n");
        assert_eq!(int(&out, ".c.k"), crate::Value::Int(9), "the shadow wins");
        assert_eq!(
            int(&out, ".m.k"),
            crate::Value::Int(1),
            "the definition keeps its value"
        );
    }

    #[test]
    fn overrides_a_merged_key_in_a_merge_only_mapping() {
        let out = assign(".c.k = 9", "m: &m\n  k: 1\nc:\n  <<: *m\n").unwrap();
        assert_eq!(out, "m: &m\n  k: 1\nc:\n  <<: *m\n  k: 9\n");
        assert_eq!(int(&out, ".c.k"), crate::Value::Int(9));
        assert_eq!(int(&out, ".m.k"), crate::Value::Int(1));
    }

    #[test]
    fn the_engine_spells_the_override_value() {
        // A value that needs quoting: the splice writes only the
        // placeholder, and the ordinary assignment spells the value.
        let out = assign(".c.k = \"a: b\"", "m: &m\n  k: x\nc:\n  <<: *m\n").unwrap();
        assert_eq!(
            int(&out, ".c.k"),
            crate::Value::String("a: b".into()),
            "out: {out}"
        );
        assert!(
            out.starts_with("m: &m\n  k: x\nc:\n  <<: *m\n"),
            "out: {out}"
        );
    }

    #[test]
    fn a_collection_override_goes_through_the_matching_placeholder() {
        // The RHS is a mapping copied from the document; the placeholder
        // is `{}` so the engine writes collection over collection. The
        // flow spelling is the splice route's recorded behaviour: the
        // placeholder is flow, and the engine replaces like with like
        // (`f025` §4) — the insertion route spells block, and the
        // difference is spelling, never meaning.
        let out = assign(".c.k = .m", "m: &m\n  k: 1\nc:\n  <<: *m\n").unwrap();
        assert_eq!(out, "m: &m\n  k: 1\nc:\n  <<: *m\n  k: {k: 1}\n");
        assert_eq!(int(&out, ".c.k.k"), crate::Value::Int(1), "out: {out}");
    }

    #[test]
    fn a_brand_new_key_lands_in_a_merge_only_mapping_too() {
        // The splice is the general answer to "no own entry to anchor
        // against": the inherited-key override and a brand-new sibling
        // go through the same route, with the same wording.
        let out = assign(".c.sub = 9", "m: &m\n  k: 1\nc:\n  <<: *m\n").unwrap();
        assert_eq!(out, "m: &m\n  k: 1\nc:\n  <<: *m\n  sub: 9\n");
        assert_eq!(int(&out, ".c.sub"), crate::Value::Int(9));
        assert_eq!(int(&out, ".c.k"), crate::Value::Int(1));
    }

    #[test]
    fn a_merge_only_sequence_item_takes_the_override() {
        // The docker-compose shape: list items sharing defaults through
        // `- <<: *m`. The shadow continues at the column the item's keys
        // would, past the `- ` marker.
        let out = assign(".xs[0].k = 9", "m: &m\n  k: 1\nxs:\n  - <<: *m\n").unwrap();
        assert_eq!(out, "m: &m\n  k: 1\nxs:\n  - <<: *m\n    k: 9\n");
        assert_eq!(int(&out, ".xs[0].k"), crate::Value::Int(9));
    }

    #[test]
    fn an_override_inside_a_shared_anchor_reflects_at_the_alias() {
        // The anchor rule, for insertion: the entry is created at the
        // definition and every alias site shows it — the mirror of the
        // `f038` deletion rule, accepted by the same kind of proof.
        let out = assign(
            ".orig.c.k = 9",
            "m: &m\n  k: 1\norig: &o\n  c:\n    <<: *m\ncopy: *o\n",
        )
        .unwrap();
        assert_eq!(
            out,
            "m: &m\n  k: 1\norig: &o\n  c:\n    <<: *m\n    k: 9\ncopy: *o\n"
        );
        assert_eq!(
            int(&out, ".copy.c.k"),
            crate::Value::Int(9),
            "the alias sees it"
        );
        assert_eq!(
            int(&out, ".m.k"),
            crate::Value::Int(1),
            "the merge source does not"
        );
    }

    #[test]
    fn a_flow_shaped_merge_only_mapping_takes_the_engines_insertion() {
        // The engine's own insertion handles a flow mapping, so the
        // splice (which cannot express one) never fires for it.
        let out = assign(".c.k = 9", "m: &m\n  k: 1\nc: {<<: *m}\n").unwrap();
        assert_eq!(out, "m: &m\n  k: 1\nc: {<<: *m, k: 9}\n");
        assert_eq!(int(&out, ".c.k"), crate::Value::Int(9));
    }

    #[test]
    fn crlf_and_missing_final_newline_are_kept() {
        let out = assign(".c.k = 9", "m: &m\r\n  k: 1\r\nc:\r\n  <<: *m\r\n").unwrap();
        assert_eq!(out, "m: &m\r\n  k: 1\r\nc:\r\n  <<: *m\r\n  k: 9\r\n");
        let out = assign(".c.k = 9", "m: &m\n  k: 1\nc:\n  <<: *m").unwrap();
        assert_eq!(out, "m: &m\n  k: 1\nc:\n  <<: *m\n  k: 9");
        // A mixed-ending file without a final newline: the shadow's own
        // break is the nearest preceding line's (LF), not the stray CRLF
        // elsewhere, so the edit site is not made worse.
        let out = assign(".c.k = 9", "a: 1\r\nm: &m\n  k: 1\nc:\n  <<: *m").unwrap();
        assert_eq!(out, "a: 1\r\nm: &m\n  k: 1\nc:\n  <<: *m\n  k: 9");
    }

    #[test]
    fn an_alias_valued_parent_stays_refused() {
        // An explicit entry cannot exist in `c: *m` without rewriting
        // the alias into a block; the refusal names the way out.
        let err = assign(".c.k = 9", "m: &m\n  k: 1\nc: *m\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("alias"), "error: {err}");
        assert!(err.contains("where the key is defined"), "error: {err}");
    }

    #[test]
    fn an_alias_valued_entry_takes_a_scalar_now() {
        // Flipped by noyalib 0.0.57 (bug b035): writing a scalar at the
        // entry replaces the `*m` reference and touches nothing else.
        // The alias-reached *parent* refusal above is the rule that
        // stays.
        let out = assign(".c = 1", "m: &m\n  k: 1\nc: *m\n").unwrap();
        assert_eq!(out, "m: &m\n  k: 1\nc: 1\n");
    }

    #[test]
    fn the_merge_only_refusal_marker_is_pinned() {
        // The splice route is chosen by the engine's wording; if upstream
        // rewords it, this fails loudly instead of the route silently
        // dying.
        let cfg = crate::fidelity::cst_config();
        let mut d =
            ::noyalib::cst::parse_document_with_config("m: &m\n  k: 1\nc:\n  <<: *m\n", &cfg)
                .unwrap();
        let err = d
            .insert_entry_value("c", "x", &::noyalib::Value::Null)
            .unwrap_err();
        assert!(super::is_merge_only_refusal(&err), "marker moved: {err}");
    }
}
