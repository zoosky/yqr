//! Error type and exit-code mapping for `yqr`.
//!
//! Exit codes loosely follow jq's conventions so that scripts wrapping `yqr`
//! behave predictably:
//!
//! | code | meaning                                            |
//! |------|----------------------------------------------------|
//! | 0    | success                                             |
//! | 2    | usage error (handled by `clap`)                    |
//! | 3    | filter failed to compile (lex/parse error)         |
//! | 5    | runtime error while evaluating the filter or YAML  |

use std::fmt;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, YqrError>;

/// All the ways a `yqr` invocation can fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YqrError {
    /// The filter could not be tokenized.
    Lex(String),
    /// The filter could not be parsed into an AST.
    Parse(String),
    /// Evaluation against the input document failed.
    Eval(String),
    /// The input could not be read or parsed as YAML, or output could not be
    /// emitted.
    Io(String),
}

impl YqrError {
    /// Process exit code associated with this error category.
    pub fn exit_code(&self) -> i32 {
        match self {
            YqrError::Lex(_) | YqrError::Parse(_) => 3,
            YqrError::Eval(_) | YqrError::Io(_) => 5,
        }
    }

    /// Construct a runtime evaluation error from anything displayable.
    pub fn eval(msg: impl Into<String>) -> Self {
        YqrError::Eval(msg.into())
    }

    /// Construct a parse error from anything displayable.
    pub fn parse(msg: impl Into<String>) -> Self {
        YqrError::Parse(msg.into())
    }

    /// Construct a lexer error from anything displayable.
    pub fn lex(msg: impl Into<String>) -> Self {
        YqrError::Lex(msg.into())
    }

    /// Construct an I/O error from anything displayable.
    pub fn io(msg: impl Into<String>) -> Self {
        YqrError::Io(msg.into())
    }
}

impl fmt::Display for YqrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YqrError::Lex(m) => write!(f, "lex error: {m}"),
            YqrError::Parse(m) => write!(f, "parse error: {m}"),
            YqrError::Eval(m) => write!(f, "runtime error: {m}"),
            YqrError::Io(m) => write!(f, "io error: {m}"),
        }
    }
}

impl std::error::Error for YqrError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_category() {
        assert_eq!(YqrError::lex("x").exit_code(), 3);
        assert_eq!(YqrError::parse("x").exit_code(), 3);
        assert_eq!(YqrError::eval("x").exit_code(), 5);
        assert_eq!(YqrError::io("x").exit_code(), 5);
    }

    #[test]
    fn display_is_prefixed() {
        assert_eq!(YqrError::parse("boom").to_string(), "parse error: boom");
    }
}
