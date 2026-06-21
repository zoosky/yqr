//! `yqr` — a jq-style query/transform tool for YAML.
//!
//! This crate is split into small, independently testable layers:
//!
//! - [`lexer`] turns a filter string into tokens,
//! - [`parser`] turns tokens into an [`ast::Ast`],
//! - [`eval`] applies an `Ast` to a [`rust_yaml::Value`], producing a stream of
//!   output values.
//!
//! The two convenience entry points most callers want are [`eval_str`] (parse a
//! filter and run it over a YAML string, returning values) and
//! [`render`] (turn output values back into a YAML/raw string).

pub mod ast;
pub mod error;
pub mod eval;
pub mod lexer;
pub mod parser;

pub use error::{Result, YqrError};
pub use rust_yaml::Value;

use rust_yaml::Yaml;

/// Parse `filter`, load the first YAML document from `input`, and evaluate the
/// filter against it, returning the output stream.
pub fn eval_str(filter: &str, input: &str) -> Result<Vec<Value>> {
    let ast = parser::parse(filter)?;
    let value = Yaml::new()
        .load_str(input)
        .map_err(|e| YqrError::io(format!("failed to parse YAML input: {e}")))?;
    eval::eval(&ast, &value)
}

/// Render a stream of output values to a string.
///
/// Each value is emitted as its own YAML document. When `raw` is set, top-level
/// string results are printed verbatim (without YAML quoting), matching jq's
/// `--raw-output`.
pub fn render(values: &[Value], raw: bool) -> Result<String> {
    let yaml = Yaml::new();
    let mut out = String::new();
    for value in values {
        if raw && let Value::String(s) = value {
            out.push_str(s);
            out.push('\n');
            continue;
        }
        let dumped = yaml
            .dump_str(value)
            .map_err(|e| YqrError::io(format!("failed to emit YAML: {e}")))?;
        out.push_str(dumped.trim_end_matches('\n'));
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_str_field() {
        let out = eval_str(".a", "a: 1\nb: 2").unwrap();
        assert_eq!(out, vec![Value::Int(1)]);
    }

    #[test]
    fn render_raw_string() {
        let rendered = render(&[Value::String("hello".into())], true).unwrap();
        assert_eq!(rendered, "hello\n");
    }

    #[test]
    fn render_non_raw_quotes_when_needed() {
        // Without raw mode the emitter is responsible for formatting; we just
        // assert it produces a single trailing newline and contains the value.
        let rendered = render(&[Value::Int(42)], false).unwrap();
        assert_eq!(rendered, "42\n");
    }

    #[test]
    fn render_multiple_values() {
        let rendered = render(&[Value::Int(1), Value::Int(2)], false).unwrap();
        assert_eq!(rendered, "1\n2\n");
    }
}
