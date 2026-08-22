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
    APP_CONFIG, CRLF_APP_CONFIG, DOCKER_COMPOSE, FIDELITY_RICH, GH_ACTIONS, GH_ACTIONS_FLOW,
    HELM_VALUES, K8S_DEPLOYMENT, MULTI_DOC,
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
        // -- to_entries (f017) ------------------------------------------------
        // The `yqr-r003` task, which sent an agent to a Python script: for
        // each named entry, report the name alongside a field of its value.
        // `.services[]` iterates the *values*, so the names were gone by the
        // time anything could use them; this is that task as one filter.
        Case {
            id: "to_entries/pairs-a-named-mapping",
            doc: DOCKER_COMPOSE,
            filter: ".services | to_entries",
            expect: Expect::Values(
                "- key: web\n  value:\n    image: nginx:1.27\n    ports:\n      - \"80:80\"\n      - \"443:443\"\n    depends_on:\n      - db\n    environment:\n      NGINX_HOST: example.com\n      NGINX_PORT: \"80\"\n- key: db\n  value:\n    image: postgres:16\n    volumes:\n      - pgdata:/var/lib/postgresql/data\n    environment:\n      POSTGRES_PASSWORD: secret\n",
            ),
        },
        Case {
            id: "to_entries/keys-come-out-in-document-order",
            doc: DOCKER_COMPOSE,
            // `web` before `db` is the file's order and not sorted order, so
            // a future sort cannot land here quietly.
            filter: ".services | to_entries[] | .key",
            expect: Expect::Raw("web\ndb\n"),
        },
        Case {
            id: "to_entries/reaches-the-value-half-of-the-pair",
            doc: DOCKER_COMPOSE,
            filter: ".services | to_entries[] | .value.image",
            expect: Expect::Raw("nginx:1.27\npostgres:16\n"),
        },
        Case {
            id: "to_entries/refuses-a-sequence",
            doc: DOCKER_COMPOSE,
            filter: ".services.web.ports | to_entries",
            expect: Expect::Err(5),
        },
        Case {
            id: "to_entries/is-not-a-place-to-write",
            doc: DOCKER_COMPOSE,
            filter: ".services | to_entries = 1",
            expect: Expect::Err(3),
        },
        Case {
            id: "to_entries/is-still-an-ordinary-field-name",
            doc: DOCKER_COMPOSE,
            filter: ".services.to_entries",
            expect: Expect::Values("null"),
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
        // -- Feature f007: the key selector reads the document's own token ---
        EngineCase {
            id: "engine/key/nested-plain",
            doc: K8S_DEPLOYMENT,
            filter: "key(.metadata.name)",
            raw: false,
            expect: "name\n",
        },
        EngineCase {
            id: "engine/key/merge-produced-key-is-null",
            // `retries` reaches `service` through `<<: *defaults`, so it is in
            // the typed value but owns no token in the file. Answering from
            // the filter's own path segment would report `retries` here — the
            // exact case that separates a document read from an echo.
            doc: FIDELITY_RICH,
            filter: "key(.service.retries)",
            raw: false,
            expect: "null\n",
        },
        // -- Feature f007: comment reads (a002 slice 2) ----------------------
        EngineCase {
            id: "engine/comment/inline-on-a-real-manifest",
            doc: FIDELITY_RICH,
            filter: "line_comment(.defaults.timeout)",
            raw: true,
            expect: "seconds\n",
        },
        EngineCase {
            id: "engine/comment/head-block-is-detached-so-null",
            // `# deployment defaults` is separated from `defaults:` by nothing,
            // but it is the document's first line — the run is attached, so
            // this reads it. The detached case is covered in the unit tests.
            doc: FIDELITY_RICH,
            filter: "head_comment(.defaults)",
            raw: true,
            expect: "deployment defaults\n",
        },
        EngineCase {
            id: "engine/comment/absent-is-null",
            doc: K8S_DEPLOYMENT,
            filter: "line_comment(.metadata.name)",
            raw: false,
            expect: "null\n",
        },
        EngineCase {
            id: "engine/key/sequence-item-is-null",
            doc: GH_ACTIONS,
            filter: "key(.jobs.build.steps[0])",
            raw: false,
            expect: "null\n",
        },
        EngineCase {
            id: "engine/raw/top-level-string",
            doc: APP_CONFIG,
            filter: ".logging.level",
            raw: true,
            expect: "warn\n",
        },
    ]
}

