//! End-to-end tests of the fidelity engine read path: with an engine
//! selected, the identity filter must reproduce the input byte-for-byte, and
//! path projections must emit the selected node's original bytes.

use yqr::fidelity::{BackendId, run};

/// Shorthand: run `filter` over `input` through the noyalib engine.
fn fid(filter: &str, input: &str, raw: bool) -> String {
    run(BackendId::NoyalibCst, filter, input, raw).expect("fidelity run succeeds")
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
fn special_char_key_degrades_to_typed_rendering() {
    let input = "'a.b': 'quoted value'\n";
    // The engine cannot address the dotted key; output falls back to the
    // typed value (quotes normalized away) instead of failing.
    assert_eq!(fid(r#".["a.b"]"#, input, false), "quoted value\n");
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
fn colliding_stringified_keys_error_instead_of_dropping_entries() {
    // `1` and `"1"` are distinct YAML keys; the engine's string-only key
    // model would silently collapse them, so it must refuse instead.
    let err = run(BackendId::NoyalibCst, ".[]", "1: a\n\"1\": b\n", false)
        .expect_err("collision must be an error");
    assert!(err.to_string().contains("collide"), "got: {err}");
}

#[test]
fn non_string_keys_match_by_spelling() {
    // Documented engine divergence: the classic pipeline keys this entry by
    // Bool(true), so `.true` misses; the engine matches the spelling.
    assert_eq!(fid(".true", "true: yes\n", false), "yes\n");
}

#[test]
fn invalid_input_is_an_error() {
    assert!(run(BackendId::NoyalibCst, ".", "items: [1, 2", false).is_err());
}
