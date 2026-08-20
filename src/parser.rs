//! Recursive-descent parser: [`Token`]s → [`Program`].
//!
//! Grammar implemented for milestone M0 plus the write tier's top-level forms:
//!
//! ```text
//! program  := 'del' '(' target ')'            ; delete mutation
//!           | reorder                          ; reorder mutation
//!           | target '=' rhs                   ; assignment / rename
//!           | pipeline '+=' rhs                ; append (value-only)
//!           | target                           ; read-only query
//! target   := selector
//!           | pipeline                         ; a value node
//! selector := 'key' '(' pipeline ')'          ; the entry's key token
//! reorder  := ('swap' | 'move') '(' pipeline ';' Int ';' Int ')'
//! rhs      := number | Str | 'true' | 'false' | 'null'   ; scalar literal
//!           | pipeline                         ; a '.'-rooted path
//! pipeline := term ('|' term)*
//! term     := (path | builtin chain) ('?')*
//! builtin  := 'to_entries'                    ; takes its input from the pipe
//! path     := '.' component* | '.'            ; leading dot, then chained steps
//! component:= Ident                           ; .foo
//!           | '.' Ident                       ; chained .bar
//!           | '.'? bracket
//! bracket  := '[' ']'        -> iterate
//!           | '[' Int ']'    -> index
//!           | '[' Str ']'    -> field-by-string
//! ```

use crate::Value;
use crate::ast::{Ast, Builtin, FOOT_COMMENT_REFUSAL, Mutation, Program, ReorderOp, Rhs, Target};
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

/// Every reorder verb, paired with the operation it performs.
///
/// Recognized in function position at the start of a program only, on the
/// same rule as the selectors: `swap` and `move` are ordinary YAML field
/// names, so `.swap` keeps reading a field called `swap`.
// Feature f007.
const REORDERS: &[(&str, ReorderOp)] = &[("swap", ReorderOp::Swap), ("move", ReorderOp::Move)];

/// Every builtin, paired with the node it compiles to.
///
/// Recognized wherever a term may start, which is the difference from
/// [`SELECTORS`] and [`REORDERS`]: those are function words, spotted by the
/// `(` after them, while a builtin is spotted by being an identifier where a
/// path was expected. All three rest on the same property — a yqr path always
/// begins with `.`, so none of these words is reserved and `.to_entries` still
/// reads a field.
// Feature f017.
const BUILTINS: &[(&str, Builtin)] = &[("to_entries", Builtin::ToEntries)];

