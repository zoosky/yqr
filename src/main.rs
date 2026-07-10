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
use yqr::{YqrError, eval_str, render};

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
    let program = yqr::parser::parse_program(&args.filter)?;
    if let Program::Mutate(mutation) = program {
        let input = read_input(args.file.as_deref())?;
        let output = fidelity::write::apply(backend, &mutation, &input)?;
        if args.in_place {
            let path = in_place_path(args.file.as_deref())?;
            write_in_place(path, &output)?;
            return Ok(String::new());
        }
        return Ok(output);
    }

    // Read-only query path.
    if args.in_place {
        return Err(YqrError::io(
            "--in-place requires a mutating filter (e.g. '.a = 5', '.xs += 1', 'del(.a)')"
                .to_string(),
        ));
    }
    let input = read_input(args.file.as_deref())?;
    if args.preserve {
        return fidelity::run(backend, &args.filter, &input, args.raw_output);
    }
    // Standard re-serializing pipeline. It is backend-independent today, so a
    // bare `--engine` without `--preserve` is inert beyond name validation.
    let values = eval_str(&args.filter, &input)?;
    render(&values, args.raw_output)
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

/// Atomically replace `path` with `contents`: write a sibling temp file, then
/// rename it over the original.
///
/// The temp file lives in the same directory as the target so the rename stays
/// on one filesystem (a cross-device rename is not atomic). The original file's
/// permissions are carried onto the replacement — a fresh temp file is created
/// with default (umask) permissions, which would otherwise silently relax a
/// restrictive mode (e.g. a `0600` secret becoming `0644`). On failure the temp
/// file is cleaned up and the original is left untouched.
// Feature f006.
fn write_in_place(path: &str, contents: &str) -> Result<(), YqrError> {
    let tmp = format!("{path}.yqr-tmp.{}", std::process::id());
    std::fs::write(&tmp, contents.as_bytes())
        .map_err(|e| YqrError::io(format!("failed to write temporary file {tmp:?}: {e}")))?;
    // Preserve the original mode. If the original's metadata is unreadable
    // (it existed a moment ago when we read it), fall back to default perms
    // rather than failing the whole edit.
    if let Ok(meta) = std::fs::metadata(path)
        && let Err(e) = std::fs::set_permissions(&tmp, meta.permissions())
    {
        let _ = std::fs::remove_file(&tmp);
        return Err(YqrError::io(format!(
            "failed to preserve permissions on {path:?}: {e}"
        )));
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        YqrError::io(format!("failed to replace {path:?}: {e}"))
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
