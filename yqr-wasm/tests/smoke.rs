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
