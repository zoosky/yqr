//! Tokenizer for the `yqr` filter language.
//!
//! Turns a filter string such as `.items[0].name?` into a flat [`Token`]
//! stream that the [`crate::parser`] consumes. The lexer is intentionally tiny;
//! it only recognizes the tokens needed by the M0 grammar.

use crate::error::{Result, YqrError};

/// A lexical token of the filter language.
///
/// `PartialEq` but not `Eq`: the [`Token::Float`] payload is an `f64`, which has
/// no total equality. Tokens are only ever compared for structural equality in
/// the parser (`expect`) and tests, where partial equality is sufficient.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `.`
    Dot,
    /// `|`
    Pipe,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `?`
    Question,
    // Feature f006: mutation operators and grouping for the write tier.
    /// `=` — assignment.
    Eq,
    /// `+=` — append (block-sequence push).
    PlusEq,
    /// `|=` — computed update (reserved; not yet supported).
    PipeEq,
    /// `(` — opens a `del(...)` form.
    LParen,
    /// `)` — closes a `del(...)` form.
    RParen,
    /// A bare identifier, e.g. `foo` in `.foo`.
    Ident(String),
    /// An integer literal (used for indexing and as an assignment RHS), e.g.
    /// `-1` in `.[-1]`.
    Int(i64),
    /// A floating-point literal, only valid as an assignment RHS, e.g. `1.5`
    /// in `.x = 1.5`.
    Float(f64),
    /// A double-quoted string literal, e.g. `"key"` in `.["key"]`.
    Str(String),
}

/// Tokenize `src` into a vector of [`Token`]s.
pub fn lex(src: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            '.' => {
                tokens.push(Token::Dot);
                i += 1;
            }
            // Feature f006: `|` and `|=` share a prefix; peek to disambiguate.
            '|' => {
                if chars.get(i + 1) == Some(&'=') {
                    tokens.push(Token::PipeEq);
                    i += 2;
                } else {
                    tokens.push(Token::Pipe);
                    i += 1;
                }
            }
            '[' => {
                tokens.push(Token::LBracket);
                i += 1;
            }
            ']' => {
                tokens.push(Token::RBracket);
                i += 1;
            }
            '?' => {
                tokens.push(Token::Question);
                i += 1;
            }
            // Feature f006: mutation operators and `del(...)` grouping.
            '=' => {
                tokens.push(Token::Eq);
                i += 1;
            }
            '+' => {
                if chars.get(i + 1) == Some(&'=') {
                    tokens.push(Token::PlusEq);
                    i += 2;
                } else {
                    return Err(YqrError::lex(format!(
                        "unexpected character '+' at position {i} (did you mean '+=' ?)"
                    )));
                }
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '"' => {
                let (s, next) = lex_string(&chars, i)?;
                tokens.push(Token::Str(s));
                i = next;
            }
            c if c == '-' || c.is_ascii_digit() => {
                let (tok, next) = lex_number(&chars, i)?;
                tokens.push(tok);
                i = next;
            }
            c if is_ident_start(c) => {
                let (s, next) = lex_ident(&chars, i);
                tokens.push(Token::Ident(s));
                i = next;
            }
            other => {
                return Err(YqrError::lex(format!(
                    "unexpected character {other:?} at position {i}"
                )));
            }
        }
    }

    Ok(tokens)
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn lex_ident(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start;
    while i < chars.len() && is_ident_continue(chars[i]) {
        i += 1;
    }
    (chars[start..i].iter().collect(), i)
}

/// Lex a numeric literal into a [`Token::Int`] or [`Token::Float`].
///
/// An integer is an optional `-` followed by digits (the indexing form the M0
/// grammar has always used). A fractional part (`.` then digits) or an exponent
/// (`e`/`E`) promotes the literal to a float, which is only meaningful as an
/// assignment RHS (`.x = 1.5`). A trailing `.` not followed by a digit is left
/// for the [`Token::Dot`] rule, so `5.` lexes as `Int(5)` then `Dot`.
// Feature f006: float RHS support.
fn lex_number(chars: &[char], start: usize) -> Result<(Token, usize)> {
    let mut i = start;
    if chars[i] == '-' {
        i += 1;
    }
    let digits_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return Err(YqrError::lex(format!(
            "expected digits after '-' at position {start}"
        )));
    }

    let mut is_float = false;
    // Fractional part: only consume the '.' when a digit follows, so a path dot
    // after an integer is not swallowed.
    if chars.get(i) == Some(&'.') && chars.get(i + 1).is_some_and(char::is_ascii_digit) {
        is_float = true;
        i += 1; // '.'
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }
    // Exponent: e / E, optional sign, then digits. Only consume when the shape
    // is complete, otherwise leave the 'e' for the ident rule.
    if matches!(chars.get(i), Some('e' | 'E')) {
        let mut j = i + 1;
        if matches!(chars.get(j), Some('+' | '-')) {
            j += 1;
        }
        if chars.get(j).is_some_and(char::is_ascii_digit) {
            is_float = true;
            i = j;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
    }

    let text: String = chars[start..i].iter().collect();
    if is_float {
        let f = text
            .parse::<f64>()
            .map_err(|e| YqrError::lex(format!("invalid number {text:?}: {e}")))?;
        // `parse::<f64>` maps an out-of-range magnitude to `±inf` (Ok, not Err).
        // Accepting it would silently write the bare token `inf`, which any YAML
        // reader reloads as the string "inf" — a silent type change. Refuse it.
        if !f.is_finite() {
            return Err(YqrError::lex(format!(
                "number {text:?} is out of range for a 64-bit float"
            )));
        }
        Ok((Token::Float(f), i))
    } else {
        let n = text
            .parse::<i64>()
            .map_err(|e| YqrError::lex(format!("invalid integer {text:?}: {e}")))?;
        Ok((Token::Int(n), i))
    }
}

