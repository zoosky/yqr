//! End-to-end tests of the fidelity engine read path on backend A (the
//! rust-yaml fork's `RoundTripDocument`): with the engine selected, the identity
//! filter must reproduce the input byte-for-byte and path projections must emit
//! the selected node's original bytes.
//!
//! These tests require the rust-yaml backend:
//! `cargo test --features backend-rust-yaml --test fidelity_engine_rustyaml`
//!
//! Where the fork's model is richer than the CST backend the expected output
//! differs (and is better): special-character keys are addressable, duplicate
//! keys resolve last-wins to real bytes, non-string keys are preserved, and
//! distinct keys that share a spelling never collide.

#![cfg(feature = "backend-rust-yaml")]

use yqr::fidelity::{BackendId, run};

/// Shorthand: run `filter` over `input` through the rust-yaml fork engine.
fn fid(filter: &str, input: &str, raw: bool) -> String {
    run(BackendId::RustYamlRoundTrip, filter, input, raw).expect("fidelity run succeeds")
}

/// The identity/`cat` property over every formatting dimension the default
/// pipeline destroys (the reproduction corpus of the round-trip bug).
#[test]
fn identity_reproduces_input_byte_for_byte() {
    let corpus: &[&str] = &[
        // comments + blank lines
        concat!(
            "# Top-level header comment\n",
            "name: my-app   # inline comment on a scalar\n",
            "\n",
            "# Section: replicas\n",
            "replicas: 3\n",
            "\n",
            "config:\n",
            "  # nested comment\n",
            "  debug: true\n",
            "  level: info\n",
        ),
        // blank lines only
        "a: 1\n\nb: 2\n\n\nc: 3\n",
        // 4-space indentation
        "root:\n    child:\n        leaf: value\n    sibling: other\n",
        // quote styles
        "bare: hello\nsingle: 'hello world'\ndouble: \"hello world\"\nforced_string: \"123\"\nspecial: 'it''s a test'\n",
        // block scalars
        "literal: |\n  line one\n  line two\nfolded: >\n  this is\n  folded text\n",
        // number spellings
        "replicas: 3\nratio: 1.0\nzip: 007\nbig_id: 12345678901234567\nport: 8080\nneg: -5\n",
        // flow style
        "flow_map: {a: 1, b: 2}\nflow_seq: [1, 2, 3]\nnested: {list: [x, y], n: 1}\n",
        // key order
        "zebra: 1\napple: 2\nmango: 3\n",
        // anchors + merge keys
        "defaults: &defaults\n  timeout: 30\n  retries: 3\nservice:\n  <<: *defaults\n  name: web\n",
        // CRLF
        "a: 1\r\nb: 2\r\n",
        // BOM + multiple nodes
        "\u{feff}a: 1\nb: 2\n",
        // trailing whitespace
        "a: 1   \nb: 2\t\n",
        // multi-document stream
        "---\na: 1\n---\nb: 2\n",
        // realistic manifest
        concat!(
            "# Production deployment\n",
            "apiVersion: apps/v1\n",
            "kind: Deployment\n",
            "metadata:\n",
            "  name: web        # the web frontend\n",
            "  labels:\n",
            "    app: web\n",
            "spec:\n",
            "  replicas: 3      # scale here\n",
            "\n",
            "  template:\n",
            "    spec:\n",
            "      containers:\n",
            "        - name: web\n",
            "          image: nginx:1.25   # pin the tag\n",
        ),
    ];
    for input in corpus {
        assert_eq!(&fid(".", input, false), input, "identity for {input:?}");
    }
}

#[test]
fn projection_emits_original_bytes() {
    let input = "zip: 007\nquoted: 'hello world'\nblock: |\n  line\n";
    // Leading zeros survive (the typed value would be `7`).
    assert_eq!(fid(".zip", input, false), "007\n");
    // Quote style survives (the typed render would drop the quotes).
    assert_eq!(fid(".quoted", input, false), "'hello world'\n");
    // Block scalars keep their header and body.
    assert_eq!(fid(".block", input, false), "|\n  line\n");
}

#[test]
fn nested_and_indexed_projection() {
    let input = "spec:\n  items:\n    - alpha   # first\n    - beta\n";
    assert_eq!(fid(".spec.items[0]", input, false), "alpha\n");
    assert_eq!(fid(".spec.items[-1]", input, false), "beta\n");
}

#[test]
fn iteration_slices_each_element() {
    let input = "a: 'x'\nb: \"y\"\n";
    assert_eq!(fid(".[]", input, false), "'x'\n\"y\"\n");
}

#[test]
fn missing_path_is_null() {
    assert_eq!(fid(".nope", "a: 1\n", false), "null\n");
}

#[test]
fn raw_output_prints_string_values() {
    // -r prints the string VALUE, not its quoted source bytes.
    assert_eq!(fid(".s", "s: 'hello world'\n", true), "hello world\n");
}

#[test]
fn multi_document_filter_runs_per_document() {
    let input = "---\nname: one\n---\nname: two\n";
    assert_eq!(fid(".name", input, false), "one\ntwo\n");
}

