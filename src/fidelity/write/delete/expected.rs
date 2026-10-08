//! The typed-value yardstick of a structural delete.
//!
//! The splice in [`super`] edits bytes; this module answers, in yqr's
//! value model, what the edit is *supposed to mean*: the document with
//! the addressed node removed ([`remove_at_path`]), and whether a
//! re-parsed candidate differs from that yardstick only where a shared
//! value reflects the removal ([`changes_are_the_deletion`]). Keeping
//! the value arithmetic apart from the byte arithmetic keeps each
//! reviewable on its own terms.

// Feature f007: the expected-value guard; f038: the reflection rule.

use crate::Value;
use crate::fidelity::PathSeg;
use crate::fidelity::noyalib::walk_value;

/// `root` (consumed) with the node at `segs` removed, or `None` when the path
/// does not address a removable mapping key / sequence index. Order is
/// preserved for mappings and indices shift for sequences, matching
/// block-delete semantics. Takes ownership so building the yardstick value does
/// not clone the whole document.
pub(super) fn remove_at_path(root: Value, segs: &[PathSeg]) -> Option<Value> {
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
pub(super) fn parent_len(root: &Value, segs: &[PathSeg]) -> Option<usize> {
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

/// Whether `got` differs from `expected` only where a subtree reflects
/// the deletion — every divergent collection is the expected one with
/// exactly the deleted segment removed, the value it loses equal to the
/// value the delete removed, and its surviving entries matching in order
/// (recursively, since a reflection can sit inside another shared
/// value).
///
/// This is `del`'s counterpart to the assignment rule
/// (`changes_are_the_assignment`): the alias sites of an anchor, and the
/// mappings a `<<` merge expanded it into, all lose the entry when the
/// definition does. Assignment's rule cannot express a merge site, where
/// the reflection is a larger mapping losing one key rather than a
/// subtree swapping old for new. Any divergence that is not this removal
/// refuses.
///
/// `deleted` — the value the target held — is load-bearing, not
/// decoration: a reflection is a copy of the shared value, so the entry
/// a site loses must have held exactly what the delete removed. Without
/// it, an over-broad splice that swallowed a same-named key elsewhere
/// (the `b036` span-defect class) would be excused by the name alone;
/// with it, such a site only passes when it also held the same value,
/// the residual coincidence class the assignment rule already accepts.
/// The caller additionally applies this rule only to documents that
/// contain an alias at all — an anchor-free document keeps the strict
/// equality check, where no reflection is possible.
// Feature f038.
pub(super) fn changes_are_the_deletion(
    expected: &Value,
    got: &Value,
    last: &PathSeg,
    deleted: &Value,
) -> bool {
    // The deleted key's `Value` form is built once; the recursion below
    // runs on every structural delete of a shared value, and a per-frame
    // allocation for the same key would be pure waste.
    let key = match last {
        PathSeg::Key(k) => Some(Value::String(k.clone())),
        PathSeg::Index(_) => None,
    };
    reflects(expected, got, last, key.as_ref(), deleted)
}

/// The recursive half of [`changes_are_the_deletion`], with the deleted
/// key pre-built.
fn reflects(
    expected: &Value,
    got: &Value,
    last: &PathSeg,
    key: Option<&Value>,
    deleted: &Value,
) -> bool {
    if expected == got {
        return true;
    }
    match (expected, got) {
        (Value::Mapping(a), Value::Mapping(b)) => {
            if a.len() == b.len() {
                return a
                    .iter()
                    .zip(b.iter())
                    .all(|((ka, va), (kb, vb))| ka == kb && reflects(va, vb, last, key, deleted));
            }
            let Some(key) = key else {
                return false;
            };
            a.len() == b.len() + 1
                && a.get(key) == Some(deleted)
                && a.iter().filter(|(ka, _)| *ka != key).zip(b.iter()).all(
                    |((ka, va), (kb, vb))| ka == kb && reflects(va, vb, last, Some(key), deleted),
                )
        }
        (Value::Sequence(a), Value::Sequence(b)) => {
            if a.len() == b.len() {
                return a
                    .iter()
                    .zip(b.iter())
                    .all(|(va, vb)| reflects(va, vb, last, key, deleted));
            }
            let PathSeg::Index(i) = last else {
                return false;
            };
            a.len() == b.len() + 1
                && a.get(*i) == Some(deleted)
                && a.iter()
                    .enumerate()
                    .filter(|(j, _)| *j != *i)
                    .map(|(_, v)| v)
                    .zip(b.iter())
                    .all(|(va, vb)| reflects(va, vb, last, key, deleted))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Value {
        crate::eval_str(".", src).unwrap().remove(0)
    }

    fn key(k: &str) -> PathSeg {
        PathSeg::Key(k.to_string())
    }

    #[test]
    fn identical_values_are_the_deletion() {
        let v = parse("a: 1\nb: 2\n");
        assert!(changes_are_the_deletion(
            &v,
            &v.clone(),
            &key("k"),
            &Value::Int(1)
        ));
    }

    #[test]
    fn a_reflection_losing_the_deleted_entry_is_accepted() {
        // The merge-site shape: a larger mapping lost exactly the key,
        // which held exactly the deleted value.
        let expected = parse("m:\n  k: 1\n  own: 3\n");
        let got = parse("m:\n  own: 3\n");
        assert!(changes_are_the_deletion(
            &expected,
            &got,
            &key("k"),
            &Value::Int(1)
        ));
    }

    #[test]
    fn losing_a_same_named_key_with_a_different_value_refuses() {
        // The b036 hazard class: an over-broad splice swallowed an
        // unrelated `k: 2`. The name matches, the value does not.
        let expected = parse("m:\n  k: 2\n  own: 3\n");
        let got = parse("m:\n  own: 3\n");
        assert!(!changes_are_the_deletion(
            &expected,
            &got,
            &key("k"),
            &Value::Int(1)
        ));
    }

    #[test]
    fn losing_a_different_key_refuses() {
        let expected = parse("m:\n  k: 1\n  own: 3\n");
        let got = parse("m:\n  k: 1\n");
        assert!(!changes_are_the_deletion(
            &expected,
            &got,
            &key("k"),
            &Value::Int(1)
        ));
    }

    #[test]
    fn a_candidate_longer_than_expected_refuses() {
        let expected = parse("m:\n  own: 3\n");
        let got = parse("m:\n  k: 1\n  own: 3\n");
        assert!(!changes_are_the_deletion(
            &expected,
            &got,
            &key("k"),
            &Value::Int(1)
        ));
    }

    #[test]
    fn an_equal_length_value_change_refuses() {
        // The layered-merge shape (`<<: [*x, *y]`): the key survives with
        // the next source's value. A changed value is not a removal, so
        // the rule refuses and the delete stays refused (f038 §3.1).
        let expected = parse("m:\n  k: 1\n  own: 3\n");
        let got = parse("m:\n  k: 9\n  own: 3\n");
        assert!(!changes_are_the_deletion(
            &expected,
            &got,
            &key("k"),
            &Value::Int(1)
        ));
    }

    #[test]
    fn a_sequence_reflection_checks_the_removed_items_value() {
        let expected = parse("- 1\n- 2\n");
        let got = parse("- 2\n");
        let last = PathSeg::Index(0);
        assert!(changes_are_the_deletion(
            &expected,
            &got,
            &last,
            &Value::Int(1)
        ));
        assert!(!changes_are_the_deletion(
            &expected,
            &got,
            &last,
            &Value::Int(7)
        ));
    }
}
