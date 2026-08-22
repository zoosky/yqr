//! The evaluation engine: applies an [`Ast`] to a [`Value`], producing a
//! stream (`Vec<Value>`) of results, mirroring jq's model where any filter can
//! yield zero or more outputs.
//!
//! Every result additionally carries *provenance*: the concrete [`Path`] it
//! was derived from, or `None` when it was computed rather than selected. The
//! fidelity engine uses that path to emit untouched nodes by slicing their
//! original bytes; the classic pipeline simply discards it.

use crate::Value;

use crate::ast::{Ast, BinOp, Builtin, Rhs};
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
        // A builtin computes its output, so it has no source node and hands
        // on `None` — which is exactly the provenance the renderer needs to
        // fall back to the typed emitter (`yqr-f017` §6).
        Ast::Builtin(Builtin::ToEntries) => Ok(vec![(to_entries(value)?, None)]),
        // Computed, like a builtin: no source node, so no provenance.
        // Feature f008.
        Ast::Literal(v) => Ok(vec![(v.clone(), None)]),
        Ast::Binary(op, lhs, rhs) => {
            let a = eval_single(lhs, value, &format!("the left side of '{}'", op.symbol()))?;
            let b = eval_single(rhs, value, &format!("the right side of '{}'", op.symbol()))?;
            Ok(vec![(arithmetic(*op, &a, &b)?, None)])
        }
    }
}

/// Apply an arithmetic operator to two values.
///
/// The number model is `yqr-a001` §6's, ratified long before anything needed
/// it: **preserve types**. `Int op Int` stays `Int` while the result is exact,
/// and becomes `Float` only when the operation is genuinely fractional. That
/// is a fidelity rule rather than a numeric-tower preference — `replicas: 3`
/// must not become `3.0`, and an `i64` identifier must not lose precision on
/// the way through an `f64`.
///
/// Overflow is an **error**, not a promotion. Promoting to `Float` is exactly
/// the precision loss the rule exists to prevent, so it cannot be the
/// overflow strategy.
///
/// `+` also concatenates strings, which is the one non-numeric case jq and
/// every other language agree on. Mixed operands are refused, naming both
/// types, rather than coerced.
///
/// # Errors
///
/// Returns an error for non-numeric operands, mixed string/number operands,
/// division or remainder by zero, and `i64` overflow.
// Feature f008.
fn arithmetic(op: BinOp, a: &Value, b: &Value) -> Result<Value> {
    // String concatenation, the only non-numeric operation in scope.
    if let (Value::String(x), Value::String(y)) = (a, b) {
        return if op == BinOp::Add {
            Ok(Value::String(format!("{x}{y}")))
        } else {
            Err(YqrError::eval(format!(
                "cannot apply '{}' to two strings; only '+' concatenates",
                op.symbol()
            )))
        };
    }

    match (a, b) {
        (Value::Int(x), Value::Int(y)) => int_arithmetic(op, *x, *y),
        // Any float operand makes the result a float; there is no exactness
        // to preserve once one side already lost it.
        (Value::Int(_) | Value::Float(_), Value::Int(_) | Value::Float(_)) => {
            float_arithmetic(op, as_f64(a), as_f64(b))
        }
        _ => Err(YqrError::eval(format!(
            "cannot apply '{}' to {} and {}",
            op.symbol(),
            type_name(a),
            type_name(b)
        ))),
    }
}

/// `Int op Int`, staying `Int` while the result is exact.
// Feature f008.
fn int_arithmetic(op: BinOp, x: i64, y: i64) -> Result<Value> {
    let overflow = || {
        YqrError::eval(format!(
            "integer overflow in {x} {} {y}; the result does not fit i64, and \
             widening it to a float would lose the precision fidelity protects",
            op.symbol()
        ))
    };
    match op {
        BinOp::Add => x.checked_add(y).map(Value::Int).ok_or_else(overflow),
        BinOp::Sub => x.checked_sub(y).map(Value::Int).ok_or_else(overflow),
        BinOp::Mul => x.checked_mul(y).map(Value::Int).ok_or_else(overflow),
        BinOp::Div => {
            if y == 0 {
                return Err(YqrError::eval(format!("division by zero in {x} / {y}")));
            }
            // Exact division stays an integer; anything else is genuinely
            // fractional and becomes one. `4 / 2` is `2`, `3 / 2` is `1.5`.
            //
            // `checked_rem`, not `%`: the one overflowing case is
            // `i64::MIN % -1`, which panics the process on a bare `%`. It is
            // also exactly the case `checked_div` would reject a line later,
            // so the exactness test has to survive long enough to get there.
            let Some(remainder) = x.checked_rem(y) else {
                return Err(overflow());
            };
            if remainder == 0 {
                x.checked_div(y).map(Value::Int).ok_or_else(overflow)
            } else {
                #[allow(clippy::cast_precision_loss)]
                Ok(Value::Float(x as f64 / y as f64))
            }
        }
        BinOp::Rem => {
            if y == 0 {
                return Err(YqrError::eval(format!("remainder by zero in {x} % {y}")));
            }
            x.checked_rem(y).map(Value::Int).ok_or_else(overflow)
        }
    }
}

