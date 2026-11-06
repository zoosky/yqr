//! The diagnostic vocabulary of the validator: the stable [`Code`] set and
//! the [`Diagnostic`] finding the checks produce and [`super::render`]
//! renders. Split from the check logic to keep each file readable on its
//! own (the repository's module-size rule).

/// Stable identifier of a validation finding.
///
/// Codes are part of yqr's CLI contract: scripts may match on them, so they
/// are never renumbered or reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// `Y001` — the input is not well-formed YAML.
    Syntax,
    /// `Y002` — the parsed documents do not reproduce the input
    /// byte-for-byte (the fidelity invariant does not hold).
    StreamIntegrity,
    /// `Y003` — the input bytes are not valid UTF-8.
    Encoding,
    /// `Y101` — a mapping declares the same key twice (strict mode).
    DuplicateKey,
    /// `Y102` — two distinct keys collapse to the same string key.
    KeyCollision,
    /// `Y103` — a block mapping's value is not indented past its key.
    BlockValueIndent,
    /// `Y201` — a document violates the provided JSON Schema.
    SchemaViolation,
    /// `Y202` — the provided schema itself is unusable (not valid YAML,
    /// not a single document, or not a valid JSON Schema 2020-12
    /// document).
    SchemaUnusable,
    /// `Y203` — the document holds a value the JSON data model cannot
    /// represent, so it cannot be validated against a JSON Schema.
    OutsideJsonModel,
}

impl Code {
    /// The stable code string rendered in the diagnostic header.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Code::Syntax => "Y001",
            Code::StreamIntegrity => "Y002",
            Code::Encoding => "Y003",
            Code::DuplicateKey => "Y101",
            Code::KeyCollision => "Y102",
            Code::BlockValueIndent => "Y103",
            Code::SchemaViolation => "Y201",
            Code::SchemaUnusable => "Y202",
            Code::OutsideJsonModel => "Y203",
        }
    }
}

/// Serializes as the stable code string (`"Y101"`) — the identifier scripts
/// and editors already match on — rather than the Rust variant name.
// Feature f048.
#[cfg(feature = "serde")]
impl serde::Serialize for Code {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// One validation finding, ready to render.
///
/// `position` is a 1-based `(line, column)` in the checked source, present
/// when the finding has one, and `offset` is the same location as a byte
/// offset, so an editor can place a marker without re-scanning the
/// document. `note` adds context (such as where the first occurrence of a
/// duplicate key sits) and `help` suggests a fix when a concrete one
/// exists.
///
/// With the `serde` cargo feature, the finding serializes with `code` as
/// its stable string (`"Y101"`), `position` as a `[line, column]` pair or
/// `null`, and every other field by name. That shape is an interface for
/// editor hosts: fields may be added, but removing or renaming one is a
/// breaking change. These types are outputs — they serialize but never
/// deserialize, and the struct is `#[non_exhaustive]` so a field addition
/// is not a breaking Rust-API change either: hosts read findings, they do
/// not construct them.
// Feature f048: the serialized shape and the byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Diagnostic {
    /// Which finding this is.
    pub code: Code,
    /// One-line description of the problem.
    pub message: String,
    /// 1-based `(line, column)` when the finding has a location.
    pub position: Option<(usize, usize)>,
    /// Byte offset into the checked source of the same location
    /// `position` names.
    pub offset: Option<usize>,
    /// Additional context line, rendered as `= note: ...`.
    pub note: Option<String>,
    /// Suggested fix, rendered as `= help: ...`.
    pub help: Option<String>,
}
