//! Black-box tests for the compiled `yqr` binary.
//!
//! Uses `CARGO_BIN_EXE_yqr` (set by Cargo for integration tests) so no extra
//! dev-dependencies are needed.

use std::io::Write;
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
fn engine_identity_reproduces_input_bytes() {
    let input = "# comment\nname: web   # inline\n\nreplicas: 3\n";
    let out = run(&["--engine", "noyalib", "."], input);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, input, "identity must be byte-for-byte");
}

#[test]
fn engine_projection_keeps_original_spelling() {
    let out = run(&["--engine", "noyalib", ".zip"], "zip: 007\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "007\n");
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
