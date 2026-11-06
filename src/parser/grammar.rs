//! The grammar productions of the recursive-descent parser.
//!
//! The [`Parser`] struct, its cursor and error helpers live in the parent
//! module; this file holds the productions themselves, from the program
//! forms down to a bracket step. Split out to keep each file readable on
//! its own (the repository's module-size rule).

use super::{BUILTINS, Parser, REORDERS, SELECTORS, check_writable, fold_steps};
use crate::Value;
use crate::ast::{
    Ast, BinOp, Builtin, FOOT_COMMENT_REFUSAL, Mutation, Program, ReorderOp, Rhs, Target,
};
use crate::error::{Result, YqrError};
use crate::lexer::Token;

impl Parser<'_> {
    /// Parse a whole program: a `del(...)` form, an assignment/append, or a
    /// bare read-only query.
    // Feature f006.
    pub(super) fn parse_program(&mut self) -> Result<Program> {
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
                check_writable(target.path(), "=")?;
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
                check_writable(&path, "+=")?;
                let rhs = self.parse_rhs()?;
                Ok(Program::Mutate(Mutation::Append { path, rhs }))
            }
            // `|=` runs its right-hand filter on the *node*, so the RHS is a
            // whole pipeline rather than an `Rhs`: `(. + 1)`, `to_entries`,
            // `.b.c`. Feature f008.
            Some(Token::PipeEq) => {
                self.advance();
                // A key or a comment is not a value the filter could receive,
                // and jq has no update form for either. Refused here so the
                // message names the operator the user reached for.
                let Target::Value(path) = target else {
                    return Err(YqrError::parse(format!(
                        "'|=' runs a filter on the value at a path, so its left side must be \
                         a path to one; '{}(...)' does not name a value",
                        target
                            .selector_name()
                            .expect("a non-Value target has a selector name")
                    )));
                };
                check_writable(&path, "|=")?;
                let rhs = self.parse_pipeline()?;
                Ok(Program::Mutate(Mutation::Update { path, rhs }))
            }
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
        check_writable(target.path(), "del")?;
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
        // The fourth mutation site, and the one that hides a miss best: an
        // unresolvable reorder path is an *absent* path, which the write
        // driver is specified to leave alone at exit 0. A builtin is not
        // absent — it is unwritable — so without this the verb reports
        // success for a reorder that did nothing.
        // Feature f017.
        check_writable(&path, op.word())?;
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
        Err(self.err_here(format!(
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
            other => Err(self.err_here(format!(
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
                        return Err(self.err_here(format!(
                            "expected a scalar literal or path on the right of '='/'+=' , \
                             found bare identifier {other:?} (only true/false/null are keywords)"
                        )));
                    }
                };
                self.advance();
                Ok(Rhs::Literal(literal))
            }
            Some(Token::Dot) => Ok(Rhs::Path(self.parse_pipeline()?)),
            other => Err(self.err_here(format!(
                "expected a scalar literal or path on the right of '='/'+=' , found {other:?}"
            ))),
        }
    }

    fn parse_pipeline(&mut self) -> Result<Ast> {
        let mut node = self.parse_additive()?;
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            let rhs = self.parse_additive()?;
            node = Ast::pipe(node, rhs);
        }
        Ok(node)
    }

    /// Additive level: `a + b`, `a - b`. Lowest-binding arithmetic.
    ///
    /// Sits between `parse_pipeline` and `parse_term`, so `|` binds looser
    /// than `+` (jq's precedence) and a pipeline stage can be an expression.
    // Feature f008.
    fn parse_additive(&mut self) -> Result<Ast> {
        let mut node = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_multiplicative()?;
            node = Ast::Binary(op, Box::new(node), Box::new(rhs));
        }
        Ok(node)
    }

    /// Multiplicative level: `a * b`, `a / b`, `a % b`. Binds tighter than
    /// additive, so `1 + 2 * 3` is `7`.
    // Feature f008.
    fn parse_multiplicative(&mut self) -> Result<Ast> {
        let mut node = self.parse_term()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                Some(Token::Percent) => BinOp::Rem,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_term()?;
            node = Ast::Binary(op, Box::new(node), Box::new(rhs));
        }
        Ok(node)
    }

    fn parse_term(&mut self) -> Result<Ast> {
        let mut node = match self.peek() {
            // `(...)` groups an expression, which is how `(1 + 2) * 3` beats
            // precedence and how `. + 1` is written as a `|=` right-hand side.
            // Feature f008.
            Some(Token::LParen) => {
                self.advance();
                let inner = self.parse_pipeline()?;
                self.expect(&Token::RParen)?;
                inner
            }
            // A bare scalar in filter position: the `1` in `. + 1`.
            // Feature f008.
            Some(Token::Int(n)) => {
                let n = *n;
                self.advance();
                Ast::Literal(Value::Int(n))
            }
            Some(Token::Float(f)) => {
                let f = *f;
                self.advance();
                Ast::Literal(Value::Float(f))
            }
            Some(Token::Str(text)) => {
                let text = text.clone();
                self.advance();
                Ast::Literal(Value::String(text))
            }
            _ => match self.builtin_at_cursor() {
                Some(b) => self.parse_builtin(b)?,
                None => self.parse_path()?,
            },
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
            return Err(self.err_here(format!(
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

        // Optional first component immediately after the leading dot. A quoted
        // key (`."a.b"`, jq's spelling) is the same step as `.["a.b"]`.
        // Feature f030.
        match self.peek() {
            Some(Token::Ident(name) | Token::Str(name)) => {
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
    /// component or a builtin: `[...]`, `.field`, `."quoted key"`, `.[...]`.
    ///
    /// Shared by both so that `to_entries[].key` and `.a[].key` cannot drift
    /// apart in what they accept.
    // Feature f017: extracted from `parse_path` so a builtin can take a chain.
    fn parse_chain(&mut self, steps: &mut Vec<Ast>) -> Result<()> {
        loop {
            match self.peek() {
                Some(Token::LBracket) => steps.push(self.parse_bracket()?),
                Some(Token::Dot) => match self.peek_at(1) {
                    // Feature f030: `."a.b"` alongside `.a`.
                    Some(Token::Ident(name) | Token::Str(name)) => {
                        let name = name.clone();
                        self.advance(); // dot
                        self.advance(); // ident or quoted key
                        steps.push(Ast::Field(name));
                    }
                    Some(Token::LBracket) => {
                        self.advance(); // dot
                        steps.push(self.parse_bracket()?);
                    }
                    other => {
                        // The offending token is the one after the dot the
                        // cursor sits on, or the end of the source.
                        return Err(YqrError::parse_at(
                            format!(
                                "expected a field name, a quoted key or '[' after '.', found {other:?}"
                            ),
                            self.byte_span_at(self.pos + 1),
                        ));
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
                let msg = format!("expected ']', integer, or string inside '[]', found {other:?}");
                // `advance` consumed the offending token when there was one;
                // otherwise the cursor sits at the end of the source.
                return Err(match other {
                    Some(_) => self.err_prev(msg),
                    None => self.err_here(msg),
                });
            }
        };
        self.expect(&Token::RBracket)?;
        Ok(step)
    }
}
