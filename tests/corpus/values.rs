//! The tenants values corpus: one production values file, verbatim, and a
//! generator that reproduces its shape at any size.
//!
//! [`VALUES_YAML`] is a real Helm-style values file for a multi-tenant
//! deployment: a handful of anchored default blocks at the top, then
//! hundreds of tenant entries that merge those defaults with `<<: *anchor`
//! and override a few keys each. It is the shape the alias-to-anchor ratio
//! heuristic mis-fires on (`yqr-b025`), and the shape most users of a
//! values file actually have. Its profile, for the record:
//!
//! | dimension | count |
//! |---|---|
//! | bytes / lines | 282 127 / 7 889 |
//! | tenants under `argo.tenants` | 355 |
//! | anchors / aliases / `<<` merges | 23 / 923 / 281 |
//! | flow collections (`{}` and `[…]`) | 259 |
//! | double-quoted scalars | 3 616 |
//! | comment lines / inline comments | 17 / 18 |
//! | blank lines | 508 |
//!
//! [`tenants`] builds the same shape from a template — anchored default
//! blocks, tenant entries merging them, flow values, double-quoted strings,
//! comments and blank lines — with the alias-to-anchor ratio held below the
//! default cap at every size, so the default byte-preserving path reads it
//! today and the corpus can scale without waiting on the upstream fix.

use std::sync::LazyLock;

/// The production values file, byte for byte (`tests/data/values.yaml`).
///
/// On the shipped noyalib pin the default byte-preserving path refuses it
/// (221 merges over 22 anchors trip the ratio heuristic, `yqr-b025`); the
/// classic pipeline (`--normalize`) reads it. Cases that touch the default
/// path pin that refusal and flip with `yqr-f026`.
pub const VALUES_YAML: &str = include_str!("../data/values.yaml");

/// Number of entries under `argo.tenants` in [`VALUES_YAML`].
pub const VALUES_TENANTS: usize = 355;

/// First and last tenant keys in document order.
pub const VALUES_FIRST_TENANT: &str = "sandbox01";
pub const VALUES_LAST_TENANT: &str = "berufsbildcom";

