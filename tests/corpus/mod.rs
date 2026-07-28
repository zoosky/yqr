//! Shared, real-world corpus for yqr — one source of truth consumed by both
//! the validation suite (`tests/corpus_validation.rs`) and the benchmark
//! (`benches/corpus_bench.rs`).
//!
//! Every implemented filter operation and pipeline behavior has at least one
//! case built on a genuine document (Kubernetes, GitHub Actions, Docker
//! Compose, Helm, application config). Classic-pipeline expectations are stated
//! *semantically* (the expected value stream as YAML) so they are robust to the
//! emitter's exact formatting; raw-output and fidelity-engine expectations are
//! stated as exact bytes because their whole point is byte-level behavior.
//!
//! The module deliberately depends on nothing but `std` and its own data types,
//! so it compiles unchanged whether it is pulled into the test crate or the
//! bench crate.

// yqr-m003: shared validation + benchmark corpus.

// This module is compiled into both the validation-test crate and the
// benchmark crate; each consumes a different subset of the case fields, so
// per-crate dead-code analysis flags the other crate's fields. Silence it here
// rather than sprinkling per-item allows.
#![allow(dead_code)]

pub mod docs;

/// Expected result of running a case through the classic (re-serializing)
/// pipeline (`eval_str` + optional `render`).
#[derive(Debug, Clone, Copy)]
pub enum Expect {
    /// The output value stream equals the documents parsed from this YAML.
    /// Compared *semantically* (value equality), not byte-for-byte, so the
    /// emitter's formatting choices do not matter.
    Values(&'static str),
    /// The output stream is empty (e.g. an error swallowed by `?`).
    Empty,
    /// `render(output, raw = true)` equals this exact string.
    Raw(&'static str),
    /// The pipeline fails, and the error's jq-style exit code equals this
    /// (`3` for lex/parse, `5` for eval/IO).
    Err(i32),
}

/// One classic-pipeline case: a filter over a document with an expectation.
#[derive(Debug, Clone, Copy)]
pub struct Case {
    /// Stable identifier (`category/name`) used in test and benchmark labels.
    pub id: &'static str,
    /// The input document.
    pub doc: &'static str,
    /// The filter source.
    pub filter: &'static str,
    /// What the classic pipeline must produce.
    pub expect: Expect,
}

/// One fidelity-engine case: a filter whose output is checked byte-for-byte
/// against the original source.
#[derive(Debug, Clone, Copy)]
pub struct EngineCase {
    /// Stable identifier.
    pub id: &'static str,
    /// The input document.
    pub doc: &'static str,
    /// The filter source.
    pub filter: &'static str,
    /// Whether `--raw-output` is set.
    pub raw: bool,
    /// The exact bytes the engine must emit.
    pub expect: &'static str,
}

use docs::{
    APP_CONFIG, DOCKER_COMPOSE, FIDELITY_RICH, GH_ACTIONS, HELM_VALUES, K8S_DEPLOYMENT, MULTI_DOC,
};

/// Every classic-pipeline case. Covers identity, field access (top-level,
/// nested, deep, bracketed special-character keys), indexing (positive,
/// negative, out-of-range), iteration (sequences and mapping values), pipes,
/// the optional operator, null propagation, raw output, multi-document
/// first-doc semantics, and the full error taxonomy.
#[must_use]
pub fn classic_cases() -> Vec<Case> {
    vec![
        // -- identity ---------------------------------------------------------
        Case {
            id: "identity/k8s",
            doc: K8S_DEPLOYMENT,
            filter: ".",
            expect: Expect::Values(K8S_DEPLOYMENT),
        },
        // -- field access -----------------------------------------------------
        Case {
            id: "field/top-level",
            doc: HELM_VALUES,
            filter: ".replicaCount",
            expect: Expect::Values("2"),
        },
        Case {
            id: "field/nested",
            doc: K8S_DEPLOYMENT,
            filter: ".metadata.namespace",
            expect: Expect::Values("production"),
        },
        Case {
            id: "field/deep-through-index",
            doc: K8S_DEPLOYMENT,
            filter: ".spec.template.spec.containers[0].image",
            expect: Expect::Values("registry.example.com/web:1.4.2"),
        },
        Case {
            id: "field/bracket-special-key",
            doc: K8S_DEPLOYMENT,
            filter: ".metadata.labels[\"app.kubernetes.io/name\"]",
            expect: Expect::Values("web"),
        },
        Case {
            id: "field/quoted-scalar-value",
            doc: APP_CONFIG,
            filter: ".zip",
            expect: Expect::Values("\"007\""),
        },
        // -- indexing ---------------------------------------------------------
        Case {
            id: "index/positive",
            doc: APP_CONFIG,
            filter: ".features[0]",
            expect: Expect::Values("search"),
        },
        Case {
            id: "index/negative",
            doc: APP_CONFIG,
            filter: ".features[-1]",
            expect: Expect::Values("audit-log"),
        },
        Case {
            id: "index/out-of-range-null",
            doc: APP_CONFIG,
            filter: ".features[9]",
            expect: Expect::Values("null"),
        },
        // -- iteration --------------------------------------------------------
        Case {
            id: "iterate/sequence",
            doc: APP_CONFIG,
            filter: ".features[]",
            expect: Expect::Values("search\n---\nexport\n---\naudit-log"),
        },
        Case {
            id: "iterate/mapping-values",
            doc: DOCKER_COMPOSE,
            filter: ".services.web.environment[]",
            expect: Expect::Values("example.com\n---\n\"80\""),
        },
        // -- pipes ------------------------------------------------------------
        Case {
            id: "pipe/iterate-then-field",
            doc: K8S_DEPLOYMENT,
            filter: ".spec.template.spec.containers[] | .name",
            expect: Expect::Values("web\n---\nsidecar"),
        },
        Case {
            id: "pipe/multi-stage",
            doc: GH_ACTIONS,
            filter: ".jobs.build.steps[] | .name",
            expect: Expect::Values("Checkout\n---\nBuild\n---\nTest"),
        },
        Case {
            id: "pipe/explicit-stages",
            doc: K8S_DEPLOYMENT,
            filter: ".spec | .template | .spec | .containers | .[0] | .name",
            expect: Expect::Values("web"),
        },
        // -- optional ---------------------------------------------------------
        Case {
            id: "optional/suppresses-type-error",
            doc: K8S_DEPLOYMENT,
            filter: ".metadata.name[]?",
            expect: Expect::Empty,
        },
        Case {
            id: "optional/passes-value-through",
            doc: HELM_VALUES,
            filter: ".service.port?",
            expect: Expect::Values("8080"),
        },
        // -- null propagation -------------------------------------------------
        Case {
            id: "null/missing-field",
            doc: HELM_VALUES,
            filter: ".missing",
            expect: Expect::Values("null"),
        },
        Case {
            id: "null/propagates-through-field",
            doc: HELM_VALUES,
            filter: ".missing.deeper",
            expect: Expect::Values("null"),
        },
        // -- raw output -------------------------------------------------------
        Case {
            id: "raw/top-level-string",
            doc: APP_CONFIG,
            filter: ".logging.level",
            expect: Expect::Raw("warn\n"),
        },
        Case {
            id: "raw/iterate-strings",
            doc: APP_CONFIG,
            filter: ".features[]",
            expect: Expect::Raw("search\nexport\naudit-log\n"),
        },
        Case {
            id: "raw/non-string-falls-back-to-yaml",
            doc: HELM_VALUES,
            filter: ".replicaCount",
            expect: Expect::Raw("2\n"),
        },
        // -- multi-document (classic loads the first document) ----------------
        Case {
            id: "multidoc/classic-first-document",
            doc: MULTI_DOC,
            filter: ".kind",
            expect: Expect::Values("ConfigMap"),
        },
        // -- error taxonomy ---------------------------------------------------
        Case {
            id: "err/field-on-scalar",
            doc: APP_CONFIG,
            filter: ".server.port.foo",
            expect: Expect::Err(5),
        },
        Case {
            id: "err/iterate-scalar",
            doc: HELM_VALUES,
            filter: ".replicaCount[]",
            expect: Expect::Err(5),
        },
        Case {
            id: "err/index-mapping",
            doc: K8S_DEPLOYMENT,
            filter: ".metadata[0]",
            expect: Expect::Err(5),
        },
        Case {
            id: "err/parse-non-dot-start",
            doc: APP_CONFIG,
            filter: "features",
            expect: Expect::Err(3),
        },
        Case {
            id: "err/parse-trailing-bracket",
            doc: APP_CONFIG,
            filter: ".features]",
            expect: Expect::Err(3),
        },
        Case {
            id: "err/lex-unexpected-char",
            doc: APP_CONFIG,
            filter: ".features @ 1",
            expect: Expect::Err(3),
        },
    ]
}

/// Every fidelity-engine case. Covers byte-for-byte identity across real
/// documents and formatting dimensions (comments, anchors/aliases, block
/// scalars, quotes, multi-document streams), source-preserving projections,
/// and raw output on the engine path. Expectations that every backend must
/// honor identically use [`ALL`].
#[must_use]
pub fn engine_cases() -> Vec<EngineCase> {
    vec![
        // -- identity is byte-for-byte on every backend -----------------------
        EngineCase {
            id: "engine/identity/k8s",
            doc: K8S_DEPLOYMENT,
            filter: ".",
            raw: false,
            expect: K8S_DEPLOYMENT,
        },
        EngineCase {
            id: "engine/identity/compose",
            doc: DOCKER_COMPOSE,
            filter: ".",
            raw: false,
            expect: DOCKER_COMPOSE,
        },
        EngineCase {
            id: "engine/identity/rich-formatting",
            doc: FIDELITY_RICH,
            filter: ".",
            raw: false,
            expect: FIDELITY_RICH,
        },
        EngineCase {
            id: "engine/identity/multidoc",
            doc: MULTI_DOC,
            filter: ".",
            raw: false,
            expect: MULTI_DOC,
        },
        // -- projections emit the node's original bytes -----------------------
        EngineCase {
            id: "engine/projection/quoted-scalar",
            doc: APP_CONFIG,
            filter: ".zip",
            raw: false,
            expect: "\"007\"\n",
        },
        EngineCase {
            id: "engine/projection/plain-scalar",
            doc: HELM_VALUES,
            filter: ".image.repository",
            raw: false,
            expect: "registry.example.com/api\n",
        },
        // -- raw output on the engine path ------------------------------------
        EngineCase {
            id: "engine/raw/top-level-string",
            doc: APP_CONFIG,
            filter: ".logging.level",
            raw: true,
            expect: "warn\n",
        },
    ]
}
