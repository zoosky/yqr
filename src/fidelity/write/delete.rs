//! Structural delete: removing multi-line and nested block entries.
//!
//! noyalib 0.0.14's first-class [`remove`](noyalib::cst::Document::remove)
//! deletes only *single-line* block entries; a multi-line value, a nested
//! collection, the sole entry of a block, and flow collections are all refused
//! upstream. This module is the interim fallback for the first two: it computes
//! the source lines the entry owns, splices them out with the raw
//! `replace_span` escape hatch, and **commits only when the result re-parses to
//! exactly the original document minus the target** — the structural-integrity
//! guard. Sole-entry and flow deletes stay refused, with a clear message.
//!
//! The computed span always runs from the start of the entry's key/`-` line
//! through the end of its last more-indented content line, so it can never eat
//! a preceding comment or a following sibling; the guard backstops any residual
//! case by refusing rather than emitting a restructured document.

// Feature f007 (see specs/features/): write tier — structural edits.

use super::{FidelityWriter, NoyalibWriter, noyalib_path};
use crate::Value;
use crate::error::{Result, YqrError};
use crate::fidelity::{Path, PathSeg};

impl NoyalibWriter {
    /// Delete the block entry at `path` when noyalib's first-class `remove`
    /// refused it (a multi-line or nested value).
    ///
    /// The edit is applied to a private copy and committed only if it removes
    /// exactly the target node and leaves every surviving node byte-identical;
    /// otherwise the document is left untouched and a clear error is returned.
    ///
    /// # Errors
    ///
    /// Errors when the path is unaddressable, is the sole entry of its block,
    /// is an item of a flow collection, uses a layout the fallback cannot map,
    /// or the edit would restructure the document.
    pub(super) fn delete_structural(&mut self, doc: usize, path: &Path) -> Result<()> {
        // Fail early on a key the string-path grammar cannot express (the same
        // honest gap the assign path declares); this also names the target in
        // every message below.
        let path_str = noyalib_path(path)?;

        let Some((last, parent_segs)) = path.segments().split_last() else {
            return Err(YqrError::eval(
                "cannot delete the document root".to_string(),
            ));
        };

        // The exact document value with the target removed — the yardstick the
        // spliced result must re-parse to. Computed in yqr's model so key order
        // (mapping) and index shifting (sequence) match block-delete semantics.
        let doc_value = self.value(doc)?;
        let expected = remove_at_path(&doc_value, path.segments()).ok_or_else(|| {
            YqrError::eval(format!(
                "cannot delete {path_str}: it does not address a removable entry"
            ))
        })?;

        // Removing the only entry would leave an empty block, which re-parses as
        // `null` — a structural change the caller must ask for explicitly.
        if parent_len(&doc_value, parent_segs) == Some(1) {
            return Err(YqrError::eval(format!(
                "cannot delete {path_str}: it is the only entry of its {}; removing it \
                 would leave an empty block (a structural change) and is not supported",
                collection_noun(last),
            )));
        }

        // Read spans and source bytes, then build the spliced source. All
        // shared borrows of the document end before the mutating commit below.
        let new_source = {
            let d = self.doc_ref(doc)?;

            // A flow collection (`[a, b]` / `{a: 1}`) is line-shaped
            // differently; whole-line deletion cannot express removing one of
            // its items. Detect it from the parent's own bytes for a clear
            // message (the guard would otherwise refuse with a generic one).
            if !parent_segs.is_empty()
                && let Some(parent_str) = segs_to_noyalib_path(parent_segs)
                && let Some(parent_bytes) = d.get(&parent_str)
                && parent_bytes.trim_start().starts_with(['[', '{'])
            {
                return Err(YqrError::eval(format!(
                    "cannot delete {path_str}: removing an item from a flow collection is not supported"
                )));
            }

            let src = d.source();
            let (value_start, _) = d.span_at(&path_str).ok_or_else(|| {
                YqrError::eval(format!("cannot delete {path_str}: cannot locate its bytes"))
            })?;
            let (start, end) = owned_line_span(src, value_start, last).ok_or_else(|| {
                YqrError::eval(format!(
                    "cannot delete {path_str}: its source layout is not supported by the delete fallback"
                ))
            })?;

            let mut out = String::with_capacity(src.len() - (end - start));
            out.push_str(&src[..start]);
            out.push_str(&src[end..]);
            out
        };

        // `replace_span` guarantees only *valid YAML*, not structure
        // preservation (b004 2.5), so yqr owns the guard: re-parse the edited
        // source and require it to lower to the expected value. A dangling
        // alias, an over-broad span, or a flow mis-edit all diverge here and
        // are refused with the document untouched.
        let candidate = ::noyalib::cst::parse_document(&new_source).map_err(|e| {
            YqrError::eval(format!(
                "cannot delete {path_str}: the edit does not re-parse ({e})"
            ))
        })?;
        if Value::from(&*candidate.as_value()) != expected {
            return Err(YqrError::eval(format!(
                "cannot delete {path_str}: the edit would change the document structure and was refused"
            )));
        }

        *self.doc_mut(doc)? = candidate;
        Ok(())
    }
}

