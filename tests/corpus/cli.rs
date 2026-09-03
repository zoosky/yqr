//! The command-line tier of the corpus: every option and variant of the
//! `yqr` binary as a table of invocations, each with its observable
//! contract — exit status, standard output, standard error, and the file it
//! leaves behind.
//!
//! The other tiers call the library; this one runs the compiled binary, so
//! it is the only place the corpus covers argument parsing, the stdin /
//! `-` / file input variants, `--in-place` on a real file, the `validate`
//! subcommand, help and version, and the exit-code contract end to end.
//! Most cases run over the generated tenants shape ([`Doc::Tenants`]); the
//! production file ([`super::values::VALUES_YAML`]) has its own group,
//! which on the shipped noyalib pin pins the `yqr-b025` refusals on the
//! default path and flips with `yqr-f026`.
//!
//! Checks that need more than a substring — a re-parse of normalized
//! output, say — are [`Out::Satisfies`] functions; they may use the `yqr`
//! library, which both consumers of the corpus link.

use super::values::{self, VALUES_TENANTS, VALUES_YAML};

/// Which document an invocation runs over.
#[derive(Debug, Clone, Copy)]
pub enum Doc {
    /// No document: help, version and usage errors.
    None,
    /// A fixed document.
    Static(&'static str),
    /// The generated tenants shape at this size (`values::tenants`).
    Tenants(usize),
}

impl Doc {
    /// The document's text; empty for [`Doc::None`].
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Doc::None => String::new(),
            Doc::Static(s) => s.to_string(),
            Doc::Tenants(n) => values::tenants(n),
        }
    }
}

/// How the document reaches the binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feed {
    /// Piped to standard input. The args name no `@doc` file, or name it
    /// alongside (`validate @doc -`).
    Stdin,
    /// Written to a temporary file that replaces every `@doc` token in the
    /// args; standard input is empty.
    File,
}

/// An expectation on an output stream or on the file left behind.
#[derive(Debug, Clone, Copy)]
pub enum Out {
    /// Exactly these bytes.
    Exact(&'static str),
    /// Exactly the case's document, byte for byte.
    Input,
    /// The case's document with each `(from, to)` rewrite applied once, in
    /// order, and no other byte changed — the write contract, stated the
    /// way `WriteExpect::Rewrites` states it.
    Rewrites(&'static [(&'static str, &'static str)]),
    /// Nothing at all.
    Empty,
    /// Contains every one of these substrings.
    Contains(&'static [&'static str]),
    /// Exactly this many lines.
    Lines(usize),
    /// Passes this check, which reports what was wrong otherwise.
    Satisfies(fn(&str) -> Result<(), String>),
}

/// One invocation of the binary and its contract.
#[derive(Debug, Clone, Copy)]
pub struct CliCase {
    /// Stable identifier (`cli/<group>/<name>`); the runner selects by group.
    pub id: &'static str,
    /// The document.
    pub doc: Doc,
    /// How it reaches the binary.
    pub feed: Feed,
    /// Arguments. `@doc` names the document's file, `@invalid` a file that
    /// does not parse, `@dup` a file with a duplicate key, `@missing` a
    /// path that does not exist.
    pub args: &'static [&'static str],
    /// Expected exit status.
    pub status: i32,
    /// Expected standard output.
    pub stdout: Out,
    /// Expected standard error.
    pub stderr: Out,
    /// What the `@doc` file holds after the run, when the args name one.
    pub after: Option<Out>,
}

/// A file that does not parse, for `@invalid`.
pub const INVALID_YAML: &str = "key: [unclosed\n";

/// A file with a duplicate mapping key, for `@dup`: valid by default,
/// a finding under `--strict`.
pub const DUPLICATE_KEYS: &str = "a: 1\na: 2\n";

/// The short version line, exactly as `-V` prints it.
pub const SHORT_VERSION_LINE: &str = concat!("yqr ", env!("CARGO_PKG_VERSION"), "\n");

const SHAPE: Doc = Doc::Tenants(40);
const VALUES: Doc = Doc::Static(VALUES_YAML);

/// The first line of both the shape and the production file once
/// normalized: the anchor is gone, the quotes stay.
const NORMALIZED_FIRST_LINE: &str = "preImage: \"6.7.0-RC.5-2eb4505e\"\n";

