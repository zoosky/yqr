//! Recursive-descent parser: [`Token`]s → [`Program`].
//!
//! Grammar implemented for milestone M0 plus the write tier's top-level forms:
//!
//! ```text
//! program  := 'del' '(' target ')'            ; delete mutation
//!           | target '=' rhs                   ; assignment / rename
//!           | pipeline '+=' rhs                ; append (value-only)
//!           | target                           ; read-only query
//! target   := selector
//!           | pipeline                         ; a value node
//! selector := 'key' '(' pipeline ')'          ; the entry's key token
//! rhs      := number | Str | 'true' | 'false' | 'null'   ; scalar literal
//!           | pipeline                         ; a '.'-rooted path
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

use crate::Value;
use crate::ast::{Ast, FOOT_COMMENT_REFUSAL, Mutation, Program, Rhs, Target};
use crate::error::{Result, YqrError};
use crate::lexer::{Token, lex};

/// Compile a filter source string into an [`Ast`], rejecting mutations.
///
/// This is the read-only entry point used by the classic and fidelity read
/// pipelines. A filter that performs a mutation (`=`, `+=`, `del(...)`) is a
/// parse error here — mutations are driven through the write path via
/// [`parse_program`].
pub fn parse(src: &str) -> Result<Ast> {
    match parse_program(src)? {
        Program::Query(Target::Value(ast)) => Ok(ast),
        Program::Query(target) => Err(YqrError::parse(format!(
            "'{}(...)' reads from the document's own bytes, so it is not available \
             in the re-serializing pipeline; drop --normalize",
            target
                .selector_name()
                .expect("a non-Value target has a selector name")
        ))),
        Program::Mutate(_) => Err(YqrError::parse(
            "this filter performs a mutation; run it without a read-only pipeline \
             (mutations are handled by the write path)"
                .to_string(),
        )),
    }
}

/// Compile a filter source string into a [`Program`] — a read-only query or a
/// single mutation.
// Feature f006: mutation-aware top-level parse.
pub fn parse_program(src: &str) -> Result<Program> {
    let tokens = lex(src)?;
    if tokens.is_empty() {
        // An empty program is treated as identity, which is friendlier than jq.
        return Ok(Program::Query(Target::Value(Ast::Identity)));
    }
    let mut p = Parser { tokens, pos: 0 };
    let program = p.parse_program()?;
    if let Some(tok) = p.peek() {
        return Err(YqrError::parse(format!(
            "unexpected trailing token {tok:?}"
        )));
    }
    Ok(program)
}

/// Builds the [`Target`] a selector word names, from the path it wraps.
type SelectorBuilder = fn(Ast) -> Target;