/// Why a builtin cannot be written to, spelled out per builtin.
///
/// The pairs `to_entries` produces are a view yqr invented; they exist in no
/// file, so there is no byte range an assignment could land in. Refused at
/// parse rather than at eval, so the message can say *that* instead of
/// reporting a path that resolved to nothing (`yqr-a002` §8's pattern).
// Feature f017.
fn builtin_is_not_writable(b: Builtin, op: &str) -> YqrError {
    YqrError::parse(format!(
        "'{}' computes a value rather than naming one in the document, so it \
         cannot appear on the left of '{op}': there is nothing to write back to. \
         Read it with a query, or address the entry itself by path",
        b.word()
    ))
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

    /// Parse a whole program: a `del(...)` form, an assignment/append, or a
    /// bare read-only query.
    // Feature f006.
    fn parse_program(&mut self) -> Result<Program> {
        // `del(...)` is the only *mutation* that starts with a bare
        // identifier; every value query starts with '.'.
        if self.at_function("del") {
            return self.parse_del();
        }
        // A reorder has no target to build — an ordering is not a node — so it
        // is dispatched here rather than through `parse_target`.
        for (word, op) in REORDERS {
            if self.at_function(word) {
                return self.parse_reorder(*op);
            }
        }

        let target = self.parse_target()?;
        match self.peek() {
            Some(Token::Eq) => {
                self.advance();
                if let Some(b) = target.path().builtin() {
                    return Err(builtin_is_not_writable(b, "="));
                }
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
                if let Some(b) = path.builtin() {
                    return Err(builtin_is_not_writable(b, "+="));
                }
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
        if let Some(b) = target.path().builtin() {
            return Err(builtin_is_not_writable(b, "del"));
        }
        Ok(Program::Mutate(Mutation::Delete { target }))
    }

    /// Parse a `swap(<path>; i; j)` / `move(<path>; from; to)` mutation. The
    /// verb and its `(` have been confirmed by the caller but not consumed.
    ///
    /// Indices are parsed as plain integers rather than as an [`Rhs`]: an
    /// ordering is a position, and a path or a string there would have no
    /// meaning to fall back on.
    // Feature f007: the reorder verb (a002 slice 3).
    fn parse_reorder(&mut self, op: ReorderOp) -> Result<Program> {
        self.advance(); // the verb
        self.expect(&Token::LParen)?;
        let path = self.parse_pipeline()?;
        self.expect_semi(op)?;
        let from = self.parse_index(op)?;
        self.expect_semi(op)?;
        let to = self.parse_index(op)?;
        self.expect(&Token::RParen)?;
        Ok(Program::Mutate(Mutation::Reorder { path, op, from, to }))
    }

    /// Consume the `;` separating a reorder verb's arguments, naming the whole
    /// form when it is missing — `;` is the one token of this grammar a jq or
    /// yq user has no reason to expect.
    // Feature f007.
    fn expect_semi(&mut self, op: ReorderOp) -> Result<()> {
        if matches!(self.peek(), Some(Token::Semi)) {
            self.advance();
            return Ok(());
        }
        Err(YqrError::parse(format!(
            "expected ';' between the arguments of {0}(...): the form is \
             {0}(<path>; {1}; {2}), found {3:?}",
            op.word(),
            op.arg_name(true),
            op.arg_name(false),
            self.peek()
        )))
    }

    /// Consume one integer index of a reorder verb.
    // Feature f007.
    fn parse_index(&mut self, op: ReorderOp) -> Result<i64> {
        match self.peek() {
            Some(Token::Int(n)) => {
                let n = *n;
                self.advance();
                Ok(n)
            }
            other => Err(YqrError::parse(format!(
                "{}(...) takes two integer indices, found {other:?} \
                 (negative indices count from the end, as '.[-1]' does)",
                op.word()
            ))),
        }
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
        let mut node = match self.builtin_at_cursor() {
            Some(b) => self.parse_builtin(b)?,
            None => self.parse_path()?,
        };
        while matches!(self.peek(), Some(Token::Question)) {
            self.advance();
            node = Ast::optional(node);
        }
        Ok(node)
    }

    /// The builtin the cursor sits on, if it sits on one.
    ///
    /// No lookahead past the word is needed, and that is the point: a builtin
    /// is an identifier appearing where a path was expected, and a path can
    /// never start with one because every path starts with `.`.
    // Feature f017.
    fn builtin_at_cursor(&self) -> Option<Builtin> {
        let Some(Token::Ident(word)) = self.peek() else {
            return None;
        };
        BUILTINS
            .iter()
            .find(|(name, _)| name == word)
            .map(|(_, b)| *b)
    }

    /// Consume a builtin word and whatever path steps are chained onto it.
    ///
    /// `to_entries[]` has to mean "the builtin, then iterate", so the chain
    /// that follows a path also follows a builtin — the pairs are an ordinary
    /// sequence once produced, and indexing into them should not need a pipe.
    // Feature f017.
    fn parse_builtin(&mut self, b: Builtin) -> Result<Ast> {
        self.advance(); // the builtin word

        // `to_entries(...)` is a plausible thing to type, coming from a
        // language where every builtin takes parentheses. Naming the mistake
        // beats reporting an unexpected '('.
        if matches!(self.peek(), Some(Token::LParen)) {
            return Err(YqrError::parse(format!(
                "'{}' takes no arguments: it reads whatever the pipe hands it. \
                 Write '<path> | {}' instead of '{}(<path>)'",
                b.word(),
                b.word(),
                b.word()
            )));
        }

        let mut steps = vec![Ast::Builtin(b)];
        self.parse_chain(&mut steps)?;
        Ok(fold_steps(steps))
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

        self.parse_chain(&mut steps)?;
        Ok(fold_steps(steps))
    }

    /// Consume the chained components that may follow a path's first
    /// component or a builtin: `[...]`, `.field`, `.[...]`.
    ///
    /// Shared by both so that `to_entries[].key` and `.a[].key` cannot drift
    /// apart in what they accept.
    // Feature f017: extracted from `parse_path` so a builtin can take a chain.
    fn parse_chain(&mut self, steps: &mut Vec<Ast>) -> Result<()> {
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
        Ok(())
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

    // -- Feature f007: the reorder verbs -----------------------------------

    #[test]
    fn parses_swap_and_move() {
        assert_eq!(
            parse_program("swap(.spec.containers; 0; 2)").unwrap(),
            Program::Mutate(Mutation::Reorder {
                path: Ast::pipe(Ast::Field("spec".into()), Ast::Field("containers".into())),
                op: ReorderOp::Swap,
                from: 0,
                to: 2,
            })
        );
        assert_eq!(
            parse_program("move(.; 1; 0)").unwrap(),
            Program::Mutate(Mutation::Reorder {
                path: Ast::Identity,
                op: ReorderOp::Move,
                from: 1,
                to: 0,
            })
        );
    }

    #[test]
    fn a_reorder_index_may_be_negative() {
        let Program::Mutate(Mutation::Reorder { from, to, .. }) =
            parse_program("swap(.xs; 0; -1)").unwrap()
        else {
            panic!("expected a reorder");
        };
        assert_eq!((from, to), (0, -1));
    }

    #[test]
    fn a_missing_separator_names_the_whole_form() {
        // `;` is the one token of this grammar a jq or yq user has no reason
        // to expect, so the message spells the form rather than the token.
        let text = format!("{}", parse_program("swap(.xs 0 1)").unwrap_err());
        assert!(text.contains("swap(<path>; i; j)"), "got: {text}");
        // `move`'s arguments are not interchangeable, and the message says so.
        let text = format!("{}", parse_program("move(.xs 0 1)").unwrap_err());
        assert!(text.contains("move(<path>; from; to)"), "got: {text}");
    }

    #[test]
    fn a_reorder_index_must_be_an_integer() {
        for src in [
            "swap(.xs; \"a\"; 1)",
            "swap(.xs; 0; .y)",
            "move(.xs; 1.5; 0)",
        ] {
            let text = format!("{}", parse_program(src).unwrap_err());
            assert!(
                text.contains("two integer indices"),
                "{src} should say so: {text}"
            );
        }
    }

    #[test]
    fn a_reorder_is_not_a_target() {
        // There is no node to name, so a reorder composes with neither `del`
        // nor `=`; both are parse errors rather than silently accepted forms.
        assert!(parse_program("del(swap(.xs; 0; 1))").is_err());
        assert!(parse_program("swap(.xs; 0; 1) = 5").is_err());
    }

    #[test]
    fn the_read_only_entry_point_rejects_a_reorder() {
        let text = format!("{}", parse("swap(.xs; 0; 1)").unwrap_err());
        assert!(text.contains("mutation"), "got: {text}");
    }

    // -- Feature f017: the to_entries builtin ------------------------------

    #[test]
    fn parses_a_builtin_in_term_position() {
        assert_eq!(
            parse(".m | to_entries").unwrap(),
            Ast::pipe(Ast::Field("m".into()), Ast::Builtin(Builtin::ToEntries))
        );
    }

    #[test]
    fn a_builtin_takes_a_chain_the_way_a_path_does() {
        // `to_entries[]` must be "the builtin, then iterate" rather than a
        // parse error, and `to_entries[].key` must keep going.
        assert_eq!(
            parse("to_entries[]").unwrap(),
            Ast::pipe(Ast::Builtin(Builtin::ToEntries), Ast::Iterate)
        );
        assert_eq!(
            parse("to_entries[].key").unwrap(),
            Ast::pipe(
                Ast::pipe(Ast::Builtin(Builtin::ToEntries), Ast::Iterate),
                Ast::Field("key".into())
            )
        );
        assert_eq!(
            parse("to_entries[0]").unwrap(),
            Ast::pipe(Ast::Builtin(Builtin::ToEntries), Ast::Index(0))
        );
    }

    #[test]
    fn a_builtin_takes_the_optional_suffix() {
        assert_eq!(
            parse("to_entries?").unwrap(),
            Ast::optional(Ast::Builtin(Builtin::ToEntries))
        );
    }

    #[test]
    fn to_entries_costs_no_reserved_word() {
        // The whole reason the word is safe: a path starts with '.', so an
        // identifier there can only be a field name. This test fails the day
        // someone reserves it.
        assert_eq!(
            parse(".to_entries").unwrap(),
            Ast::Field("to_entries".into())
        );
        assert_eq!(
            parse(".a.to_entries").unwrap(),
            Ast::pipe(Ast::Field("a".into()), Ast::Field("to_entries".into()))
        );
        assert_eq!(
            parse(".[\"to_entries\"]").unwrap(),
            Ast::Field("to_entries".into())
        );
    }

    #[test]
    fn a_builtin_called_with_parentheses_is_told_it_takes_none() {
        let err = parse("to_entries(.m)").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("takes no arguments"), "{msg}");
        assert!(
            msg.contains(".m | to_entries") || msg.contains("| to_entries"),
            "{msg}"
        );
    }

    #[test]
    fn every_write_form_refuses_a_builtin_at_parse() {
        for (filter, op) in [
            (".m | to_entries = 1", "'='"),
            (".m | to_entries += 1", "'+='"),
            ("del(.m | to_entries)", "'del'"),
            ("to_entries = 1", "'='"),
        ] {
            let err = parse_program(filter).unwrap_err();
            assert!(
                matches!(err, YqrError::Parse(_)),
                "{filter}: expected a parse error, got {err:?}"
            );
            let msg = err.to_string();
            assert!(
                msg.contains("to_entries") && msg.contains(op),
                "{filter}: message should name the builtin and {op}, got {msg}"
            );
            assert!(
                msg.contains("nothing to write back to"),
                "{filter}: message should say why, got {msg}"
            );
        }
    }

    #[test]
    fn a_builtin_query_is_a_query_program() {
        assert_eq!(
            parse_program(".m | to_entries").unwrap(),
            Program::Query(Target::Value(Ast::pipe(
                Ast::Field("m".into()),
                Ast::Builtin(Builtin::ToEntries)
            )))
        );
    }
}