/// Build a tenants values file with `n` tenant entries in the shape of
/// [`VALUES_YAML`].
///
/// Every eight tenants share one anchored operations block under
/// `argo.global.opsDefaults`, merged into each tenant's `ops` mapping with
/// `<<: *oK`. That keeps the alias-to-anchor ratio under 8 at every `n`,
/// below the parser's default cap of 10, so the default path reads the
/// document; the absolute alias budget (1 024 expansions) is the ceiling —
/// `n > 1023` is refused on every path, and the corpus pins that too.
///
/// The layout is deterministic: tenant `i` is `t{i}`, its hosts are
/// `host-{i}.example.invalid`, its default block is `o{i / 8}`, and its
/// default language rotates through `de`, `fr`, `it`, `en` by block.
#[must_use]
pub fn tenants(n: usize) -> String {
    const LANGUAGES: [&str; 4] = ["de", "fr", "it", "en"];
    let blocks = n.div_ceil(8).max(1);
    let mut s = String::with_capacity(n * 640 + blocks * 220 + 512);

    s.push_str("preImage: &preImage \"6.7.0-RC.5-2eb4505e\"\n");
    s.push_str("# shared operations flags, aliased once under argo.global\n");
    s.push_str("preOps: &preOps\n");
    s.push_str("  OPS_RELEASE_INSTALL_STAGE: \"\"\n");
    s.push_str("  DOCS_RUN_TASKS: \"\"\n");
    s.push_str("  DOCS_DB_MIGRATE: \"\"\n");
    s.push('\n');
    s.push_str("officeOps: &officeOps {}\n");
    s.push('\n');
    s.push_str("argo:\n");
    s.push_str("  global:\n");
    s.push_str("    additionalValuesFiles:\n");
    s.push_str("      - values-sdw02.yaml\n");
    s.push_str("      - values-sdw03.yaml\n");
    s.push_str("    envVars:\n");
    s.push_str("      NSB_URL: \"http://host-5.example.invalid:3000/NSBSubscriber\"\n");
    s.push_str("    ops: *preOps\n");
    s.push_str("    opsDefaults:\n");
    for b in 0..blocks {
        let lang = LANGUAGES[b % LANGUAGES.len()];
        s.push_str(&format!(
            "      o{b}: &o{b}\n        DOCS_RUN_TASKS: \"\"\n        DEFAULT_LANGUAGE: \"{lang}\"\n        ENABLED_LANGUAGES: \"de;fr;it;en;rm\"\n"
        ));
    }
    s.push('\n');
    s.push_str("  tenants:\n");
    s.push_str("    # generated tenants, one default block per eight\n");
    for i in 0..n {
        let block = i / 8;
        let liveness = if i % 3 == 0 { "temporary" } else { "permanent" };
        let head = if i % 8 == 0 {
            format!("      # first tenant of default block o{block}\n")
        } else {
            String::new()
        };
        if i > 0 && i % 8 == 0 {
            s.push_str(&format!(
                "    # default block o{block}: tenants t{i} to t{}\n",
                i + 7
            ));
        }
        s.push_str(&format!(
            "    t{i}:\n\
             \x20     imgixUrl: \"https://host-{i}.example.invalid\"\n\
             \x20     editorDomain: \"host-{i}.example.invalid\"  # editor endpoint\n\
             \x20     categories:\n\
             \x20       stage: prd\n\
             \x20       liveness: {liveness}\n\
             \x20       weight: {i}\n\
             {head}\
             \x20     enabledProjects: \"web-site\"\n\
             \x20     contentModelType: \"standardsite\"\n\
             \x20     imageTag: {{}}\n\
             \x20     ops:\n\
             \x20       <<: *o{block}\n\
             \x20       DOCS_ES_REINDEX: \"1\"\n\
             \x20     envVars:\n\
             \x20       DOCS_IMAGE_AWS_S3_BUCKET: \"website-docs-prod-t{i}-images\"\n\
             \x20       DOCS_FILE_AWS_S3_BUCKET: \"website-docs-prod-t{i}-files\"\n\
             \n"
        ));
    }
    s
}

/// The shape at forty tenants (five default blocks): small enough to read
/// in a failure message, large enough to have every feature of the file.
pub static TENANTS_40: LazyLock<String> = LazyLock::new(|| tenants(40));

/// The shape at a thousand tenants, the largest size under the parser's
/// absolute alias budget.
pub static TENANTS_1000: LazyLock<String> = LazyLock::new(|| tenants(1000));

use super::{Case, EngineCase, Expect, WriteCase, WriteExpect};