/// Every selector word, paired with the [`Target`] it builds.
///
/// A word is a selector only in **function position** — immediately followed
/// by `(` — so none of these is a reserved identifier: `.key` and
/// `.head_comment` keep reading fields of those names. `foot_comment` is in
/// the table on purpose despite having no implementation: parsing it is what
/// lets the refusal name a reason instead of reporting an unexpected token.
// Feature f007.
const SELECTORS: &[(&str, SelectorBuilder)] = &[
    ("key", Target::Key),
    ("line_comment", Target::LineComment),
    ("head_comment", Target::HeadComment),
    ("foot_comment", Target::FootComment),
];

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

    /// Parse a whole program: a `del(...)` form, an assignment/append, or a
    /// bare read-only query.
    // Feature f006.
    fn parse_program(&mut self) -> Result<Program> {
        // `del(...)` is the only *mutation* that starts with a bare
        // identifier; every value query starts with '.'.
        if self.at_function("del") {
            return self.parse_del();
        }

        let target = self.parse_target()?;
        match self.peek() {
            Some(Token::Eq) => {
                self.advance();
                let rhs = self.parse_rhs()?;
                Ok(Program::Mutate(Mutation::Assign { target, rhs }))
            }
            Some(Token::PlusEq) => {
                self.advance();
                // `+=` appends to a sequence, and a key is not one. Refused
                // here rather than at eval so the message names the operator
                // the user reached for.
                let Target::Value(path) = target else {
                    return Err(YqrError::parse(format!(
                        "'+=' appends an item to a sequence, so its left side must be a \
                         path to one; '{}(...)' does not name a sequence",
                        target
                            .selector_name()
                            .expect("a non-Value target has a selector name")
                    )));
                };
                let rhs = self.parse_rhs()?;
                Ok(Program::Mutate(Mutation::Append { path, rhs }))
            }
            Some(Token::PipeEq) => Err(YqrError::parse(
                "the '|=' computed-update operator is not yet supported \
                 (planned for a future release); use '=' with a literal or path"
                    .to_string(),
            )),
            _ => Ok(Program::Query(target)),
        }
    }

    /// Whether the cursor sits on `name` used as a function — an identifier
    /// immediately followed by `(`.
    ///
    /// Function position is the whole reason these words cost no reserved
    /// identifiers: `.key` is a field access because the `(` is missing, and
    /// a mapping whose key is `key` keeps reading exactly as before.
    // Feature f007.
    fn at_function(&self, name: &str) -> bool {
        matches!(self.peek(), Some(Token::Ident(got)) if got == name)
            && matches!(self.peek_at(1), Some(Token::LParen))
    }

    /// Parse a target: a selector wrapping a path, or a bare path.
    // Feature f007.
    fn parse_target(&mut self) -> Result<Target> {
        for (word, build) in SELECTORS {
            if self.at_function(word) {
                let target = build(self.parse_selector_arg()?);
                // Parsed, then refused — which is the whole reason the word is
                // in the grammar (`yqr-a002` §8).
                if matches!(target, Target::FootComment(_)) {
                    return Err(YqrError::parse(FOOT_COMMENT_REFUSAL.to_string()));
                }
                return Ok(target);
            }
        }
        Ok(Target::Value(self.parse_pipeline()?))
    }

    /// Consume `name '(' pipeline ')'`, returning the wrapped path. The
    /// identifier and `(` have been confirmed by the caller but not consumed.
    // Feature f007.
    fn parse_selector_arg(&mut self) -> Result<Ast> {
        self.advance(); // the selector word
        self.expect(&Token::LParen)?;
        let path = self.parse_pipeline()?;
        self.expect(&Token::RParen)?;
        Ok(path)
    }

    /// Parse a `del(<target>)` mutation. The opening `del` identifier and `(`
    /// have been confirmed by the caller but not yet consumed.
    // Feature f006, extended for f007's targets.
    fn parse_del(&mut self) -> Result<Program> {
        self.advance(); // `del`
        self.expect(&Token::LParen)?;
        let target = self.parse_target()?;
        self.expect(&Token::RParen)?;
        // A key has no existence apart from its entry, so there is no edit
        // "delete the key" could mean that `del(<path>)` does not already
        // spell. The grammar admits the form, so the refusal lives here.
        if let Target::Key(_) = target {
            return Err(YqrError::parse(
                "del(key(...)) is not an edit: a key cannot outlive its entry. \
                 Use del(<path>) to remove the whole entry, or key(<path>) = \"new\" \
                 to rename it"
                    .to_string(),
            ));
        }
        Ok(Program::Mutate(Mutation::Delete { target }))
    }

    /// Parse the right-hand side of an assignment or append: a scalar literal
    /// or a `.`-rooted path.
    // Feature f006.
    fn parse_rhs(&mut self) -> Result<Rhs> {
        match self.peek() {
            Some(Token::Int(n)) => {
                let n = *n;
                self.advance();
                Ok(Rhs::Literal(Value::Int(n)))
            }
            Some(Token::Float(f)) => {
                let f = *f;
                self.advance();
                Ok(Rhs::Literal(Value::Float(f)))
            }
            Some(Token::Str(s)) => {
                let s = s.clone();
                self.advance();
                Ok(Rhs::Literal(Value::String(s)))
            }
            Some(Token::Ident(name)) => {
                let literal = match name.as_str() {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    "null" => Value::Null,
                    other => {
                        return Err(YqrError::parse(format!(
                            "expected a scalar literal or path on the right of '='/'+=' , \
                             found bare identifier {other:?} (only true/false/null are keywords)"
                        )));
                    }
                };
                self.advance();
                Ok(Rhs::Literal(literal))
            }
            Some(Token::Dot) => Ok(Rhs::Path(self.parse_pipeline()?)),
            other => Err(YqrError::parse(format!(
                "expected a scalar literal or path on the right of '='/'+=' , found {other:?}"
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

    // -- Feature f006: mutation programs ---------------------------------------

    #[test]
    fn plain_query_is_a_query_program() {
        assert_eq!(
            parse_program(".a.b").unwrap(),
            Program::Query(Target::Value(Ast::pipe(
                Ast::Field("a".into()),
                Ast::Field("b".into())
            )))
        );
    }

    #[test]
    fn parses_scalar_assignment() {
        assert_eq!(
            parse_program(".spec.replicas = 5").unwrap(),
            Program::Mutate(Mutation::Assign {
                target: Target::Value(Ast::pipe(
                    Ast::Field("spec".into()),
                    Ast::Field("replicas".into())
                )),
                rhs: Rhs::Literal(Value::Int(5)),
            })
        );
    }

    #[test]
    fn parses_literal_rhs_variants() {
        let cases = [
            ("=1.5", Rhs::Literal(Value::Float(1.5))),
            (r#"= "hi""#, Rhs::Literal(Value::String("hi".into()))),
            ("= true", Rhs::Literal(Value::Bool(true))),
            ("= false", Rhs::Literal(Value::Bool(false))),
            ("= null", Rhs::Literal(Value::Null)),
        ];
        for (op, want) in cases {
            let src = format!(".a {op}");
            match parse_program(&src).unwrap() {
                Program::Mutate(Mutation::Assign { rhs, .. }) => assert_eq!(rhs, want),
                other => panic!("expected assign for {src:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn parses_path_rhs() {
        assert_eq!(
            parse_program(".a = .b.c").unwrap(),
            Program::Mutate(Mutation::Assign {
                target: Target::Value(Ast::Field("a".into())),
                rhs: Rhs::Path(Ast::pipe(Ast::Field("b".into()), Ast::Field("c".into()))),
            })
        );
    }

    #[test]
    fn parses_append() {
        assert_eq!(
            parse_program(".ports += 9090").unwrap(),
            Program::Mutate(Mutation::Append {
                path: Ast::Field("ports".into()),
                rhs: Rhs::Literal(Value::Int(9090)),
            })
        );
    }

    #[test]
    fn parses_delete() {
        assert_eq!(
            parse_program("del(.metadata.labels)").unwrap(),
            Program::Mutate(Mutation::Delete {
                target: Target::Value(Ast::pipe(
                    Ast::Field("metadata".into()),
                    Ast::Field("labels".into())
                )),
            })
        );
    }

    #[test]
    fn pipe_equals_is_a_clear_not_supported_error() {
        let err = parse_program(".a |= .b").unwrap_err();
        assert!(matches!(err, YqrError::Parse(ref m) if m.contains("not yet supported")));
    }

    #[test]
    fn bare_identifier_rhs_is_rejected() {
        assert!(matches!(parse_program(".a = foo"), Err(YqrError::Parse(_))));
    }

    #[test]
    fn read_only_parse_rejects_a_mutation() {
        // The query-only `parse` entry point must refuse a mutating filter.
        assert!(matches!(parse(".a = 5"), Err(YqrError::Parse(_))));
        assert!(matches!(parse("del(.a)"), Err(YqrError::Parse(_))));
    }

    #[test]
    fn del_without_parens_is_a_parse_error() {
        // `del` alone (no call parens) is not a valid query start.
        assert!(parse_program("del").is_err());
    }

    // -- Feature f007: the key selector -----------------------------------

    #[test]
    fn parses_a_key_read() {
        assert_eq!(
            parse_program("key(.metadata.name)").unwrap(),
            Program::Query(Target::Key(Ast::pipe(
                Ast::Field("metadata".into()),
                Ast::Field("name".into())
            )))
        );
    }

    #[test]
    fn parses_a_key_rename() {
        let Program::Mutate(Mutation::Assign { target, rhs }) =
            parse_program("key(.a) = \"b\"").unwrap()
        else {
            panic!("expected an assignment");
        };
        assert_eq!(target, Target::Key(Ast::Field("a".into())));
        assert_eq!(rhs, Rhs::Literal(Value::String("b".into())));
    }

    #[test]
    fn selector_words_stay_field_accesses_outside_function_position() {
        // The whole "no new reserved words" claim, checked against the words
        // this grammar actually spends. `swap` and `move` are ordinary YAML
        // field names, so a regression here would break read-only queries.
        for word in [
            "key",
            "del",
            "swap",
            "move",
            "line_comment",
            "head_comment",
            "foot_comment",
        ] {
            let src = format!(".{word}");
            assert_eq!(
                parse_program(&src).unwrap(),
                Program::Query(Target::Value(Ast::Field(word.into()))),
                "{src} should parse as a field access"
            );
        }
    }

    #[test]
    fn a_key_selector_iterates_like_the_path_it_wraps() {
        assert_eq!(
            parse_program("key(.items[])").unwrap(),
            Program::Query(Target::Key(Ast::pipe(
                Ast::Field("items".into()),
                Ast::Iterate
            )))
        );
    }

    #[test]
    fn del_still_takes_a_pipeline() {
        // a002 2.3 keeps `del`'s existing argument rather than narrowing it to
        // a bare path; this is the regression that narrowing would cause.
        let Program::Mutate(Mutation::Delete { target }) = parse_program("del(.a | .b)").unwrap()
        else {
            panic!("expected a delete");
        };
        assert_eq!(
            target,
            Target::Value(Ast::pipe(Ast::Field("a".into()), Ast::Field("b".into())))
        );
    }

    #[test]
    fn del_of_a_key_is_refused_with_a_reason() {
        let err = parse_program("del(key(.a))").unwrap_err();
        let text = format!("{err}");
        assert!(text.contains("cannot outlive its entry"), "got: {text}");
        assert!(
            text.contains("key(<path>) = "),
            "should suggest rename: {text}"
        );
    }

    #[test]
    fn append_to_a_key_is_refused_with_a_reason() {
        let err = parse_program("key(.a) += 1").unwrap_err();
        assert!(
            format!("{err}").contains("does not name a sequence"),
            "got: {err}"
        );
    }

    #[test]
    fn parses_the_comment_selectors() {
        assert_eq!(
            parse_program("line_comment(.a)").unwrap(),
            Program::Query(Target::LineComment(Ast::Field("a".into())))
        );
        assert_eq!(
            parse_program("head_comment(.a)").unwrap(),
            Program::Query(Target::HeadComment(Ast::Field("a".into())))
        );
    }

    #[test]
    fn foot_comment_is_parsed_then_refused_with_a_reason() {
        // The word is in the grammar precisely so this message exists rather
        // than an unexpected-token report (a002 §8).
        for src in [
            "foot_comment(.a)",
            "foot_comment(.a) = \"x\"",
            "del(foot_comment(.a))",
        ] {
            let text = format!("{}", parse_program(src).unwrap_err());
            assert!(text.contains("foot_comment"), "should name it: {text}");
            assert!(text.contains("head_comment"), "should redirect: {text}");
        }
    }

    #[test]
    fn del_composes_with_the_comment_selectors() {
        let Program::Mutate(Mutation::Delete { target }) =
            parse_program("del(line_comment(.a))").unwrap()
        else {
            panic!("expected a delete");
        };
        assert_eq!(target, Target::LineComment(Ast::Field("a".into())));
    }

    #[test]
    fn the_read_only_entry_point_rejects_a_selector() {
        // `parse` feeds the classic pipeline, which has no document bytes.
        let err = parse("key(.a)").unwrap_err();
        assert!(format!("{err}").contains("normalize"), "got: {err}");
    }
}
