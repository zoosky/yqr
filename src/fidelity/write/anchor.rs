//! Assignment at an anchor definition: rewriting a shared value at its source.
//!
//! Two upstream behaviours make noyalib's `set_value` unusable for a write at
//! an anchor's own definition, and both land on the same remedy here:
//!
//! - The span it rewrites for an anchored scalar starts at the `&name`
//!   property, so the edit deletes the anchor definition — silently when
//!   nothing references it, and with an "unknown anchor" complaint about the
//!   alias the edit itself orphaned when something does. The anchor is a
//!   property of the node, not part of its value, and the value is what the
//!   filter assigns.
//! - Since noyalib 0.0.29 every typed mutator refuses a write into a value
//!   that live `*name` sites share, the definition included, and points at
//!   `materialise_aliases_of` — an API yqr does not expose, and the opposite
//!   of what the user asked for: changing the shared value once, at its
//!   source, is the YAML meaning of an anchor. It is also the one remedy
//!   yqr's own merged-key refusal names ("assign where the key is defined").
//!
//! So yqr performs the write itself, the way structural delete already edits:
//! resolve the value's byte span, keep any leading `&name` property, splice
//! the rendered scalar over the remaining bytes with
//! [`replace_span`](noyalib::cst::Document::replace_span), and **commit only
//! when the result re-parses to exactly the original document with the
//! assignment applied** — at the target, and at the alias sites that share
//! the anchored value, where the same old-to-new change is the write's
//! documented meaning. Any other divergence refuses with the document
//! untouched.
//!
//! A tagged value (`!!str 1`) is refused by name instead: rewriting the
//! scalar under a tag can change what the tag makes of it, so the honest
//! answer is a refusal that says which tag is in the way.

// Bug b026 and noyalib#338 (see specs/bugs/yqr-b026, specs/features/yqr-f026).

use super::{FidelityWriter, NoyalibWriter};
use crate::Value;
use crate::error::{Result, YqrError};
use crate::fidelity::noyalib::walk_value;
use crate::fidelity::{Path, PathSeg, cst_config};

impl NoyalibWriter {
    /// Whether the value bytes at `path_str` begin with a node property
    /// (`&anchor` or `!tag`).
    ///
    /// noyalib's `set_value` treats such a property as part of the value and
    /// rewrites it away, so a caller routes these targets to
    /// [`assign_at_definition`](Self::assign_at_definition) instead.
    ///
    /// # Errors
    ///
    /// Errors when `doc` is out of range. An unresolvable path reads as
    /// `false` — the mutator's own refusal names it better.
    pub(super) fn value_has_leading_property(&self, doc: usize, path_str: &str) -> Result<bool> {
        let d = self.doc_ref(doc)?;
        Ok(d.span_at(path_str).is_some_and(|(start, _)| {
            matches!(d.source().as_bytes().get(start), Some(b'&' | b'!'))
        }))
    }

    /// Assign `value` at `path`, preserving a leading `&name` anchor property
    /// and leaving every alias of that anchor pointing at the updated value.
    ///
    /// The edit is validated against a private re-parse and committed only if
    /// the result is the original document with the assignment applied — the
    /// only permitted differences beyond the target are the alias sites that
    /// share the anchored value, which must show exactly the same old-to-new
    /// change. Otherwise the document is left untouched.
    ///
    /// # Errors
    ///
    /// Errors when the path cannot be located, the value carries a tag, the
    /// rendered scalar does not fit on one line, or the edit would change the
    /// document beyond the assignment and its alias reflections.
    pub(super) fn assign_at_definition(
        &mut self,
        doc: usize,
        path: &Path,
        path_str: &str,
        value: &Value,
        rendered_value: &::noyalib::Value,
    ) -> Result<()> {
        let root = self.value(doc)?;
        let old = walk_value(&root, path.segments()).cloned().ok_or_else(|| {
            YqrError::eval(format!(
                "cannot assign at {path_str:?}: cannot locate its value"
            ))
        })?;

        let (start, end, rendered, new_source) = {
            let d = self.doc_ref(doc)?;
            let src = d.source();
            let (value_start, value_end) = d.span_at(path_str).ok_or_else(|| {
                YqrError::eval(format!(
                    "cannot assign at {path_str:?}: cannot locate its bytes"
                ))
            })?;

            let start = value_start + skip_anchor_property(&src[value_start..value_end]);
            let rest = &src[start..value_end];
            if let Some(tag) = leading_tag(rest) {
                return Err(YqrError::eval(format!(
                    "cannot assign at {path_str:?}: the value carries the tag `{tag}`, and \
                     rewriting the scalar under it could change what the tag makes of it; \
                     remove the tag first"
                )));
            }
            if rest.is_empty() || rest.starts_with('\n') || rest.starts_with('\r') {
                return Err(YqrError::eval(format!(
                    "cannot assign at {path_str:?}: its source layout is not supported"
                )));
            }

            let rendered = render_matching_quote_style(rest, rendered_value)
                .map_err(|e| YqrError::eval(format!("cannot assign at {path_str:?}: {e}")))?;
            if rendered.contains('\n') {
                return Err(YqrError::eval(format!(
                    "cannot assign at {path_str:?}: the value does not fit on a single line here"
                )));
            }

            let mut out = String::with_capacity(src.len() - (value_end - start) + rendered.len());
            out.push_str(&src[..start]);
            out.push_str(&rendered);
            out.push_str(&src[value_end..]);
            (start, value_end, rendered, out)
        };

        // `replace_span` guarantees only *valid YAML*, not structure
        // preservation, so yqr owns the guard: re-parse the edited source and
        // require it to lower to the original value with the assignment
        // applied — reflected at alias sites, and nowhere else.
        let candidate = ::noyalib::cst::parse_document_with_config(&new_source, &cst_config())
            .map_err(|e| {
                YqrError::eval(format!(
                    "cannot assign at {path_str:?}: the edit does not re-parse ({e})"
                ))
            })?;
        let expected = assign_at(&root, path.segments(), value).ok_or_else(|| {
            YqrError::eval(format!(
                "cannot assign at {path_str:?}: cannot locate its value"
            ))
        })?;
        let got = Value::from(&*candidate.as_value());
        if !changes_are_the_assignment(&expected, &got, &old, value) {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: the edit would change the document structure \
                 and was refused"
            )));
        }

        // Commit via the byte-preserving in-place splice; the guard above
        // already proved this exact source re-parses as required.
        self.doc_mut(doc)?
            .replace_span(start, end, &rendered)
            .map_err(|e| YqrError::eval(format!("cannot assign at {path_str:?}: {e}")))
    }
}

