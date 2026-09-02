//! Command-line half of the shared corpus: every case in `tests/corpus/cli.rs`
//! is run through the compiled `yqr` binary and its exit status, standard
//! output, standard error, and the file it leaves behind are asserted.
//!
//! Cases are grouped by the middle segment of their id (`cli/<group>/…`),
//! one test per group so a failure names its area and the groups run in
//! parallel. `every_cli_case_belongs_to_a_group` keeps the two lists in
//! step, so no case can be added and silently never run.

#[path = "corpus/mod.rs"]
mod corpus;

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use corpus::cli::{CliCase, DUPLICATE_KEYS, Feed, INVALID_YAML, Out, cli_cases};

/// The groups the runner knows; `cli_cases` ids must use one of these.
const GROUPS: &[&str] = &[
    "help", "version", "usage", "read", "write", "validate", "scale", "values",
];

/// What the binary produced.
struct Output {
    status: i32,
    stdout: String,
    stderr: String,
}

/// Run the binary with `args`, feeding `stdin`.
fn run(args: &[String], stdin: &str) -> Output {
    let bin = env!("CARGO_BIN_EXE_yqr");
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn yqr");
    {
        // A child that exits before reading (a usage error, say) closes its
        // end first; the resulting BrokenPipe is not a failure of the case.
        let mut sin = child.stdin.take().expect("stdin");
        match sin.write_all(stdin.as_bytes()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(e) => panic!("write stdin: {e}"),
        }
    }
    let out = child.wait_with_output().expect("wait");
    Output {
        status: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// A per-case scratch directory, removed when the case finishes — on
/// failure too, since the assertion unwinds through the drop.
struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(id: &str) -> Self {
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "yqr-corpus-cli-{}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed),
            id.replace('/', "-")
        ));
        fs::create_dir_all(&dir).expect("create sandbox");
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    fn file(&self, name: &str, contents: &str) -> String {
        let path = self.path(name);
        fs::write(&path, contents).expect("write fixture");
        path.display().to_string()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Build the expected document from `doc` and the rewrites, the way the
/// write tier does: each anchor must match exactly one span, and the
/// result must differ from the input.
fn rewritten(id: &str, doc: &str, edits: &[(&str, &str)]) -> String {
    let mut want = doc.to_string();
    for (from, to) in edits {
        assert_eq!(
            want.matches(from).count(),
            1,
            "[{id}] rewrite anchor {from:?} must match exactly one span"
        );
        want = want.replacen(from, to, 1);
    }
    assert_ne!(want, doc, "[{id}] a rewrite case must change bytes");
    want
}

/// Assert `got` (the `what` stream or file) against `want`.
fn check(id: &str, what: &str, got: &str, want: Out, doc: &str) {
    match want {
        Out::Exact(s) => assert_eq!(got, s, "[{id}] {what} mismatch"),
        Out::Input => assert!(
            got == doc,
            "[{id}] {what} must equal the input byte for byte (got {} bytes, want {})",
            got.len(),
            doc.len()
        ),
        Out::Rewrites(edits) => {
            let want = rewritten(id, doc, edits);
            assert!(
                got == want,
                "[{id}] {what} must be the input with only the named rewrites; first difference at byte {}",
                got.bytes()
                    .zip(want.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(got.len().min(want.len()))
            );
        }
        Out::Empty => assert!(got.is_empty(), "[{id}] {what} must be empty, got {got:?}"),
        Out::Contains(needles) => {
            for needle in needles {
                assert!(
                    got.contains(needle),
                    "[{id}] {what} must contain {needle:?}, got:\n{got}"
                );
            }
        }
        Out::Lines(n) => assert_eq!(got.lines().count(), n, "[{id}] {what} line count"),
        Out::Satisfies(f) => {
            if let Err(why) = f(got) {
                panic!("[{id}] {what}: {why}");
            }
        }
    }
}

/// Run one case end to end.
fn run_case(case: &CliCase) {
    let doc = case.doc.text();
    let sandbox = Sandbox::new(case.id);
    let doc_path = sandbox.path("doc.yaml");
    let mut names_doc = false;
    let args: Vec<String> = case
        .args
        .iter()
        .map(|arg| match *arg {
            "@doc" => {
                names_doc = true;
                doc_path.display().to_string()
            }
            "@invalid" => sandbox.file("invalid.yaml", INVALID_YAML),
            "@dup" => sandbox.file("dup.yaml", DUPLICATE_KEYS),
            "@missing" => sandbox.path("missing.yaml").display().to_string(),
            other => other.to_string(),
        })
        .collect();
    if names_doc {
        fs::write(&doc_path, &doc).expect("write document");
    }
    assert!(
        case.after.is_none() || names_doc,
        "[{}] `after` needs an @doc file to inspect",
        case.id
    );

    let stdin = if case.feed == Feed::Stdin {
        doc.as_str()
    } else {
        ""
    };
    let out = run(&args, stdin);
    assert_eq!(
        out.status, case.status,
        "[{}] exit status; stderr: {}",
        case.id, out.stderr
    );
    check(case.id, "stdout", &out.stdout, case.stdout, &doc);
    check(case.id, "stderr", &out.stderr, case.stderr, &doc);
    if let Some(after) = case.after {
        let left = read_back(&doc_path);
        check(case.id, "the file afterwards", &left, after, &doc);
    }
}

fn read_back(path: &Path) -> String {
    fs::read_to_string(path).expect("read the document back")
}

/// The group segment of a case id (`cli/<group>/…`).
fn group_of(id: &str) -> &str {
    id.split('/').nth(1).unwrap_or("")
}

fn run_group(group: &str) {
    let cases: Vec<CliCase> = cli_cases()
        .into_iter()
        .filter(|c| group_of(c.id) == group)
        .collect();
    assert!(!cases.is_empty(), "group {group:?} has no cases");
    for case in &cases {
        run_case(case);
    }
}

#[test]
fn every_cli_case_belongs_to_a_group() {
    for case in cli_cases() {
        assert!(
            case.id.starts_with("cli/") && GROUPS.contains(&group_of(case.id)),
            "case id {:?} must be cli/<group>/<name> with a known group",
            case.id
        );
    }
}

#[test]
fn cli_help() {
    run_group("help");
}

#[test]
fn cli_version() {
    run_group("version");
}

#[test]
fn cli_usage() {
    run_group("usage");
}

#[test]
fn cli_read() {
    run_group("read");
}

#[test]
fn cli_write() {
    run_group("write");
}

#[test]
fn cli_validate() {
    run_group("validate");
}

#[test]
fn cli_scale() {
    run_group("scale");
}

#[test]
fn cli_values() {
    run_group("values");
}
