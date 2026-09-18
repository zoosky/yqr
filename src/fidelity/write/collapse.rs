//! Assignment of a scalar over a block collection: the entry collapses onto
//! its key's line.
//!
//! `.k = 5` over `k:` / `  a: 1` means `k: 5`. noyalib's `set_value` rewrites
//! only the value's span, and a block collection's value starts on the line
//! after its key, so the scalar lands there: `k:` / `  5`. That is valid YAML
//! and means `k: 5`, but it is a layout no author writes, and a reviewer
//! reading the diff sees a value that moved to the next line for no reason
//! the edit gave. Sequence items never had the problem, because an item's
//! value starts on its dash's line.
//!
//! So yqr performs the edit in two steps on a copy of the document:
//!
//! 1. Splice a `null` placeholder over everything from just after the key's
//!    `:` to the end of the value, keeping a comment on the key's own line.
//!    The removed bytes are the ones `del` removes with the entry's value:
//!    the children and the head comments above them. A comment below the
//!    last child stays, as it does for `del`.
//! 2. Write the real value over the placeholder with `set_value`. That is a
//!    scalar over a scalar, so the engine spells the value, block scalars and
//!    the document's line break included, and yqr owns no rendering.
//!
//! The copy replaces the document only when it loads as the original with
//! the assignment applied, and every byte before the `:` and after the value
//! is unchanged.
//!
//! A block collection led by a `&anchor` or `!tag` property is refused: a
//! scalar written in its place would drop the property, and an anchor may be
//! what other entries refer to. An alias to a block collection is not this
//! module's case; the engine refuses it in its own, accurate words.

// Feature f036; bug b029.

use super::anchor::assign_at;
use super::guards::after_properties;
use super::{FidelityWriter, NoyalibWriter};
use crate::Value;
use crate::error::{Result, YqrError};
use crate::fidelity::noyalib::walk_value;
use crate::fidelity::{Path, PathSeg};

impl NoyalibWriter {
    /// Whether assigning `new` at `path` replaces a block collection under a
    /// mapping key with a scalar, which is
    /// [`assign_over_block`](Self::assign_over_block)'s edit.
    ///
    /// `false` for a collection on the right-hand side, a sequence item, a
    /// flow collection (its value shares the key's line already), and an
    /// alias, whose bytes are not a block at all.
    ///
    /// # Errors
    ///
    /// Errors when `doc` is out of range.
    pub(super) fn is_scalar_over_block(
        &self,
        doc: usize,
        path: &Path,
        path_str: &str,
        new: &Value,
    ) -> Result<bool> {
        if matches!(new, Value::Sequence(_) | Value::Mapping(_))
            || !matches!(path.segments().last(), Some(PathSeg::Key(_)))
        {
            return Ok(false);
        }
        let current = self.value(doc)?;
        if !matches!(
            walk_value(&current, path.segments()),
            Some(Value::Sequence(_) | Value::Mapping(_))
        ) {
            return Ok(false);
        }
        let d = self.doc_ref(doc)?;
        if key_line_after_colon(d.source(), d.key_span(path_str)).starts_with('*') {
            return Ok(false);
        }
        Ok(!d
            .get(path_str)
            .is_some_and(|bytes| after_properties(bytes).starts_with(['[', '{'])))
    }

