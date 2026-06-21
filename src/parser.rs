//! Recursive-descent parser: [`Token`]s → [`Ast`].
//!
//! Grammar implemented for milestone M0
//! (see `specs/features/0001-yaml-jq-clone.md`):
//!
//! ```text
//! pipeline := term ('|' term)*
//! term     := path ('?')*
//! path     := '.' component* | '.'            ; leading dot, then chained steps
//! component:= Ident                           ; .foo
//!           | '.' Ident                       ; chained .bar
//!           | '.'? bracket
//! bracket  := '[' ']'        -> iterate
//!           | '[' Int ']'    -> index
//!           | '[' Str ']'    -> field-by-string
//! ```

use crate::ast::Ast;
use crate::error::{Result, YqrError};
use crate::lexer::{Token, lex};

/// Compile a filter source string into an [`Ast`].
pub fn parse(src: &str) -> Result<Ast> {
    let tokens = lex(src)?;
    if tokens.is_empty() {
        // An empty program is treated as identity, which is friendlier than jq.
        return Ok(Ast::Identity);
    }
    let mut p = Parser { tokens, pos: 0 };
    let ast = p.parse_pipeline()?;
    if let Some(tok) = p.peek() {
        return Err(YqrError::parse(format!(
            "unexpected trailing token {tok:?}"
        )));
    }
    Ok(ast)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn advance(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, want: &Token) -> Result<()> {
        match self.advance() {
            Some(ref got) if got == want => Ok(()),
            Some(got) => Err(YqrError::parse(format!(
                "expected {want:?} but found {got:?}"
            ))),
            None => Err(YqrError::parse(format!(
                "expected {want:?} but reached end"
            ))),
        }
    }

    fn parse_pipeline(&mut self) -> Result<Ast> {
        let mut node = self.parse_term()?;
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            let rhs = self.parse_term()?;
            node = Ast::pipe(node, rhs);
        }
        Ok(node)
    }

    fn parse_term(&mut self) -> Result<Ast> {
        let mut node = self.parse_path()?;
        while matches!(self.peek(), Some(Token::Question)) {
            self.advance();
            node = Ast::optional(node);
        }
        Ok(node)
    }

    fn parse_path(&mut self) -> Result<Ast> {
        self.expect(&Token::Dot)?;
        let mut steps: Vec<Ast> = Vec::new();

        // Optional first component immediately after the leading dot.
        match self.peek() {
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.advance();
                steps.push(Ast::Field(name));
            }
            Some(Token::LBracket) => steps.push(self.parse_bracket()?),
            _ => {}
        }

        // Chained components.
        loop {
            match self.peek() {
                Some(Token::LBracket) => steps.push(self.parse_bracket()?),
                Some(Token::Dot) => match self.peek_at(1) {
                    Some(Token::Ident(name)) => {
                        let name = name.clone();
                        self.advance(); // dot
                        self.advance(); // ident
                        steps.push(Ast::Field(name));
                    }
                    Some(Token::LBracket) => {
                        self.advance(); // dot
                        steps.push(self.parse_bracket()?);
                    }
                    other => {
                        return Err(YqrError::parse(format!(
                            "expected field name or '[' after '.', found {other:?}"
                        )));
                    }
                },
                _ => break,
            }
        }

        Ok(fold_steps(steps))
    }

    fn parse_bracket(&mut self) -> Result<Ast> {
        self.expect(&Token::LBracket)?;
        let step = match self.advance() {
            Some(Token::RBracket) => return Ok(Ast::Iterate),
            Some(Token::Int(n)) => Ast::Index(n),
            Some(Token::Str(s)) => Ast::Field(s),
            other => {
                return Err(YqrError::parse(format!(
                    "expected ']', integer, or string inside '[]', found {other:?}"
                )));
            }
        };
        self.expect(&Token::RBracket)?;
        Ok(step)
    }
}

/// Fold a left-to-right list of steps into a pipe chain (or identity if empty).
fn fold_steps(steps: Vec<Ast>) -> Ast {
    let mut iter = steps.into_iter();
    match iter.next() {
        None => Ast::Identity,
        Some(first) => iter.fold(first, Ast::pipe),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_identity() {
        assert_eq!(parse(".").unwrap(), Ast::Identity);
        assert_eq!(parse("").unwrap(), Ast::Identity);
    }

    #[test]
    fn parses_single_field() {
        assert_eq!(parse(".foo").unwrap(), Ast::Field("foo".into()));
    }

    #[test]
    fn parses_chained_fields() {
        assert_eq!(
            parse(".a.b").unwrap(),
            Ast::pipe(Ast::Field("a".into()), Ast::Field("b".into()))
        );
    }

    #[test]
    fn parses_index_and_iterate() {
        assert_eq!(parse(".[0]").unwrap(), Ast::Index(0));
        assert_eq!(parse(".[-1]").unwrap(), Ast::Index(-1));
        assert_eq!(parse(".[]").unwrap(), Ast::Iterate);
    }

    #[test]
    fn parses_field_then_index() {
        assert_eq!(
            parse(".items[0]").unwrap(),
            Ast::pipe(Ast::Field("items".into()), Ast::Index(0))
        );
    }

    #[test]
    fn parses_bracket_string_field() {
        assert_eq!(parse(r#".["a b"]"#).unwrap(), Ast::Field("a b".into()));
    }

    #[test]
    fn parses_pipe() {
        assert_eq!(
            parse(".a | .b").unwrap(),
            Ast::pipe(Ast::Field("a".into()), Ast::Field("b".into()))
        );
    }

    #[test]
    fn parses_optional() {
        assert_eq!(parse(".a?").unwrap(), Ast::optional(Ast::Field("a".into())));
    }

    #[test]
    fn parses_iterate_then_field() {
        assert_eq!(
            parse(".items[].name").unwrap(),
            Ast::pipe(
                Ast::pipe(Ast::Field("items".into()), Ast::Iterate),
                Ast::Field("name".into())
            )
        );
    }

    #[test]
    fn rejects_non_dot_start() {
        assert!(matches!(parse("foo"), Err(YqrError::Parse(_))));
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(matches!(parse(".a]"), Err(YqrError::Parse(_))));
    }
}
