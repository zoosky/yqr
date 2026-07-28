//! `yqr` — a jq-style query/transform tool for YAML.
//!
//! This crate is split into small, independently testable layers:
//!
//! - [`lexer`] turns a filter string into tokens,
//! - [`parser`] turns tokens into an [`ast::Ast`],
//! - [`eval`] applies an `Ast` to a [`Value`], producing a stream of
//!   output values,
//! - [`fidelity`] provides byte-preserving execution over noyalib's lossless
//!   CST (untouched nodes are emitted as their original source bytes),
//! - [`validate`] checks inputs for YAML correctness and renders
//!   compiler-style diagnostics.
//!
//! The two convenience entry points most callers want are [`eval_str`] (parse a
//! filter and run it over a YAML string, returning values) and
//! [`render`] (turn output values back into a YAML/raw string). For
//! byte-preserving output, see [`fidelity::run`].

pub mod ast;
pub mod error;
pub mod eval;
pub mod fidelity;
pub mod lexer;
pub mod parser;
pub mod validate;
pub mod value;

pub use error::{Result, YqrError};
pub use value::Value;

/// Parse `filter`, load the first YAML document from `input`, and evaluate the
/// filter against it, returning the output stream.
pub fn eval_str(filter: &str, input: &str) -> Result<Vec<Value>> {
    let ast = parser::parse(filter)?;
    eval_ast_str(&ast, input)
}

/// Like [`eval_str`], but over an already-compiled [`ast::Ast`], so a caller
/// that has already parsed the filter does not lex and parse it again.
///
/// # Errors
///
/// Returns an error when the input is not valid YAML or evaluation fails.
pub fn eval_ast_str(ast: &ast::Ast, input: &str) -> Result<Vec<Value>> {
    let value: Value = noyalib::from_str::<noyalib::Value>(input)
        .map(Value::from)
        .map_err(|e| YqrError::io(format!("failed to parse YAML input: {e}")))?;
    eval::eval(ast, &value)
}

/// Render a stream of output values to a string.
///
/// Each value is emitted as its own YAML document. When `raw` is set, top-level
/// string results are printed verbatim (without YAML quoting), matching jq's
/// `--raw-output`.
pub fn render(values: &[Value], raw: bool) -> Result<String> {
    let mut out = String::new();
    for value in values {
        if raw && let Value::String(s) = value {
            out.push_str(s);
            out.push('\n');
            continue;
        }
        let dumped = noyalib::to_string_value(&noyalib::Value::from(value))
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