    /// Assign the scalar `value` at `path`, whose value is a block
    /// collection, by collapsing the entry onto its key's line.
    ///
    /// # Errors
    ///
    /// - The collection carries a `&anchor` or `!tag` property.
    /// - The key and value spans do not have the layout the splice assumes.
    /// - The engine refuses the write over the placeholder.
    /// - The result does not load as the original with the assignment
    ///   applied, or changes a byte outside the entry's value.
    pub(super) fn assign_over_block(
        &mut self,
        doc: usize,
        path: &Path,
        path_str: &str,
        value: &Value,
        rendered_value: &::noyalib::Value,
    ) -> Result<()> {
        let root = self.value(doc)?;
        let d = self.doc_ref(doc)?;
        let src = d.source();
        let layout = || {
            YqrError::eval(format!(
                "cannot assign at {path_str:?}: its source layout is not supported"
            ))
        };
        let (key_end, value_start, value_end) = match (d.key_span(path_str), d.span_at(path_str)) {
            (Some((_, key_end)), Some((start, end))) if key_end <= start => (key_end, start, end),
            _ => return Err(layout()),
        };
        if let Some(property) = leading_property(&src[value_start..value_end]) {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: the block collection there carries `{property}`, \
                 which a scalar written in its place would drop. Remove the entry and write it \
                 again, as in `del(.{path_str})` then `.{path_str} = <value>`, which places it \
                 at the end of its mapping"
            )));
        }

        // The `:` must follow the key with nothing but spaces between, and the
        // value must start on a later line; anything else is not the block
        // layout this splice is written for.
        let gap = &src[key_end..value_start];
        let colon = key_end + gap.find(':').ok_or_else(layout)?;
        if !src[key_end..colon].trim_matches([' ', '\t']).is_empty() {
            return Err(layout());
        }
        let after_colon = colon + 1;
        let line_end = after_colon
            + src[after_colon..value_start]
                .find(['\r', '\n'])
                .ok_or_else(layout)?;
        let trailing = &src[after_colon..line_end];
        let kept = if trailing.trim_start_matches([' ', '\t']).starts_with('#') {
            trailing
        } else if trailing.trim_matches([' ', '\t']).is_empty() {
            ""
        } else {
            return Err(layout());
        };

        let mut candidate = d.clone();
        candidate
            .replace_span(after_colon, value_end, &format!(" null{kept}"))
            .and_then(|()| candidate.set_value(path_str, rendered_value))
            .map_err(|e| YqrError::eval(format!("cannot assign at {path_str:?}: {e}")))?;

        let expected = assign_at(&root, path.segments(), value).ok_or_else(layout)?;
        let out = candidate.source();
        if Value::from(&*candidate.as_value()) != expected
            || !out.starts_with(&src[..after_colon])
            || !out.ends_with(&src[value_end..])
        {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: the edit would change the document outside the \
                 entry and was refused"
            )));
        }
        *self.doc_mut(doc)? = candidate;
        Ok(())
    }
}

/// The rest of the key's line after its `:`, or `""` when the key cannot be
/// located.
fn key_line_after_colon(src: &str, key_span: Option<(usize, usize)>) -> &str {
    let Some((_, key_end)) = key_span else {
        return "";
    };
    let rest = &src[key_end..];
    let Some(colon) = rest.find(':') else {
        return "";
    };
    let line = &rest[colon + 1..];
    line[..line.find(['\r', '\n']).unwrap_or(line.len())].trim_start_matches([' ', '\t'])
}

/// The first `&anchor` or `!tag` property leading `bytes`, if any.
fn leading_property(bytes: &str) -> Option<&str> {
    let rest = bytes.trim_start();
    if !rest.starts_with(['&', '!']) {
        return None;
    }
    Some(&rest[..rest.find(char::is_whitespace).unwrap_or(rest.len())])
}

#[cfg(test)]
mod tests {
    use super::super::apply;
    use super::super::testutil::*;
    use super::*;
    use crate::ast::Rhs;

    fn set(path: &str, value: Value, input: &str) -> Result<String> {
        apply(&assign(path, Rhs::Literal(value)), input)
    }

    #[test]
    fn a_scalar_replaces_a_block_on_the_keys_own_line() {
        for (path, input, want) in [
            (".k", "k:\n  a: 1\nafter: 1\n", "k: 5\nafter: 1\n"),
            (".k", "k:\n  - 1\n  - 2\nafter: 1\n", "k: 5\nafter: 1\n"),
            (".k", "k:\n- 1\n- 2\nafter: 1\n", "k: 5\nafter: 1\n"),
            (
                ".top.k",
                "top:\n  k:\n    a: 1\n  after: 2\n",
                "top:\n  k: 5\n  after: 2\n",
            ),
            (
                ".xs[0].k",
                "xs:\n  - k:\n      a: 1\n    z: 2\n",
                "xs:\n  - k: 5\n    z: 2\n",
            ),
        ] {
            assert_eq!(set(path, Value::Int(5), input).unwrap(), want, "{input:?}");
        }
    }

