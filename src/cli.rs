//! Command-line interface definition (`clap` derive).

use clap::Parser;

/// A jq-style command-line processor for YAML.
///
/// Reads a YAML document from a file or stdin, applies a jq-style filter, and
/// writes the resulting value(s) back as YAML.
#[derive(Debug, Parser)]
#[command(
    name = "yqr",
    version,
    about = "A jq-style command-line processor for YAML",
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
    }
}