/// Byte length of a leading `&name` anchor property in `bytes`, including the
/// spaces separating it from the value; `0` when there is none.
fn skip_anchor_property(bytes: &str) -> usize {
    if !bytes.starts_with('&') {
        return 0;
    }
    let token_end = bytes
        .find(|c: char| c.is_whitespace())
        .unwrap_or(bytes.len());
    let after = &bytes[token_end..];
    token_end + (after.len() - after.trim_start_matches(' ').len())
}

/// The leading `!tag` token of `bytes`, when the value carries one.
fn leading_tag(bytes: &str) -> Option<&str> {
    if !bytes.starts_with('!') {
        return None;
    }
    let end = bytes
        .find(|c: char| c.is_whitespace())
        .unwrap_or(bytes.len());
    Some(&bytes[..end])
}

/// Render `value` for the slot whose current bytes are `old`, keeping the
/// slot's quote style where the content permits — the same courtesy
/// noyalib's `set_value` extends, so a write does not change how a file
/// spells its strings. Every other value takes the emitter's spelling.
///
/// # Errors
///
/// Errors when the emitter cannot spell the value.
fn render_matching_quote_style(old: &str, value: &::noyalib::Value) -> ::noyalib::Result<String> {
    if let ::noyalib::Value::String(s) = value {
        let plain = !s.contains(|c: char| c.is_control());
        if old.starts_with('"') && plain && !s.contains(['"', '\\']) {
            return Ok(format!("\"{s}\""));
        }
        if old.starts_with('\'') && plain && !s.contains('\'') {
            return Ok(format!("'{s}'"));
        }
    }
    let rendered = ::noyalib::to_string_value(value)?;
    Ok(rendered.trim_end_matches('\n').to_string())
}

/// `root` with `new` assigned at the path given by `segs`, or `None` when the
/// path does not resolve.
fn assign_at(root: &Value, segs: &[PathSeg], new: &Value) -> Option<Value> {
    let Some((first, rest)) = segs.split_first() else {
        return Some(new.clone());
    };
    match (first, root) {
        (PathSeg::Key(k), Value::Mapping(map)) => {
            let key = Value::String(k.clone());
            let child = map.get(&key)?;
            let updated = assign_at(child, rest, new)?;
            let mut m = map.clone();
            m.insert(key, updated);
            Some(Value::Mapping(m))
        }
        (PathSeg::Index(i), Value::Sequence(items)) => {
            let child = items.get(*i)?;
            let updated = assign_at(child, rest, new)?;
            let mut v = items.clone();
            v[*i] = updated;
            Some(Value::Sequence(v))
        }
        _ => None,
    }
}

/// Whether `got` differs from `expected` only where an alias of the edited
/// anchor reflects the assignment — that is, every divergent subtree is
/// exactly the `old`-to-`new` change the write asked for.
///
/// `expected` carries the assignment at the target path, so the target
/// compares equal; an alias site of the covering anchor still holds `old` in
/// `expected` and `new` in `got`, which is the write's documented meaning.
/// Any other divergence — a different value, a changed shape, a reordered
/// key — is a corrupted edit and must refuse.
fn changes_are_the_assignment(expected: &Value, got: &Value, old: &Value, new: &Value) -> bool {
    if expected == got {
        return true;
    }
    if expected == old && got == new {
        return true;
    }
    match (expected, got) {
        (Value::Mapping(a), Value::Mapping(b)) => {
            a.len() == b.len()
                && a.iter().zip(b.iter()).all(|((ka, va), (kb, vb))| {
                    ka == kb && changes_are_the_assignment(va, vb, old, new)
                })
        }
        (Value::Sequence(a), Value::Sequence(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b.iter())
                    .all(|(va, vb)| changes_are_the_assignment(va, vb, old, new))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_an_anchor_and_its_separating_spaces() {
        assert_eq!(skip_anchor_property("&x 1"), 3);
        assert_eq!(skip_anchor_property("&long  v"), 7);
        assert_eq!(skip_anchor_property("1"), 0);
    }

    #[test]
    fn reads_a_leading_tag_token() {
        assert_eq!(leading_tag("!!str 1"), Some("!!str"));
        assert_eq!(leading_tag("1"), None);
    }

    #[test]
    fn reflected_changes_accept_only_the_assignment() {
        let old = Value::Int(1);
        let new = Value::Int(9);
        // The reflection itself.
        assert!(changes_are_the_assignment(&old, &new, &old, &new));
        // An unrelated change is refused.
        assert!(!changes_are_the_assignment(
            &Value::Int(2),
            &Value::Int(3),
            &old,
            &new
        ));
        // A shape change is refused.
        assert!(!changes_are_the_assignment(
            &Value::Sequence(vec![old.clone()]),
            &Value::Sequence(vec![new.clone(), new.clone()]),
            &old,
            &new
        ));
    }
}