    #[test]
    fn a_comment_on_the_keys_line_stays_with_the_entry() {
        assert_eq!(
            set(".k", Value::Int(5), "k:  # tuned\n  a: 1\nafter: 1\n").unwrap(),
            "k: 5  # tuned\nafter: 1\n"
        );
    }

    #[test]
    fn comments_inside_the_block_go_as_del_takes_them() {
        // The head comment above a child documents that child and goes with
        // it. A comment below the last child is not the value's, for `del`
        // or here.
        let input = "# about k\nk:\n  # about a\n  a: 1\n  # trailing\nafter: 1\n";
        assert_eq!(
            set(".k", Value::Int(5), input).unwrap(),
            "# about k\nk: 5\n  # trailing\nafter: 1\n"
        );
    }

    #[test]
    fn the_engine_spells_the_value() {
        let input = "k:\n  a: 1\nafter: 1\n";
        assert_eq!(
            set(".k", Value::String("two words".into()), input).unwrap(),
            "k: two words\nafter: 1\n"
        );
        assert_eq!(
            set(".k", Value::String("yes".into()), input).unwrap(),
            "k: \"yes\"\nafter: 1\n"
        );
        assert_eq!(
            set(".k", Value::String("a\nb".into()), input).unwrap(),
            "k: |-\n  a\n  b\nafter: 1\n"
        );
        assert_eq!(
            set(".k", Value::Null, input).unwrap(),
            "k: null\nafter: 1\n"
        );
    }

    #[test]
    fn crlf_is_kept() {
        assert_eq!(
            set(
                ".k",
                Value::String("a\nb".into()),
                "k:\r\n  a: 1\r\ny: 3\r\n"
            )
            .unwrap(),
            "k: |-\r\n  a\r\n  b\r\ny: 3\r\n"
        );
    }

    #[test]
    fn a_path_right_hand_side_collapses_too() {
        // `.k = .missing` resolves to null: the shape reached without naming a
        // scalar at all.
        assert_eq!(
            assign_path(".k", ".missing", "k:\n  a: 1\n").unwrap(),
            "k: null\n"
        );
        assert_eq!(
            assign_path(".k", ".n", "n: 3\nk:\n  a: 1\n").unwrap(),
            "n: 3\nk: 3\n"
        );
    }

    #[test]
    fn a_property_on_the_block_is_refused_by_name() {
        for (input, property) in [
            ("k: &an\n  a: 1\nafter: 1\n", "&an"),
            ("k: &an\n  a: 1\nj: *an\n", "&an"),
            ("k: !t\n  a: 1\nafter: 1\n", "!t"),
        ] {
            let err = set(".k", Value::Int(5), input).unwrap_err().to_string();
            assert!(
                err.contains(&format!("carries `{property}`")),
                "{input:?}: {err}"
            );
        }
    }

    #[test]
    fn an_alias_to_a_block_is_the_engines_refusal() {
        let err = set(".k", Value::Int(5), "a: &x\n  n: 1\nk: *x\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("alias"), "{err}");
        assert!(!err.contains("block collection"), "wrong diagnosis: {err}");
    }

    #[test]
    fn leading_property_reads_the_first_token() {
        assert_eq!(leading_property("&an\n  a: 1"), Some("&an"));
        assert_eq!(leading_property("!t\n  a: 1"), Some("!t"));
        assert_eq!(leading_property("  a: 1"), None);
    }

    #[test]
    fn key_line_after_colon_skips_to_the_value_or_comment() {
        let src = "k:  *x\n";
        assert_eq!(key_line_after_colon(src, Some((0, 1))), "*x");
        assert_eq!(key_line_after_colon("k:\n  a: 1\n", Some((0, 1))), "");
        assert_eq!(key_line_after_colon(src, None), "");
    }
}
