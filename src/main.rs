//! `yqr` binary entry point.
//!
//! Wires together argument parsing, input reading, filter evaluation, and
//! output rendering, mapping any failure to a jq-style process exit code.

mod cli;

use std::io::{self, Read, Write};
use std::process::ExitCode;

use cli::Cli;
use yqr::ast::Program;
use yqr::fidelity::{self, BackendId};
use yqr::{YqrError, render};

fn main() -> ExitCode {
    let args = Cli::parse_args();
    match run(&args) {
        Ok(output) => {
            if let Err(e) = io::stdout().write_all(output.as_bytes()) {
                eprintln!("yqr: io error: {e}");
                return ExitCode::from(5);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("yqr: {e}");
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

fn run(args: &Cli) -> Result<String, YqrError> {
    // Feature f005: `--engine` selects the backend; `--preserve` decides whether
    // to preserve bytes. Resolve the backend name (defaulting to the always-
    // available noyalib) before consuming stdin/the file, so a typo in --engine
    // is diagnosed immediately instead of after reading input.
    let backend = match args.engine.as_deref() {
        Some(engine) => BackendId::parse(engine).ok_or_else(|| {
            YqrError::io(format!(
                "unknown engine {engine:?} (available: {})",
                BackendId::known_names()
            ))
        })?,
        None => BackendId::NoyalibCst,
    };

    // Feature f006: decide read vs write before consuming input, so a filter
    // error (or a misused `-i`) is diagnosed up front. A mutating filter always
    // goes through the fidelity write path, regardless of `--preserve`.
    match yqr::parser::parse_program(&args.filter)? {
        Program::Mutate(mutation) => {
            // Validate the `-i` target before consuming stdin or applying the
            // mutation, so a misused `-i` (stdin, no file) fails immediately
            // instead of blocking on input or doing throwaway work.
            let in_place_target = if args.in_place {
                Some(in_place_path(args.file.as_deref())?)
            } else {
                None
            };
            let input = read_input(args.file.as_deref())?;
            let output = fidelity::write::apply(backend, &mutation, &input)?;
            match in_place_target {
                Some(path) => {
                    write_in_place(path, &output)?;
                    Ok(String::new())
                }
                None => Ok(output),
            }
        }
        Program::Query(ast) => {
            if args.in_place {
                return Err(YqrError::io(
                    "--in-place requires a mutating filter (e.g. '.a = 5', '.xs += 1', 'del(.a)')"
                        .to_string(),
                ));
            }
            let input = read_input(args.file.as_deref())?;
            if args.preserve {
                return fidelity::run_ast(backend, &ast, &input, args.raw_output);
            }
            // Standard re-serializing pipeline. It is backend-independent today,
            // so a bare `--engine` without `--preserve` is inert beyond name
            // validation.
            let values = yqr::eval_ast_str(&ast, &input)?;
            render(&values, args.raw_output)
        }
    }
}

/// Resolve the file path to rewrite for `-i`, rejecting stdin.
///
/// In-place editing needs a concrete file to atomically replace; `-` and an
/// omitted path both mean stdin, which cannot be rewritten.
// Feature f006.
fn in_place_path(path: Option<&str>) -> Result<&str, YqrError> {
    match path {
        Some(p) if p != "-" => Ok(p),
        _ => Err(YqrError::io(
            "--in-place cannot be used with stdin input; provide a file path".to_string(),
        )),
    }
}

/// Atomically replace the file at `path` with `contents`.
///
/// Symlinks are resolved first (via [`std::fs::canonicalize`]) so the real file
/// is edited and the link is preserved — the rename would otherwise swap the
/// link entry for a fresh regular file. A sibling temp file in the target's
/// directory is written, `fsync`ed, then renamed over the target so the rename
/// stays on one filesystem (a cross-device rename is not atomic) and a crash
/// cannot leave a truncated file. The temp is created with owner-only
/// permissions and only widened to the original's mode after the bytes are
/// written, so a restricted file's contents are never briefly world-readable.
///
/// Not preserved across the replace (an inherent temp-file+rename tradeoff,
/// shared with `sed -i`): hardlinks (the other names keep the old content),
/// owner/group, SELinux/security context, ACLs, and extended attributes. On any
/// failure the temp file is cleaned up and the original is left untouched.
// Feature f006.
fn write_in_place(path: &str, contents: &str) -> Result<(), YqrError> {
    let target = std::fs::canonicalize(path)
        .map_err(|e| YqrError::io(format!("failed to resolve {path:?}: {e}")))?;
    // Sibling temp path, built without a lossy string conversion.
    let mut tmp = target.clone().into_os_string();
    tmp.push(format!(".yqr-tmp.{}", std::process::id()));
    let tmp = std::path::PathBuf::from(tmp);

    let mode = std::fs::metadata(&target).ok().map(|m| m.permissions());
    write_private_synced(&tmp, contents.as_bytes())?;

    // Widen the owner-only temp to the original's mode before the swap. If the
    // original's metadata was unreadable, leave the temp owner-only rather than
    // failing the edit.
    if let Some(perms) = mode
        && let Err(e) = std::fs::set_permissions(&tmp, perms)
    {
        let _ = std::fs::remove_file(&tmp);
        return Err(YqrError::io(format!(
            "failed to preserve permissions on {path:?}: {e}"
        )));
    }
    std::fs::rename(&tmp, &target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        YqrError::io(format!("failed to replace {path:?}: {e}"))
    })
}

/// Create `tmp` with owner-only permissions, write `bytes`, and `fsync` before
/// returning, cleaning up the temp on any I/O error.
// Feature f006.
fn write_private_synced(tmp: &std::path::Path, bytes: &[u8]) -> Result<(), YqrError> {
    use std::io::Write;

    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    // Scope the file handle so it is closed before any cleanup on error.
    let write = (|| -> std::io::Result<()> {
        let mut file = opts.open(tmp)?;
        file.write_all(bytes)?;
        file.sync_all()
    })();

    write.map_err(|e| {
        let _ = std::fs::remove_file(tmp);
        YqrError::io(format!("failed to write temporary file {tmp:?}: {e}"))
    })
}

/// Read the input YAML from a file path, or from stdin when the path is absent
/// or `-`.
fn read_input(path: Option<&str>) -> Result<String, YqrError> {
    match path {
        None | Some("-") => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| YqrError::io(format!("failed to read stdin: {e}")))?;
            Ok(buf)
        }
        Some(p) => std::fs::read_to_string(p)
            .map_err(|e| YqrError::io(format!("failed to read {p:?}: {e}"))),
    }
}