/// Count the tenant keys in a normalized document through the library.
fn tenant_keys(out: &str) -> Result<usize, String> {
    yqr::eval_str(".argo.tenants | to_entries | .[] | .key", out)
        .map(|keys| keys.len())
        .map_err(|e| format!("output does not evaluate as YAML: {e}"))
}

/// `--normalize .` over the shape: comments gone, anchors resolved, every
/// tenant still there.
fn normalized_shape(out: &str) -> Result<(), String> {
    if !out.starts_with(NORMALIZED_FIRST_LINE) {
        return Err(format!(
            "does not start with the normalized first line: {out:.80?}"
        ));
    }
    if out.contains('#') {
        return Err("still carries a comment".into());
    }
    if out.contains("&preImage") || out.contains("<<:") {
        return Err("still carries an anchor or a merge key".into());
    }
    match tenant_keys(out)? {
        40 => Ok(()),
        n => Err(format!("expected 40 tenants, found {n}")),
    }
}

/// `--normalize .` over the production file: anchors resolved, every
/// tenant still there. Comments cannot be checked by `#` here — the file's
/// values contain `#` inside quoted strings.
fn normalized_values(out: &str) -> Result<(), String> {
    if !out.starts_with(NORMALIZED_FIRST_LINE) {
        return Err(format!(
            "does not start with the normalized first line: {out:.80?}"
        ));
    }
    if out.contains("&preImage") || out.contains("<<:") {
        return Err("still carries an anchor or a merge key".into());
    }
    match tenant_keys(out)? {
        VALUES_TENANTS => Ok(()),
        n => Err(format!("expected {VALUES_TENANTS} tenants, found {n}")),
    }
}

const EDIT_T3: &str = ".argo.tenants.t3.editorDomain = \"host-3b.example.invalid\"";
const EDIT_T3_REWRITE: &[(&str, &str)] = &[(
    "\"host-3.example.invalid\"  # editor endpoint\n",
    "\"host-3b.example.invalid\"  # editor endpoint\n",
)];

