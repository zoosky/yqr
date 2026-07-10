//! The evaluation engine: applies an [`Ast`] to a [`Value`], producing a
//! stream (`Vec<Value>`) of results, mirroring jq's model where any filter can
//! yield zero or more outputs.
//!
//! Every result additionally carries *provenance*: the concrete [`Path`] it
//! was derived from, or `None` when it was computed rather than selected. The
//! fidelity engine uses that path to emit untouched nodes by slicing their
//! original bytes; the classic pipeline simply discards it.

use crate::Value;

use crate::ast::Ast;
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

fn index(value: &Value, idx: i64, path: Option<&Path>) -> Result<Traced> {
    match value {
        Value::Null => Ok((Value::Null, None)),
        Value::Sequence(items) => {
            let len = items.len() as i64;
            let resolved = if idx < 0 { len + idx } else { idx };
            if resolved < 0 || resolved >= len {
                // Out of range yields null with no source node to slice.
                Ok((Value::Null, None))
            } else {
                #[allow(clippy::cast_sign_loss)] // just checked 0 <= resolved
                let i = resolved as usize;
                let child = path.map(|p| p.child(PathSeg::Index(i)));
                Ok((items[i].clone(), child))
            }
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
}
