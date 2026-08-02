//! Structural delete: removing a block entry with everything it owns.
//!
//! This module *is* yqr's delete. It derives the source range the entry owns,
//! splices it out with the raw byte-preserving
//! [`replace_span`](noyalib::cst::Document::replace_span) escape hatch, and
//! **commits only when the result re-parses to exactly the original document
//! minus the target** — the structural-integrity guard. Deleting the sole entry
//! of a block, or an item of a flow collection, stays refused with a clear
//! message.
//!
//! noyalib's first-class [`remove`](noyalib::cst::Document::remove) is not used,
//! and the reason is a deliberate difference in what an entry *is* rather than a
//! missing API. Upstream maps the same shapes this module does, but scopes a
//! delete to the key/value lines: a head comment above the entry survives (and
//! silently documents the next sibling instead), a keep-chomped scalar's kept
//! trailing blank lines are stranded, and a comment that follows the value but
//! belongs to the next sibling is swallowed. Here an entry owns its trivia, so
//! each of those three cases is handled instead of quietly getting it wrong.
//!
//! The owned range is derived from the value's authoritative source span
//! ([`span_at`](noyalib::cst::Document::span_at)), not an indentation heuristic,
//! so it is exact for the cases a heuristic gets wrong:
//!
//! - a keep-chomped (`|+`) block scalar whose trailing blank lines are content;
//! - a block sequence written at its key's own column (`on:\n- a\n- b`), which
//!   noyalib's span resolver under-reports to just its first `-`;
//! - a comment following the value but belonging to the next sibling (excluded)
//!   versus one interleaved inside the value (included).
//!
//! A contiguous run of same-indent comment lines *directly above* the entry is
//! its head comment and is removed with it, so a delete never silently
//! re-attributes a comment to the following sibling. The commit keeps every
//! surviving byte verbatim (`replace_span` splices the source buffer in place —
//! no parse→emit round-trip that could normalize an untouched node), and the
//! re-parse guard backstops any residual case by refusing rather than emitting
//! a restructured document.

// Feature f007 (see specs/features/): write tier — structural edits.

use super::{FidelityWriter, NoyalibWriter, noyalib_path};
use crate::Value;
use crate::error::{Result, YqrError};
use crate::fidelity::noyalib::walk_value;
use crate::fidelity::{Path, PathSeg};

