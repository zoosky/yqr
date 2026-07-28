//! Validation half of the shared corpus: every case in `tests/corpus` is run
//! through the classic pipeline and (where applicable) the fidelity engine,
//! and its output is asserted against the recorded expectation.
//!
//! The same corpus drives `benches/corpus_bench.rs`, so a case added here is
//! measured there for free.

#[path = "corpus/mod.rs"]
mod corpus;

use corpus::{Case, EngineCase, Expect};
use yqr::{eval_str, render};

/// Run one classic-pipeline case and assert its expectation.
fn check_classic(case: &Case) {
    match case.expect {
        Expect::Values(expected_yaml) => {
            let out = eval_str(case.filter, case.doc)
                .unwrap_or_else(|e| panic!("[{}] should evaluate, got error: {e}", case.id));
            // `want` is parsed with the same engine, so this asserts *semantic*
            // equality (value stream), robust to the emitter's formatting.
            // Byte-exact, engine-independent behavior — the check a
            // parse-both-sides comparison cannot make — is pinned separately in
            // tests/golden_pipeline.rs.
            let want: Vec<yqr::Value> = noyalib::load_all_as::<noyalib::Value>(expected_yaml)
                .unwrap_or_else(|e| panic!("[{}] expected YAML must parse: {e}", case.id))
                .into_iter()
                .map(yqr::Value::from)
                .collect();
            assert_eq!(out, want, "[{}] value stream mismatch", case.id);
        }
        Expect::Empty => {
            let out = eval_str(case.filter, case.doc)
                .unwrap_or_else(|e| panic!("[{}] should evaluate, got error: {e}", case.id));
            assert!(
                out.is_empty(),
                "[{}] expected empty stream, got {out:?}",
                case.id
            );
        }
        Expect::Raw(expected) => {
            let out = eval_str(case.filter, case.doc)
                .unwrap_or_else(|e| panic!("[{}] should evaluate, got error: {e}", case.id));
            let got =
                render(&out, true).unwrap_or_else(|e| panic!("[{}] should render: {e}", case.id));
            assert_eq!(got, expected, "[{}] raw output mismatch", case.id);
        }
        Expect::Err(code) => {
            let err = eval_str(case.filter, case.doc)
                .expect_err(&format!("[{}] expected an error", case.id));
            assert_eq!(
                err.exit_code(),
                code,
                "[{}] exit code mismatch: {err}",
                case.id
            );
        }
    }
}

/// Run one engine case through the fidelity engine.
fn check_engine(case: &EngineCase) {
    let got = yqr::fidelity::run(case.filter, case.doc, case.raw)
        .unwrap_or_else(|e| panic!("[{}] engine run failed: {e}", case.id));
    assert_eq!(got, case.expect, "[{}] byte output mismatch", case.id);
}

#[test]
fn classic_corpus_matches_expectations() {
    for case in corpus::classic_cases() {
        check_classic(&case);
    }
}

#[test]
fn engine_corpus_is_byte_exact() {
    for case in corpus::engine_cases() {
        check_engine(&case);
    }
}

// Feature f012: every corpus document is real-world YAML the validate
// command must accept, in both default and strict mode — the corpus is the
// no-false-positives guard for the validator.
#[test]
fn corpus_documents_validate_cleanly() {
    let mut docs: Vec<&str> = corpus::classic_cases().iter().map(|c| c.doc).collect();
    docs.extend(corpus::engine_cases().iter().map(|c| c.doc));
    docs.sort_unstable();
    docs.dedup();
    for doc in docs {
        let findings = yqr::validate::check_str(doc, true);
        assert!(
            findings.is_empty(),
            "corpus document must validate cleanly, got {findings:?}\ndoc:\n{doc}"
        );
    }
}

#[test]
fn corpus_ids_are_unique() {
    // A duplicate id would make benchmark groups collide silently.
    let mut ids: Vec<&str> = corpus::classic_cases()
        .iter()
        .map(|c| c.id)
        .chain(corpus::engine_cases().iter().map(|c| c.id))
        .collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), total, "corpus case ids must be unique");
}
