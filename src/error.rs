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
use std::ops::Range;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, YqrError>;

/// A filter compilation failure: the human-readable message and, when the
/// failing site is known, the byte range of the offending token in the
/// filter source string.
///
/// `span` lets an editor underline the token a diagnostic is about without
/// re-scanning the filter; it is `None` for refusals that are about the
/// filter's meaning rather than a single token (for example, a mutation
/// handed to a read-only pipeline).
///
/// With the `serde` cargo feature, this type serializes with its `message`
/// and `span` fields (`span` as `{ "start": .., "end": .. }` byte offsets,
/// or `null`). That shape is an interface for editor hosts: fields may be
/// added, but removing or renaming one is a breaking change. The struct is
/// `#[non_exhaustive]` so a field addition is not a breaking Rust-API
/// change either: hosts read these errors through [`YqrError`]'s
/// constructors, they do not build them.
// Feature f048.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct FilterError {
    /// One-line description of the problem, exactly as the CLI prints it
    /// (after its `lex error: ` / `parse error: ` prefix).
    pub message: String,
    /// Byte range of the offending token in the filter source, when the
    /// failing site is a single locatable token. An empty range at the end
    /// of the source means the filter ended too early.
    pub span: Option<Range<usize>>,
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// All the ways a `yqr` invocation can fail.
///
/// With the `serde` cargo feature, the error serializes as a single-key map
/// naming the category (`{"Parse": {...}}`), so a host can route on the
/// category without string-matching the rendered message. These errors are
/// outputs: they serialize but never deserialize.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum YqrError {
    /// The filter could not be tokenized.
    Lex(FilterError),
    /// The filter could not be parsed into an AST.
    Parse(FilterError),
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

    /// Construct a parse error from anything displayable, with no span.
    pub fn parse(msg: impl Into<String>) -> Self {
        YqrError::Parse(FilterError {
            message: msg.into(),
            span: None,
        })
    }

    /// Construct a parse error anchored at `span`, a byte range into the
    /// filter source string.
    // Feature f048.
    pub fn parse_at(msg: impl Into<String>, span: Range<usize>) -> Self {
        YqrError::Parse(FilterError {
            message: msg.into(),
            span: Some(span),
        })
    }

    /// Construct a lexer error from anything displayable, with no span.
    pub fn lex(msg: impl Into<String>) -> Self {
        YqrError::Lex(FilterError {
            message: msg.into(),
            span: None,
        })
    }

    /// Construct a lexer error anchored at `span`, a byte range into the
    /// filter source string.
    // Feature f048.
    pub fn lex_at(msg: impl Into<String>, span: Range<usize>) -> Self {
        YqrError::Lex(FilterError {
            message: msg.into(),
            span: Some(span),
        })
    }

    /// Construct an I/O error from anything displayable.
    pub fn io(msg: impl Into<String>) -> Self {
        YqrError::Io(msg.into())
    }

    /// The byte range of the offending token in the filter source, when this
    /// is a filter compilation error that knows it.
    // Feature f048.
    #[must_use]
    pub fn span(&self) -> Option<Range<usize>> {
        match self {
            YqrError::Lex(e) | YqrError::Parse(e) => e.span.clone(),
            YqrError::Eval(_) | YqrError::Io(_) => None,
        }
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

    #[test]
    fn span_is_carried_and_absent_by_default() {
        assert_eq!(YqrError::parse("boom").span(), None);
        assert_eq!(YqrError::parse_at("boom", 2..5).span(), Some(2..5));
        assert_eq!(YqrError::lex_at("boom", 0..1).span(), Some(0..1));
        assert_eq!(YqrError::eval("boom").span(), None);
    }

    /// Pins the serialized shape: editor hosts deserialize this, so a field
    /// rename or removal here is a breaking change.
    #[cfg(feature = "serde")]
    #[test]
    fn serializes_category_message_and_span() {
        let err = YqrError::parse_at("unexpected trailing token", 3..7);
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(
            json,
            r#"{"Parse":{"message":"unexpected trailing token","span":{"start":3,"end":7}}}"#
        );
    }
}