/// Every command-line case.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn cli_cases() -> Vec<CliCase> {
    vec![
        // -- help and version ---------------------------------------------
        CliCase {
            id: "cli/help/short-flag",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["-h"],
            status: 0,
            stdout: Out::Contains(&[
                "Usage",
                "--raw-output",
                "--normalize",
                "--in-place",
                "validate",
            ]),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/help/long-flag",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["--help"],
            status: 0,
            stdout: Out::Contains(&["Usage", "<FILTER>", "[FILE]"]),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/help/validate-subcommand",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["validate", "--help"],
            status: 0,
            stdout: Out::Contains(&["--strict", "Exit codes"]),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/version/short-is-the-bare-version",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["-V"],
            status: 0,
            stdout: Out::Exact(SHORT_VERSION_LINE),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/version/long-carries-build-provenance",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["--version"],
            status: 0,
            stdout: Out::Contains(&[env!("CARGO_PKG_VERSION"), ", built ", "target: "]),
            stderr: Out::Empty,
            after: None,
        },
        // -- usage errors: exit 2, nothing on stdout -----------------------
        CliCase {
            id: "cli/usage/no-arguments",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &[],
            status: 2,
            stdout: Out::Empty,
            stderr: Out::Contains(&["<FILTER>", "Usage"]),
            after: None,
        },
        CliCase {
            id: "cli/usage/unknown-flag",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["--bogus", "."],
            status: 2,
            stdout: Out::Empty,
            stderr: Out::Contains(&["unexpected argument", "--bogus"]),
            after: None,
        },
        CliCase {
            id: "cli/usage/removed-preserve-flag",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["--preserve", "."],
            status: 2,
            stdout: Out::Empty,
            stderr: Out::Contains(&["unexpected argument"]),
            after: None,
        },
        CliCase {
            id: "cli/usage/removed-engine-flag",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["--engine", "noyalib", "."],
            status: 2,
            stdout: Out::Empty,
            stderr: Out::Contains(&["unexpected argument"]),
            after: None,
        },
        CliCase {
            id: "cli/usage/help-word-is-a-filter",
            doc: SHAPE,
            feed: Feed::Stdin,
            args: &["help"],
            status: 3,
            stdout: Out::Empty,
            stderr: Out::Contains(&["parse error"]),
            after: None,
        },
        CliCase {
            id: "cli/usage/flag-before-validate-is-diagnosed",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-r", "validate", "@doc"],
            status: 2,
            stdout: Out::Empty,
            stderr: Out::Contains(&["subcommand and must come first"]),
            after: None,
        },
        CliCase {
            id: "cli/usage/normalize-before-validate-is-diagnosed",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-N", "validate", "@doc"],
            status: 2,
            stdout: Out::Empty,
            stderr: Out::Contains(&["subcommand and must come first"]),
            after: None,
        },
        CliCase {
            id: "cli/usage/validate-without-files",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["validate"],
            status: 2,
            stdout: Out::Empty,
            stderr: Out::Contains(&["no input files"]),
            after: None,
        },
        CliCase {
            id: "cli/usage/validate-stdin-twice",
            doc: SHAPE,
            feed: Feed::Stdin,
            args: &["validate", "-", "-"],
            status: 2,
            stdout: Out::Empty,
            stderr: Out::Contains(&["at most once"]),
            after: None,
        },
        // -- reads over the shape: input variants ---------------------------
        CliCase {
            id: "cli/read/identity-from-a-file",
            doc: SHAPE,
            feed: Feed::File,
            args: &[".", "@doc"],
            status: 0,
            stdout: Out::Input,
            stderr: Out::Empty,
            after: Some(Out::Input),
        },
        CliCase {
            id: "cli/read/identity-from-stdin",
            doc: SHAPE,
            feed: Feed::Stdin,
            args: &["."],
            status: 0,
            stdout: Out::Input,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/identity-from-dash",
            doc: SHAPE,
            feed: Feed::Stdin,
            args: &[".", "-"],
            status: 0,
            stdout: Out::Input,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/double-dash-ends-the-options",
            doc: SHAPE,
            feed: Feed::File,
            args: &["--", ".", "@doc"],
            status: 0,
            stdout: Out::Input,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/missing-file",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &[".", "@missing"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["failed to read"]),
            after: None,
        },
        // -- reads over the shape: raw output in every spelling ------------
        CliCase {
            id: "cli/read/own-scalar-keeps-its-quotes",
            doc: SHAPE,
            feed: Feed::File,
            args: &[".argo.tenants.t0.editorDomain", "@doc"],
            status: 0,
            stdout: Out::Exact("\"host-0.example.invalid\"\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/raw-short-flag",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-r", ".argo.tenants.t0.editorDomain", "@doc"],
            status: 0,
            stdout: Out::Exact("host-0.example.invalid\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/raw-long-flag",
            doc: SHAPE,
            feed: Feed::File,
            args: &["--raw-output", ".argo.tenants.t0.editorDomain", "@doc"],
            status: 0,
            stdout: Out::Exact("host-0.example.invalid\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/flag-after-the-positionals",
            doc: SHAPE,
            feed: Feed::File,
            args: &[".argo.tenants.t0.editorDomain", "@doc", "-r"],
            status: 0,
            stdout: Out::Exact("host-0.example.invalid\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/raw-leaves-a-number-alone",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-r", ".argo.tenants.t0.categories.weight", "@doc"],
            status: 0,
            stdout: Out::Exact("0\n"),
            stderr: Out::Empty,
            after: None,
        },
        // -- reads over the shape: the filter language through the binary --
        CliCase {
            id: "cli/read/merged-key-resolves",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-r", ".argo.tenants.t9.ops.DEFAULT_LANGUAGE", "@doc"],
            status: 0,
            stdout: Out::Exact("fr\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/iterate-every-tenant",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-r", ".argo.tenants[] | .editorDomain", "@doc"],
            status: 0,
            stdout: Out::Lines(40),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/to_entries-lists-every-key",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-r", ".argo.tenants | to_entries | .[] | .key", "@doc"],
            status: 0,
            stdout: Out::Lines(40),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/to_entries-negative-index",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-r", ".argo.tenants | to_entries | .[-1].key", "@doc"],
            status: 0,
            stdout: Out::Exact("t39\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/key-selector",
            doc: SHAPE,
            feed: Feed::File,
            args: &["key(.argo.tenants.t0)", "@doc"],
            status: 0,
            stdout: Out::Exact("t0\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/line-comment-selector",
            doc: SHAPE,
            feed: Feed::File,
            args: &["line_comment(.argo.tenants.t0.editorDomain)", "@doc"],
            status: 0,
            stdout: Out::Exact("editor endpoint\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/head-comment-selector",
            doc: SHAPE,
            feed: Feed::File,
            args: &["head_comment(.argo.tenants.t8.enabledProjects)", "@doc"],
            status: 0,
            stdout: Out::Exact("first tenant of default block o1\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/foot-comment-is-refused-at-parse",
            doc: SHAPE,
            feed: Feed::File,
            args: &["foot_comment(.argo.tenants.t0.editorDomain)", "@doc"],
            status: 3,
            stdout: Out::Empty,
            stderr: Out::Contains(&["foot_comment(...) is not supported"]),
            after: None,
        },
        CliCase {
            id: "cli/read/missing-path-is-null",
            doc: SHAPE,
            feed: Feed::File,
            args: &[".nope", "@doc"],
            status: 0,
            stdout: Out::Exact("null\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/optional-suppresses-a-type-error",
            doc: SHAPE,
            feed: Feed::File,
            args: &[".argo.tenants.t0.categories.stage.x?", "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/runtime-error-exits-five",
            doc: SHAPE,
            feed: Feed::File,
            args: &[".argo.tenants.t0.categories.stage.x", "@doc"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["runtime error"]),
            after: None,
        },
        CliCase {
            id: "cli/read/parse-error-exits-three",
            doc: SHAPE,
            feed: Feed::File,
            args: &["foo", "@doc"],
            status: 3,
            stdout: Out::Empty,
            stderr: Out::Contains(&["parse error"]),
            after: None,
        },
        // -- reads over the shape: --normalize in every spelling -----------
        CliCase {
            id: "cli/read/normalize-short-flag",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-N", ".", "@doc"],
            status: 0,
            stdout: Out::Satisfies(normalized_shape),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/normalize-long-flag",
            doc: SHAPE,
            feed: Feed::Stdin,
            args: &["--normalize", "."],
            status: 0,
            stdout: Out::Satisfies(normalized_shape),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/normalize-resolves-a-merge",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-N", ".argo.tenants.t9.ops.DEFAULT_LANGUAGE", "@doc"],
            status: 0,
            stdout: Out::Exact("fr\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/combined-short-flags",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-rN", ".argo.tenants.t0.editorDomain", "@doc"],
            status: 0,
            stdout: Out::Exact("host-0.example.invalid\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/flag-order-does-not-matter",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-N", "-r", ".argo.tenants.t0.editorDomain", "@doc"],
            status: 0,
            stdout: Out::Exact("host-0.example.invalid\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/read/normalize-refuses-a-selector",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-N", "key(.argo.tenants.t0)", "@doc"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["--normalize discards"]),
            after: None,
        },
        CliCase {
            id: "cli/read/in-place-needs-a-mutating-filter",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-i", ".", "@doc"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["requires a mutating filter"]),
            after: Some(Out::Input),
        },
        // -- writes over the shape: stdout and --in-place ------------------
        CliCase {
            id: "cli/write/assign-prints-and-leaves-the-file",
            doc: SHAPE,
            feed: Feed::File,
            args: &[EDIT_T3, "@doc"],
            status: 0,
            stdout: Out::Rewrites(EDIT_T3_REWRITE),
            stderr: Out::Empty,
            after: Some(Out::Input),
        },
        CliCase {
            id: "cli/write/assign-from-stdin-prints",
            doc: SHAPE,
            feed: Feed::Stdin,
            args: &[EDIT_T3],
            status: 0,
            stdout: Out::Rewrites(EDIT_T3_REWRITE),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/write/in-place-short-flag",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-i", EDIT_T3, "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(EDIT_T3_REWRITE)),
        },
        CliCase {
            id: "cli/write/in-place-long-flag",
            doc: SHAPE,
            feed: Feed::File,
            args: &["--in-place", EDIT_T3, "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(EDIT_T3_REWRITE)),
        },
        // A mutation always takes the byte-preserving write path;
        // `--normalize` changes nothing about it.
        CliCase {
            id: "cli/write/in-place-ignores-normalize",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-iN", EDIT_T3, "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(EDIT_T3_REWRITE)),
        },
        CliCase {
            id: "cli/write/in-place-with-stdin-is-refused",
            doc: SHAPE,
            feed: Feed::Stdin,
            args: &["-i", EDIT_T3],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["cannot be used with stdin"]),
            after: None,
        },
        CliCase {
            id: "cli/write/in-place-with-dash-is-refused",
            doc: SHAPE,
            feed: Feed::Stdin,
            args: &["-i", EDIT_T3, "-"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["cannot be used with stdin"]),
            after: None,
        },
        CliCase {
            id: "cli/write/update-in-place",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-i", ".argo.tenants.t3.categories.weight |= . + 1", "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(&[("weight: 3\n", "weight: 4\n")])),
        },
        CliCase {
            id: "cli/write/append-in-place",
            doc: SHAPE,
            feed: Feed::File,
            args: &[
                "-i",
                ".argo.global.additionalValuesFiles += \"values-extra.yaml\"",
                "@doc",
            ],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(&[(
                "      - values-sdw03.yaml\n",
                "      - values-sdw03.yaml\n      - values-extra.yaml\n",
            )])),
        },
        CliCase {
            id: "cli/write/delete-in-place",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-i", "del(.argo.tenants.t3.imageTag)", "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(&[(
                "        weight: 3\n      enabledProjects: \"web-site\"\n      contentModelType: \"standardsite\"\n      imageTag: {}\n",
                "        weight: 3\n      enabledProjects: \"web-site\"\n      contentModelType: \"standardsite\"\n",
            )])),
        },
        CliCase {
            id: "cli/write/rename-in-place",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-i", "key(.argo.tenants.t3) = \"t3-renamed\"", "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(&[("    t3:\n", "    t3-renamed:\n")])),
        },
        CliCase {
            id: "cli/write/line-comment-in-place",
            doc: SHAPE,
            feed: Feed::File,
            args: &[
                "-i",
                "line_comment(.argo.tenants.t3.editorDomain) = \"primary\"",
                "@doc",
            ],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(&[(
                "\"host-3.example.invalid\"  # editor endpoint\n",
                "\"host-3.example.invalid\"  # primary\n",
            )])),
        },
        CliCase {
            id: "cli/write/head-comment-in-place",
            doc: SHAPE,
            feed: Feed::File,
            args: &[
                "-i",
                "head_comment(.argo.tenants.t8.enabledProjects) = \"block two\"",
                "@doc",
            ],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(&[(
                "# first tenant of default block o1\n",
                "# block two\n",
            )])),
        },
        CliCase {
            id: "cli/write/swap-in-place",
            doc: SHAPE,
            feed: Feed::File,
            args: &[
                "-i",
                "swap(.argo.global.additionalValuesFiles; 0; 1)",
                "@doc",
            ],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(&[(
                "      - values-sdw02.yaml\n      - values-sdw03.yaml\n",
                "      - values-sdw03.yaml\n      - values-sdw02.yaml\n",
            )])),
        },
        CliCase {
            id: "cli/write/move-in-place",
            doc: SHAPE,
            feed: Feed::File,
            args: &[
                "-i",
                "move(.argo.global.additionalValuesFiles; 0; -1)",
                "@doc",
            ],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(&[(
                "      - values-sdw02.yaml\n      - values-sdw03.yaml\n",
                "      - values-sdw03.yaml\n      - values-sdw02.yaml\n",
            )])),
        },
        CliCase {
            id: "cli/write/refused-edit-leaves-the-file",
            doc: SHAPE,
            feed: Feed::File,
            args: &[
                "-i",
                ".argo.tenants.t3.ops.DEFAULT_LANGUAGE = \"rm\"",
                "@doc",
            ],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["Assign where the key is defined"]),
            after: Some(Out::Input),
        },
        CliCase {
            id: "cli/write/no-op-assignment-is-not-an-error",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-i", ".argo.tenants.t3.categories.weight = 3", "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Input),
        },
        CliCase {
            id: "cli/write/absent-delete-is-a-no-op",
            doc: SHAPE,
            feed: Feed::File,
            args: &["-i", "del(.argo.tenants.t3.nope)", "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Input),
        },
        // -- validate over the shape ---------------------------------------
        CliCase {
            id: "cli/validate/file",
            doc: SHAPE,
            feed: Feed::File,
            args: &["validate", "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Input),
        },
        CliCase {
            id: "cli/validate/strict-before-the-files",
            doc: SHAPE,
            feed: Feed::File,
            args: &["validate", "--strict", "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/validate/strict-after-the-files",
            doc: SHAPE,
            feed: Feed::File,
            args: &["validate", "@doc", "--strict"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/validate/stdin",
            doc: SHAPE,
            feed: Feed::Stdin,
            args: &["validate", "-"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/validate/file-and-stdin",
            doc: SHAPE,
            feed: Feed::Stdin,
            args: &["validate", "@doc", "-"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/validate/invalid-among-valid",
            doc: SHAPE,
            feed: Feed::File,
            args: &["validate", "@doc", "@invalid"],
            status: 1,
            stdout: Out::Empty,
            stderr: Out::Contains(&["error[Y001]", "invalid.yaml"]),
            after: None,
        },
        CliCase {
            id: "cli/validate/missing-among-valid",
            doc: SHAPE,
            feed: Feed::File,
            args: &["validate", "@missing", "@doc"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["failed to read", "missing.yaml"]),
            after: None,
        },
        CliCase {
            id: "cli/validate/duplicate-key-passes-by-default",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["validate", "@dup"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/validate/duplicate-key-is-a-strict-finding",
            doc: Doc::None,
            feed: Feed::Stdin,
            args: &["validate", "--strict", "@dup"],
            status: 1,
            stdout: Out::Empty,
            stderr: Out::Contains(&["error[Y101]", "duplicate mapping key"]),
            after: None,
        },
        // -- scale: the shape at a thousand tenants, and past the budget ----
        CliCase {
            id: "cli/scale/identity-1000-from-a-file",
            doc: Doc::Tenants(1000),
            feed: Feed::File,
            args: &[".", "@doc"],
            status: 0,
            stdout: Out::Input,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/scale/identity-1000-through-the-pipe",
            doc: Doc::Tenants(1000),
            feed: Feed::Stdin,
            args: &["."],
            status: 0,
            stdout: Out::Input,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/scale/merged-read-at-1000",
            doc: Doc::Tenants(1000),
            feed: Feed::File,
            args: &["-r", ".argo.tenants.t999.ops.DEFAULT_LANGUAGE", "@doc"],
            status: 0,
            stdout: Out::Exact("de\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/scale/validate-1000",
            doc: Doc::Tenants(1000),
            feed: Feed::File,
            args: &["validate", "--strict", "@doc"],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/scale/write-in-place-at-1000",
            doc: Doc::Tenants(1000),
            feed: Feed::File,
            args: &[
                "-i",
                ".argo.tenants.t999.categories.weight |= . + 1",
                "@doc",
            ],
            status: 0,
            stdout: Out::Empty,
            stderr: Out::Empty,
            after: Some(Out::Rewrites(&[("weight: 999\n", "weight: 1000\n")])),
        },
        // Past the parser's absolute alias budget every path refuses; this
        // is the ceiling the generator documents.
        CliCase {
            id: "cli/scale/alias-budget-refuses-1100-on-the-default-path",
            doc: Doc::Tenants(1100),
            feed: Feed::File,
            args: &[".", "@doc"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["alias expansion limit exceeded"]),
            after: None,
        },
        CliCase {
            id: "cli/scale/alias-budget-refuses-1100-under-normalize",
            doc: Doc::Tenants(1100),
            feed: Feed::File,
            args: &["-N", ".", "@doc"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["alias expansion limit exceeded"]),
            after: None,
        },
        CliCase {
            id: "cli/scale/alias-budget-fails-validate-at-1100",
            doc: Doc::Tenants(1100),
            feed: Feed::File,
            args: &["validate", "@doc"],
            status: 1,
            stdout: Out::Empty,
            stderr: Out::Contains(&["error[Y001]", "alias expansion limit exceeded"]),
            after: None,
        },
        // -- the production file ------------------------------------------
        // Bug b025: on the shipped pin the default path refuses the file on
        // the alias-to-anchor ratio; these four pin that refusal and its
        // wording, and flip with yqr-f026.
        CliCase {
            id: "cli/values/default-read-is-refused-on-this-pin",
            doc: VALUES,
            feed: Feed::File,
            args: &[".", "@doc"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["alias_anchor_ratio", "--normalize"]),
            after: Some(Out::Input),
        },
        CliCase {
            id: "cli/values/default-read-from-stdin-is-refused-on-this-pin",
            doc: VALUES,
            feed: Feed::Stdin,
            args: &[".preImage"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["alias_anchor_ratio", "--normalize"]),
            after: None,
        },
        CliCase {
            id: "cli/values/validate-is-refused-on-this-pin",
            doc: VALUES,
            feed: Feed::File,
            args: &["validate", "--strict", "@doc"],
            status: 1,
            stdout: Out::Empty,
            stderr: Out::Contains(&["error[Y001]", "parser resource heuristic"]),
            after: None,
        },
        CliCase {
            id: "cli/values/in-place-write-is-refused-on-this-pin",
            doc: VALUES,
            feed: Feed::File,
            args: &["-i", ".preImage = \"x\"", "@doc"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["alias_anchor_ratio"]),
            after: Some(Out::Input),
        },
        CliCase {
            id: "cli/values/filter-errors-come-before-the-input",
            doc: VALUES,
            feed: Feed::File,
            args: &["foo", "@doc"],
            status: 3,
            stdout: Out::Empty,
            stderr: Out::Contains(&["parse error"]),
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-identity",
            doc: VALUES,
            feed: Feed::File,
            args: &["-N", ".", "@doc"],
            status: 0,
            stdout: Out::Satisfies(normalized_values),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-identity-through-the-pipe",
            doc: VALUES,
            feed: Feed::Stdin,
            args: &["--normalize", "."],
            status: 0,
            stdout: Out::Satisfies(normalized_values),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-scalar",
            doc: VALUES,
            feed: Feed::File,
            args: &["-N", ".preImage", "@doc"],
            status: 0,
            stdout: Out::Exact("\"6.7.0-RC.5-2eb4505e\"\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-raw-scalar",
            doc: VALUES,
            feed: Feed::File,
            args: &["-rN", ".preImage", "@doc"],
            status: 0,
            stdout: Out::Exact("6.7.0-RC.5-2eb4505e\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-resolves-an-alias",
            doc: VALUES,
            feed: Feed::File,
            args: &[
                "-N",
                ".argo.global.authentication.defaults.keycloak.issuer",
                "@doc",
            ],
            status: 0,
            stdout: Out::Exact("\"https://host-6.example.invalid/realms/eIAM-Intranet\"\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-resolves-a-tenant-merge",
            doc: VALUES,
            feed: Feed::File,
            args: &[
                "-N",
                ".argo.tenants[\"pre-web-site\"].ops.DOCS_RUN_TASKS",
                "@doc",
            ],
            status: 0,
            stdout: Out::Exact("\"\"\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-own-key-beside-a-merge",
            doc: VALUES,
            feed: Feed::File,
            args: &[
                "-N",
                ".argo.tenants[\"pre-web-site\"].ops.DOCS_ES_REINDEX",
                "@doc",
            ],
            status: 0,
            stdout: Out::Exact("\"1\"\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-lists-every-tenant",
            doc: VALUES,
            feed: Feed::File,
            args: &["-rN", ".argo.tenants | to_entries | .[] | .key", "@doc"],
            status: 0,
            stdout: Out::Lines(VALUES_TENANTS),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-first-tenant",
            doc: VALUES,
            feed: Feed::File,
            args: &["-rN", ".argo.tenants | to_entries | .[0].key", "@doc"],
            status: 0,
            stdout: Out::Exact(concat!("sandbox01", "\n")),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-last-tenant",
            doc: VALUES,
            feed: Feed::File,
            args: &["-rN", ".argo.tenants | to_entries | .[-1].key", "@doc"],
            status: 0,
            stdout: Out::Exact(concat!("berufsbildcom", "\n")),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-flow-empty-mapping",
            doc: VALUES,
            feed: Feed::File,
            args: &["-N", ".argo.tenants.sandbox01.imageTag", "@doc"],
            status: 0,
            stdout: Out::Exact("{}\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-block-mapping",
            doc: VALUES,
            feed: Feed::File,
            args: &["-N", ".argo.tenants.sandbox01.categories", "@doc"],
            status: 0,
            stdout: Out::Exact("stage: prd\nliveness: temporary\n"),
            stderr: Out::Empty,
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-runtime-error",
            doc: VALUES,
            feed: Feed::File,
            args: &["-N", ".preImage[]", "@doc"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["cannot iterate"]),
            after: None,
        },
        CliCase {
            id: "cli/values/normalize-refuses-a-selector",
            doc: VALUES,
            feed: Feed::File,
            args: &["-N", "key(.preImage)", "@doc"],
            status: 5,
            stdout: Out::Empty,
            stderr: Out::Contains(&["--normalize discards"]),
            after: None,
        },
    ]
}
