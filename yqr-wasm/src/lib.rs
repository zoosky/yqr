//! wasm-bindgen bindings for yqr: the boundary a browser editor calls.
//!
//! Three exports, kept to the editor's needs:
//!
//! - [`eval`] — run a read-only filter, values out as plain JS data,
//! - [`fidelity_run`] (`fidelityRun` in JS) — byte-preserving execution:
//!   reads slice original bytes, mutations rewrite only the bytes they
//!   touch, the whole document comes back as a string,
//! - [`validate`] — correctness diagnostics as structured objects, with
//!   line/column and byte offsets ready for editor markers.
//!
//! Errors cross the boundary as structured objects (see [`ErrorEnvelope`]),
//! never as thrown strings: the editor routes on `kind` and underlines
//! `span` without parsing prose.

use serde::Serialize;
use wasm_bindgen::prelude::*;
use yqr::Value;
use yqr::ast::Program;

/// The structured error every export rejects with.
///
/// `kind` is the error category (`lex`, `parse`, `eval`, `io`, or this
/// crate's own `data` for a value the JS data model cannot carry),
/// `message` is the same prose the CLI prints, and `span` — present for
/// filter compilation errors that know it — is the byte range of the
/// offending token in the filter string.
#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    /// Error category: `lex`, `parse`, `eval`, `io`, or `data`.
    pub kind: &'static str,
    /// Human-readable description, as the CLI would print it.
    pub message: String,
    /// Byte range of the offending token in the filter source, when known.
    pub span: Option<std::ops::Range<usize>>,
}

impl ErrorEnvelope {
    fn from_yqr(err: &yqr::YqrError) -> Self {
        let kind = match err {
            yqr::YqrError::Lex(_) => "lex",
            yqr::YqrError::Parse(_) => "parse",
            yqr::YqrError::Eval(_) => "eval",
            yqr::YqrError::Io(_) => "io",
        };
        ErrorEnvelope {
            kind,
            message: err.to_string(),
            span: err.span(),
        }
    }

    fn data(message: String) -> Self {
        ErrorEnvelope {
            kind: "data",
            message,
            span: None,
        }
    }

    fn into_js(self) -> JsValue {
        serde_wasm_bindgen::to_value(&self).unwrap_or_else(|_| JsValue::from_str(&self.message))
    }
}

/// Serializes a [`Value`] as plain JS data: scalars as themselves,
/// sequences as arrays, mappings as objects.
///
/// Mapping keys must be strings — a mapping keyed by a number or a
/// collection has no JS-object spelling, so it is refused (the `data`
/// error kind) rather than silently renamed.
struct JsData<'a>(&'a Value);

impl Serialize for JsData<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{Error, SerializeMap, SerializeSeq};
        match self.0 {
            Value::Null => s.serialize_unit(),
            Value::Bool(b) => s.serialize_bool(*b),
            Value::Int(n) => s.serialize_i64(*n),
            Value::Float(f) => s.serialize_f64(*f),
            Value::String(text) => s.serialize_str(text),
            Value::Sequence(items) => {
                let mut seq = s.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(&JsData(item))?;
                }
                seq.end()
            }
            Value::Mapping(map) => {
                let mut out = s.serialize_map(Some(map.len()))?;
                for (key, value) in map {
                    let Value::String(key) = key else {
                        return Err(S::Error::custom(
                            "a mapping key is not a string, so the value has no \
                             JS-object spelling",
                        ));
                    };
                    out.serialize_entry(key, &JsData(value))?;
                }
                out.end()
            }
        }
    }
}

/// The serializer every export uses: maps become plain JS objects, not
/// `Map` instances, so the editor consumes ordinary JSON-shaped data.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| ErrorEnvelope::data(e.to_string()).into_js())
}

/// Run a read-only filter over a YAML document, returning the output
/// values as a JS array of plain data.
///
/// # Errors
///
/// Rejects with an [`ErrorEnvelope`] when the filter does not compile,
/// the input does not parse, evaluation fails, or a result has no JS
/// spelling (a mapping keyed by a non-string).
#[wasm_bindgen]
pub fn eval(filter: &str, yaml: &str) -> Result<JsValue, JsValue> {
    let values = yqr::eval_str(filter, yaml).map_err(|e| ErrorEnvelope::from_yqr(&e).into_js())?;
    let wrapped: Vec<JsData<'_>> = values.iter().map(JsData).collect();
    to_js(&wrapped)
}

/// Byte-preserving execution: a read slices original bytes, a mutation
/// rewrites only the bytes it touches, and the result comes back as a
/// string — the same engine behind the CLI's default mode.
///
/// # Errors
///
/// Rejects with an [`ErrorEnvelope`] when the filter does not compile,
/// the engine refuses the input or the edit, or evaluation fails.
#[wasm_bindgen(js_name = fidelityRun)]
pub fn fidelity_run(filter: &str, yaml: &str) -> Result<String, JsValue> {
    let result = match yqr::parser::parse_program(filter) {
        Ok(Program::Query(target)) => yqr::fidelity::run_target(&target, yaml, false),
        Ok(Program::Mutate(mutation)) => yqr::fidelity::write::apply(&mutation, yaml),
        Err(err) => Err(err),
    };
    result.map_err(|e| ErrorEnvelope::from_yqr(&e).into_js())
}

/// Check a YAML document and return every finding as structured data:
/// code (`"Y101"`), message, 1-based `[line, column]`, byte offset, note
/// and help — the serialized shape of [`yqr::validate::Diagnostic`].
///
/// An empty array means the document is valid. `strict` adds the
/// duplicate-key findings ordinary reads accept silently.
///
/// # Errors
///
/// Rejects with an [`ErrorEnvelope`] only when the findings cannot be
/// serialized; an invalid document is a normal result, not an error.
#[wasm_bindgen]
pub fn validate(yaml: &str, strict: bool) -> Result<JsValue, JsValue> {
    to_js(&yqr::validate::check_str(yaml, strict))
}