/// Arithmetic once either operand is a float.
// Feature f008.
fn float_arithmetic(op: BinOp, x: f64, y: f64) -> Result<Value> {
    match op {
        BinOp::Add => Ok(Value::Float(x + y)),
        BinOp::Sub => Ok(Value::Float(x - y)),
        BinOp::Mul => Ok(Value::Float(x * y)),
        BinOp::Div if y == 0.0 => Err(YqrError::eval(format!("division by zero in {x} / {y}"))),
        BinOp::Div => Ok(Value::Float(x / y)),
        BinOp::Rem if y == 0.0 => Err(YqrError::eval(format!("remainder by zero in {x} % {y}"))),
        BinOp::Rem => Ok(Value::Float(x % y)),
    }
}

/// Widen a numeric value for float arithmetic.
#[allow(clippy::cast_precision_loss)]
fn as_f64(v: &Value) -> f64 {
    match v {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => f64::NAN,
    }
}

/// `to_entries` — a mapping becomes a sequence of `{key, value}` pairs.
///
/// The field names are jq's, kept deliberately: a shape nobody can transfer to
/// the tool next door is worth much less than one they can.
///
/// Order is the mapping's own, not sorted. yqr's value model is
/// insertion-ordered all the way from the parser, and the pairing this builtin
/// exists to provide is only sound if the pairs come out in the order the
/// entries were written — a sorted `to_entries` would silently break the
/// stream alignment it is meant to replace. jq sorts object keys; here that
/// difference is load-bearing rather than cosmetic (`yqr-f017` §4).
///
/// The key is cloned, never re-typed. On a document that means it is always a
/// string — `1: one` pairs as `key: "1"` — because the engine's typed mapping
/// is string-keyed and the conversion at the parse boundary has already
/// decided. yqr's own model is `Value`-keyed and would carry an `Int` here
/// unchanged; re-deciding that in a builtin would be making the parse
/// boundary's call a second time, in the wrong place and quietly.
///
/// # Errors
///
/// Returns an error naming the actual type when the input is not a mapping;
/// jq refuses the same inputs.
// Feature f017.
fn to_entries(value: &Value) -> Result<Value> {
    let Value::Mapping(map) = value else {
        return Err(YqrError::eval(format!(
            "to_entries takes an object, but this is {}; \
             it turns a mapping's entries into {{key, value}} pairs, so there \
             is nothing for it to enumerate here",
            type_name(value)
        )));
    };

    Ok(Value::Sequence(
        map.iter()
            .map(|(k, v)| {
                let mut pair = crate::value::Mapping::new();
                pair.insert(Value::String("key".to_string()), k.clone());
                pair.insert(Value::String("value".to_string()), v.clone());
                Value::Mapping(pair)
            })
            .collect(),
    ))
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
    ///
    /// Carries the value currently there, not only the path. An assignment
    /// whose new value equals the old one must not write: `set_value`
    /// re-emits the scalar from the typed model, which cannot carry a
    /// number's spelling, so writing `0640` back as the same `Int` emits
    /// `640`. Comparing needs the old value, so the resolver returns it.
    // Feature f006; the value added for `yqr-b018`.
    Existing {
        /// Where the node is.
        path: Path,
        /// What is in it.
        current: Value,
    },
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

/// Resolve an update's left-hand path to the node it targets **and** that
/// node's current value.
///
/// [`resolve_target`] keeps the path and drops the value, which is all `=`
/// needs. `|=` needs both: the path to write through, and the value to hand
/// the right-hand filter. Same single-node contract, same absent-path skip.
///
/// # Errors
///
/// Propagates the "exactly one node" contract from [`resolve_target`].
// Feature f008.
pub(crate) fn resolve_update_target(ast: &Ast, value: &Value) -> Result<Option<(Path, Value)>> {
    let mut traced = eval_traced(ast, value, Some(&Path::root()))?;
    if traced.len() != 1 {
        return Err(YqrError::eval(format!(
            "a mutation must target exactly one node, but the filter selected {}",
            traced.len()
        )));
    }
    // `pop` is safe: length was just checked to be 1.
    let (found, path) = traced.pop().expect("length checked to be 1");
    Ok(path.map(|p| (p, found)))
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
    if let Some((path, current)) = resolve_update_target(ast, value)? {
        return Ok(Some(AssignTarget::Existing { path, current }));
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
        Rhs::Path(ast) => eval_single(ast, value, "the right-hand path"),
    }
}

/// Evaluate `ast` against `value` and require exactly one result.
///
/// Shared by `=`'s path right-hand side, `|=`'s filter, and each side of a
/// binary operator — three callers that differ in what they evaluate against
/// and in what they are *for*. `what` names the caller so the diagnostic does
/// too: a `+` inside a read query has no right-hand side and performs no
/// write, and saying otherwise sends the reader looking for an assignment
/// that is not there.
// Feature f006, generalised for f008.
pub(crate) fn eval_single(ast: &Ast, value: &Value, what: &str) -> Result<Value> {
    let mut out = eval(ast, value)?;
    match out.len() {
        1 => Ok(out.pop().expect("length checked to be 1")),
        0 => Err(YqrError::eval(format!("{what} selected no value"))),
        n => Err(YqrError::eval(format!(
            "{what} selected {n} values, but exactly one is needed"
        ))),
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
            Some(AssignTarget::Existing {
                path: Path::root().child(PathSeg::Key("a".into())),
                // The resolver carries what is there, so the caller can tell
                // an assignment that changes nothing from one that does
                // (`yqr-b018`).
                current: Value::Int(1),
            })
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

    // -- Feature f017: to_entries ------------------------------------------

    #[test]
    fn to_entries_pairs_each_key_with_its_value() {
        let out = run(".m | to_entries", "m:\n  a: 1\n  b: two\n").unwrap();
        assert_eq!(out.len(), 1, "one sequence, not one output per entry");
        assert_eq!(
            out[0],
            load("- key: a\n  value: 1\n- key: b\n  value: two\n")
        );
    }

    #[test]
    fn to_entries_keeps_document_order_not_sorted_order() {
        // The keys are deliberately not in sorted order, and not in reverse
        // either, so neither a forward nor a backward sort could pass. jq
        // sorts object keys; the §2 pairing this builtin exists for is only
        // sound if these come out as written.
        let out = run(
            ".m | to_entries[] | .key",
            "m:\n  zebra: 1\n  apple: 2\n  mango: 3\n",
        )
        .unwrap();
        let keys: Vec<&str> = out
            .iter()
            .map(|v| match v {
                Value::String(s) => s.as_str(),
                other => panic!("expected a string key, got {other:?}"),
            })
            .collect();
        assert_eq!(keys, ["zebra", "apple", "mango"]);
    }

    #[test]
    fn to_entries_output_is_computed_so_it_carries_no_provenance() {
        // The pairs exist in no file, so there is no span to slice. This is
        // what routes the result through the typed renderer rather than
        // through the byte path (`yqr-f017` §6).
        let traced = run_traced(".m | to_entries", "m:\n  a: 1\n");
        assert_eq!(traced.len(), 1);
        assert!(
            traced[0].1.is_none(),
            "a computed value must not claim a source path"
        );
    }

    #[test]
    fn to_entries_streams_its_pairs_when_iterated() {
        let out = run(".m | to_entries[]", "m:\n  a: 1\n  b: 2\n").unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], load("key: a\nvalue: 1\n"));
        assert_eq!(out[1], load("key: b\nvalue: 2\n"));
    }

    #[test]
    fn to_entries_carries_the_key_it_was_given_without_re_typing_it() {
        // A YAML key that looks like a number or a boolean arrives here as a
        // *string*, and not because of anything this builtin does: the engine's
        // typed mapping is string-keyed (`yqr-b002` §2.7), so the conversion at
        // the parse boundary has already decided. `to_entries` clones the key
        // it is handed and does not re-type it, which is what keeps this the
        // engine's decision to change rather than one buried in a builtin.
        let out = run(".m | to_entries[] | .key", "m:\n  1: one\n  true: yes\n").unwrap();
        assert_eq!(
            out,
            vec![Value::String("1".into()), Value::String("true".into())]
        );
    }

    #[test]
    fn to_entries_on_a_non_mapping_names_the_actual_type() {
        for (yaml, want) in [
            ("m:\n  - 1\n", "array"),
            ("m: 1\n", "number"),
            ("m: hi\n", "string"),
            ("m:\n", "null"),
        ] {
            let err = run(".m | to_entries", yaml).unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains(want),
                "{yaml:?}: message should name {want}, got {msg}"
            );
        }
    }

    #[test]
    fn to_entries_of_an_empty_mapping_is_an_empty_sequence() {
        let out = run(".m | to_entries", "m: {}\n").unwrap();
        assert_eq!(out, vec![Value::Sequence(Vec::new())]);
    }

    // -- Feature f008: arithmetic ------------------------------------------

    #[test]
    fn int_arithmetic_stays_int_while_exact() {
        // a001 §6: `replicas: 3` must not become `3.0`.
        for (filter, want) in [
            ("3 + 1", Value::Int(4)),
            ("3 - 1", Value::Int(2)),
            ("3 * 2", Value::Int(6)),
            ("4 / 2", Value::Int(2)),
            ("7 % 3", Value::Int(1)),
        ] {
            assert_eq!(run(filter, "a: 0").unwrap(), vec![want], "{filter}");
        }
    }

    #[test]
    fn division_becomes_float_only_when_genuinely_fractional() {
        assert_eq!(run("3 / 2", "a: 0").unwrap(), vec![Value::Float(1.5)]);
        assert_eq!(run("4 / 2", "a: 0").unwrap(), vec![Value::Int(2)]);
        // A float operand carries: there is no exactness left to preserve.
        assert_eq!(run("1.5 + 1", "a: 0").unwrap(), vec![Value::Float(2.5)]);
    }

    #[test]
    fn overflow_is_an_error_not_a_promotion_to_float() {
        // Promoting would lose the precision the preserve-types rule exists
        // to protect, so it cannot be the overflow strategy.
        let err = run(&format!("{} + 1", i64::MAX), "a: 0").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("overflow"), "{msg}");
        assert!(msg.contains("float"), "the message must say why not: {msg}");
    }

    #[test]
    fn i64_min_divided_by_minus_one_errors_rather_than_panicking() {
        // The exactness pre-check used a bare `%`, and `i64::MIN % -1`
        // overflows -- panicking the process before `checked_div` could
        // report it. The one input where the remainder itself overflows.
        let err = run(&format!("{} / -1", i64::MIN), "a: 0").unwrap_err();
        assert!(err.to_string().contains("overflow"), "{err}");
        // Its sibling: the same operands through `%`.
        let err = run(&format!("{} % -1", i64::MIN), "a: 0").unwrap_err();
        assert!(err.to_string().contains("overflow"), "{err}");
    }

    #[test]
    fn an_arity_error_in_a_read_query_does_not_mention_writing() {
        // `eval_single` is shared with the write path; a `+` in a read filter
        // has no right-hand side and performs no write, so saying otherwise
        // sends the reader looking for an assignment that is not there.
        let err = run(".xs[] + 1", "xs:\n  - 1\n  - 2\n").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("left side of '+'"), "{msg}");
        assert!(
            !msg.contains("write"),
            "a read query must not mention writing: {msg}"
        );
    }

    #[test]
    fn division_and_remainder_by_zero_are_errors() {
        assert!(run("1 / 0", "a: 0").is_err());
        assert!(run("1 % 0", "a: 0").is_err());
        assert!(run("1.0 / 0", "a: 0").is_err());
    }

    #[test]
    fn plus_concatenates_strings_and_nothing_else_does() {
        assert_eq!(
            run(r#""a" + "b""#, "x: 0").unwrap(),
            vec![Value::String("ab".into())]
        );
        let err = run(r#""a" - "b""#, "x: 0").unwrap_err();
        assert!(err.to_string().contains("only \'+\' concatenates"), "{err}");
    }

    #[test]
    fn mixed_operands_are_refused_naming_both_types() {
        let err = run(r#".s + 1"#, "s: hi").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("string") && msg.contains("number"), "{msg}");
    }

    #[test]
    fn arithmetic_reads_the_input_like_any_other_filter() {
        // Not a `|=`-only construct: the evaluator is shared, so it works in
        // a read filter too.
        assert_eq!(run(".a + 1", "a: 41").unwrap(), vec![Value::Int(42)]);
        assert_eq!(run(".a * .b", "a: 6\nb: 7").unwrap(), vec![Value::Int(42)]);
    }
}