/// Classic-pipeline cases on the production file. The classic pipeline is
/// the one path that reads it on the shipped pin, so these are also the
/// corpus's in-process answer to "what is in this file".
#[must_use]
pub fn classic_cases() -> Vec<Case> {
    vec![
        Case {
            id: "values/classic/top-level-scalar",
            doc: VALUES_YAML,
            filter: ".preImage",
            expect: Expect::Values("\"6.7.0-RC.5-2eb4505e\""),
        },
        Case {
            id: "values/classic/raw-top-level-scalar",
            doc: VALUES_YAML,
            filter: ".preImage",
            expect: Expect::Raw("6.7.0-RC.5-2eb4505e\n"),
        },
        Case {
            id: "values/classic/alias-resolves-to-the-anchor-value",
            doc: VALUES_YAML,
            filter: ".argo.global.authentication.defaults.keycloak.issuer",
            expect: Expect::Values("\"https://host-6.example.invalid/realms/eIAM-Intranet\""),
        },
        Case {
            id: "values/classic/top-level-merge-provides-the-key",
            doc: VALUES_YAML,
            filter: ".preOfficeOps.DOCS_DB_MIGRATE",
            expect: Expect::Values("\"\""),
        },
        Case {
            id: "values/classic/tenant-merge-provides-the-key",
            doc: VALUES_YAML,
            filter: ".argo.tenants[\"pre-web-site\"].ops.DOCS_RUN_TASKS",
            expect: Expect::Values("\"\""),
        },
        Case {
            id: "values/classic/own-key-beside-a-merge",
            doc: VALUES_YAML,
            filter: ".argo.tenants[\"pre-web-site\"].ops.DOCS_ES_REINDEX",
            expect: Expect::Values("\"1\""),
        },
        Case {
            id: "values/classic/alias-valued-entry",
            doc: VALUES_YAML,
            filter: ".argo.tenants[\"pre-web-site\"].imageTag",
            expect: Expect::Values("\"6.7.0-RC.5-2eb4505e\""),
        },
        Case {
            id: "values/classic/index-first",
            doc: VALUES_YAML,
            filter: ".argo.global.additionalValuesFiles[0]",
            expect: Expect::Values("values-sdw02.yaml"),
        },
        Case {
            id: "values/classic/index-last",
            doc: VALUES_YAML,
            filter: ".argo.global.additionalValuesFiles[-1]",
            expect: Expect::Values("values-sdw02.yaml"),
        },
        Case {
            id: "values/classic/block-mapping-value",
            doc: VALUES_YAML,
            filter: ".argo.tenants.sandbox01.categories",
            expect: Expect::Values("stage: prd\nliveness: temporary\n"),
        },
        Case {
            id: "values/classic/flow-empty-mapping",
            doc: VALUES_YAML,
            filter: ".argo.tenants.sandbox01.imageTag",
            expect: Expect::Values("{}"),
        },
        Case {
            id: "values/classic/to_entries-first-key",
            doc: VALUES_YAML,
            filter: ".argo.tenants | to_entries | .[0].key",
            expect: Expect::Values(VALUES_FIRST_TENANT),
        },
        Case {
            id: "values/classic/to_entries-last-key",
            doc: VALUES_YAML,
            filter: ".argo.tenants | to_entries | .[-1].key",
            expect: Expect::Values(VALUES_LAST_TENANT),
        },
        Case {
            id: "values/classic/to_entries-counts-every-tenant",
            doc: VALUES_YAML,
            filter: ".argo.tenants | to_entries | .[] | .key",
            expect: Expect::Count(VALUES_TENANTS),
        },
        Case {
            id: "values/classic/missing-field-is-null",
            doc: VALUES_YAML,
            filter: ".nope",
            expect: Expect::Values("null"),
        },
        Case {
            id: "values/classic/optional-missing-field-is-null",
            doc: VALUES_YAML,
            filter: ".nope?",
            expect: Expect::Values("null"),
        },
        Case {
            id: "values/classic/field-on-a-string-is-a-runtime-error",
            doc: VALUES_YAML,
            filter: ".preImage.nope",
            expect: Expect::Err(5),
        },
        Case {
            id: "values/classic/iterating-a-string-is-a-runtime-error",
            doc: VALUES_YAML,
            filter: ".preImage[]",
            expect: Expect::Err(5),
        },
        Case {
            id: "values/classic/filter-parse-error",
            doc: VALUES_YAML,
            filter: "foo",
            expect: Expect::Err(3),
        },
    ]
}

