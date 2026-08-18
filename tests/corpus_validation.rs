//! Validation half of the shared corpus: every case in `tests/corpus` is run
//! through the classic pipeline and (where applicable) the fidelity engine,
//! and its output is asserted against the recorded expectation.
//!
//! The same corpus drives `benches/corpus_bench.rs`, so a case added here is
//! measured there for free.

#[path = "corpus/mod.rs"]
mod corpus;

use corpus::{Case, EngineCase, Expect, WriteCase, WriteExpect};
use yqr::ast::Program;
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

/// Apply one write case's mutation to its document.
fn run_write(case: &WriteCase) -> yqr::Result<String> {
    match yqr::parser::parse_program(case.filter)? {
        Program::Mutate(mutation) => yqr::fidelity::write::apply(&mutation, case.doc),
        Program::Query(_) => panic!("[{}] filter must be a mutation", case.id),
    }
}

/// Run one write-tier case and assert its expectation.
///
/// A [`WriteExpect::Rewrites`] expectation is checked by *building* the
/// expected document from the input, so the assertion covers the whole file:
/// every byte the case does not name has to come back unchanged.
fn check_write(case: &WriteCase) {
    match case.expect {
        WriteExpect::Rewrites(edits) => {
            let mut want = case.doc.to_string();
            for (from, to) in edits {
                assert_eq!(
                    want.matches(from).count(),
                    1,
                    "[{}] rewrite anchor {from:?} must match exactly one span",
                    case.id
                );
                want = want.replacen(from, to, 1);
            }
            assert_ne!(
                want, case.doc,
                "[{}] a rewrite case must change bytes",
                case.id
            );
            let got =
                run_write(case).unwrap_or_else(|e| panic!("[{}] mutation failed: {e}", case.id));
            assert_eq!(got, want, "[{}] byte output mismatch", case.id);
            // An edit may not produce a document yqr would reject: the write
            // path's own re-parse guard proves it loads, this proves it is
            // still clean YAML by the validator's standards.
            let findings = yqr::validate::check_str(&got, true);
            assert!(
                findings.is_empty(),
                "[{}] edited document must validate cleanly, got {findings:?}\n{got}",
                case.id
            );
        }
        WriteExpect::Unchanged => {
            let got =
                run_write(case).unwrap_or_else(|e| panic!("[{}] mutation failed: {e}", case.id));
            assert_eq!(got, case.doc, "[{}] document must be untouched", case.id);
        }
        WriteExpect::Err(code) => {
            let err = run_write(case).expect_err(&format!("[{}] expected a refusal", case.id));
            assert_eq!(
                err.exit_code(),
                code,
                "[{}] exit code mismatch: {err}",
                case.id
            );
        }
    }
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

#[test]
fn write_corpus_edits_only_the_targeted_bytes() {
    for case in corpus::write_cases() {
        check_write(&case);
    }
}

// Feature f012: every corpus document is real-world YAML the validate
// command must accept, in both default and strict mode — the corpus is the
// no-false-positives guard for the validator.
#[test]
fn corpus_documents_validate_cleanly() {
    let mut docs: Vec<&str> = corpus::classic_cases().iter().map(|c| c.doc).collect();
    docs.extend(corpus::engine_cases().iter().map(|c| c.doc));
    docs.extend(corpus::write_cases().iter().map(|c| c.doc));
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
        .chain(corpus::write_cases().iter().map(|c| c.id))
        .collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), total, "corpus case ids must be unique");
}
