//! Command-line interface definition (`clap` derive).

use clap::Parser;

/// Short version (`-V`): just the crate version, e.g. `0.1.0`.
const SHORT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Long version (`--version`): the crate version plus the build provenance
/// captured by `build.rs` — git commit, build timestamp (UTC), and target
/// triple. Rendered by clap as `yqr <LONG_VERSION>`.
const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("GIT_HASH"),
    ", built ",
    env!("BUILD_TIME"),
    ")\ntarget: ",
    env!("BUILD_TARGET"),
);

/// A fidelity-first, jq-style CLI for YAML.
///
/// Reads a YAML document from a file or stdin, applies a jq-style filter, and
/// writes the result back as YAML -- preserving the input's bytes by default,
/// and applying surgical edits (`=`, `+=`, `del`, `-i`) that touch only the
/// targeted bytes.
#[derive(Debug, Parser)]
#[command(
    name = "yqr",
    version = SHORT_VERSION,
    long_version = LONG_VERSION,
    about = "A fidelity-first, jq-style CLI for YAML (query and surgically edit, byte-for-byte)",
    long_about = None,
)]
pub struct Cli {
    /// The filter to apply, e.g. '.foo.bar', '.items[]', '.[-1]'.
    pub filter: String,

    /// Input YAML file. Reads from stdin when omitted or set to '-'.
    pub file: Option<String>,

    /// Output raw strings instead of YAML-quoted ones.
    #[arg(short = 'r', long = "raw-output")]
    pub raw_output: bool,

    // Feature f006: in-place edits for the write tier.
    /// Edit the input file in place (write the mutated document back).
    ///
    /// Only valid with a mutating filter (`.a = 5`, `.xs += 1`, `del(.a)`).
    /// The file is rewritten atomically (temp file + rename); using it with a
    /// read-only filter, or with stdin input, is an error. Without it, a
    /// mutated document is printed to stdout (byte-exact except the edit).
    #[arg(short = 'i', long = "in-place")]
    pub in_place: bool,

    // Feature f009: byte fidelity is the default read behaviour; `--normalize`
    // opts back into the classic re-serializing pipeline.
    /// Normalize output instead of preserving the input's bytes.
    ///
    /// By default yqr preserves byte-for-byte formatting on reads: untouched
    /// nodes are emitted as their original source bytes, so comments, quoting,
    /// indentation, scalar spellings, and line endings survive, and the
    /// identity filter reproduces the input exactly. Pass `--normalize` to run
    /// the classic re-serializing pipeline instead, which canonicalizes scalars
    /// and drops comments and other formatting.
    #[arg(short = 'N', long = "normalize")]
    pub normalize: bool,
}

impl Cli {
    /// Parse arguments from the process environment.
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        // Catches conflicting args / bad derive setups at test time.
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_filter_and_flags() {
        let cli = Cli::try_parse_from(["yqr", "-r", ".a.b", "in.yaml"]).unwrap();
        assert_eq!(cli.filter, ".a.b");
        assert_eq!(cli.file.as_deref(), Some("in.yaml"));
        assert!(cli.raw_output);
    }

    #[test]
    fn file_is_optional() {
        let cli = Cli::try_parse_from(["yqr", "."]).unwrap();
        assert_eq!(cli.filter, ".");
        assert_eq!(cli.file, None);
        assert!(!cli.raw_output);
        assert!(!cli.normalize);
        assert!(!cli.in_place);
    }

    #[test]
    fn parses_in_place_flag() {
        let long = Cli::try_parse_from(["yqr", "--in-place", ".a = 5", "in.yaml"]).unwrap();
        assert!(long.in_place);
        let short = Cli::try_parse_from(["yqr", "-i", ".a = 5", "in.yaml"]).unwrap();
        assert!(short.in_place);
    }

    #[test]
    fn parses_normalize_flag() {
        let long = Cli::try_parse_from(["yqr", "--normalize", "."]).unwrap();
        assert!(long.normalize);

        let short = Cli::try_parse_from(["yqr", "-N", "."]).unwrap();
        assert!(short.normalize);
    }

    #[test]
    fn preserve_flag_is_removed() {
        // `--preserve`/`-p` were removed when fidelity became the default read
        // behaviour; clap must reject them rather than silently accept.
        assert!(Cli::try_parse_from(["yqr", "--preserve", "."]).is_err());
        assert!(Cli::try_parse_from(["yqr", "-p", "."]).is_err());
    }

    #[test]
    fn engine_flag_is_removed() {
        // Feature f011: `--engine` was removed when yqr settled on noyalib as
        // its only engine; clap must reject it rather than silently accept.
        assert!(Cli::try_parse_from(["yqr", "--engine", "noyalib", "."]).is_err());
    }
}
