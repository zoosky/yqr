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
/// exactly the deleted segment removed, its surviving entries matching
/// in order (recursively, since a reflection can sit inside another
/// shared value).
///
/// This is `del`'s counterpart to the assignment rule
/// (`changes_are_the_assignment`): the alias sites of an anchor, and the
/// mappings a `<<` merge expanded it into, all lose the entry when the
/// definition does. Assignment's rule cannot express a merge site, where
/// the reflection is a larger mapping losing one key rather than a
/// subtree swapping old for new. Any divergence that is not this removal
/// refuses.
// Feature f038.
pub(super) fn changes_are_the_deletion(expected: &Value, got: &Value, last: &PathSeg) -> bool {
    if expected == got {
        return true;
    }
    match (expected, got) {
        (Value::Mapping(a), Value::Mapping(b)) => {
            if a.len() == b.len() {
                return a.iter().zip(b.iter()).all(|((ka, va), (kb, vb))| {
                    ka == kb && changes_are_the_deletion(va, vb, last)
                });
            }
            let PathSeg::Key(key) = last else {
                return false;
            };
            let key = Value::String(key.clone());
            a.len() == b.len() + 1
                && a.contains_key(&key)
                && a.iter()
                    .filter(|(ka, _)| **ka != key)
                    .zip(b.iter())
                    .all(|((ka, va), (kb, vb))| ka == kb && changes_are_the_deletion(va, vb, last))
        }
        (Value::Sequence(a), Value::Sequence(b)) => {
            if a.len() == b.len() {
                return a
                    .iter()
                    .zip(b.iter())
                    .all(|(va, vb)| changes_are_the_deletion(va, vb, last));
            }
            let PathSeg::Index(i) = last else {
                return false;
            };
            a.len() == b.len() + 1
                && *i < a.len()
                && a.iter()
                    .enumerate()
                    .filter(|(j, _)| *j != *i)
                    .map(|(_, v)| v)
                    .zip(b.iter())
                    .all(|(va, vb)| changes_are_the_deletion(va, vb, last))
        }
        _ => false,
    }
}
