//! Abstract syntax tree for the `yqr` filter language.
//!
//! The AST mirrors jq's "a filter maps one input to a stream of outputs" model.
//! Compound paths like `.a.b[0]` are desugared into a left-to-right [`Ast::Pipe`]
//! of atomic steps, which keeps the evaluator small and uniform.
//!
//! A whole filter compiles to a [`Program`]: either a read-only [`Ast`] query
//! (the classic streaming model) or a single [`Mutation`] that edits the source
//! document in place. The two are mutually exclusive — mixing a query and a
//! mutation in one filter is a parse error.

use crate::Value;

/// A compiled filter: a read-only query or a single in-place mutation.
///
/// The distinction is drawn at parse time so the driver can pick the read
/// pipeline or the fidelity write path without re-inspecting the filter.
// Feature f006: top-level query/mutation split.
#[derive(Debug, Clone, PartialEq)]
pub enum Program {
    /// A read-only, streaming query (the M0 model).
    Query(Ast),
    /// A single mutation applied to the source document.
    Mutate(Mutation),
}

/// A surgical edit targeting exactly one addressable node.
///
/// Each variant maps to one of noyalib's guarded, re-parse-checked mutators, so
/// an edit that would restructure the document is refused rather than emitted.
// Feature f006: value-assignment mutation surface (`=`, `+=`, `del`).
#[derive(Debug, Clone, PartialEq)]
pub enum Mutation {
    /// `<path> = <rhs>` — replace the value at `path` (or create a new mapping
    /// key when the final segment is absent).
    Assign {
        /// The left-hand path selecting the node to write.
        path: Ast,
        /// The value source.
        rhs: Rhs,
    },
    /// `<path> += <rhs>` — append an item to the block sequence at `path`.
    Append {
        /// The left-hand path selecting the sequence.
        path: Ast,
        /// The item to append.
        rhs: Rhs,
    },
    /// `del(<path>)` — remove the single-line block entry at `path`.
    Delete {
        /// The path selecting the entry to remove.
        path: Ast,
    },
}

/// The right-hand side of an assignment or append.
///
/// A scalar literal is emitted with style-matched quoting; a path copies the
/// value found at another location in the same document.
// Feature f006: scalar-literal / path RHS (a subset of M1 literals).
#[derive(Debug, Clone, PartialEq)]
pub enum Rhs {
    /// A scalar literal (`5`, `1.5`, `"hi"`, `true`, `false`, `null`).
    Literal(Value),
    /// A `.`-rooted path; its resolved value is copied to the target.
    Path(Ast),
}

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
