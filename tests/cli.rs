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

// -- Feature f009: byte fidelity is the default read; --normalize opts out -----

#[test]
fn identity_reproduces_input_bytes_by_default() {
    // Fidelity is the default: no flag needed to round-trip byte-for-byte.
    let input = "# comment\nname: web   # inline\n\nreplicas: 3\n";
    let out = run(&["."], input);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, input, "identity must be byte-for-byte");
}

#[test]
fn projection_keeps_original_spelling_by_default() {
    let out = run(&[".zip"], "zip: 007\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "007\n", "leading zero must survive by default");
}

#[test]
fn normalize_re_serializes_and_drops_comments() {
    // `--normalize` runs the classic pipeline: comments are dropped and scalars
    // are canonicalized. The `-N` short flag is equivalent.
    for flag in ["--normalize", "-N"] {
        let out = run(&[flag, "."], "name: web  # inline\n");
        assert_eq!(out.status, 0, "stderr: {}", out.stderr);
        assert_eq!(out.stdout, "name: web\n", "comment must be normalized away");
    }
}

#[test]
fn normalize_canonicalizes_scalars() {
    // The classic pipeline reinterprets `007` as the integer 7.
    let out = run(&["--normalize", ".zip"], "zip: 007\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "7\n");
}

#[test]
fn preserve_flag_is_rejected() {
    // `--preserve`/`-p` were removed when fidelity became the default; clap must
    // reject them rather than silently accept.
    let long = run(&["--preserve", "."], "a: 1\n");
    assert_ne!(long.status, 0, "--preserve must no longer be accepted");
    let short = run(&["-p", "."], "a: 1\n");
    assert_ne!(short.status, 0, "-p must no longer be accepted");
}

#[test]
fn engine_flag_is_rejected() {
    // Feature f011: `--engine` was removed when yqr settled on noyalib as its
    // only engine; the binary must reject it rather than silently accept.
    let out = run(&["--engine", "noyalib", "."], "a: 1\n");
    assert_ne!(out.status, 0, "--engine must no longer be accepted");
}

// -- Feature f012: the validate subcommand ------------------------------------

#[test]
fn validate_valid_stdin_is_silent_and_exits_zero() {
    let out = run(&["validate", "-"], "a: 1\nb:\n  - x\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert!(out.stdout.is_empty(), "stdout: {}", out.stdout);
    assert!(out.stderr.is_empty(), "stderr: {}", out.stderr);
}

// Bug b014: a document noyalib reads and the rest of the ecosystem refuses.
#[test]
fn validate_reports_an_under_indented_block_value() {
    let out = run(&["validate", "-"], "on:\n[]\njobs: {}\n");
    assert_eq!(out.status, 1);
    let err = &out.stderr;
    assert!(
        err.contains("error[Y103]: block mapping value is not indented past its key"),
        "unexpected stderr: {err}"
    );
    // Default mode, not `--strict`: the file is invalid, not merely dubious.
    assert!(
        err.contains("-->"),
        "the finding must carry a location: {err}"
    );
    assert!(out.stdout.is_empty(), "validate writes findings to stderr");
}

#[test]
fn validate_accepts_a_block_sequence_at_its_keys_column() {
    // The GitHub Actions idiom, which is valid and must not be flagged.
    let out = run(&["validate", "--strict", "-"], "on:\n- push\njobs: {}\n");
    assert_eq!(out.status, 0);
    assert!(out.stderr.is_empty());
}

#[test]
fn validate_without_inputs_is_a_usage_error() {
    // No silent stdin fallback: a validation gate whose file list came up
    // empty must fail loudly instead of reporting "all valid" over
    // nothing (and instead of hanging on an interactive terminal).
    let out = run(&["validate"], "a: 1\n");
    assert_eq!(out.status, 2, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("'-'"), "stderr: {}", out.stderr);
}

#[test]
fn validate_rejects_stdin_twice() {
    // A second '-' would re-read an exhausted stream as an empty,
    // vacuously valid input.
    let out = run(&["validate", "-", "-"], "a: 1\n");
    assert_eq!(out.status, 2, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("at most once"),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn validate_non_utf8_input_is_a_coded_finding() {
    // Wrong encoding is a content defect (exit 1, Y003), not an
    // unreadable-input environment error (exit 5).
    let path = temp_yaml("placeholder");
    std::fs::write(&path, b"a: 1\nb: \xff\xfe\n").expect("write bytes");
    let out = run(&["validate", path.to_str().unwrap()], "");
    assert_eq!(out.status, 1, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("error[Y003]"), "stderr: {}", out.stderr);
    assert!(out.stderr.contains("UTF-8"), "stderr: {}", out.stderr);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn validate_full_conflict_block_is_located_with_help() {
    // A complete three-marker git conflict parses to an unlocated error;
    // the diagnostic must still name the marker and anchor at it.
    let out = run(
        &["validate", "-"],
        "a: 1\n<<<<<<< HEAD\nb: 2\n=======\nb: 3\n>>>>>>> feature\n",
    );
    assert_eq!(out.status, 1, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("merge-conflict"),
        "stderr: {}",
        out.stderr
    );
    assert!(out.stderr.contains("<stdin>:"), "stderr: {}", out.stderr);
}

#[test]
fn validate_strict_reports_every_duplicate_with_positions() {
    let out = run(&["validate", "--strict", "-"], "a: 1\nb: 2\na: 9\nb: 9\n");
    assert_eq!(out.status, 1, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("\"a\""), "stderr: {}", out.stderr);
    assert!(out.stderr.contains("\"b\""), "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("first occurrence at line 1"),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn validate_strict_reports_duplicate_merge_keys() {
    let out = run(
        &["validate", "--strict", "-"],
        "x: &a\n  k: 1\ny: &b\n  k: 2\nz:\n  <<: *a\n  <<: *b\n",
    );
    assert_eq!(out.status, 1, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("merge key"), "stderr: {}", out.stderr);
}

#[test]
fn help_word_is_still_a_filter_error() {
    // clap's auto `help` subcommand is disabled: the word `help` keeps its
    // pre-f012 meaning (an invalid filter, exit 3) so wrapper scripts
    // never mistake help text for filter output.
    let out = run(&["help"], "a: 1\n");
    assert_eq!(out.status, 3, "stderr: {}", out.stderr);
    assert!(out.stdout.is_empty(), "stdout: {}", out.stdout);
}

#[test]
fn flag_before_validate_gets_a_usage_hint() {
    // `-r` commits clap to the filter form; the binary recognizes the
    // stranded subcommand word and explains, instead of a bare filter
    // parse error.
    let out = run(&["-r", "validate", "a.yaml"], "");
    assert_eq!(out.status, 2, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("subcommand"), "stderr: {}", out.stderr);
}

#[test]
fn validate_invalid_yaml_exits_one_with_located_diagnostic() {
    let path = temp_yaml("a: 1\n---\nb: [1,\n");
    let out = run(&["validate", path.to_str().unwrap()], "");
    assert_eq!(out.status, 1, "stderr: {}", out.stderr);
    assert!(out.stdout.is_empty(), "stdout: {}", out.stdout);
    assert!(out.stderr.contains("error[Y001]"), "stderr: {}", out.stderr);
    let location = format!("--> {}:3:3", path.display());
    assert!(out.stderr.contains(&location), "stderr: {}", out.stderr);
    assert!(out.stderr.contains("3 | b: [1,"), "stderr: {}", out.stderr);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn validate_unreadable_file_exits_five_and_checks_the_rest() {
    let good = temp_yaml("a: 1\n");
    let bad = temp_yaml("b: [1,\n");
    let out = run(
        &[
            "validate",
            "/nonexistent/f012.yaml",
            good.to_str().unwrap(),
            bad.to_str().unwrap(),
        ],
        "",
    );
    // The unreadable input dominates the exit code, but the broken file is
    // still diagnosed in the same run.
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("failed to read"),
        "stderr: {}",
        out.stderr
    );
    assert!(out.stderr.contains("error[Y001]"), "stderr: {}", out.stderr);
    let _ = std::fs::remove_file(&good);
    let _ = std::fs::remove_file(&bad);
}

#[test]
fn validate_duplicate_key_needs_strict() {
    let dup = "a: 1\nb: 2\na: 3\n";
    let default = run(&["validate", "-"], dup);
    assert_eq!(default.status, 0, "stderr: {}", default.stderr);

    let strict = run(&["validate", "--strict", "-"], dup);
    assert_eq!(strict.status, 1, "stderr: {}", strict.stderr);
    assert!(
        strict.stderr.contains("error[Y101]") && strict.stderr.contains("\"a\""),
        "stderr: {}",
        strict.stderr
    );
    assert!(
        strict.stderr.contains("= help:"),
        "stderr: {}",
        strict.stderr
    );
}

#[test]
fn validate_key_collision_is_reported_by_default() {
    // The parser refuses stringified-key collisions outright, so the finding
    // does not need --strict.
    let out = run(&["validate", "-"], "1: a\n\"1\": b\n");
    assert_eq!(out.status, 1, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("error[Y102]"), "stderr: {}", out.stderr);
}

#[test]
fn validate_reports_every_file_in_one_run() {
    let first = temp_yaml("x: [1,\n");
    let second = temp_yaml("a: 1\na: 2\n");
    let out = run(
        &[
            "validate",
            "--strict",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ],
        "",
    );
    assert_eq!(out.status, 1, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("error[Y001]"), "stderr: {}", out.stderr);
    assert!(out.stderr.contains("error[Y101]"), "stderr: {}", out.stderr);
    let _ = std::fs::remove_file(&first);
    let _ = std::fs::remove_file(&second);
}

#[test]
fn validate_merge_conflict_marker_gets_help() {
    let out = run(&["validate", "-"], "a: 1\n<<<<<<< HEAD\nb: 2\n");
    assert_eq!(out.status, 1, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("merge-conflict"),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn missing_filter_is_a_usage_error() {
    // With subcommands in play the filter positional is enforced by the
    // binary; bare `yqr` must stay a usage error (exit 2), not a crash.
    let out = run(&[], "");
    assert_eq!(out.status, 2, "stderr: {}", out.stderr);
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
    // Deleting the anchor definition would leave `*base` dangling, so the edit
    // does not re-parse to the expected document; the write is refused (exit 5)
    // and the file on disk must be byte-identical.
    let original = "defaults: &base\n  timeout: 30\nservice:\n  <<: *base\n  name: web\n";
    let file = temp_yaml(original);
    let path = file.to_str().unwrap();
    let out = run(&["-i", "del(.defaults)", path], "");
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
fn delete_of_a_same_column_block_sequence_writes_back() {
    // The GitHub Actions / Ansible / Kubernetes list style writes a key's
    // block-sequence value at the key's own column; deleting the key removes the
    // whole sequence and leaves the sibling byte-identical.
    let original = "on:\n- push\n- pull_request\njobs: {}\n";
    let file = temp_yaml(original);
    let path = file.to_str().unwrap();
    let out = run(&["-i", "del(.on)", path], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(read_back(&file), "jobs: {}\n");
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

// -- Feature f007: the key selector ---------------------------------------

#[test]
fn key_read_prints_the_document_s_own_token() {
    // Not the filter's path segment: a key authored with quotes reads back
    // with them, because the read slices source bytes like every other read.
    let out = run(&["key(.[\"a b\"])"], "\"a b\": 1\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "\"a b\"\n");
}

#[test]
fn key_read_under_raw_output_unquotes() {
    let out = run(&["-r", "key(.[\"a b\"])"], "\"a b\": 1\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "a b\n");
}

#[test]
fn key_read_streams_one_result_per_selected_node() {
    let out = run(&["key(.items[])"], "items:\n  - a: 1\n  - b: 2\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    // Each item is a mapping, so each *item* has no key of its own: reads are
    // total and report null rather than failing the batch.
    assert_eq!(out.stdout, "null\nnull\n");
}

#[test]
fn key_read_of_a_sequence_item_is_null_not_an_error() {
    let out = run(&["key(.xs[0])"], "xs:\n  - one\n  - two\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "null\n");
}

#[test]
fn key_read_of_a_merge_produced_key_is_null() {
    // The key exists in the typed view but has no token in the file, which is
    // exactly the case answering from the path segment would get wrong.
    let input = "base: &b\n  x: 1\nuse:\n  <<: *b\n  y: 2\n";
    let out = run(&["key(.use.x)"], input);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "null\n");
}

#[test]
fn key_rename_in_place_rewrites_only_the_key_token() {
    let original = "# app\nmetadata:\n  # names it\n  name: web  # why\n  tier: edge\n";
    let file = temp_yaml(original);
    let path = file.to_str().unwrap();
    let out = run(&["-i", "key(.metadata.name) = \"title\"", path], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert!(out.stdout.is_empty(), "-i must not print to stdout");
    assert_eq!(
        read_back(&file),
        "# app\nmetadata:\n  # names it\n  title: web  # why\n  tier: edge\n"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn refused_rename_leaves_the_file_unchanged_under_in_place() {
    let original = "a: 1\nb: 2\n";
    let file = temp_yaml(original);
    let path = file.to_str().unwrap();
    let out = run(&["-i", "key(.a) = \"b\"", path], "");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert_eq!(
        read_back(&file),
        original,
        "refused edit must not touch file"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn del_of_a_key_is_a_parse_error() {
    let out = run(&["del(key(.a))"], "a: 1\n");
    assert_eq!(out.status, 3, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("cannot outlive its entry"),
        "{}",
        out.stderr
    );
}

#[test]
fn a_field_named_key_still_reads() {
    let out = run(&[".key"], "key: value\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "value\n");
}

#[test]
fn key_selector_is_refused_under_normalize() {
    let out = run(&["--normalize", "key(.a)"], "a: 1\n");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("--normalize"), "{}", out.stderr);
}

#[test]
fn sole_entry_delete_empties_the_collection_in_place() {
    // The class yqr used to refuse. `{}` is the only spelling an empty block
    // mapping has, and the entry's head comment goes with it.
    let file = temp_yaml("a:\n  # documents x\n  x: 1\nb: 2\n");
    let path = file.to_str().unwrap();
    let out = run(&["-i", "del(.a.x)", path], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(read_back(&file), "a:\n  {}\nb: 2\n");
    let _ = std::fs::remove_file(&file);
}

#[test]
fn flow_collection_item_delete_works() {
    let out = run(&["del(.ports[0])"], "ports: [80, 443]\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "ports: [443]\n");
}

// -- Feature f007: comment editing (a002 slice 2) -------------------------

#[test]
fn comment_reads_report_the_body_without_hash_or_its_leading_space() {
    let doc = "spec:\n  # documents replicas\n  replicas: 3  # tuned\n";
    let inline = run(&["-r", "line_comment(.spec.replicas)"], doc);
    assert_eq!(inline.status, 0, "stderr: {}", inline.stderr);
    assert_eq!(inline.stdout, "tuned\n");
    let head = run(&["-r", "head_comment(.spec.replicas)"], doc);
    assert_eq!(head.stdout, "documents replicas\n");
}

#[test]
fn comment_reads_are_total() {
    // No comment, a value on the next line, and a blank-detached block above:
    // all null, none an error, so a batch is never failed by a missing comment.
    for (filter, doc) in [
        ("line_comment(.a)", "a: 1\n"),
        ("head_comment(.a)", "a: 1\n"),
        ("line_comment(.a)", "a:\n  b: 1  # child\n"),
        ("head_comment(.a)", "# detached\n\na: 1\n"),
        ("line_comment(.nope)", "a: 1\n"),
    ] {
        let out = run(&[filter], doc);
        assert_eq!(out.status, 0, "{filter} on {doc:?}: {}", out.stderr);
        assert_eq!(out.stdout, "null\n", "{filter} on {doc:?}");
    }
}

#[test]
fn comment_selector_iterates_like_the_path_it_wraps() {
    let out = run(
        &["-r", "line_comment(.xs[])"],
        "xs:\n  - one  # first\n  - two\n",
    );
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "first\nnull\n");
}

#[test]
fn the_comment_round_trip_property_holds() {
    // a002 §4.3, as an executable check: setting a body and reading it back
    // yields the same body, for every shape that survives the shell.
    for body in [
        "plain",
        "with  inner  spaces",
        "  leading spaces",
        "#hash",
        "a: colon",
        "trailing  ",
    ] {
        let file = temp_yaml("a: 1\n");
        let path = file.to_str().unwrap();
        let set = run(&["-i", &format!("line_comment(.a) = \"{body}\""), path], "");
        assert_eq!(set.status, 0, "set {body:?}: {}", set.stderr);
        let got = run(&["-r", "line_comment(.a)", path], "");
        assert_eq!(got.stdout, format!("{body}\n"), "round trip of {body:?}");
        let _ = std::fs::remove_file(&file);
    }
}

#[test]
fn comment_edits_in_place_touch_only_the_comment() {
    let original =
        "# header\n\nspec:\n  # documents replicas\n  replicas: 3  # tuned\n  image: web\n";
    let file = temp_yaml(original);
    let path = file.to_str().unwrap();
    let out = run(
        &["-i", "line_comment(.spec.replicas) = \"now five\"", path],
        "",
    );
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(
        read_back(&file),
        "# header\n\nspec:\n  # documents replicas\n  replicas: 3  # now five\n  image: web\n"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_refused_comment_edit_leaves_the_file_unchanged() {
    let original = "# detached\n\na: 1\n";
    let file = temp_yaml(original);
    let path = file.to_str().unwrap();
    let out = run(&["-i", "del(head_comment(.a))", path], "");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert_eq!(read_back(&file), original);
    let _ = std::fs::remove_file(&file);
}

#[test]
fn foot_comment_refuses_with_its_own_reason() {
    let out = run(&["foot_comment(.a)"], "a: 1\n");
    assert_eq!(out.status, 3, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("head_comment"), "{}", out.stderr);
}

#[test]
fn comment_selector_words_still_read_fields() {
    let doc = "line_comment: one\nhead_comment: two\nfoot_comment: three\n";
    for (filter, want) in [
        (".line_comment", "one\n"),
        (".head_comment", "two\n"),
        (".foot_comment", "three\n"),
    ] {
        let out = run(&[filter], doc);
        assert_eq!(out.status, 0, "{filter}: {}", out.stderr);
        assert_eq!(out.stdout, want, "{filter}");
    }
}

// -- Feature f007: sequence reorder (a002 slice 3) -------------------------

#[test]
fn swap_reorders_a_block_sequence_on_stdout() {
    let out = run(&["swap(.xs; 0; 2)"], "xs:\n  - a\n  - b\n  - c\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "xs:\n  - c\n  - b\n  - a\n");
}

#[test]
fn move_shifts_the_items_between() {
    let out = run(&["move(.xs; 0; 2)"], "xs:\n  - a\n  - b\n  - c\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "xs:\n  - b\n  - c\n  - a\n");
}

#[test]
fn a_reorder_index_may_count_from_the_end() {
    let out = run(&["swap(.xs; 0; -1)"], "xs:\n  - a\n  - b\n  - c\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "xs:\n  - c\n  - b\n  - a\n");
}

#[test]
fn reorder_in_place_moves_each_item_with_its_comments() {
    // The whole point of the slice, end to end: a commented list is the normal
    // shape of the files yqr targets, and a reorder that left the comments
    // behind would re-document every step in the file.
    let original = "# workflow\nsteps:\n  # check out first\n  - uses: checkout  # pinned\n  - name: test\n    run: cargo test\n";
    let file = temp_yaml(original);
    let path = file.to_str().unwrap();
    let out = run(&["-i", "swap(.steps; 0; 1)", path], "");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert!(out.stdout.is_empty(), "-i must not print to stdout");
    assert_eq!(
        read_back(&file),
        "# workflow\nsteps:\n  - name: test\n    run: cargo test\n  # check out first\n  - uses: checkout  # pinned\n"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn a_refused_reorder_leaves_the_file_unchanged() {
    let original = "xs:\n  - a\n  - b\n";
    let file = temp_yaml(original);
    let path = file.to_str().unwrap();
    let out = run(&["-i", "swap(.xs; 0; 7)", path], "");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("-2..=1"), "{}", out.stderr);
    assert_eq!(
        read_back(&file),
        original,
        "refused edit must not touch file"
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn reordering_something_that_is_not_a_sequence_is_a_runtime_error() {
    let out = run(&["swap(.xs; 0; 1)"], "xs:\n  a: 1\n");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("addresses a sequence"),
        "{}",
        out.stderr
    );
}

#[test]
fn a_reorder_verb_needs_semicolons_and_says_so() {
    let out = run(&["swap(.xs, 0, 1)"], "xs:\n  - a\n  - b\n");
    assert_eq!(out.status, 3, "stderr: {}", out.stderr);
    assert!(out.stderr.contains("';'"), "{}", out.stderr);
}

#[test]
fn reorder_words_still_read_fields() {
    // `swap` and `move` are ordinary YAML key names; only the `(` makes them
    // verbs, so a document that uses them as keys still reads.
    let doc = "swap: one\nmove: two\n";
    for (filter, want) in [(".swap", "one\n"), (".move", "two\n")] {
        let out = run(&[filter], doc);
        assert_eq!(out.status, 0, "{filter}: {}", out.stderr);
        assert_eq!(out.stdout, want, "{filter}");
    }
}

#[test]
fn a_reorder_needs_a_file_for_in_place_like_every_other_edit() {
    let out = run(&["-i", "swap(.xs; 0; 1)"], "xs:\n  - a\n  - b\n");
    assert_eq!(out.status, 5, "stderr: {}", out.stderr);
}

// -- shapes unblocked by the noyalib 0.0.25 pin (`yqr-f019`) ----------------

// Bug b011: a flow collection wrapped over several lines, with the closing
// indicator at the parent key's column, is ordinary YAML that noyalib refused
// to parse. The refusal was on the read path, so no filter ran at all.
#[test]
fn a_wrapped_flow_collection_reads_byte_for_byte() {
    let doc = "ports: [\n  80,\n  443,\n]\nopts: {\n  retries: 3,\n}\n";
    let out = run(&["."], doc);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, doc, "identity must be byte-for-byte");

    let out = run(&[".ports[1]"], doc);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "443\n");
}

#[test]
fn a_wrapped_flow_collection_edits_only_at_the_site() {
    let doc = "ports: [\n  80,\n  443,\n]\n";
    let out = run(&[".ports[1] = 8443"], doc);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "ports: [\n  80,\n  8443,\n]\n");
}

// The narrow half of the b011 fix: only the closing *indicator* was exempted,
// because a line opening with `]` or `}` cannot begin block content. Flow
// *content* at or below the block's indent stays ambiguous with a sibling
// block entry, and the yaml-test-suite marks it an error (9C9N, VJP3).
#[test]
fn under_indented_flow_content_is_still_refused() {
    let out = run(&["."], "ports: [\n80,\n]\n");
    assert_eq!(out.status, 5, "stdout: {}", out.stdout);
    assert!(
        out.stderr.contains("flow content must be indented"),
        "unexpected stderr: {}",
        out.stderr
    );
}

// Bug b015: deleting a member of a wrapped flow collection left the removed
// member's indentation behind as a whitespace-only line. Upstream's, through
// the flow class yqr delegates (`yqr-f016` §5); fixed by yqr's noyalib#296 and
// released in noyalib 0.0.26. Unreachable before b011, because the file could
// not be parsed at all — which is why this test arrives two releases after the
// delete path it covers.
#[test]
fn deleting_from_a_wrapped_flow_collection_takes_the_whole_line() {
    for (filter, doc, want) in [
        (
            "del(.ports[0])",
            "ports: [\n  80,\n  443,\n]\n",
            "ports: [\n  443,\n]\n",
        ),
        // The last member leaves the line above alone, comma and all: a
        // trailing comma before `]` is valid, and reaching up a line to delete
        // a separator the removal does not own would be reformatting.
        (
            "del(.ports[1])",
            "ports: [\n  80,\n  443,\n]\n",
            "ports: [\n  80,\n]\n",
        ),
        (
            "del(.cfg.a)",
            "cfg: {\n  a: 1,\n  b: 2,\n}\n",
            "cfg: {\n  b: 2,\n}\n",
        ),
    ] {
        let out = run(&[filter], doc);
        assert_eq!(out.status, 0, "{filter}: stderr: {}", out.stderr);
        assert_eq!(out.stdout, want, "{filter}");
        assert!(
            !out.stdout
                .lines()
                .any(|l| !l.is_empty() && l != l.trim_end()),
            "{filter}: left trailing whitespace: {:?}",
            out.stdout
        );
    }
}

#[test]
fn a_flow_delete_leaves_a_line_it_does_not_own_standing() {
    // The rule is "the member is alone on its line", not "the collection is
    // wrapped". Each of these keeps its line because something else survives
    // on it, and each is the control for the test above.
    for (filter, doc, want) in [
        // single line: nothing to take
        ("del(.ports[0])", "ports: [80, 443]\n", "ports: [443]\n"),
        // the opening indicator holds the line
        (
            "del(.ports[0])",
            "ports: [80,\n  443,\n]\n",
            "ports: [\n  443,\n]\n",
        ),
        // the closing indicator holds it
        (
            "del(.ports[1])",
            "ports: [\n  80,\n  443]\n",
            "ports: [\n  80,\n  ]\n",
        ),
        // a comment holds it — what an orphaned comment means is a semantics
        // question, not one for a whitespace rule to settle
        (
            "del(.ports[0])",
            "ports: [\n  80, # http\n  443,\n]\n",
            "ports: [\n  # http\n  443,\n]\n",
        ),
    ] {
        let out = run(&[filter], doc);
        assert_eq!(out.status, 0, "{filter}: stderr: {}", out.stderr);
        assert_eq!(out.stdout, want, "{filter}");
    }
}

// Bug b012: the insert anchor used to be found by composing a candidate key
// back into a path string and re-parsing it, and a `.` in the key made that
// round trip lossy — so no key could be inserted beside the Kubernetes label
// block, and the refusal blamed a `<<` merge the document does not contain.
#[test]
fn a_key_inserts_into_a_mapping_whose_keys_hold_a_dot() {
    let doc = "labels:\n  app.kubernetes.io/name: web\n  app.kubernetes.io/component: frontend\n";
    let out = run(&[".labels.tier = \"web\""], doc);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(
        out.stdout,
        "labels:\n  app.kubernetes.io/name: web\n  app.kubernetes.io/component: frontend\n  tier: web\n"
    );
}

// Bug b013: an inserted scalar took the document's dominant quote style, and
// the vote counted only quoted scalars against each other — so one quoted
// scalar anywhere made every later insertion quoted. The vote is now scored at
// the edit site, so a line the edit does not touch cannot decide its spelling.
#[test]
fn an_inserted_scalar_is_spelled_like_its_neighbours() {
    let plain = "labels:\n  app: web\n";
    let with_a_quote_elsewhere = "quoted: \"30\"\nlabels:\n  app: web\n";

    let a = run(&[".labels.tier = \"web\""], plain);
    assert_eq!(a.status, 0, "stderr: {}", a.stderr);
    assert_eq!(a.stdout, "labels:\n  app: web\n  tier: web\n");

    let b = run(&[".labels.tier = \"web\""], with_a_quote_elsewhere);
    assert_eq!(b.status, 0, "stderr: {}", b.stderr);
    assert_eq!(
        b.stdout, "quoted: \"30\"\nlabels:\n  app: web\n  tier: web\n",
        "an untouched line four lines up must not decide the edit's spelling"
    );

    // A quoted *neighbour* still carries, which is the behaviour the vote is
    // for: matching the file's conventions at the place being edited.
    let c = run(&[".labels.tier = \"web\""], "labels:\n  app: \"web\"\n");
    assert_eq!(c.status, 0, "stderr: {}", c.stderr);
    assert_eq!(c.stdout, "labels:\n  app: \"web\"\n  tier: \"web\"\n");
}

// -- Feature f017: to_entries -----------------------------------------------

#[test]
fn to_entries_pairs_a_mapping_in_document_order() {
    // `zebra` first is the file's order, not sorted order.
    let out = run(
        [".m | to_entries"].as_slice(),
        "m:\n  zebra: 1\n  apple: 2\n",
    );
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(
        out.stdout,
        "- key: zebra\n  value: 1\n- key: apple\n  value: 2\n"
    );
}

#[test]
fn to_entries_makes_the_r003_task_one_invocation() {
    // The task that sent an agent to a Python script (`yqr-r003` §2): report
    // each named entry's name alongside a field of its value. `.services[]`
    // iterates the values, so the names were gone.
    let doc = "services:\n  alpha:\n    domain: alpha.example.com\n  beta:\n    domain: beta.example.com\n  gamma:\n    tier: core\n";

    let keys = run(&["-r", ".services | to_entries[] | .key"], doc);
    assert_eq!(keys.stdout, "alpha\nbeta\ngamma\n");

    // The missing-field entry still emits one line, so the halves of the pair
    // stay in step — the `yqr-r003` §5 property, now inside a single filter.
    let domains = run(&["-r", ".services | to_entries[] | .value.domain"], doc);
    assert_eq!(
        domains.stdout,
        "alpha.example.com\nbeta.example.com\nnull\n"
    );
}

// Bug b016: the emitter used to write `value: ` with a trailing space before a
// block collection reached through a sequence item, so every pair whose value
// was a mapping or a sequence carried one. Fixed upstream by yqr's
// noyalib#298, released in noyalib 0.0.27. This test was the pin that told
// this bump the bump had changed something (`yqr-f023`).
#[test]
fn to_entries_output_carries_no_trailing_whitespace() {
    let out = run(&[".m | to_entries"], "m:\n  a:\n    x: 1\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "- key: a\n  value:\n    x: 1\n");
    assert!(
        !out.stdout
            .lines()
            .any(|l| !l.is_empty() && l != l.trim_end()),
        "no line may carry trailing whitespace: {:?}",
        out.stdout
    );
}

// The other half of b016: a block scalar's empty line carried the block's
// indent. Reachable through `--normalize` on any document with one.
#[test]
fn an_empty_block_scalar_line_carries_no_indent() {
    let out = run(&["--normalize", "."], "k: |\n  a\n\n  b\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "k: |\n  a\n\n  b\n");
}

// The control b016 turned on: whitespace a string owns must survive, which is
// why the fix keys on what the string holds rather than on how the line looks.
#[test]
fn trailing_spaces_that_belong_to_a_string_survive() {
    let doc = "k: \"a  \\nb\\n\"\n";
    let out = run(&["--normalize", "-r", ".k"], doc);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(
        out.stdout, "a  \nb\n\n",
        "the string's own spaces must survive"
    );
}

#[test]
fn to_entries_reads_a_key_as_a_value_where_key_reads_it_as_a_token() {
    // The two ways to enumerate keys are not redundant, and this is exactly
    // where they part: `key(...)` emits the document's own key token, quotes
    // included, while `to_entries` yields the decoded string and lets the
    // renderer spell it. `-r` collapses the difference, because asking for raw
    // output is asking for the value rather than the spelling.
    let doc = "m:\n  \"quoted\": 1\n  plain: 2\n";

    let token = run(&["key(.m[])"], doc);
    assert_eq!(token.stdout, "\"quoted\"\nplain\n");

    let decoded = run(&[".m | to_entries[] | .key"], doc);
    assert_eq!(decoded.stdout, "quoted\nplain\n");

    for filter in ["key(.m[])", ".m | to_entries[] | .key"] {
        let raw = run(&["-r", filter], doc);
        assert_eq!(raw.stdout, "quoted\nplain\n", "-r {filter}");
    }
}

#[test]
fn to_entries_on_a_non_mapping_names_the_type_and_exits_five() {
    let out = run(&[".m | to_entries"], "m:\n  - 1\n");
    assert_eq!(out.status, 5, "stdout: {}", out.stdout);
    assert!(
        out.stderr.contains("to_entries takes an object") && out.stderr.contains("array"),
        "unexpected stderr: {}",
        out.stderr
    );
}

#[test]
fn a_write_to_to_entries_is_refused_at_parse_with_a_reason() {
    for (args, op) in [
        (vec![".m | to_entries = 1"], "'='"),
        (vec![".m | to_entries += 1"], "'+='"),
        (vec!["del(.m | to_entries)"], "'del'"),
        // The reorder verbs resolve their path through the write driver,
        // which leaves an *absent* path alone at exit 0. A builtin is not
        // absent but unwritable, so without a guard here the verb reported
        // success for a reorder that did nothing -- exit 0, document
        // unchanged, and with `-i` the file untouched and no complaint.
        (vec!["swap(.m | to_entries; 0; 1)"], "'swap'"),
        (vec!["move(to_entries; 0; 1)"], "'move'"),
    ] {
        let out = run(&args, "m:\n  a: 1\n");
        assert_eq!(out.status, 3, "{args:?} stdout: {}", out.stdout);
        assert!(
            out.stderr.contains("to_entries") && out.stderr.contains(op),
            "{args:?}: unexpected stderr: {}",
            out.stderr
        );
    }
}

#[test]
fn to_entries_is_still_an_ordinary_field_name() {
    // The word costs no reserved identifier: a path starts with '.', so an
    // identifier there can only be a field.
    let out = run(&["-r", ".to_entries"], "to_entries: hi\n");
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "hi\n");
}

#[test]
fn to_entries_works_on_the_normalize_pipeline_too() {
    // It computes a value, so there is nothing for the byte path to slice and
    // nothing for `--normalize` to lose.
    let out = run(
        &["--normalize", "-r", ".m | to_entries[] | .key"],
        "m:\n  a: 1\n  b: 2\n",
    );
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "a\nb\n");
}

#[test]
fn to_entries_with_parentheses_says_it_takes_none() {
    let out = run(&["to_entries(.m)"], "m:\n  a: 1\n");
    assert_eq!(out.status, 3);
    assert!(
        out.stderr.contains("takes no arguments"),
        "unexpected stderr: {}",
        out.stderr
    );
}
