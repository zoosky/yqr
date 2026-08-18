//! The evaluation engine: applies an [`Ast`] to a [`Value`], producing a
//! stream (`Vec<Value>`) of results, mirroring jq's model where any filter can
//! yield zero or more outputs.
//!
//! Every result additionally carries *provenance*: the concrete [`Path`] it
//! was derived from, or `None` when it was computed rather than selected. The
//! fidelity engine uses that path to emit untouched nodes by slicing their
//! original bytes; the classic pipeline simply discards it.

use crate::Value;

use crate::ast::{Ast, Rhs};
use crate::error::{Result, YqrError};
use crate::fidelity::{Path, PathSeg};

/// A result value paired with the concrete path it was selected from
/// (`None` for computed values with no single source node).
pub(crate) type Traced = (Value, Option<Path>);

/// Evaluate `ast` against a single input `value`, returning the output stream.
pub fn eval(ast: &Ast, value: &Value) -> Result<Vec<Value>> {
    // Feature f002: `eval` is a thin projection of the traced evaluation.
    Ok(eval_traced(ast, value, None)?
        .into_iter()
        .map(|(v, _)| v)
        .collect())
}

/// Evaluate `ast` against `value`, threading the concrete path each produced
/// value was derived from. `path` is the provenance of `value` itself
/// (`None` disables path tracking entirely, which is the classic pipeline).
pub(crate) fn eval_traced(ast: &Ast, value: &Value, path: Option<&Path>) -> Result<Vec<Traced>> {
    match ast {
        Ast::Identity => Ok(vec![(value.clone(), path.cloned())]),
        Ast::Field(name) => Ok(vec![field(value, name, path)?]),
        Ast::Index(idx) => Ok(vec![index(value, *idx, path)?]),
        Ast::Iterate => iterate(value, path),
        Ast::Pipe(lhs, rhs) => {
            let mut out = Vec::new();
            for (v, p) in eval_traced(lhs, value, path)? {
                out.extend(eval_traced(rhs, &v, p.as_ref())?);
            }
            Ok(out)
        }
        Ast::Optional(inner) => match eval_traced(inner, value, path) {
            Ok(vs) => Ok(vs),
            Err(_) => Ok(Vec::new()),
        },
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Int(_) => "number",
        Value::Float(_) => "number",
        Value::String(_) => "string",
        Value::Sequence(_) => "array",
        Value::Mapping(_) => "object",
    }
}

fn field(value: &Value, name: &str, path: Option<&Path>) -> Result<Traced> {
    match value {
        Value::Null => Ok((Value::Null, None)),
        Value::Mapping(map) => {
            let key = Value::String(name.to_string());
            // Absent members share one convention with out-of-range indexing:
            // null with no provenance (there is no source node to slice).
            Ok(map.get(&key).map_or((Value::Null, None), |v| {
                (
                    v.clone(),
                    path.map(|p| p.child(PathSeg::Key(name.to_string()))),
                )
            }))
        }
        other => Err(YqrError::eval(format!(
            "cannot index {} with field {:?}",
            type_name(other),
            name
        ))),
    }
}

/// Resolve a possibly-negative sequence index against a length, or `None`
/// when it falls outside the sequence.
///
/// A negative index counts from the end, so `-1` is the last item. Shared with
/// the write path's reorder verb, whose indices are specified to resolve
/// "as `.[-1]` does" — one implementation is what keeps that true.
// Feature f007: shared with `swap`/`move`.
pub(crate) fn resolve_seq_index(idx: i64, len: usize) -> Option<usize> {
    let len = i64::try_from(len).ok()?;
    let resolved = if idx < 0 { len + idx } else { idx };
    if resolved < 0 || resolved >= len {
        return None;
    }
    usize::try_from(resolved).ok()
}

fn index(value: &Value, idx: i64, path: Option<&Path>) -> Result<Traced> {
    match value {
        Value::Null => Ok((Value::Null, None)),
        Value::Sequence(items) => {
            // Out of range yields null with no source node to slice.
            let Some(i) = resolve_seq_index(idx, items.len()) else {
                return Ok((Value::Null, None));
            };
            let child = path.map(|p| p.child(PathSeg::Index(i)));
            Ok((items[i].clone(), child))
        }
        other => Err(YqrError::eval(format!(
            "cannot index {} with number {}",
            type_name(other),
            idx
        ))),
    }
}