/// Fidelity-engine cases on the generated shape: every projection the
/// default path makes over a merge-heavy values file, byte for byte.
#[must_use]
pub fn engine_cases() -> Vec<EngineCase> {
    vec![
        EngineCase {
            id: "values/engine/identity-40",
            doc: &TENANTS_40,
            filter: ".",
            raw: false,
            expect: &TENANTS_40,
        },
        EngineCase {
            id: "values/engine/own-scalar-keeps-its-quotes",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t0.editorDomain",
            raw: false,
            expect: "\"host-0.example.invalid\"\n",
        },
        EngineCase {
            id: "values/engine/raw-drops-the-quotes",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t0.editorDomain",
            raw: true,
            expect: "host-0.example.invalid\n",
        },
        // A merged-in value has no bytes of its own at the path, so the
        // engine renders it from the typed view: unquoted, the emitter's
        // spelling.
        EngineCase {
            id: "values/engine/merged-key-renders-typed",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t9.ops.DEFAULT_LANGUAGE",
            raw: false,
            expect: "fr\n",
        },
        EngineCase {
            id: "values/engine/own-key-beside-a-merge-keeps-its-bytes",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t9.ops.DOCS_ES_REINDEX",
            raw: false,
            expect: "\"1\"\n",
        },
        EngineCase {
            id: "values/engine/mapping-with-a-merge-renders-typed",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t0.ops",
            raw: false,
            expect: "DOCS_ES_REINDEX: \"1\"\nDOCS_RUN_TASKS: \"\"\nDEFAULT_LANGUAGE: de\nENABLED_LANGUAGES: de;fr;it;en;rm\n",
        },
        EngineCase {
            id: "values/engine/block-mapping-keeps-its-indent",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t0.categories",
            raw: false,
            expect: "        stage: prd\n        liveness: temporary\n        weight: 0\n",
        },
        EngineCase {
            id: "values/engine/alias-to-a-mapping-prints-the-anchor-bytes",
            doc: &TENANTS_40,
            filter: ".argo.global.ops",
            raw: false,
            expect: "  OPS_RELEASE_INSTALL_STAGE: \"\"\n  DOCS_RUN_TASKS: \"\"\n  DOCS_DB_MIGRATE: \"\"\n",
        },
        EngineCase {
            id: "values/engine/flow-empty-mapping",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t0.imageTag",
            raw: false,
            expect: "{}\n",
        },
        EngineCase {
            id: "values/engine/integer-projection",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t5.categories.weight",
            raw: false,
            expect: "5\n",
        },
        EngineCase {
            id: "values/engine/negative-index",
            doc: &TENANTS_40,
            filter: ".argo.global.additionalValuesFiles[-1]",
            raw: false,
            expect: "values-sdw03.yaml\n",
        },
        EngineCase {
            id: "values/engine/iterate-a-sequence",
            doc: &TENANTS_40,
            filter: ".argo.global.additionalValuesFiles[]",
            raw: false,
            expect: "values-sdw02.yaml\nvalues-sdw03.yaml\n",
        },
        EngineCase {
            id: "values/engine/key-of-an-own-entry",
            doc: &TENANTS_40,
            filter: "key(.argo.tenants.t0)",
            raw: false,
            expect: "t0\n",
        },
        EngineCase {
            id: "values/engine/key-of-a-merged-entry-is-null",
            doc: &TENANTS_40,
            filter: "key(.argo.tenants.t0.ops.DEFAULT_LANGUAGE)",
            raw: false,
            expect: "null\n",
        },
        EngineCase {
            id: "values/engine/line-comment-body",
            doc: &TENANTS_40,
            filter: "line_comment(.argo.tenants.t0.editorDomain)",
            raw: false,
            expect: "editor endpoint\n",
        },
        // The comment after `t0:` sits on a key whose value starts below;
        // no single-line node owns it, so it reads as absent.
        EngineCase {
            id: "values/engine/line-comment-on-a-block-valued-key-is-null",
            doc: &TENANTS_40,
            filter: "line_comment(.argo.tenants.t0)",
            raw: false,
            expect: "null\n",
        },
        EngineCase {
            id: "values/engine/head-comment-above-a-scalar-entry",
            doc: &TENANTS_40,
            filter: "head_comment(.argo.tenants.t8.enabledProjects)",
            raw: false,
            expect: "first tenant of default block o1\n",
        },
        // `# default block o1: ...` stands above `t8:` after a blank line;
        // the engine treats a run above a block-valued entry as belonging to
        // the section, not the entry, so no path addresses it.
        EngineCase {
            id: "values/engine/head-comment-above-a-block-valued-entry-is-null",
            doc: &TENANTS_40,
            filter: "head_comment(.argo.tenants.t8)",
            raw: false,
            expect: "null\n",
        },
        EngineCase {
            id: "values/engine/missing-field-is-null",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t0.nope",
            raw: false,
            expect: "null\n",
        },
    ]
}

