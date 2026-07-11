//! Black-box tests for the compiled `yqr` binary.
//!
//! Uses `CARGO_BIN_EXE_yqr` (set by Cargo for integration tests) so no extra
//! dev-dependencies are needed.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Output {
    status: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str], stdin: &str) -> Output {
    let bin = env!("CARGO_BIN_EXE_yqr");
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn yqr");

    // Write the input, then drop the handle to close the pipe (so a reading
    // child sees EOF). A child that rejects its arguments and exits *before*
    // reading stdin closes its read end first, which surfaces here as a
    // `BrokenPipe`; that is not a test failure — the child's exit status and
    // output are what the assertions inspect.
    {
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

#[test]
fn field_access_from_stdin() {
    let out = run(&[".name"], "name: yqr\nversion: 1\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "yqr\n");
}

#[test]
fn raw_output_flag() {
    let out = run(&["-r", ".greeting"], "greeting: hello world\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "hello world\n");
}

#[test]
fn iterate_emits_multiple_lines() {
    let out = run(&["-r", ".tags[]"], "tags:\n  - a\n  - b\n  - c\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "a\nb\nc\n");
}

#[test]
fn parse_error_exits_three() {
    let out = run(&["foo"], "a: 1\n");
    assert_eq!(out.status, 3);
    assert!(out.stderr.contains("parse error"), "stderr: {}", out.stderr);
}

#[test]
fn runtime_error_exits_five() {
    // Iterating a scalar is a runtime error.
    let out = run(&[".x[]"], "x: 5\n");
    assert_eq!(out.status, 5);
    assert!(
        out.stderr.contains("runtime error"),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn help_flag_succeeds() {
    let out = run(&["--help"], "");
    assert_eq!(out.status, 0);
    assert!(out.stdout.contains("jq-style"));
}

#[test]
fn short_version_is_plain() {
    let out = run(&["-V"], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(
        out.stdout,
        format!("yqr {}\n", env!("CARGO_PKG_VERSION")),
        "-V should print just the crate version"
    );
}

#[test]
fn long_version_includes_build_info() {
    let out = run(&["--version"], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    // The long version embeds the crate version plus build provenance from
    // build.rs (commit, timestamp, target). Assert on stable substrings.
    assert!(
        out.stdout
            .starts_with(&format!("yqr {}", env!("CARGO_PKG_VERSION"))),
        "stdout: {}",
        out.stdout
    );
    assert!(out.stdout.contains("built "), "stdout: {}", out.stdout);
    assert!(out.stdout.contains("target: "), "stdout: {}", out.stdout);
}

#[test]
fn unknown_engine_is_an_io_error() {
    let out = run(&["--engine", "bogus", "."], "a: 1\n");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("unknown engine"),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn preserve_identity_reproduces_input_bytes() {
    // `--preserve` alone (no `--engine`) uses the default noyalib backend.
    let input = "# comment\nname: web   # inline\n\nreplicas: 3\n";
    let out = run(&["--preserve", "."], input);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, input, "identity must be byte-for-byte");
}

#[test]
fn preserve_short_flag_reproduces_input_bytes() {
    let input = "a: 1  # keep\n";
    let out = run(&["-p", "."], input);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, input);
}

#[test]
fn preserve_projection_keeps_original_spelling() {
    let out = run(&["--preserve", ".zip"], "zip: 007\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "007\n");
}

#[test]
fn engine_selects_backend_for_preserve() {
    // Feature f005: `--engine` names the backend; `--preserve` turns fidelity on.
    let input = "s: 'hi'  # quoted\n";
    let out = run(&["--engine", "noyalib", "--preserve", "."], input);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, input, "explicit noyalib backend must preserve");
}

#[test]
fn engine_without_preserve_re_serializes() {
    // Feature f005 clean break: `--engine noyalib` no longer implies preserve.
    // Without `--preserve` the classic pipeline runs and the comment is dropped.
    let out = run(&["--engine", "noyalib", "."], "name: web  # inline\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "name: web\n", "comment must be normalized away");
}

#[test]
fn unknown_engine_is_diagnosed_before_reading_input() {
    // A bad --engine must fail even when the input file does not exist:
    // engine validation happens before input is consumed.
    let out = run(&["--engine", "bogus", ".", "/nonexistent/input.yaml"], "");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("unknown engine"),
        "stderr: {}",
        out.stderr
    );
}

// -- Feature f006: write tier (assignment, +=, del, -i) -----------------------

/// Create a uniquely-named temp file seeded with `contents`, for `-i` tests.
fn temp_yaml(contents: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!("yqr-f006-{}-{n}.yaml", std::process::id()));
    std::fs::write(&path, contents).expect("write temp yaml");
    path
}

fn read_back(path: &Path) -> String {
    std::fs::read_to_string(path).expect("read temp yaml")
}

#[test]
fn assignment_to_stdout_is_byte_exact_except_the_edit() {
    let input = "# manifest\nspec:\n  replicas: 3   # keep\n  image: web\n";
    let out = run(&[".spec.replicas = 5"], input);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(
        out.stdout,
        "# manifest\nspec:\n  replicas: 5   # keep\n  image: web\n"
    );
}

#[test]
fn assignment_matches_neighbouring_quote_style() {
    let out = run(&[".name = \"web2\""], "name: 'web'\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "name: 'web2'\n");
}

#[test]
fn in_place_rewrites_only_the_target_line() {
    let file = temp_yaml("# app\nspec:\n  replicas: 3\n  image: web\n");
    let path = file.to_str().unwrap();
    let out = run(&["-i", ".spec.replicas = 5", path], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert!(out.stdout.is_empty(), "-i must not print to stdout");
    assert_eq!(
        read_back(&file),
        "# app\nspec:\n  replicas: 5\n  image: web\n"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
#[cfg(unix)]
fn in_place_preserves_file_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let file = temp_yaml("secret: value\n");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    let path = file.to_str().unwrap();
    let out = run(&["-i", ".secret = \"rotated\"", path], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(read_back(&file), "secret: rotated\n");
    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "in-place edit must preserve the original mode");
    let _ = std::fs::remove_file(&file);
}

#[test]
#[cfg(unix)]
fn in_place_edits_through_a_symlink() {
    // Editing a symlink must change the real file and leave the link intact,
    // not replace the link entry with a fresh regular file.
    let real = temp_yaml("a: 1\n");
    let mut link = real.clone().into_os_string();
    link.push(".link");
    let link = std::path::PathBuf::from(link);
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&real, &link).unwrap();

    let out = run(&["-i", ".a = 2", link.to_str().unwrap()], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(read_back(&real), "a: 2\n", "the real file must be edited");
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the symlink must survive the edit"
    );
    let _ = std::fs::remove_file(&link);
    let _ = std::fs::remove_file(&real);
}

#[test]
fn no_match_mutation_is_a_noop_not_an_error() {
    // `del` of an absent key succeeds and prints the input unchanged (jq/yq
    // semantics), so batch edits do not fail files that lack the field.
    let out = run(&["del(.deprecated)"], "kept: 1\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "kept: 1\n");
}

#[test]
fn float_overflow_literal_is_rejected() {
    // `1e999` overflows f64 to infinity; it must be a lex error, not silently
    // written as the bare token `inf`.
    let out = run(&[".x = 1e999"], "x: 1\n");
    assert_eq!(out.status, 3, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("out of range"),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn append_in_place_adds_a_block_sequence_item() {
    let file = temp_yaml("spec:\n  ports:\n    - 8080\n");
    let path = file.to_str().unwrap();
    let out = run(&["--in-place", ".spec.ports += 9090", path], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(
        read_back(&file),
        "spec:\n  ports:\n    - 8080\n    - 9090\n"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn delete_single_line_entry_to_stdout() {
    let out = run(
        &["del(.metadata.labels)"],
        "metadata:\n  name: app\n  labels: prod\n",
    );
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "metadata:\n  name: app\n");
}

#[test]
fn in_place_with_stdin_is_an_error() {
    let out = run(&["-i", ".a = 5"], "a: 1\n");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("stdin"), "stderr: {}", out.stderr);
}

#[test]
fn in_place_with_read_only_filter_is_an_error() {
    let file = temp_yaml("a: 1\n");
    let path = file.to_str().unwrap();
    let out = run(&["-i", ".a", path], "");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("requires a mutating filter"),
        "stderr: {}",
        out.stderr
    );
    // The file must be untouched.
    assert_eq!(read_back(&file), "a: 1\n");
    let _ = std::fs::remove_file(&file);
}

#[test]
fn refused_edit_leaves_the_file_unchanged_under_in_place() {
    // Deleting the only entry of a block would empty it (a structural change);
    // the write is refused (exit 5) and the file on disk must be byte-identical.
    let original = "only:\n  inner: 1\n";
    let file = temp_yaml(original);
    let path = file.to_str().unwrap();
    let out = run(&["-i", "del(.only)", path], "");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert_eq!(
        read_back(&file),
        original,
        "refused edit must not touch file"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn multi_line_delete_writes_back_in_place() {
    // A nested/multi-line entry is removed by the structural fallback; `-i`
    // writes the closed-up document back and leaves the sibling byte-identical.
    let original = "outer:\n  inner: 1\nother: 2\n";
    let file = temp_yaml(original);
    let path = file.to_str().unwrap();
    let out = run(&["-i", "del(.outer)", path], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(read_back(&file), "other: 2\n");
    let _ = std::fs::remove_file(&file);
}

#[test]
fn computed_update_operator_is_a_parse_error() {
    // `|=` is deferred; it must fail at parse time (exit 3) with a clear message.
    let out = run(&[".a |= 5"], "a: 1\n");
    assert_eq!(out.status, 3, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("not yet supported"),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn multi_document_edit_leaves_other_documents_byte_identical() {
    let input = "spec:\n  replicas: 1\n---\nkind: Service\n";
    let out = run(&[".spec.replicas = 9"], input);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "spec:\n  replicas: 9\n---\nkind: Service\n");
}
