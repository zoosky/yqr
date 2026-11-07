//! Runtime proof for the wasm boundary: the compile check in CI pins that
//! the lib builds for the target, this suite pins that it *behaves* there.
//! Run with `wasm-pack test --node yqr-wasm`.
//!
//! One read, one byte-preserving edit, one validate — a representative
//! slice of the corpus shapes (Kubernetes manifest formatting: comments,
//! quoting, block style), executed inside a wasm runtime.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::wasm_bindgen_test;
use yqr_wasm::{eval, fidelity_run, validate};

/// A corpus-shaped input: comments, mixed quoting, nested blocks — the
/// formatting a fidelity read must reproduce byte-for-byte.
const DEPLOY: &str = "\
# production replica count -- agreed with ops
apiVersion: apps/v1
kind: Deployment
metadata:
  name: 'web'   # single-quoted on purpose
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: web
          image: \"nginx:1.27\"
";

#[wasm_bindgen_test]
fn a_read_returns_plain_js_data() {
    let out = eval(".spec.replicas", DEPLOY).expect("read succeeds");
    let values: Vec<i64> = serde_wasm_bindgen::from_value(out).expect("a JS array of numbers");
    assert_eq!(values, vec![3]);
}

#[wasm_bindgen_test]
fn a_fidelity_read_is_byte_identical() {
    let out = fidelity_run(".", DEPLOY).expect("identity read succeeds");
    assert_eq!(
        out, DEPLOY,
        "untouched nodes must come back as their own bytes"
    );
}

#[wasm_bindgen_test]
fn a_fidelity_edit_rewrites_only_the_target_bytes() {
    let out = fidelity_run(".spec.replicas = 5", DEPLOY).expect("edit succeeds");
    assert_eq!(out, DEPLOY.replace("replicas: 3", "replicas: 5"));
}

#[wasm_bindgen_test]
fn validate_reports_structured_diagnostics() {
    let clean = validate(DEPLOY, true).expect("validate runs");
    let findings: Vec<serde_json_shape::Finding> =
        serde_wasm_bindgen::from_value(clean).expect("an array");
    assert!(findings.is_empty(), "the corpus document is valid");

    let dirty = validate("a: 1\na: 2\n", true).expect("validate runs");
    let findings: Vec<serde_json_shape::Finding> =
        serde_wasm_bindgen::from_value(dirty).expect("an array");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "Y101");
    assert_eq!(findings[0].position, Some((2, 1)));
    assert_eq!(findings[0].offset, Some(5));
}

#[wasm_bindgen_test]
fn a_parsed_mapping_with_scalar_keys_crosses_as_an_object() {
    // The engine string-keys every parsed mapping (`1:` reads as the key
    // "1"), so YAML input cannot reach JsData's non-string-key refusal;
    // that arm guards computed values only. Pin the string-keying here so
    // a future engine change surfaces in this suite.
    let out = eval(".", "1: a\n2: b\n").expect("scalar keys read as strings");
    let values: Vec<std::collections::HashMap<String, String>> =
        serde_wasm_bindgen::from_value(out).expect("a JS object keyed by strings");
    assert_eq!(values[0].get("1").map(String::as_str), Some("a"));
}

#[wasm_bindgen_test]
fn an_integer_past_js_safe_range_is_refused_not_rounded() {
    // 2^53 + 1 has no exact f64 spelling; silently rounding it would make
    // the editor show and write back a different number.
    let err = eval(".id", "id: 9007199254740993\n").expect_err("unsafe integer must refuse");
    let envelope: serde_json_shape::Envelope =
        serde_wasm_bindgen::from_value(err).expect("a structured object");
    assert_eq!(envelope.kind, "data");

    // The largest safe integer still crosses exactly.
    let out = eval(".id", "id: 9007199254740991\n").expect("safe integer crosses");
    let values: Vec<f64> = serde_wasm_bindgen::from_value(out).expect("a JS array");
    assert_eq!(values, vec![9007199254740991.0]);
}

#[wasm_bindgen_test]
fn a_filter_error_crosses_as_a_structured_object() {
    let err = eval(".a = 5", DEPLOY).expect_err("a mutation is not a read");
    let envelope: serde_json_shape::Envelope =
        serde_wasm_bindgen::from_value(err).expect("a structured object, not a string");
    assert_eq!(envelope.kind, "parse");
    assert!(!envelope.message.is_empty());
}

/// Deserialization mirrors of the serialized shapes, local to the test:
/// the real types are Serialize-only on purpose.
mod serde_json_shape {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct Finding {
        pub code: String,
        pub position: Option<(usize, usize)>,
        pub offset: Option<usize>,
        #[allow(dead_code)]
        pub message: String,
    }

    #[derive(Deserialize)]
    pub struct Envelope {
        pub kind: String,
        pub message: String,
    }
}

/// Feature f050: every typed edit op crosses the boundary as JSON and
/// applies through one entry point.
#[wasm_bindgen_test]
fn a_typed_change_set_crosses_as_json_and_applies_in_order() {
    use yqr_wasm::apply_all;

    let ops = js_sys::JSON::parse(
        r#"[
            {"Set": {"path": [{"Key": "title"}], "value": "New"}},
            {"Set": {"path": [{"Key": "menu"}, {"Key": "visible"}], "value": false}},
            {"Append": {"path": [{"Key": "tags"}], "value": "b"}},
            {"Rename": {"path": [{"Key": "lead"}], "to": "summary"}},
            {"Remove": {"path": [{"Key": "draft"}]}},
            {"SetHeadComment": {"path": [{"Key": "title"}], "text": "the page name"}}
        ]"#,
    )
    .expect("valid JSON");

    let input = "title: Old\nlead: L  # keep\ndraft: true\ntags:\n  - a\n";
    let out = apply_all(ops, input).expect("the change set applies");
    assert!(out.contains("# the page name"), "{out}");
    assert!(out.contains("title: New"), "{out}");
    assert!(out.contains("menu:\n  visible: false"), "{out}");
    assert!(out.contains("summary: L  # keep"), "{out}");
    assert!(out.contains("  - a\n  - b"), "{out}");
    assert!(!out.contains("draft"), "{out}");

    // An empty change set is the input, byte for byte.
    let empty = js_sys::JSON::parse("[]").expect("valid JSON");
    assert_eq!(apply_all(empty, input).expect("empty applies"), input);

    // A refusal carries the op's index, as a structured error.
    let refused =
        js_sys::JSON::parse(r#"[{"Append": {"path": [{"Key": "title"}], "value": "x"}}]"#)
            .expect("valid JSON");
    let err = apply_all(refused, input).expect_err("append to a scalar refuses");
    let envelope: serde_json_shape::Envelope =
        serde_wasm_bindgen::from_value(err).expect("a structured object");
    assert_eq!(envelope.kind, "eval");
    assert!(
        envelope.message.contains("op 0 (append)"),
        "{}",
        envelope.message
    );
}
