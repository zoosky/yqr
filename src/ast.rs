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
    /// A read-only query. [`Target::Value`] is the M0 streaming model; the
    /// other variants read something attached to a node instead.
    Query(Target),
    /// A single mutation applied to the source document.
    Mutate(Mutation),
}

/// What a read or a mutation addresses: a value node, or something attached
/// to one.
///
/// yqr's path grammar addresses value nodes and only value nodes — a `Path`
/// resolves to a *value's* byte span. A key token is attached to a node
/// without being one, so it cannot be named by a path alone; a naming
/// function wraps the path instead. The same wrapper is what a comment
/// selector will use.
///
/// The inner [`Ast`] is the ordinary path, so it resolves and iterates
/// exactly as it does anywhere else.
// Feature f007: the non-value addressing target.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// A value node — a bare path, unchanged from M0.
    Value(Ast),
    /// `key(<path>)` — the key token of the mapping entry at `<path>`.
    Key(Ast),
    /// `line_comment(<path>)` — the `# ...` comment following the value on the
    /// entry's own line.
    LineComment(Ast),
    /// `head_comment(<path>)` — the run of comment lines immediately above the
    /// entry, at its own indentation.
    HeadComment(Ast),
    /// `foot_comment(<path>)` — parsed only so the refusal can name a reason;
    /// there is no upstream mutator and no design for it.
    FootComment(Ast),
}

impl Target {
    /// The path this target wraps, whichever kind it is.
    #[must_use]
    pub fn path(&self) -> &Ast {
        match self {
            Target::Value(ast)
            | Target::Key(ast)
            | Target::LineComment(ast)
            | Target::HeadComment(ast)
            | Target::FootComment(ast) => ast,
        }
    }

    /// The selector's spelling, for diagnostics. `None` for a plain value
    /// path, which has no function name to quote back at the user.
    #[must_use]
    pub fn selector_name(&self) -> Option<&'static str> {
        match self {
            Target::Value(_) => None,
            Target::Key(_) => Some("key"),
            Target::LineComment(_) => Some("line_comment"),
            Target::HeadComment(_) => Some("head_comment"),
            Target::FootComment(_) => Some("foot_comment"),
        }
    }
}

/// Why `foot_comment(...)` is refused wherever it appears.
///
/// The word is in the grammar so the parser can build a target and then say
/// this, rather than reporting an unexpected token and leaving the user to
/// guess whether the spelling or the feature is missing.
// Feature f007.
pub const FOOT_COMMENT_REFUSAL: &str = "foot_comment(...) is not supported: a comment below an entry belongs to \
     whatever follows it as often as to the entry itself, so there is no \
     unambiguous block to address, and the YAML engine has no mutator for one. \
     Use head_comment(<path>) for the block above an entry";

/// A surgical edit targeting exactly one addressable node.
///
/// Each variant maps to one of noyalib's guarded, re-parse-checked mutators, so
/// an edit that would restructure the document is refused rather than emitted.
// Feature f006: value-assignment mutation surface (`=`, `+=`, `del`).
#[derive(Debug, Clone, PartialEq)]
pub enum Mutation {
    /// `<target> = <rhs>` — replace the value at a path (or create a new
    /// mapping key when the final segment is absent), or rename a key when
    /// the target is [`Target::Key`].
    Assign {
        /// The left-hand target selecting what to write.
        target: Target,
        /// The value source.
        rhs: Rhs,
    },
    /// `<path> += <rhs>` — append an item to the block sequence at `path`.
    ///
    /// Value-only: there is nothing to append to a key.
    Append {
        /// The left-hand path selecting the sequence.
        path: Ast,
        /// The item to append.
        rhs: Rhs,
    },
    /// `del(<target>)` — remove the block entry at a path.
    Delete {
        /// The target selecting what to remove.
        target: Target,
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