fn iterate(value: &Value, path: Option<&Path>) -> Result<Vec<Traced>> {
    match value {
        Value::Sequence(items) => Ok(items
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), path.map(|p| p.child(PathSeg::Index(i)))))
            .collect()),
        Value::Mapping(map) => Ok(map
            .iter()
            .map(|(k, v)| {
                // Only string keys are path-addressable; values under other
                // key types are still yielded, just without provenance.
                let child = match k {
                    Value::String(s) => path.map(|p| p.child(PathSeg::Key(s.clone()))),
                    _ => None,
                };
                (v.clone(), child)
            })
            .collect()),
        other => Err(YqrError::eval(format!(
            "cannot iterate over {}",
            type_name(other)
        ))),
    }
}

// -- Feature f006: mutation-target resolution ---------------------------------

/// Where an assignment's left-hand side lands.
///
/// A path whose final segment already exists overwrites that node in place
/// ([`AssignTarget::Existing`]); a path whose final segment is an absent mapping
/// key creates it under an existing parent ([`AssignTarget::NewKey`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AssignTarget {
    /// Overwrite the node already at this path.
    Existing(Path),
    /// Create a new mapping key `key` under the mapping at `parent`.
    NewKey {
        /// Path of the (existing) parent mapping.
        parent: Path,
        /// The new key to insert.
        key: String,
    },
}

/// Resolve a mutation's left-hand path to the single node it targets.
///
/// Returns `Ok(Some(path))` when the filter selects exactly one node that
/// exists in `value`, `Ok(None)` when the path is simply absent (the caller
/// leaves that document untouched), and an error when the filter selects more
/// than one node — a mutation must address exactly one node.
///
/// # Errors
///
/// Errors when the filter fails to evaluate, or selects zero or more than one
/// result (a mutation is not a stream).
pub(crate) fn resolve_target(ast: &Ast, value: &Value) -> Result<Option<Path>> {
    let mut traced = eval_traced(ast, value, Some(&Path::root()))?;
    if traced.len() != 1 {
        return Err(YqrError::eval(format!(
            "a mutation must target exactly one node, but the filter selected {}",
            traced.len()
        )));
    }
    // `pop` is safe: length was just checked to be 1.
    Ok(traced.pop().expect("length checked to be 1").1)
}

/// Resolve an assignment's left-hand path into a concrete [`AssignTarget`].
///
/// An existing node overwrites in place. An absent *final mapping key* under an
/// existing parent creates a new entry. Any other absence (a missing parent, an
/// out-of-range index, an absent non-key leaf) yields `Ok(None)` so the caller
/// leaves the document untouched.
///
/// # Errors
///
/// Propagates the "exactly one node" contract from [`resolve_target`].
pub(crate) fn resolve_assign_target(ast: &Ast, value: &Value) -> Result<Option<AssignTarget>> {
    if let Some(path) = resolve_target(ast, value)? {
        return Ok(Some(AssignTarget::Existing(path)));
    }
    // Absent leaf: the only node yqr can create is a new mapping key.
    let Some((parent_ast, key)) = final_field(ast) else {
        return Ok(None);
    };
    let parent = match parent_ast {
        None => Path::root(),
        Some(inner) => match resolve_target(inner, value)? {
            Some(path) => path,
            None => return Ok(None), // parent does not exist in this document
        },
    };
    Ok(Some(AssignTarget::NewKey { parent, key }))
}

/// Split a path AST into `(parent, final_key)` when its last step is a field
/// access, so an assignment to an absent key can be routed to key insertion.
///
/// A `None` parent denotes the document root (a bare `.foo`).
fn final_field(ast: &Ast) -> Option<(Option<&Ast>, String)> {
    match ast {
        Ast::Field(key) => Some((None, key.clone())),
        Ast::Pipe(lhs, rhs) => match rhs.as_ref() {
            Ast::Field(key) => Some((Some(lhs.as_ref()), key.clone())),
            _ => None,
        },
        _ => None,
    }
}

