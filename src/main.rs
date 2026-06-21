//! `yqr` binary entry point.
//!
//! Wires together argument parsing, input reading, filter evaluation, and
//! output rendering, mapping any failure to a jq-style process exit code.

mod cli;

use std::io::{self, Read, Write};
use std::process::ExitCode;

use cli::Cli;
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
    let input = read_input(args.file.as_deref())?;
    let values = eval_str(&args.filter, &input)?;
    render(&values, args.raw_output)
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