/// What a write case's mutation must do to its document.
///
/// The write tier's contract is *narrower* than "these bytes come out": an
/// accepted edit changes only the targeted node's bytes and leaves every other
/// byte identical. [`Rewrites`](WriteExpect::Rewrites) states exactly that —
/// the edited spans, and by omission the whole rest of the file — so a case
/// reads as the edit it describes rather than as a wall of expected document.
#[derive(Debug, Clone, Copy)]
pub enum WriteExpect {
    /// The output is the input with each `(from, to)` pair applied once, in
    /// order, and **no other byte changed**. Each `from` must occur exactly
    /// once at the point it applies, so an anchor can never match the wrong
    /// span and quietly weaken the case.
    Rewrites(&'static [(&'static str, &'static str)]),
    /// The output is byte-identical to the input: either the target resolves
    /// nowhere (a no-op, not an error, so a batch edit skips files that lack
    /// the path) or the edit writes back what was already there.
    Unchanged,
    /// The mutation is refused, and the error's jq-style exit code equals this
    /// (`3` for lex/parse, `5` for eval/IO). Refusal is half the write
    /// contract: an edit that would restructure the document must fail rather
    /// than emit something plausible.
    Err(i32),
}

/// One write-tier case: a mutating filter over a document, with the byte-level
/// edit it must make.
#[derive(Debug, Clone, Copy)]
pub struct WriteCase {
    /// Stable identifier.
    pub id: &'static str,
    /// The input document.
    pub doc: &'static str,
    /// The filter source. Must parse as a mutation.
    pub filter: &'static str,
    /// What the write path must do.
    pub expect: WriteExpect,
}

/// Every write-tier case: the mutating half of the corpus.
///
/// The read tiers cannot reach any of this — `fidelity::run` refuses a
/// mutating filter, by way of `parser::parse` — so without this table an
/// engine bump could reintroduce a corruption in insert, delete, rename,
/// comment or reorder arithmetic and still pass the whole corpus. Every
/// shipped write operation has at least one case here, on a genuine document,
/// plus the refusals that keep an edit from restructuring a file.
#[must_use]
pub fn write_cases() -> Vec<WriteCase> {
    vec![
        // -- value assignment -------------------------------------------------
        WriteCase {
            id: "write/assign/scalar-keeps-every-other-byte",
            doc: K8S_DEPLOYMENT,
            filter: ".spec.replicas = 5",
            expect: WriteExpect::Rewrites(&[("  replicas: 3\n", "  replicas: 5\n")]),
        },
        WriteCase {
            id: "write/assign/quote-style-follows-the-neighbour",
            doc: FIDELITY_RICH,
            filter: ".service.name = \"web-backend\"",
            expect: WriteExpect::Rewrites(&[("'web-frontend'", "'web-backend'")]),
        },
        WriteCase {
            id: "write/assign/idempotent-write-is-byte-identical",
            doc: HELM_VALUES,
            filter: ".replicaCount = 2",
            expect: WriteExpect::Unchanged,
        },
        // Bug b018: the case the one above could not catch. `2` is already
        // canonical, so it stayed byte-identical even while the guard was
        // missing. `0640` is the scalar that exposes it — the typed model
        // holds it as the same `Int` as `640`, so an unguarded re-emit drops
        // the leading zero and changes the permission.
        WriteCase {
            id: "write/assign/idempotent-write-keeps-a-non-canonical-spelling",
            doc: APP_CONFIG,
            filter: ".logging.file_mode = .logging.file_mode",
            expect: WriteExpect::Unchanged,
        },
        // Bug b018, the `|=` half: same rule, the other operator, reaching the
        // same guard by the other resolver.
        WriteCase {
            id: "write/update/idempotent-update-keeps-a-non-canonical-spelling",
            doc: APP_CONFIG,
            filter: ".logging.file_mode |= .",
            expect: WriteExpect::Unchanged,
        },
        // Bug b019: the value guard may not skip past a refusal. `retries` is
        // reached through `<<: *defaults`, so it is not `service`'s own entry
        // to write — assigning the value it already reports must refuse, not
        // silently leave the merge in place.
        WriteCase {
            id: "write/assign/a-no-op-does-not-swallow-a-borrowed-site",
            doc: FIDELITY_RICH,
            filter: ".service.retries = 3",
            expect: WriteExpect::Err(5),
        },
        // Bug b020: `service.retries` reads through `<<: *defaults`, so the
        // refusal is right -- and its reason must name that, not deny a path
        // the read tier resolves. Pinned as an exit code here; the wording is
        // asserted at the CLI level, where a message belongs.
        WriteCase {
            id: "write/assign/a-merged-in-key-is-refused",
            doc: FIDELITY_RICH,
            filter: ".service.retries = 9",
            expect: WriteExpect::Err(5),
        },
        // Bug b019: the same rule on a comment. `#primary` and `# primary`
        // carry one body, and the body is all a comment write is given, so
        // writing it back must leave the line alone. It has to be the *tight*
        // comment: re-emitting the wide-gutter one lands on its own spelling
        // and the case would pass with no guard at all.
        WriteCase {
            id: "write/comment/idempotent-comment-write-keeps-the-line",
            doc: FIDELITY_RICH,
            filter: "line_comment(.service.region) = \"primary\"",
            expect: WriteExpect::Unchanged,
        },
        // Bug b008: a multi-line string is a block scalar whose continuation
        // lines belong to the *insertion site's* indentation, not to the
        // rendering's. Getting this wrong re-parses as extra nodes.
        WriteCase {
            id: "write/assign/multi-line-string-is-indented-for-its-site",
            doc: GH_ACTIONS,
            filter: ".jobs.build.steps[2].run = \"cargo test --all-features\\ncargo test --doc\"",
            expect: WriteExpect::Rewrites(&[(
                "        run: cargo test --all-features\n",
                "        run: |-\n          cargo test --all-features\n          cargo test --doc\n",
            )]),
        },
        // -- new-key insertion ------------------------------------------------
        // Bug b013: the inserted value takes the spelling of its neighbours,
        // not of the document. Two quoted numbers elsewhere in this manifest
        // (`value: "30"` in an env block, `cpu: "1"` in a resource limit) used
        // to carry a document-wide dominance vote and quote this line; the
        // vote is now scored at the edit site, where every sibling is plain.
        WriteCase {
            id: "write/insert/new-key-under-a-nested-mapping",
            doc: K8S_DEPLOYMENT,
            filter: ".spec.template.metadata.labels.tier = \"web\"",
            expect: WriteExpect::Rewrites(&[(
                "        app: web\n",
                "        app: web\n        tier: web\n",
            )]),
        },
        // Bug b012: the same insert into `.metadata.labels`, whose keys all
        // hold a `.`. The insert anchor used to be found by composing each
        // candidate key back into a path string and re-parsing it, which no
        // dotted key survives, so the standard Kubernetes label block refused
        // every insertion. The anchor now comes from the span tree, so a key's
        // spelling no longer decides whether it can be inserted beside.
        WriteCase {
            id: "write/insert/mapping-whose-keys-hold-a-dot",
            doc: K8S_DEPLOYMENT,
            filter: ".metadata.labels.tier = \"web\"",
            expect: WriteExpect::Rewrites(&[(
                "    app.kubernetes.io/component: frontend\n",
                "    app.kubernetes.io/component: frontend\n    tier: web\n",
            )]),
        },
        WriteCase {
            id: "write/insert/multi-line-string-value",
            doc: K8S_DEPLOYMENT,
            filter: ".metadata.annotations = \"owner: platform\\nrotate: yearly\"",
            expect: WriteExpect::Rewrites(&[(
                "    app.kubernetes.io/component: frontend\n",
                "    app.kubernetes.io/component: frontend\n  annotations: |-\n    owner: platform\n    rotate: yearly\n",
            )]),
        },
        WriteCase {
            id: "write/insert/string-is-quoted-when-plain-would-change-type",
            doc: HELM_VALUES,
            filter: ".image.digest = \"1234\"",
            expect: WriteExpect::Rewrites(&[(
                "  pullPolicy: IfNotPresent\n",
                "  pullPolicy: IfNotPresent\n  digest: \"1234\"\n",
            )]),
        },
        // -- computed update (f008) -------------------------------------------
        // The limitation the Kubernetes guide named until now: "you cannot say
        // increment the replica count; you say what it should become".
        WriteCase {
            id: "write/update/increment-a-scalar",
            doc: K8S_DEPLOYMENT,
            filter: ".spec.replicas |= (. + 1)",
            expect: WriteExpect::Rewrites(&[("  replicas: 3\n", "  replicas: 4\n")]),
        },
        // The identity update is a no-op by construction: `set_value` would
        // re-emit the scalar from its typed value and canonicalise the
        // spelling, so an update that computes the value already there is
        // skipped rather than written (`yqr-b018`).
        WriteCase {
            id: "write/update/identity-is-byte-identical",
            doc: HELM_VALUES,
            filter: ".replicaCount |= .",
            expect: WriteExpect::Unchanged,
        },
        // `|=` inherits `=`'s scalar boundary: a collection result is refused.
        WriteCase {
            id: "write/update/refuses-a-collection-result",
            doc: K8S_DEPLOYMENT,
            filter: ".metadata.labels |= to_entries",
            expect: WriteExpect::Err(5),
        },
        // -- append -----------------------------------------------------------
        WriteCase {
            id: "write/append/sequence-item-at-the-site-indent",
            doc: APP_CONFIG,
            filter: ".features += \"billing\"",
            // b013 on the append path: a single quoted scalar (`zip: "007"`)
            // four entries away used to decide this line's spelling.
            expect: WriteExpect::Rewrites(&[("  - audit-log\n", "  - audit-log\n  - billing\n")]),
        },
        WriteCase {
            id: "write/append/multi-line-item-indents-for-its-site",
            doc: APP_CONFIG,
            filter: ".features += \"beta\\npreview\"",
            expect: WriteExpect::Rewrites(&[(
                "  - audit-log\n",
                "  - audit-log\n  - |-\n      beta\n      preview\n",
            )]),
        },
        // The other half of b008's shape: a value whose own lines are indented
        // has no unambiguous block-scalar spelling without an explicit
        // indentation indicator, so the typed tier falls back to an escaped
        // double-quoted scalar. That is the safe answer — the string loads back
        // exactly — and pinning it keeps a future emitter from "improving" it
        // into a block scalar that would re-parse as a nested sequence.
        WriteCase {
            id: "write/append/multi-line-item-with-inner-indentation-is-escaped",
            doc: APP_CONFIG,
            filter: ".features += \"beta:\\n  - preview\"",
            expect: WriteExpect::Rewrites(&[(
                "  - audit-log\n",
                "  - audit-log\n  - \"beta:\\n  - preview\"\n",
            )]),
        },
        // -- delete -----------------------------------------------------------
        WriteCase {
            id: "write/delete/nested-multi-line-mapping",
            doc: K8S_DEPLOYMENT,
            filter: "del(.spec.template.spec.containers[0].resources)",
            expect: WriteExpect::Rewrites(&[(
                "          resources:\n            requests:\n              cpu: 250m\n              memory: 256Mi\n            limits:\n              cpu: \"1\"\n              memory: 512Mi\n",
                "",
            )]),
        },
        WriteCase {
            id: "write/delete/sole-entry-spells-the-empty-collection",
            doc: MULTI_DOC,
            filter: "del(.data.LOG_LEVEL)",
            expect: WriteExpect::Rewrites(&[("data:\n  LOG_LEVEL: info\n", "data:\n  {}\n")]),
        },
        WriteCase {
            id: "write/delete/flow-sequence-member",
            doc: GH_ACTIONS_FLOW,
            filter: "del(.on[0])",
            expect: WriteExpect::Rewrites(&[("[push, pull_request]", "[pull_request]")]),
        },
        WriteCase {
            id: "write/delete/flow-mapping-member",
            doc: GH_ACTIONS_FLOW,
            filter: "del(.env.RUST_LOG)",
            expect: WriteExpect::Rewrites(&[(
                "{ RUST_LOG: warn, CARGO_TERM_COLOR: always }",
                "{ CARGO_TERM_COLOR: always }",
            )]),
        },
        WriteCase {
            id: "write/delete/absent-path-is-a-no-op",
            doc: K8S_DEPLOYMENT,
            filter: "del(.metadata.annotations)",
            expect: WriteExpect::Unchanged,
        },
        WriteCase {
            id: "write/delete/refuses-to-strand-an-alias",
            doc: FIDELITY_RICH,
            filter: "del(.defaults)",
            expect: WriteExpect::Err(5),
        },
        // -- key rename (a002 slice 1) ----------------------------------------
        WriteCase {
            id: "write/rename/rewrites-the-key-token-only",
            doc: K8S_DEPLOYMENT,
            filter: "key(.spec.replicas) = \"replicaCount\"",
            expect: WriteExpect::Rewrites(&[("  replicas: 3\n", "  replicaCount: 3\n")]),
        },
        WriteCase {
            id: "write/rename/refuses-a-sibling-collision",
            doc: K8S_DEPLOYMENT,
            filter: "key(.metadata.name) = \"namespace\"",
            expect: WriteExpect::Err(5),
        },
        // -- comment editing (a002 slice 2) -----------------------------------
        WriteCase {
            id: "write/comment/inline-added",
            doc: K8S_DEPLOYMENT,
            filter: "line_comment(.spec.replicas) = \"tuned for peak\"",
            expect: WriteExpect::Rewrites(&[(
                "  replicas: 3\n",
                "  replicas: 3  # tuned for peak\n",
            )]),
        },
        WriteCase {
            id: "write/comment/inline-changed-keeps-its-separator",
            doc: FIDELITY_RICH,
            filter: "line_comment(.defaults.timeout) = \"milliseconds\"",
            expect: WriteExpect::Rewrites(&[("# seconds", "# milliseconds")]),
        },
        WriteCase {
            id: "write/comment/head-lands-above-the-entry-at-its-indent",
            doc: HELM_VALUES,
            filter: "head_comment(.service.port) = \"cluster-internal only\"",
            expect: WriteExpect::Rewrites(&[(
                "  port: 8080\n",
                "  # cluster-internal only\n  port: 8080\n",
            )]),
        },
        // The limit a user meets first: heading a *section* is the natural
        // thing to want, and it is exactly what the engine's leading-comment
        // mutator does not do — it is restricted to single-line entries.
        WriteCase {
            id: "write/comment/refuses-a-head-comment-on-a-multi-line-entry",
            doc: HELM_VALUES,
            filter: "head_comment(.service) = \"cluster-internal only\"",
            expect: WriteExpect::Err(5),
        },
        WriteCase {
            id: "write/comment/removed-with-its-separator",
            doc: FIDELITY_RICH,
            filter: "del(line_comment(.defaults.timeout))",
            expect: WriteExpect::Rewrites(&[("  timeout: 30      # seconds\n", "  timeout: 30\n")]),
        },
        WriteCase {
            id: "write/comment/refuses-an-entry-whose-value-starts-below",
            doc: K8S_DEPLOYMENT,
            filter: "line_comment(.metadata) = \"about the metadata\"",
            expect: WriteExpect::Err(5),
        },
        // -- sequence reorder (a002 slice 3) ----------------------------------
        WriteCase {
            id: "write/reorder/swaps-whole-multi-line-items",
            doc: GH_ACTIONS,
            filter: "swap(.jobs.build.steps; 0; 2)",
            expect: WriteExpect::Rewrites(&[(
                "      - name: Checkout\n        uses: actions/checkout@v5\n      - name: Build\n        run: cargo build --release\n      - name: Test\n        run: cargo test --all-features\n",
                "      - name: Test\n        run: cargo test --all-features\n      - name: Build\n        run: cargo build --release\n      - name: Checkout\n        uses: actions/checkout@v5\n",
            )]),
        },
        WriteCase {
            id: "write/reorder/move-shifts-the-items-between",
            doc: GH_ACTIONS,
            filter: "move(.jobs.build.steps; -1; 0)",
            expect: WriteExpect::Rewrites(&[(
                "      - name: Checkout\n        uses: actions/checkout@v5\n      - name: Build\n        run: cargo build --release\n      - name: Test\n        run: cargo test --all-features\n",
                "      - name: Test\n        run: cargo test --all-features\n      - name: Checkout\n        uses: actions/checkout@v5\n      - name: Build\n        run: cargo build --release\n",
            )]),
        },
        WriteCase {
            id: "write/reorder/flow-members-exchange-values",
            doc: GH_ACTIONS_FLOW,
            filter: "swap(.on; 0; 1)",
            expect: WriteExpect::Rewrites(&[("[push, pull_request]", "[pull_request, push]")]),
        },
        WriteCase {
            id: "write/reorder/refuses-an-out-of-range-index",
            doc: APP_CONFIG,
            filter: "swap(.features; 0; 9)",
            expect: WriteExpect::Err(5),
        },
        // -- cross-cutting: line terminators and multi-document streams -------
        // Bug b009: an inserted line takes the document's terminator, not the
        // platform's. A mixed-ending file is what `-i` would write to disk.
        WriteCase {
            id: "write/crlf/an-inserted-line-keeps-the-terminator",
            doc: CRLF_APP_CONFIG,
            filter: ".logging.format = \"json\"",
            expect: WriteExpect::Rewrites(&[(
                "  level: warn\r\n",
                "  level: warn\r\n  format: json\r\n",
            )]),
        },
        WriteCase {
            id: "write/multidoc/edits-every-document-that-resolves",
            doc: MULTI_DOC,
            filter: ".metadata.name = \"core\"",
            expect: WriteExpect::Rewrites(&[
                ("name: settings", "name: core"),
                ("name: web", "name: core"),
            ]),
        },
    ]
}