/// The source-byte range `[start, end)` the entry owns, given the byte offset
/// of its resolved value and its final path segment.
///
/// The range runs from the start of the key/`-` line through the end of the
/// last line indented deeper than the key (the entry's continuation), with any
/// trailing blank lines excluded so surviving separation is preserved. Returns
/// `None` when the entry's marker cannot be located (an unsupported layout, or
/// a flow item), so the caller refuses rather than guesses.
fn owned_line_span(src: &str, value_start: usize, last: &PathSeg) -> Option<(usize, usize)> {
    let marker = match last {
        PathSeg::Key(_) => b':',
        PathSeg::Index(_) => b'-',
    };
    let bytes = src.as_bytes();

    // Walk back from the value to the entry marker over insignificant
    // whitespace/newlines, stepping over a trailing line comment on the key
    // line (`key:  # note`) so a commented block entry still resolves.
    let mut i = value_start;
    let marker_pos = loop {
        while i > 0 && matches!(bytes[i - 1], b' ' | b'\t' | b'\r' | b'\n') {
            i -= 1;
        }
        if i == 0 {
            return None;
        }
        if bytes[i - 1] == marker {
            break i - 1;
        }
        // Not the marker: if the current line carries a `#` comment, resume the
        // scan before it; otherwise the layout is unsupported.
        let line_start = src[..i].rfind('\n').map_or(0, |n| n + 1);
        let hash = src[line_start..i].rfind('#')?;
        i = line_start + hash;
    };

    let first_line_start = src[..marker_pos].rfind('\n').map_or(0, |n| n + 1);
    let entry_indent = indent_width(&src[first_line_start..]);

    let mut cursor = line_end(src, first_line_start);
    let mut content_end = cursor;
    while cursor < src.len() {
        let next = line_end(src, cursor);
        let line = &src[cursor..next];
        if line.trim().is_empty() {
            // A blank line is only owned when deeper content follows it; leave
            // `content_end` where it is so trailing blanks stay with the file.
            cursor = next;
            continue;
        }
        if indent_width(line) > entry_indent {
            cursor = next;
            content_end = next;
        } else {
            break;
        }
    }

    Some((first_line_start, content_end))
}

/// Number of leading space characters (the block-indent column) of `line`.
fn indent_width(line: &str) -> usize {
    line.bytes().take_while(|&b| b == b' ').count()
}

/// Byte offset just past the newline that ends the line starting at `pos`, or
/// the end of `src` for the final line.
fn line_end(src: &str, pos: usize) -> usize {
    src[pos..].find('\n').map_or(src.len(), |n| pos + n + 1)
}

/// `root` with the node at `segs` removed, or `None` when the path does not
/// address a removable mapping key / sequence index. Order is preserved for
/// mappings and indices shift for sequences, matching block-delete semantics.
fn remove_at_path(root: &Value, segs: &[PathSeg]) -> Option<Value> {
    let (last, parents) = segs.split_last()?;
    let mut new = root.clone();
    match (navigate_mut(&mut new, parents)?, last) {
        (Value::Mapping(map), PathSeg::Key(k)) => {
            map.shift_remove(&Value::String(k.clone()))?;
        }
        (Value::Sequence(items), PathSeg::Index(i)) => {
            if *i >= items.len() {
                return None;
            }
            items.remove(*i);
        }
        _ => return None,
    }
    Some(new)
}

/// Length of the collection at `segs`, or `None` when it is not a collection.
fn parent_len(root: &Value, segs: &[PathSeg]) -> Option<usize> {
    match navigate(root, segs)? {
        Value::Mapping(map) => Some(map.len()),
        Value::Sequence(items) => Some(items.len()),
        _ => None,
    }
}

/// Walk `segs` into `value`, returning the addressed node.
fn navigate<'a>(mut value: &'a Value, segs: &[PathSeg]) -> Option<&'a Value> {
    for seg in segs {
        value = match (seg, value) {
            (PathSeg::Key(k), Value::Mapping(map)) => map.get(&Value::String(k.clone()))?,
            (PathSeg::Index(i), Value::Sequence(items)) => items.get(*i)?,
            _ => return None,
        };
    }
    Some(value)
}

/// Walk `segs` into `value` for a mutable borrow of the addressed node.
fn navigate_mut<'a>(mut value: &'a mut Value, segs: &[PathSeg]) -> Option<&'a mut Value> {
    for seg in segs {
        value = match (seg, value) {
            (PathSeg::Key(k), Value::Mapping(map)) => map.get_mut(&Value::String(k.clone()))?,
            (PathSeg::Index(i), Value::Sequence(items)) => items.get_mut(*i)?,
            _ => return None,
        };
    }
    Some(value)
}

/// Lower a segment slice to noyalib's string-path grammar (used to fetch the
/// parent's bytes for the flow-collection check), or `None` for a non-plain key.
fn segs_to_noyalib_path(segs: &[PathSeg]) -> Option<String> {
    let mut path = Path::root();
    for seg in segs {
        path = path.child(seg.clone());
    }
    noyalib_path(&path).ok()
}

