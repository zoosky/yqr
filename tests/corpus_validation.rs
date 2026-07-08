//! Validation half of the shared corpus: every case in `tests/corpus` is run
//! through the classic pipeline and (where applicable) each fidelity engine,
//! and its output is asserted against the recorded expectation.
//!
//! The same corpus drives `benches/corpus_bench.rs`, so a case added here is
//! measured there for free.

#[path = "corpus/mod.rs"]
mod corpus;

use corpus::{Case, Engine, EngineCase, Expect};
use rust_yaml::Yaml;
use yqr::{eval_str, render};

/// Run one classic-pipeline case and assert its expectation.
fn check_classic(case: &Case) {
    match case.expect {
        Expect::Values(expected_yaml) => {
            let out = eval_str(case.filter, case.doc)
                .unwrap_or_else(|e| panic!("[{}] should evaluate, got error: {e}", case.id));
            let want = Yaml::new()
                .load_all_str(expected_yaml)
                .unwrap_or_else(|e| panic!("[{}] expected YAML must parse: {e}", case.id));
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

/// Map a corpus [`Engine`] to a compiled-in [`yqr::fidelity::BackendId`], or
/// `None` when that backend is not part of the current build.
fn backend(engine: Engine) -> Option<yqr::fidelity::BackendId> {
    // Only referenced by the compiled-in backend arms below; under
    // `--no-default-features` both arms vanish and so must the import.
    #[cfg(any(feature = "backend-noyalib", feature = "backend-rust-yaml"))]
    use yqr::fidelity::BackendId;
    match engine {
        Engine::Noyalib => {
            #[cfg(feature = "backend-noyalib")]
            {
                Some(BackendId::NoyalibCst)
            }
            #[cfg(not(feature = "backend-noyalib"))]
            {
                None
            }
        }
        Engine::RustYaml => {
            #[cfg(feature = "backend-rust-yaml")]
            {
                Some(BackendId::RustYamlRoundTrip)
            }
            #[cfg(not(feature = "backend-rust-yaml"))]
            {
                None
            }
        }
    }
}

/// Run one engine case against every applicable, compiled-in backend.
fn check_engine(case: &EngineCase) {
    for &engine in case.engines {
        let Some(backend) = backend(engine) else {
            continue;
        };
        let got = yqr::fidelity::run(backend, case.filter, case.doc, case.raw)
            .unwrap_or_else(|e| panic!("[{}/{engine:?}] engine run failed: {e}", case.id));
        assert_eq!(
            got, case.expect,
            "[{}/{engine:?}] byte output mismatch",
            case.id
        );
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
