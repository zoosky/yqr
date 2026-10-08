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
//!   scalar-over-block collapse uses.
//!
//! A parent whose value *is* an alias (`c: *m`) stays refused: an
//! explicit entry cannot exist in it without rewriting the alias into a
//! block, which is a restructuring the user must spell out.

// Feature f025: override a merged-in key by creating an explicit entry.

use super::seam::FidelityWriter;
use super::{NoyalibWriter, insertable, rebased, to_noyalib_path};
use crate::Value;
use crate::error::{Result, YqrError};
use crate::fidelity::Path;

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
            Err(e) if is_merge_only_refusal(&e) => {
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

        let expected = super::anchor::assign_at(&self.value(doc)?, path.segments(), &placeholder)
            .ok_or_else(|| {
            YqrError::eval(format!(
                "cannot assign at {path_str:?}: it does not address an entry of a mapping"
            ))
        })?;

        let (at, fragment, new_source) = {
            let d = self.doc_ref(doc)?;
            let src = d.source();
            let (span_start, span_end) = d.span_at(parent_str).ok_or_else(|| {
                YqrError::eval(format!(
                    "cannot assign at {path_str:?}: the mapping's bytes cannot be located"
                ))
            })?;
            // The shadow goes below the mapping's last line, at the same
            // indentation as its first (`<<`) line. A mapping whose bytes
            // do not open a line is flow-shaped; a line splice cannot
            // express it, so it is refused. The span may start at the
            // line's first byte (indentation included) or at its first
            // content byte, so the indent is read from the line itself.
            let line_start = src[..span_start].rfind(['\n', '\r']).map_or(0, |n| n + 1);
            if !src[line_start..span_start].chars().all(|c| c == ' ') {
                return Err(YqrError::eval(format!(
                    "cannot assign at {path_str:?}: the mapping's layout does not take a \
                     spliced entry line"
                )));
            }
            let content = src[line_start..].find(|c: char| c != ' ').unwrap_or(0);
            let indent = &src[line_start..line_start + content];
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
                // terminator; the shadow brings its own line break and
                // the file still ends without one.
                let own = if src.contains("\r\n") { "\r\n" } else { "\n" };
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
        if Value::from(&*candidate.as_value()) != expected {
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
        // is `{}` so the engine writes collection over collection.
        let out = assign(".c.k = .m", "m: &m\n  k: 1\nc:\n  <<: *m\n").unwrap();
        assert_eq!(int(&out, ".c.k.k"), crate::Value::Int(1), "out: {out}");
        assert!(
            out.starts_with("m: &m\n  k: 1\nc:\n  <<: *m\n"),
            "out: {out}"
        );
    }

    #[test]
    fn crlf_and_missing_final_newline_are_kept() {
        let out = assign(".c.k = 9", "m: &m\r\n  k: 1\r\nc:\r\n  <<: *m\r\n").unwrap();
        assert_eq!(out, "m: &m\r\n  k: 1\r\nc:\r\n  <<: *m\r\n  k: 9\r\n");
        let out = assign(".c.k = 9", "m: &m\n  k: 1\nc:\n  <<: *m").unwrap();
        assert_eq!(out, "m: &m\n  k: 1\nc:\n  <<: *m\n  k: 9");
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
    fn an_alias_valued_entry_stays_refused() {
        // Criterion: replacing the reference itself is b019's question,
        // not this feature's.
        let err = assign(".c = 1", "m: &m\n  k: 1\nc: *m\n")
            .unwrap_err()
            .to_string();
        assert!(!err.is_empty());
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