/// The noun naming the parent collection kind, for the sole-entry message.
fn collection_noun(last: &PathSeg) -> &'static str {
    match last {
        PathSeg::Key(_) => "mapping",
        PathSeg::Index(_) => "sequence",
    }
}

#[cfg(test)]
mod tests {
    use super::super::apply;
    use crate::ast::Mutation;
    use crate::error::YqrError;
    use crate::fidelity::BackendId;

    /// Run `del(<path>)` over `input` on the default backend.
    fn del(path: &str, input: &str) -> Result<String, YqrError> {
        apply(
            BackendId::NoyalibCst,
            &Mutation::Delete {
                path: crate::parser::parse(path).expect("valid path"),
            },
            input,
        )
    }

    #[test]
    fn deletes_a_nested_block_mapping() {
        let out = del(
            ".outer",
            "outer:\n  inner: 1\n  deep:\n    x: 2\nother: 3\n",
        )
        .unwrap();
        assert_eq!(out, "other: 3\n");
    }

    #[test]
    fn deletes_a_multi_line_sequence_item() {
        // `.svc.ports[0]` is a two-line mapping item; its `- name` line and the
        // deeper `port` continuation both go, and the second item stays.
        let out = del(
            ".svc.ports[0]",
            "svc:\n  ports:\n    - name: http\n      port: 80\n    - name: https\n      port: 443\n",
        )
        .unwrap();
        assert_eq!(out, "svc:\n  ports:\n    - name: https\n      port: 443\n");
    }

    #[test]
    fn preserves_surrounding_comments_and_a_trailing_sibling() {
        let input = "# header\nkeep: 1  # inline\nblock:\n  a: 1\n  b: 2\ntail: 9\n";
        let out = del(".block", input).unwrap();
        assert_eq!(out, "# header\nkeep: 1  # inline\ntail: 9\n");
    }

    #[test]
    fn steps_over_a_comment_on_the_key_line() {
        // The block value starts on the next line; the `# note` trailing the
        // key must not defeat the backward marker scan.
        let out = del(".outer", "outer:  # note\n  inner: 1\nother: 2\n").unwrap();
        assert_eq!(out, "other: 2\n");
    }

    #[test]
    fn keeps_a_more_indented_comment_with_its_entry() {
        // A comment indented under the entry belongs to it and goes with it.
        let out = del(
            ".outer",
            "outer:\n  a: 1\n  # inner note\n  b: 2\nother: 3\n",
        )
        .unwrap();
        assert_eq!(out, "other: 3\n");
    }

    #[test]
    fn preserves_a_blank_line_between_entries() {
        // The blank separating `outer` from `other` is not owned by `outer`
        // (no deeper content follows it), so it survives as a leading blank.
        let out = del(".outer", "outer:\n  a: 1\n\nother: 2\n").unwrap();
        assert_eq!(out, "\nother: 2\n");
    }

    #[test]
    fn deletes_the_last_entry_of_a_mapping() {
        let out = del(".outer", "first: 1\nouter:\n  a: 1\n  b: 2\n").unwrap();
        assert_eq!(out, "first: 1\n");
    }

    #[test]
    fn refuses_the_sole_entry_of_a_block() {
        let err = del(".only", "only:\n  a: 1\n  b: 2\n").unwrap_err();
        assert!(matches!(err, YqrError::Eval(ref m) if m.contains("only entry")));
    }

    #[test]
    fn refuses_the_sole_top_level_entry() {
        let err = del(".root", "root:\n  a: 1\n").unwrap_err();
        assert!(matches!(err, YqrError::Eval(ref m) if m.contains("only entry")));
    }

    #[test]
    fn refuses_a_flow_collection_item() {
        let err = del(".ports[0]", "ports: [80, 443]\n").unwrap_err();
        assert!(matches!(err, YqrError::Eval(ref m) if m.contains("flow collection")));
    }

    #[test]
    fn refuses_when_removing_an_anchor_breaks_an_alias() {
        // Deleting the anchor definition leaves `*base` dangling, so the edit
        // does not re-parse; it is refused with the document untouched.
        let err = del(
            ".defaults",
            "defaults: &base\n  timeout: 30\nservice:\n  <<: *base\n  name: web\n",
        )
        .unwrap_err();
        assert!(matches!(err, YqrError::Eval(_)));
    }

    #[test]
    fn multi_line_delete_applies_only_to_the_matching_document() {
        // The first document has `.outer`; the second (a Service) does not, so
        // it is left byte-identical.
        let out = del(".outer", "outer:\n  a: 1\nkeep: 2\n---\nkind: Service\n").unwrap();
        assert_eq!(out, "keep: 2\n---\nkind: Service\n");
    }

    #[test]
    fn single_line_delete_still_works_through_the_fallback_path() {
        // A single-line entry is handled by noyalib's `remove`; this pins that
        // the hybrid dispatch did not regress the common case.
        let out = del(".b", "a: 1\nb: 2\nc: 3\n").unwrap();
        assert_eq!(out, "a: 1\nc: 3\n");
    }
}
