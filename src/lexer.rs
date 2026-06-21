//! Tokenizer for the `yqr` filter language.
//!
//! Turns a filter string such as `.items[0].name?` into a flat [`Token`]
//! stream that the [`crate::parser`] consumes. The lexer is intentionally tiny;
//! it only recognizes the tokens needed by the M0 grammar.

use crate::error::{Result, YqrError};

/// A lexical token of the filter language.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// A bare identifier, e.g. `foo` in `.foo`.
    Ident(String),
    /// An integer literal (used for indexing), e.g. `-1` in `.[-1]`.
    Int(i64),
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
            '|' => {
                tokens.push(Token::Pipe);
                i += 1;
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
            '"' => {
                let (s, next) = lex_string(&chars, i)?;
                tokens.push(Token::Str(s));
                i = next;
            }
            c if c == '-' || c.is_ascii_digit() => {
                let (n, next) = lex_int(&chars, i)?;
                tokens.push(Token::Int(n));
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

fn lex_int(chars: &[char], start: usize) -> Result<(i64, usize)> {
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
    let text: String = chars[start..i].iter().collect();
    let n = text
        .parse::<i64>()
        .map_err(|e| YqrError::lex(format!("invalid integer {text:?}: {e}")))?;
    Ok((n, i))
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
}
