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

use crate::ast::{Ast, Builtin, Program, ReorderOp, Target};
use crate::error::{Result, YqrError};
use crate::lexer::{Token, lex_spanned};
use std::ops::Range;

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
    let spanned = lex_spanned(src)?;
    if spanned.is_empty() {
        // An empty program is treated as identity, which is friendlier than jq.
        return Ok(Program::Query(Target::Value(Ast::Identity)));
    }
    let (tokens, spans) = spanned.into_iter().map(|s| (s.token, s.span)).unzip();
    let mut p = Parser {
        src,
        tokens,
        spans,
        pos: 0,
    };
    let program = p.parse_program()?;
    if let Some(tok) = p.peek() {
        return Err(p.err_here(format!("unexpected trailing token {tok:?}")));
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

/// Refuse a left-hand side that computes a value rather than naming one.
///
/// A builtin's pairs, a literal, and an arithmetic result are all views yqr
/// invents; they exist in no file, so there is no byte range an edit could
/// land in. Refused at parse rather than at eval, so the message can say
/// *that* instead of reporting a path that resolved to nothing
/// (`yqr-a002` §8's pattern) — or, worse, succeeding silently, which is what a
/// computed path does when it reaches the write driver (see
/// [`Ast::unwritable`]).
// Feature f017, widened for f008.
fn check_writable(path: &Ast, op: &str) -> Result<()> {
    match path.unwritable() {
        None => Ok(()),
        Some(what) => Err(YqrError::parse(format!(
            "'{what}' computes a value rather than naming one in the document, so it \
             cannot appear on the left of '{op}': there is nothing to write back to. \
             Read it with a query, or address the entry itself by path"
        ))),
    }
}

struct Parser<'a> {
    /// The filter source, kept only to convert a failing token's character
    /// span into the byte range an error carries — a cost paid on the error
    /// path alone.
    // Feature f048.
    src: &'a str,
    tokens: Vec<Token>,
    /// Character span of each token in the filter source, parallel to
    /// `tokens` (see [`crate::lexer::SpannedToken`]).
    // Feature f048.
    spans: Vec<Range<usize>>,
    pos: usize,
}

impl Parser<'_> {
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

    /// The byte span of the token at `idx`, or an empty range at the end of
    /// the source when the cursor has run out of tokens.
    // Feature f048.
    fn byte_span_at(&self, idx: usize) -> Range<usize> {
        match self.spans.get(idx) {
            Some(span) => crate::lexer::byte_range(self.src, span.clone()),
            None => self.src.len()..self.src.len(),
        }
    }

    /// A parse error anchored at the token the cursor sits on (the usual
    /// case: the error was raised after a `peek`).
    // Feature f048.
    fn err_here(&self, msg: String) -> YqrError {
        YqrError::parse_at(msg, self.byte_span_at(self.pos))
    }

    /// A parse error anchored at the token just consumed (the error was
    /// raised after an `advance` that returned the offending token).
    // Feature f048.
    fn err_prev(&self, msg: String) -> YqrError {
        YqrError::parse_at(msg, self.byte_span_at(self.pos.saturating_sub(1)))
    }

    fn expect(&mut self, want: &Token) -> Result<()> {
        match self.advance() {
            Some(ref got) if got == want => Ok(()),
            Some(got) => Err(self.err_prev(format!("expected {want:?} but found {got:?}"))),
            None => Err(self.err_here(format!("expected {want:?} but reached end"))),
        }
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

mod grammar;

#[cfg(test)]
mod tests;
