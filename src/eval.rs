//! The evaluation engine: applies an [`Ast`] to a [`Value`], producing a
//! stream (`Vec<Value>`) of results, mirroring jq's model where any filter can
//! yield zero or more outputs.

use rust_yaml::Value;

use crate::ast::Ast;
use crate::error::{Result, YqrError};

/// Evaluate `ast` against a single input `value`, returning the output stream.
pub fn eval(ast: &Ast, value: &Value) -> Result<Vec<Value>> {
    match ast {
        Ast::Identity => Ok(vec![value.clone()]),
        Ast::Field(name) => Ok(vec![field(value, name)?]),
        Ast::Index(idx) => Ok(vec![index(value, *idx)?]),
        Ast::Iterate => iterate(value),
        Ast::Pipe(lhs, rhs) => {
            let mut out = Vec::new();
            for v in eval(lhs, value)? {
                out.extend(eval(rhs, &v)?);
            }
            Ok(out)
        }
        Ast::Optional(inner) => match eval(inner, value) {
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

fn field(value: &Value, name: &str) -> Result<Value> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::Mapping(map) => {
            let key = Value::String(name.to_string());
            Ok(map.get(&key).cloned().unwrap_or(Value::Null))
        }
        other => Err(YqrError::eval(format!(
            "cannot index {} with field {:?}",
            type_name(other),
            name
        ))),
    }
}

fn index(value: &Value, idx: i64) -> Result<Value> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::Sequence(items) => {
            let len = items.len() as i64;
            let resolved = if idx < 0 { len + idx } else { idx };
            if resolved < 0 || resolved >= len {
                Ok(Value::Null)
            } else {
                Ok(items[resolved as usize].clone())
            }
        }
        other => Err(YqrError::eval(format!(
            "cannot index {} with number {}",
            type_name(other),
            idx
        ))),
    }
}

fn iterate(value: &Value) -> Result<Vec<Value>> {
    match value {
        Value::Sequence(items) => Ok(items.clone()),
        Value::Mapping(map) => Ok(map.values().cloned().collect()),
        other => Err(YqrError::eval(format!(
            "cannot iterate over {}",
            type_name(other)
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_yaml::Yaml;

    fn load(src: &str) -> Value {
        Yaml::new().load_str(src).expect("valid yaml")
    }

    fn run(filter: &str, yaml: &str) -> Result<Vec<Value>> {
        let ast = crate::parser::parse(filter).expect("valid filter");
        eval(&ast, &load(yaml))
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
}