impl NoyalibWriter {
    /// Delete the block entry at `path`, together with the trivia it owns: its
    /// head comment, and the trailing blank lines a keep-chomped scalar keeps.
    ///
    /// The edit is validated against a private re-parse and committed only if it
    /// removes exactly the target node and leaves every surviving node
    /// byte-identical; otherwise the document is left untouched and a clear
    /// error is returned.
    ///
    /// # Errors
    ///
    /// Errors when the path is unaddressable, is the sole entry of its block,
    /// is an item of a flow collection, uses a source layout this path cannot
    /// map, or the edit would restructure the document.
    pub(super) fn delete_entry(&mut self, doc: usize, path: &Path) -> Result<()> {
        // Fail early on a key the string-path grammar cannot express (the same
        // honest gap the assign path declares); this also names the target in
        // every message below.
        let path_str = noyalib_path(path)?;

        let Some((last, parent_segs)) = path.segments().split_last() else {
            return Err(YqrError::eval(
                "cannot delete the document root".to_string(),
            ));
        };

        let doc_value = self.value(doc)?;

        // Removing the only entry would leave an empty block, which re-parses as
        // `null` — a structural change the caller must ask for explicitly.
        if parent_len(&doc_value, parent_segs) == Some(1) {
            return Err(YqrError::eval(format!(
                "cannot delete {path_str}: it is the only entry of its {}; removing it \
                 would leave an empty block (a structural change) and is not supported",
                collection_noun(last),
            )));
        }

        // Whether the target's own value is a non-empty block sequence, and how
        // many items it has: noyalib's span resolver under-reports a sequence
        // written at its key's own column to just its first `-`, and the true
        // end is recovered from the last item below.
        let target_seq_len = match walk_value(&doc_value, path.segments()) {
            Some(Value::Sequence(items)) if !items.is_empty() => Some(items.len()),
            _ => None,
        };

        // The exact document value with the target removed — the yardstick the
        // spliced result must re-parse to. Computed in yqr's model so key order
        // (mapping) and index shifting (sequence) match block-delete semantics;
        // consumes `doc_value` rather than cloning the whole document.
        let expected = remove_at_path(doc_value, path.segments()).ok_or_else(|| {
            YqrError::eval(format!(
                "cannot delete {path_str}: it does not address a removable entry"
            ))
        })?;

        // Read spans and source bytes, then compute the spliced source and the
        // exact byte range removed. All shared borrows of the document end
        // before the mutating commit below.
        let (start, end, new_source) = {
            let d = self.doc_ref(doc)?;
            let src = d.source();

            // A flow collection (`[a, b]` / `{a: 1}`) is line-shaped
            // differently; whole-line deletion cannot express removing one of
            // its items. Detect it — including a root-level flow collection,
            // whose parent is the document itself — for a clear message (the
            // guard would otherwise refuse with a generic one).
            let parent_is_flow = if parent_segs.is_empty() {
                src.trim_start().starts_with(['[', '{'])
            } else {
                segs_to_noyalib_path(parent_segs)
                    .and_then(|parent_str| d.get(&parent_str))
                    .is_some_and(|bytes| bytes.trim_start().starts_with(['[', '{']))
            };
            if parent_is_flow {
                return Err(YqrError::eval(format!(
                    "cannot delete {path_str}: removing an item from a flow collection is not supported"
                )));
            }

            let (value_start, value_end) = d.span_at(&path_str).ok_or_else(|| {
                YqrError::eval(format!("cannot delete {path_str}: cannot locate its bytes"))
            })?;

            // Recover the true end of a same-column block sequence from its last
            // item (which always resolves); a no-op for an indented sequence,
            // whose whole-sequence span is already correct.
            let value_end = match target_seq_len {
                Some(len) => d
                    .span_at(&format!("{path_str}[{}]", len - 1))
                    .map_or(value_end, |(_, last_end)| value_end.max(last_end)),
                None => value_end,
            };

            let (start, end) =
                owned_line_span(src, value_start, value_end, last).ok_or_else(|| {
                    YqrError::eval(format!(
                        "cannot delete {path_str}: its source layout is not supported"
                    ))
                })?;

            let mut out = String::with_capacity(src.len() - (end - start));
            out.push_str(&src[..start]);
            out.push_str(&src[end..]);
            (start, end, out)
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

        // Commit via the byte-preserving in-place splice: the surviving bytes
        // are the original document's bytes verbatim (`src[..start]` +
        // `src[end..]`), so an untouched node can never be normalized by a
        // parse→emit round-trip. The guard above already proved this range
        // re-parses to `expected`, so this cannot fail in practice.
        self.doc_mut(doc)?
            .replace_span(start, end, "")
            .map_err(|e| YqrError::eval(format!("cannot delete {path_str}: {e}")))?;
        Ok(())
    }
}

/// The source-byte range `[start, end)` the entry owns, given the byte offsets
/// of its resolved value span (`value_start..value_end`, from noyalib's
/// authoritative [`span_at`](noyalib::cst::Document::span_at)) and its final
/// path segment.
///
/// The range runs from the start of the entry's head comment (or its key/`-`
/// line when there is none) through the end of the line that holds the value's
/// last content byte. Deriving the end from `value_end` — rather than an
/// indentation walk — makes it exact where a heuristic errs: a keep-chomped
/// `|+` scalar (whose trailing blanks are content and are kept in `value_end`),
/// a same-column block sequence (recovered by the caller), and a following
/// comment that belongs to the next sibling (left outside `value_end`, so it
/// survives) versus one interleaved inside the value (covered by `value_end`).
///
/// Returns `None` when the entry's marker cannot be located (an unsupported
/// layout, or a flow item), so the caller refuses rather than guesses.
fn owned_line_span(
    src: &str,
    value_start: usize,
    value_end: usize,
    last: &PathSeg,
) -> Option<(usize, usize)> {
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

    // Extend the value's end to the end of the line holding its last content
    // byte — unless `value_end` already sits at a line boundary, which happens
    // only for a keep-chomped `|+` scalar whose kept trailing blank lines end
    // there; extending then would swallow the following sibling's line.
    let content_end = if value_end > 0 && bytes[value_end - 1] == b'\n' {
        value_end
    } else {
        line_end(src, value_end)
    };

    // A contiguous run of same-indent comment lines directly above the entry is
    // its head comment and is removed with it, so the delete never silently
    // re-attributes the comment to the following sibling.
    let start = absorb_head_comments(src, first_line_start, entry_indent);

    Some((start, content_end))
}

/// Move `start` up over a contiguous run of full-line comments directly above
/// the entry, each at column `indent`, stopping at a blank line, a non-comment
/// line, or a differently-indented comment. A comment immediately preceding a
/// key at its own indentation documents that key (the head-comment convention),
/// so it belongs to the entry being deleted.
fn absorb_head_comments(src: &str, mut start: usize, indent: usize) -> usize {
    while start > 0 {
        // The line ending at `start - 1` (its trailing '\n' is `src[start - 1]`).
        let prev_line_start = src[..start - 1].rfind('\n').map_or(0, |n| n + 1);
        let line = &src[prev_line_start..start];
        if indent_width(line) == indent && line.trim_start().starts_with('#') {
            start = prev_line_start;
        } else {
            break;
        }
    }
    start
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

/// `root` (consumed) with the node at `segs` removed, or `None` when the path
/// does not address a removable mapping key / sequence index. Order is
/// preserved for mappings and indices shift for sequences, matching
/// block-delete semantics. Takes ownership so building the yardstick value does
/// not clone the whole document.
fn remove_at_path(root: Value, segs: &[PathSeg]) -> Option<Value> {
    let (last, parents) = segs.split_last()?;
    let mut new = root;
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
    match walk_value(root, segs)? {
        Value::Mapping(map) => Some(map.len()),
        Value::Sequence(items) => Some(items.len()),
        _ => None,
    }
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

    /// Run `del(<path>)` over `input` on the default backend.
    fn del(path: &str, input: &str) -> Result<String, YqrError> {
        apply(
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
    fn does_not_eat_a_following_siblings_comment() {
        // The deeper-indented `# note for next` follows `outer`'s only entry and
        // documents `next`; it is not part of `outer`'s value span, so it must
        // survive rather than be swallowed by an indentation heuristic.
        let out = del(".outer", "outer:\n  a: 1\n  # note for next\nnext: 2\n").unwrap();
        assert_eq!(out, "  # note for next\nnext: 2\n");
    }

    #[test]
    fn removes_a_head_comment_with_its_entry() {
        // A comment on the line directly above the key, at the key's own
        // indentation, is the entry's head comment; deleting the entry removes
        // it too, so it is never silently re-attributed to the next sibling.
        let out = del(
            ".database",
            "# database connection settings\ndatabase:\n  host: localhost\ncache:\n  ttl: 60\n",
        )
        .unwrap();
        assert_eq!(out, "cache:\n  ttl: 60\n");
    }

    #[test]
    fn removes_multiple_contiguous_head_comment_lines() {
        let out = del(
            ".block",
            "keep: 0\n# line one\n# line two\nblock:\n  a: 1\ntail: 9\n",
        )
        .unwrap();
        assert_eq!(out, "keep: 0\ntail: 9\n");
    }

    #[test]
    fn a_detached_comment_above_a_blank_line_is_not_a_head_comment() {
        // A blank line between the comment and the key detaches it, so it is
        // left in place rather than removed with the entry.
        let out = del(".block", "# detached\n\nblock:\n  a: 1\ntail: 9\n").unwrap();
        assert_eq!(out, "# detached\n\ntail: 9\n");
    }

    #[test]
    fn keeps_trailing_blank_lines_of_a_keep_chomped_scalar() {
        // The blank line after `x` is part of the `|+` scalar's value, so it is
        // owned by `script` and goes when `script` is deleted — no spurious
        // blank line survives.
        let out = del(".script", "a: 1\nscript: |+\n  x\n\nb: 2\n").unwrap();
        assert_eq!(out, "a: 1\nb: 2\n");
    }

    #[test]
    fn deletes_a_block_sequence_at_its_keys_own_column() {
        // The GitHub Actions / Ansible / Kubernetes list style writes a key's
        // block-sequence value at the key's own column; deleting the key must
        // remove the whole sequence, not refuse it.
        let out = del(".on", "on:\n- push\n- pull_request\njobs: {}\n").unwrap();
        assert_eq!(out, "jobs: {}\n");
    }

    #[test]
    fn deletes_a_nested_same_column_sequence() {
        let out = del(".steps.on", "steps:\n  on:\n  - a\n  - b\n  run: x\n").unwrap();
        assert_eq!(out, "steps:\n  run: x\n");
    }

    #[test]
    fn refuses_a_root_flow_collection_item_with_a_clear_message() {
        // A root-level flow sequence item cannot be removed by whole-line
        // deletion; the message must name the flow collection, not fall through
        // to the generic "layout not supported" text.
        let err = del(".[0]", "[80, 443, 8080]\n").unwrap_err();
        assert!(matches!(err, YqrError::Eval(ref m) if m.contains("flow collection")));
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