/// Resolve the right-hand side of an assignment/append to a single [`Value`].
///
/// A literal is returned as-is; a path is evaluated against the same document
/// and must select exactly one value.
///
/// # Errors
///
/// Errors when a path RHS selects zero or more than one value.
pub(crate) fn resolve_rhs(rhs: &Rhs, value: &Value) -> Result<Value> {
    match rhs {
        Rhs::Literal(v) => Ok(v.clone()),
        Rhs::Path(ast) => {
            let mut out = eval(ast, value)?;
            match out.len() {
                1 => Ok(out.pop().expect("length checked to be 1")),
                0 => Err(YqrError::eval(
                    "right-hand path selected no value".to_string(),
                )),
                n => Err(YqrError::eval(format!(
                    "right-hand path selected {n} values; assignment needs exactly one"
                ))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(src: &str) -> Value {
        noyalib::from_str::<noyalib::Value>(src)
            .map(Value::from)
            .expect("valid yaml")
    }

    fn run(filter: &str, yaml: &str) -> Result<Vec<Value>> {
        let ast = crate::parser::parse(filter).expect("valid filter");
        eval(&ast, &load(yaml))
    }

    fn run_traced(filter: &str, yaml: &str) -> Vec<Traced> {
        let ast = crate::parser::parse(filter).expect("valid filter");
        eval_traced(&ast, &load(yaml), Some(&Path::root())).expect("evaluates")
    }

    #[test]
    fn identity_returns_input() {
        let out = run(".", "a: 1").unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], load("a: 1"));
    }

    #[test]
    fn field_access() {
        let out = run(".name", "name: alice\nage: 30").unwrap();
        assert_eq!(out, vec![Value::String("alice".into())]);
    }

    #[test]
    fn missing_field_is_null() {
        let out = run(".nope", "name: alice").unwrap();
        assert_eq!(out, vec![Value::Null]);
    }

    #[test]
    fn nested_field_access() {
        let out = run(".user.name", "user:\n  name: bob").unwrap();
        assert_eq!(out, vec![Value::String("bob".into())]);
    }

    #[test]
    fn index_positive_and_negative() {
        assert_eq!(run(".[0]", "[10, 20, 30]").unwrap(), vec![Value::Int(10)]);
        assert_eq!(run(".[-1]", "[10, 20, 30]").unwrap(), vec![Value::Int(30)]);
    }

    #[test]
    fn index_out_of_range_is_null() {
        assert_eq!(run(".[9]", "[1, 2]").unwrap(), vec![Value::Null]);
    }

    #[test]
    fn iterate_sequence() {
        let out = run(".[]", "[1, 2, 3]").unwrap();
        assert_eq!(out, vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    }

    #[test]
    fn iterate_mapping_values() {
        let out = run(".[]", "a: 1\nb: 2").unwrap();
        assert_eq!(out, vec![Value::Int(1), Value::Int(2)]);
    }

    #[test]
    fn pipe_iterate_then_field() {
        let yaml = "items:\n  - name: x\n  - name: y";
        let out = run(".items[].name", yaml).unwrap();
        assert_eq!(
            out,
            vec![Value::String("x".into()), Value::String("y".into())]
        );
    }

    #[test]
    fn iterate_over_scalar_errors() {
        assert!(matches!(run(".[]", "5"), Err(YqrError::Eval(_))));
    }

    #[test]
    fn optional_suppresses_error() {
        assert_eq!(run(".[]?", "5").unwrap(), Vec::<Value>::new());
    }

    #[test]
    fn field_on_scalar_errors() {
        assert!(matches!(run(".foo", "5"), Err(YqrError::Eval(_))));
    }

    // -- provenance threading -------------------------------------------------

    #[test]
    fn identity_carries_root_path() {
        let out = run_traced(".", "a: 1");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].1, Some(Path::root()));
    }

    #[test]
    fn field_and_index_extend_the_path() {
        let out = run_traced(".items[0]", "items:\n  - x\n  - y");
        assert_eq!(
            out[0].1,
            Some(
                Path::root()
                    .child(PathSeg::Key("items".into()))
                    .child(PathSeg::Index(0))
            )
        );
    }

    #[test]
    fn negative_index_resolves_in_the_path() {
        let out = run_traced(".[-1]", "[10, 20, 30]");
        assert_eq!(out[0].1, Some(Path::root().child(PathSeg::Index(2))));
    }

    #[test]
    fn out_of_range_index_has_no_path() {
        let out = run_traced(".[9]", "[1, 2]");
        assert_eq!(out[0], (Value::Null, None));
    }

    #[test]
    fn iteration_branches_paths_per_element() {
        let out = run_traced(".[]", "a: 1\nb: 2");
        let paths: Vec<_> = out.into_iter().map(|(_, p)| p).collect();
        assert_eq!(
            paths,
            vec![
                Some(Path::root().child(PathSeg::Key("a".into()))),
                Some(Path::root().child(PathSeg::Key("b".into()))),
            ]
        );
    }

    #[test]
    fn missing_field_has_no_path() {
        // Same convention as out-of-range indexing: absent -> null without
        // provenance (there is no source node the engine could slice).
        let out = run_traced(".nope", "a: 1");
        assert_eq!(out[0], (Value::Null, None));
    }

    #[test]
    fn untracked_evaluation_yields_no_paths() {
        let ast = crate::parser::parse(".a").expect("valid filter");
        let out = eval_traced(&ast, &load("a: 1"), None).unwrap();
        assert_eq!(out[0].1, None);
    }

    // -- Feature f006: mutation-target resolution ------------------------------

    fn target(filter: &str, yaml: &str) -> Result<Option<Path>> {
        let ast = crate::parser::parse(filter).expect("valid filter");
        resolve_target(&ast, &load(yaml))
    }

    #[test]
    fn resolve_target_single_existing_node() {
        let got = target(".spec.replicas", "spec:\n  replicas: 3\n").unwrap();
        assert_eq!(
            got,
            Some(
                Path::root()
                    .child(PathSeg::Key("spec".into()))
                    .child(PathSeg::Key("replicas".into()))
            )
        );
    }

    #[test]
    fn resolve_target_absent_is_none() {
        assert_eq!(target(".nope", "a: 1\n").unwrap(), None);
    }

    #[test]
    fn resolve_target_multiple_nodes_errors() {
        // Iteration selects a stream; a mutation needs exactly one node.
        assert!(matches!(
            target(".items[]", "items:\n  - 1\n  - 2\n"),
            Err(YqrError::Eval(_))
        ));
    }

    #[test]
    fn assign_target_existing_key() {
        let ast = crate::parser::parse(".a").expect("valid");
        let got = resolve_assign_target(&ast, &load("a: 1\n")).unwrap();
        assert_eq!(
            got,
            Some(AssignTarget::Existing(
                Path::root().child(PathSeg::Key("a".into()))
            ))
        );
    }

    #[test]
    fn assign_target_new_key_routes_to_parent() {
        let ast = crate::parser::parse(".metadata.env").expect("valid");
        let got = resolve_assign_target(&ast, &load("metadata:\n  name: app\n")).unwrap();
        assert_eq!(
            got,
            Some(AssignTarget::NewKey {
                parent: Path::root().child(PathSeg::Key("metadata".into())),
                key: "env".into(),
            })
        );
    }

    #[test]
    fn assign_target_new_top_level_key_has_root_parent() {
        let ast = crate::parser::parse(".added").expect("valid");
        let got = resolve_assign_target(&ast, &load("a: 1\n")).unwrap();
        assert_eq!(
            got,
            Some(AssignTarget::NewKey {
                parent: Path::root(),
                key: "added".into(),
            })
        );
    }

    #[test]
    fn assign_target_absent_parent_is_none() {
        // `.a` does not exist, so `.a.b = ...` cannot create anything here.
        let ast = crate::parser::parse(".a.b").expect("valid");
        assert_eq!(resolve_assign_target(&ast, &load("z: 1\n")).unwrap(), None);
    }

    #[test]
    fn resolve_rhs_literal_and_path() {
        let value = load("a: 1\nsrc:\n  inner: hi\n");
        let lit = crate::ast::Rhs::Literal(Value::Int(7));
        assert_eq!(resolve_rhs(&lit, &value).unwrap(), Value::Int(7));

        let path = crate::ast::Rhs::Path(crate::parser::parse(".src.inner").expect("valid"));
        assert_eq!(
            resolve_rhs(&path, &value).unwrap(),
            Value::String("hi".into())
        );
    }

    #[test]
    fn resolve_rhs_multi_valued_path_errors() {
        let value = load("items:\n  - 1\n  - 2\n");
        let path = crate::ast::Rhs::Path(crate::parser::parse(".items[]").expect("valid"));
        assert!(matches!(resolve_rhs(&path, &value), Err(YqrError::Eval(_))));
    }
}
