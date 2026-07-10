//! Golden classic-pipeline outputs — de-circularized regression pins.
//!
//! `corpus_validation.rs` compares the pipeline's output against expectations
//! it parses with the *same* engine, so a scalar-typing or formatting change in
//! the backend would move both sides together and escape notice. These cases
//! pin the **exact rendered bytes** for type-sensitive inputs against
//! hand-authored golden strings, so a regression — a quoted `"007"` coerced to
//! the integer `7`, a float flattened to an int, a bool restyled — fails
//! loudly and independently of how the backend parses. This is the check that
//! guards behavior across a YAML-engine swap or upgrade.

use yqr::{eval_str, render};

/// `(filter, input, raw, expected exact output)` — golden values authored by
/// hand, **not** produced by the engine under test.
const CASES: &[(&str, &str, bool, &str)] = &[
    // -- scalar typing is preserved through parse -> eval -> render ----------
    (".n", "n: 42\n", false, "42\n"),
    (".n", "n: -5\n", false, "-5\n"),
    (".pi", "pi: 3.14\n", false, "3.14\n"),
    (".flag", "flag: true\n", false, "true\n"),
    (".x", "x: null\n", false, "null\n"),
    (".name", "name: web\n", false, "web\n"),
    // A quoted numeric-looking scalar stays a *string* (rendered quoted), never
    // the integer 7 — the type-preservation property behind b001/a001.
    (".zip", "zip: \"007\"\n", false, "\"007\"\n"),
    // Raw mode prints a top-level string's value verbatim (jq --raw-output).
    (".zip", "zip: \"007\"\n", true, "007\n"),
    (".g", "g: hello\n", true, "hello\n"),
    // -- structure: indexing, iteration, null propagation --------------------
    (".i[1]", "i:\n  - a\n  - b\n", false, "b\n"),
    (".i[]", "i:\n  - 1\n  - 2\n  - 3\n", false, "1\n2\n3\n"),
    (".nope", "a: 1\n", false, "null\n"),
];

/// The classic pipeline (`eval_str` + `render`) produces exactly the golden
/// bytes for each case. Because the expectations are fixed strings rather than
/// re-parsed input, this catches backend behavior changes that a
/// parse-both-sides comparison cannot.
#[test]
fn classic_pipeline_golden_outputs() {
    for &(filter, input, raw, expected) in CASES {
        let values = eval_str(filter, input)
            .unwrap_or_else(|e| panic!("`{filter}` on `{input:?}` should evaluate: {e}"));
        let got = render(&values, raw)
            .unwrap_or_else(|e| panic!("`{filter}` on `{input:?}` should render: {e}"));
        assert_eq!(
            got, expected,
            "`{filter}` on `{input:?}` (raw={raw}) produced unexpected bytes"
        );
    }
}