/// Write-tier cases on the generated shape: every mutating form, each
/// stating the bytes it changes in a merge-heavy values file.
#[must_use]
pub fn write_cases() -> Vec<WriteCase> {
    vec![
        WriteCase {
            id: "values/write/assign-keeps-the-inline-comment",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t3.editorDomain = \"host-3b.example.invalid\"",
            expect: WriteExpect::Rewrites(&[(
                "\"host-3.example.invalid\"  # editor endpoint\n",
                "\"host-3b.example.invalid\"  # editor endpoint\n",
            )]),
        },
        WriteCase {
            id: "values/write/assign-a-plain-scalar",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t3.categories.liveness = \"permanent\"",
            expect: WriteExpect::Rewrites(&[(
                "liveness: temporary\n        weight: 3\n",
                "liveness: permanent\n        weight: 3\n",
            )]),
        },
        WriteCase {
            id: "values/write/assign-is-idempotent",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t3.categories.weight = 3",
            expect: WriteExpect::Unchanged,
        },
        WriteCase {
            id: "values/write/update-increments-in-place",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t3.categories.weight |= . + 1",
            expect: WriteExpect::Rewrites(&[("weight: 3\n", "weight: 4\n")]),
        },
        WriteCase {
            id: "values/write/append-a-sequence-item",
            doc: &TENANTS_40,
            filter: ".argo.global.additionalValuesFiles += \"values-extra.yaml\"",
            expect: WriteExpect::Rewrites(&[(
                "      - values-sdw03.yaml\n",
                "      - values-sdw03.yaml\n      - values-extra.yaml\n",
            )]),
        },
        WriteCase {
            id: "values/write/insert-a-new-key-at-the-end-of-the-tenant",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t3.helmBrancom = \"pre\"",
            expect: WriteExpect::Rewrites(&[(
                "DOCS_FILE_AWS_S3_BUCKET: \"website-docs-prod-t3-files\"\n",
                "DOCS_FILE_AWS_S3_BUCKET: \"website-docs-prod-t3-files\"\n      helmBrancom: \"pre\"\n",
            )]),
        },
        WriteCase {
            id: "values/write/own-key-beside-a-merge",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t3.ops.DOCS_ES_REINDEX = \"2\"",
            expect: WriteExpect::Rewrites(&[(
                "DOCS_ES_REINDEX: \"1\"\n      envVars:\n        DOCS_IMAGE_AWS_S3_BUCKET: \"website-docs-prod-t3-images\"",
                "DOCS_ES_REINDEX: \"2\"\n      envVars:\n        DOCS_IMAGE_AWS_S3_BUCKET: \"website-docs-prod-t3-images\"",
            )]),
        },
        // Bug b020: a merged-in key is not the tenant's own; the write is
        // refused and the message points at the definition.
        WriteCase {
            id: "values/write/merged-key-is-refused",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t3.ops.DEFAULT_LANGUAGE = \"rm\"",
            expect: WriteExpect::Err(5),
        },
        // The remedy that message names: assign where the key is defined.
        // noyalib 0.0.29 refuses this write (yqr-f026 s3); pinned as the
        // shipped 0.0.28 behaviour.
        WriteCase {
            id: "values/write/assign-at-the-anchor-definition",
            doc: &TENANTS_40,
            filter: ".argo.global.opsDefaults.o1.DEFAULT_LANGUAGE = \"rm\"",
            expect: WriteExpect::Rewrites(&[(
                "DEFAULT_LANGUAGE: \"fr\"\n",
                "DEFAULT_LANGUAGE: \"rm\"\n",
            )]),
        },
        WriteCase {
            id: "values/write/scalar-over-a-mapping-is-refused",
            doc: &TENANTS_40,
            filter: ".argo.tenants.t3.imageTag = \"7\"",
            expect: WriteExpect::Err(5),
        },
        WriteCase {
            id: "values/write/delete-a-flow-valued-entry",
            doc: &TENANTS_40,
            filter: "del(.argo.tenants.t3.imageTag)",
            expect: WriteExpect::Rewrites(&[(
                "        weight: 3\n      enabledProjects: \"web-site\"\n      contentModelType: \"standardsite\"\n      imageTag: {}\n",
                "        weight: 3\n      enabledProjects: \"web-site\"\n      contentModelType: \"standardsite\"\n",
            )]),
        },
        WriteCase {
            id: "values/write/delete-a-nested-block-mapping",
            doc: &TENANTS_40,
            filter: "del(.argo.tenants.t3.categories)",
            expect: WriteExpect::Rewrites(&[(
                "      categories:\n        stage: prd\n        liveness: temporary\n        weight: 3\n",
                "",
            )]),
        },
        WriteCase {
            id: "values/write/delete-of-an-absent-path-is-a-no-op",
            doc: &TENANTS_40,
            filter: "del(.argo.tenants.t3.nope)",
            expect: WriteExpect::Unchanged,
        },
        WriteCase {
            id: "values/write/rename-a-tenant-key",
            doc: &TENANTS_40,
            filter: "key(.argo.tenants.t3) = \"t3-renamed\"",
            expect: WriteExpect::Rewrites(&[("    t3:\n", "    t3-renamed:\n")]),
        },
        WriteCase {
            id: "values/write/replace-an-inline-comment",
            doc: &TENANTS_40,
            filter: "line_comment(.argo.tenants.t3.editorDomain) = \"primary\"",
            expect: WriteExpect::Rewrites(&[(
                "\"host-3.example.invalid\"  # editor endpoint\n",
                "\"host-3.example.invalid\"  # primary\n",
            )]),
        },
        WriteCase {
            id: "values/write/replace-a-head-comment",
            doc: &TENANTS_40,
            filter: "head_comment(.argo.tenants.t8.enabledProjects) = \"block two\"",
            expect: WriteExpect::Rewrites(&[(
                "# first tenant of default block o1\n",
                "# block two\n",
            )]),
        },
        WriteCase {
            id: "values/write/head-comment-above-a-block-valued-entry-is-refused",
            doc: &TENANTS_40,
            filter: "head_comment(.argo.tenants.t8) = \"second block\"",
            expect: WriteExpect::Err(5),
        },
        WriteCase {
            id: "values/write/swap-sequence-items",
            doc: &TENANTS_40,
            filter: "swap(.argo.global.additionalValuesFiles; 0; 1)",
            expect: WriteExpect::Rewrites(&[(
                "      - values-sdw02.yaml\n      - values-sdw03.yaml\n",
                "      - values-sdw03.yaml\n      - values-sdw02.yaml\n",
            )]),
        },
        WriteCase {
            id: "values/write/move-to-a-negative-index",
            doc: &TENANTS_40,
            filter: "move(.argo.global.additionalValuesFiles; 0; -1)",
            expect: WriteExpect::Rewrites(&[(
                "      - values-sdw02.yaml\n      - values-sdw03.yaml\n",
                "      - values-sdw03.yaml\n      - values-sdw02.yaml\n",
            )]),
        },
        WriteCase {
            id: "values/write/reorder-out-of-range-is-refused",
            doc: &TENANTS_40,
            filter: "swap(.argo.global.additionalValuesFiles; 0; 2)",
            expect: WriteExpect::Err(5),
        },
    ]
}