#[test]
fn merged_entry_degrades_to_typed_rendering() {
    // `.service.timeout` only exists through the merge key; it has no bytes
    // inside `service`, so it falls back to the typed value (visibly lossy,
    // but correct).
    let input = "defaults: &d\n  timeout: 30\nservice:\n  <<: *d\n  name: web\n";
    assert_eq!(fid(".service.timeout", input, false), "30\n");
}

#[test]
fn special_char_key_emits_original_bytes() {
    // Unlike the CST backend (which degrades dotted keys to typed rendering),
    // the fork addresses keys by scalar text, so the original quoted bytes are
    // emitted verbatim.
    let input = "'a.b': 'quoted value'\n";
    assert_eq!(fid(r#".["a.b"]"#, input, false), "'quoted value'\n");
}

#[test]
fn duplicate_key_emits_last_occurrence_bytes() {
    // Both the span index and the typed value keep the last occurrence, so the
    // engine emits its real bytes rather than degrading (the CST backend, which
    // resolves duplicates first-wins in the span layer, degrades here).
    let input = "k: one\nk: two\n";
    assert_eq!(fid(".k", input, false), "two\n");
}

#[test]
fn empty_input_produces_no_output() {
    // Intentional divergence from the classic pipeline (which prints null):
    // in engine mode the identity of an empty file is the empty file.
    assert_eq!(fid(".", "", false), "");
}

#[test]
fn projected_block_collection_reparses_to_selected_value() {
    // Regression: a first-line-dedented slice ('- alpha\n    - beta') would
    // silently re-parse downstream as the one-element list ["alpha - beta"].
    // The emitted slice must be uniformly indented and denote the value.
    let input = "spec:\n  items:\n    - alpha\n    - beta\n";
    let out = fid(".spec.items", input, false);
    assert_eq!(out, "    - alpha\n    - beta\n");
    let reparsed = yqr::eval_str(".", &out).expect("emitted output is valid YAML");
    assert_eq!(
        reparsed[0],
        yqr::Value::Sequence(vec![
            yqr::Value::String("alpha".into()),
            yqr::Value::String("beta".into()),
        ]),
    );
}

#[test]
fn projected_block_mapping_reparses_standalone() {
    let input = "config:\n  debug: true\n  level: info\nafter: 1\n";
    let out = fid(".config", input, false);
    assert_eq!(out, "  debug: true\n  level: info\n");
    assert!(
        yqr::eval_str(".debug", &out).is_ok(),
        "output must re-parse"
    );
}

#[test]
fn distinct_stringlike_keys_do_not_collide() {
    // `1` and `"1"` are distinct YAML keys. The CST backend refuses this input
    // (its string-only key model would drop an entry); the fork keeps both, so
    // the engine loads it and the identity filter round-trips it byte-for-byte.
    let input = "1: a\n\"1\": b\n";
    assert_eq!(fid(".", input, false), input);
}

#[test]
fn deeply_nested_block_mapping_projection_reparses_uniformly() {
    // Regression (adversarial review): `.a.b` must emit the uniformly-indented
    // "    c: 1\n    d: 2\n", not the first-line-dedented "c: 1\n    d: 2\n"
    // that stricter parsers reject. The output must re-parse to {c:1, d:2}.
    let input = "a:\n  b:\n    c: 1\n    d: 2\n";
    let out = fid(".a.b", input, false);
    assert_eq!(out, "    c: 1\n    d: 2\n");
    let reparsed = yqr::eval_str(".", &out).expect("emitted output is valid YAML");
    assert_eq!(reparsed[0].get_str("c"), Some(&yqr::Value::Int(1)));
    assert_eq!(reparsed[0].get_str("d"), Some(&yqr::Value::Int(2)));
}

#[test]
fn int_valued_nested_block_mapping_is_uniformly_indented() {
    // The fork's lenient loader accepts a dedented int-first slice; the engine
    // must still emit the uniformly-indented form.
    let input = "m:\n  a: 1\n  b: 2\n";
    assert_eq!(fid(".m", input, false), "  a: 1\n  b: 2\n");
}

#[test]
fn document_end_marker_after_block_is_a_known_limitation() {
    // Known limitation of backend A: the fork's parse_all mis-accounts a phantom
    // EOF boundary for a `...` document-end marker following a block collection,
    // so the engine errors on this otherwise-valid input (tracked as a fork bug,
    // b003). Pinned here so a future fork bump that fixes it flips this test and
    // prompts turning it into a byte-for-byte identity assertion.
    let err = run(BackendId::RustYamlRoundTrip, ".", "a: 1\n...\n", false)
        .expect_err("fork parse_all currently rejects a trailing '...' after a block");
    assert!(err.to_string().contains("parse"), "got: {err}");
}

#[test]
fn invalid_input_is_an_error() {
    assert!(run(BackendId::RustYamlRoundTrip, ".", "items: [1, 2", false).is_err());
}
