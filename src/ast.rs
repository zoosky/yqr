//! Abstract syntax tree for the `yqr` filter language.
//!
//! The AST mirrors jq's "a filter maps one input to a stream of outputs" model.
//! Compound paths like `.a.b[0]` are desugared into a left-to-right [`Ast::Pipe`]
//! of atomic steps, which keeps the evaluator small and uniform.

/// A node in a compiled filter.
#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    /// `.` — yields the input unchanged.
    Identity,
    /// `.foo` / `.["foo"]` — look up a key in a mapping.
    Field(String),
    /// `.[n]` — index into a sequence (negative counts from the end).
    Index(i64),
    /// `.[]` — iterate the values of a sequence or mapping.
    Iterate,
    /// `a | b` — feed each output of `a` into `b`.
    Pipe(Box<Ast>, Box<Ast>),
    /// `f?` — run `f`, suppressing any runtime error to an empty stream.
    Optional(Box<Ast>),
}

impl Ast {
    /// Convenience constructor for a pipe node.
    pub fn pipe(lhs: Ast, rhs: Ast) -> Ast {
        Ast::Pipe(Box::new(lhs), Box::new(rhs))
    }

    /// Convenience constructor for an optional node.
    pub fn optional(inner: Ast) -> Ast {
        Ast::Optional(Box::new(inner))
    }
}