fn lex_string(chars: &[char], start: usize) -> Result<(String, usize)> {
    // chars[start] == '"'
    let mut i = start + 1;
    let mut out = String::new();
    while i < chars.len() {
        match chars[i] {
            '"' => return Ok((out, i + 1)),
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    break;
                }
                let esc = match chars[i] {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '"' => '"',
                    '/' => '/',
                    other => {
                        return Err(YqrError::lex(format!("invalid escape sequence \\{other}")));
                    }
                };
                out.push(esc);
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    Err(YqrError::lex("unterminated string literal".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_identity() {
        assert_eq!(lex(".").unwrap(), vec![Token::Dot]);
    }

    #[test]
    fn lexes_path_with_index_and_optional() {
        let toks = lex(".items[0].name?").unwrap();
        assert_eq!(
            toks,
            vec![
                Token::Dot,
                Token::Ident("items".into()),
                Token::LBracket,
                Token::Int(0),
                Token::RBracket,
                Token::Dot,
                Token::Ident("name".into()),
                Token::Question,
            ]
        );
    }

    #[test]
    fn lexes_pipe_and_negative_index() {
        let toks = lex(". | .[-1]").unwrap();
        assert_eq!(
            toks,
            vec![
                Token::Dot,
                Token::Pipe,
                Token::Dot,
                Token::LBracket,
                Token::Int(-1),
                Token::RBracket,
            ]
        );
    }

    #[test]
    fn lexes_quoted_field() {
        let toks = lex(r#".["a b"]"#).unwrap();
        assert_eq!(
            toks,
            vec![
                Token::Dot,
                Token::LBracket,
                Token::Str("a b".into()),
                Token::RBracket,
            ]
        );
    }

    #[test]
    fn string_escapes() {
        let toks = lex(r#"."x\ty""#);
        // not a valid filter shape, but the string token should lex fine
        assert!(toks.is_ok());
    }

    #[test]
    fn unterminated_string_errors() {
        assert!(matches!(lex(r#".["bad]"#), Err(YqrError::Lex(_))));
    }

    #[test]
    fn unexpected_char_errors() {
        assert!(matches!(lex(".@"), Err(YqrError::Lex(_))));
    }

    // -- Feature f006: mutation tokens -----------------------------------------

    #[test]
    fn lexes_assignment_operators() {
        assert_eq!(
            lex(".a = 5").unwrap(),
            vec![
                Token::Dot,
                Token::Ident("a".into()),
                Token::Eq,
                Token::Int(5)
            ]
        );
        assert_eq!(
            lex(".a += 5").unwrap(),
            vec![
                Token::Dot,
                Token::Ident("a".into()),
                Token::PlusEq,
                Token::Int(5)
            ]
        );
    }

    #[test]
    fn lexes_pipe_equals_distinctly_from_pipe() {
        assert_eq!(lex("|").unwrap(), vec![Token::Pipe]);
        assert_eq!(lex("|=").unwrap(), vec![Token::PipeEq]);
    }

    #[test]
    fn lexes_del_grouping() {
        assert_eq!(
            lex("del(.a)").unwrap(),
            vec![
                Token::Ident("del".into()),
                Token::LParen,
                Token::Dot,
                Token::Ident("a".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn lexes_float_literal() {
        assert_eq!(lex("1.5").unwrap(), vec![Token::Float(1.5)]);
        assert_eq!(lex("-2.25").unwrap(), vec![Token::Float(-2.25)]);
        assert_eq!(lex("1e3").unwrap(), vec![Token::Float(1000.0)]);
    }

    #[test]
    fn trailing_dot_after_int_is_a_separate_dot() {
        // `5.` is not a float (no fractional digits): Int(5) then Dot.
        assert_eq!(lex("5.").unwrap(), vec![Token::Int(5), Token::Dot]);
    }

    #[test]
    fn bare_plus_is_an_error() {
        assert!(matches!(lex(".a + 5"), Err(YqrError::Lex(_))));
    }

    #[test]
    fn overflowing_float_is_an_error() {
        // `1e999` parses to `f64::INFINITY` (Ok, not Err); the lexer must reject
        // it rather than silently emit the bare token `inf`.
        assert!(matches!(lex("1e999"), Err(YqrError::Lex(_))));
        assert!(matches!(lex("-1e999"), Err(YqrError::Lex(_))));
    }
}
