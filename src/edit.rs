//! The typed edit API: compose edits as data, apply them through the
//! fidelity write tier.
//!
//! The filter language is one way to spell an edit; a host composing edits
//! programmatically — a CMS saving frontmatter, a browser editor — should
//! not have to render filter text (quoting rules included) and re-lex it.
//! Here a path is built from typed segments, an operation from typed parts,
//! and [`apply_all`] runs a change set through the same guarded, re-parse-
//! checked write tier the CLI uses. A key containing `.`, `"`, or filter
//! metacharacters is just data.
//!
//! Two contracts callers depend on:
//!
//! - **An empty change set returns the input bytes unchanged.**
//! - **Ops apply in order, and the first refusal names the op's index.**
//!
//! Semantics beyond the write tier's own (each recorded in the feature
//! spec's decision log):
//!
//! - A [`Op::Set`] creates missing intermediate *mapping* levels — an
//!   editor host that sets `menu.visible` means make it so. A missing
//!   level addressed by index is refused, and a scalar sitting *mid-path*,
//!   where the path needs to descend further, is a refused re-shape.
//! - A [`Op::Set`] whose **final** entry holds a scalar and whose value is
//!   a collection performs the re-shape the filter language refuses: the
//!   collection is written in block style one indent step past the key
//!   (flow inside a flow collection), and the result must load back as
//!   the value written. A sequence *item* keeps the refusal — the typed
//!   route onto a sequence is [`Op::Append`].
//! - A `Set` against contentless input (empty, whitespace, or comments
//!   only) creates the document, keeping any comment bytes in front;
//!   `Remove` and `Rename` against one are no-ops.
//! - Missing-prefix resolution reads the **first** document of a stream.
//!   When creation would actually fire on a multi-document stream, the op
//!   refuses instead of risking a sibling-clobbering write into a later
//!   document whose shape diverges — apply the change set per document.

// Features f050, f051.

use crate::ast::{Ast, Mutation, Rhs, Target};
use crate::error::{Result, YqrError};
use crate::fidelity::write;
use crate::value::{Mapping, Value};

/// One step of a [`Path`]: a mapping key or a sequence index.
///
/// With the `serde` cargo feature, a segment (de)serializes externally
/// tagged: `{"Key": "name"}` or `{"Index": -1}`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Segment {
    /// A mapping key, as data — never parsed, whatever it contains.
    Key(String),
    /// A sequence index; negative counts from the end, as `.[-1]` does.
    Index(i64),
}

/// A document path built from typed segments, with no trip through the
/// filter lexer.
///
/// ```
/// use yqr::edit::Path;
/// let path = Path::root().key("spec").key("containers").index(0).key("image");
/// ```
///
/// With the `serde` cargo feature, a path (de)serializes as the array of
/// its segments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Path(Vec<Segment>);

impl Path {
    /// The empty path. No operation accepts it as-is — extend it with
    /// [`key`](Self::key) and [`index`](Self::index).
    #[must_use]
    pub fn root() -> Self {
        Path(Vec::new())
    }

    /// Append a mapping-key step.
    #[must_use]
    pub fn key(mut self, name: impl Into<String>) -> Self {
        self.0.push(Segment::Key(name.into()));
        self
    }

    /// Append a sequence-index step (negative counts from the end).
    #[must_use]
    pub fn index(mut self, index: i64) -> Self {
        self.0.push(Segment::Index(index));
        self
    }

    /// The path's segments, in order.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.0
    }

    /// Lower the path into the evaluator's AST: one field or index step per
    /// segment, piped left to right.
    fn to_ast(&self) -> Result<Ast> {
        let mut steps = self.0.iter().map(|segment| match segment {
            Segment::Key(name) => Ast::Field(name.clone()),
            Segment::Index(index) => Ast::Index(*index),
        });
        let Some(first) = steps.next() else {
            return Err(YqrError::eval(
                "the empty path addresses the whole document; every edit \
                 operation needs at least one segment",
            ));
        };
        Ok(steps.fold(first, Ast::pipe))
    }

    /// The path truncated to its first `len` segments.
    fn prefix(&self, len: usize) -> Path {
        Path(self.0[..len].to_vec())
    }

    /// Render the path for an error message: `spec.containers[0].image`.
    fn display(&self) -> String {
        let mut out = String::new();
        for segment in &self.0 {
            match segment {
                Segment::Key(name) => {
                    if !out.is_empty() {
                        out.push('.');
                    }
                    out.push_str(name);
                }
                Segment::Index(index) => {
                    out.push('[');
                    out.push_str(&index.to_string());
                    out.push(']');
                }
            }
        }
        out
    }
}

impl From<Vec<Segment>> for Path {
    fn from(segments: Vec<Segment>) -> Self {
        Path(segments)
    }
}

/// One edit, as data. Each op maps onto exactly one guarded mutation of
/// the fidelity write tier; the refusal behavior is the tier's, unchanged,
/// except where the module doc says otherwise (parent and document
/// creation for `Set`).
///
/// With the `serde` cargo feature, an op (de)serializes externally tagged:
/// `{"Set": {"path": [{"Key": "title"}], "value": "New"}}`. These shapes
/// are an interface for hosts on both sides of the wasm boundary: fields
/// may be added, removals are breaking.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Op {
    /// Set `path` to `value`, creating a missing final key and missing
    /// intermediate mapping levels. A collection value over an existing
    /// scalar mapping entry re-shapes the entry in place (see the module
    /// doc); a scalar mid-path still refuses.
    Set {
        /// Where to write.
        path: Path,
        /// The value to write.
        value: Value,
    },
    /// Remove the entry at `path`. An absent path is a no-op.
    Remove {
        /// What to remove.
        path: Path,
    },
    /// Rename the key at `path` to `to`, keeping its value, position, and
    /// trailing comment. An absent path is a no-op.
    Rename {
        /// The entry whose key changes.
        path: Path,
        /// The new key, as data.
        to: String,
    },
    /// Append `value` to the sequence at `path`.
    Append {
        /// The sequence to push onto.
        path: Path,
        /// The item to append.
        value: Value,
    },
    /// Set the comment block above the entry at `path`.
    SetHeadComment {
        /// The entry the comment sits above.
        path: Path,
        /// The comment text, without `#` markers.
        text: String,
    },
}

impl Op {
    /// The op's name for error messages.
    const fn name(&self) -> &'static str {
        match self {
            Op::Set { .. } => "set",
            Op::Remove { .. } => "remove",
            Op::Rename { .. } => "rename",
            Op::Append { .. } => "append",
            Op::SetHeadComment { .. } => "set_head_comment",
        }
    }
}

/// Apply a change set in order, returning the edited document.
///
/// An empty change set returns the input bytes unchanged — the property
/// that makes a derived-but-empty diff safe to apply repeatedly. Ops apply
/// in order, each over the previous op's output; the first refusal is
/// reported with the op's index and name, and nothing is returned.
///
/// # Errors
///
/// The failing op's error, prefixed `op <index> (<name>)`. The categories
/// are the write tier's: a refused edit or an unreadable document.
pub fn apply_all(ops: &[Op], input: &str) -> Result<String> {
    if ops.is_empty() {
        return Ok(input.to_string());
    }
    let mut current = input.to_string();
    for (index, op) in ops.iter().enumerate() {
        current = apply_one(op, &current).map_err(|err| at_op(index, op, err))?;
    }
    Ok(current)
}

/// Prefix `err` with the failing op's index and name, keeping its category.
fn at_op(index: usize, op: &Op, err: YqrError) -> YqrError {
    let name = op.name();
    match err {
        YqrError::Eval(m) => YqrError::Eval(format!("op {index} ({name}): {m}")),
        YqrError::Io(m) => YqrError::Io(format!("op {index} ({name}): {m}")),
        other => other,
    }
}

/// Apply one op to one document state.
fn apply_one(op: &Op, input: &str) -> Result<String> {
    match op {
        Op::Set { path, value } => set(path, value, input),
        Op::Remove { path } => {
            if input.trim().is_empty() {
                return Ok(input.to_string());
            }
            write::apply(
                &Mutation::Delete {
                    target: Target::Value(path.to_ast()?),
                },
                input,
            )
        }
        Op::Rename { path, to } => {
            if input.trim().is_empty() {
                return Ok(input.to_string());
            }
            write::apply(
                &Mutation::Assign {
                    target: Target::Key(path.to_ast()?),
                    rhs: Rhs::Literal(Value::String(to.clone())),
                },
                input,
            )
        }
        Op::Append { path, value } => write::apply(
            &Mutation::Append {
                path: path.to_ast()?,
                rhs: Rhs::Literal(value.clone()),
            },
            input,
        ),
        Op::SetHeadComment { path, text } => write::apply(
            &Mutation::Assign {
                target: Target::HeadComment(path.to_ast()?),
                rhs: Rhs::Literal(Value::String(text.clone())),
            },
            input,
        ),
    }
}

/// The set op: one guarded assignment, with the target and value adjusted
/// so missing mapping levels come into being (see the module doc).
///
/// Each set costs one extra parse of the current document state (the
/// resolution walk) on top of the write tier's own. That is the price of
/// deciding creation *before* mutating; sharing the parse would need a
/// write-tier entry point that takes a parsed stream, a later
/// optimization if change-set sizes ever make it matter.
fn set(path: &Path, value: &Value, input: &str) -> Result<String> {
    if path.segments().is_empty() {
        return Err(path.to_ast().expect_err("the empty path is refused"));
    }
    let (root, document_count) = load_root(input)?;
    // No documents at all — empty, whitespace, or comments only: there is
    // nothing for the write tier to edit, so the document is created, with
    // whatever bytes were there (a leading comment) kept in front.
    let Some(root) = root else {
        return create_document(path, value, input);
    };
    let (target, nested) = resolve_set(path, value, &root)?;
    // Parent creation resolves against the first document, but the write
    // tier applies a mutation to every document of a stream; a shortened
    // target with a nested value could then overwrite a mapping that
    // exists in a later document, silently dropping its other keys.
    if target.segments().len() < path.segments().len() && document_count > 1 {
        return Err(YqrError::eval(format!(
            "cannot set '{}': the stream has {document_count} documents and \
             creating missing levels resolves against the first one only; \
             apply the change set to each document separately",
            path.display(),
        )));
    }
    // The one policy difference from the filter language: a collection
    // written over a scalar performs the guarded re-shape instead of
    // refusing. Feature f051.
    write::apply_with(
        &Mutation::Assign {
            target: Target::Value(target.to_ast()?),
            rhs: Rhs::Literal(nested),
        },
        input,
        write::WritePolicy {
            reshape_scalar_to_collection: true,
        },
    )
}

/// Name a path prefix in an error message.
fn prefix_name(path: &Path, depth: usize) -> String {
    if depth == 0 {
        "the document root".to_string()
    } else {
        format!("'{}'", path.prefix(depth).display())
    }
}

/// Decide what one assignment writes: the full path when it, or everything
/// but its final key, already exists; otherwise the prefix ending at the
/// first missing key, with the remaining segments nested as mappings
/// around `value`.
fn resolve_set(path: &Path, value: &Value, root: &Value) -> Result<(Path, Value)> {
    let segments = path.segments();
    let mut node = root;
    for (depth, segment) in segments.iter().enumerate() {
        let last = depth + 1 == segments.len();
        match (node, segment) {
            (Value::Mapping(map), Segment::Key(key)) => match node_get(map, key) {
                Some(child) => node = child,
                // The final segment is missing: the write tier's ordinary
                // new-key creation handles it on the full path.
                None if last => break,
                // An intermediate mapping level is missing: create mapping
                // levels from here down. A sequence level cannot be
                // conjured — there is no "empty sequence with an element
                // at index n" — so an index anywhere below refuses.
                None => {
                    if let Some(bad) = segments[depth + 1..].iter().find_map(|s| match s {
                        Segment::Index(index) => Some(*index),
                        Segment::Key(_) => None,
                    }) {
                        return Err(YqrError::eval(format!(
                            "cannot set '{}': creating the missing levels under {} \
                             stops at index [{bad}] — only mapping levels are created",
                            path.display(),
                            prefix_name(path, depth),
                        )));
                    }
                    let target = path.prefix(depth + 1);
                    let nested = nest(&segments[depth + 1..], value.clone());
                    return Ok((target, nested));
                }
            },
            (Value::Sequence(items), Segment::Index(index)) => match seq_get(items, *index) {
                Some(child) => node = child,
                // The sequence exists; the index does not. On the final
                // segment the write tier owns the verdict; mid-path there
                // is nothing to descend into and nothing to create.
                None if last => break,
                None => {
                    return Err(YqrError::eval(format!(
                        "cannot set '{}': the sequence at {} has no index [{index}] \
                         (len {})",
                        path.display(),
                        prefix_name(path, depth),
                        items.len(),
                    )));
                }
            },
            // A scalar (or a container of the other kind) where the path
            // needs to descend further. The final-entry re-shape (f051)
            // replaces a value the caller names in full; writing *past* a
            // scalar would discard it on the way to somewhere else, so the
            // mid-path crossing stays refused.
            _ if !last => {
                return Err(YqrError::eval(format!(
                    "cannot set '{}': {} does not hold a {} to descend into; \
                     growing it mid-path is a re-shape this API refuses",
                    path.display(),
                    prefix_name(path, depth),
                    match segment {
                        Segment::Key(_) => "mapping",
                        Segment::Index(_) => "sequence",
                    },
                )));
            }
            // A mismatched final step: the write tier's own refusal says
            // what is wrong.
            _ => break,
        }
    }
    Ok((path.clone(), value.clone()))
}

/// Wrap `value` in one mapping level per remaining key segment.
fn nest(segments: &[Segment], value: Value) -> Value {
    segments.iter().rev().fold(value, |inner, segment| {
        let Segment::Key(key) = segment else {
            unreachable!("index segments are refused before nesting");
        };
        let mut map = Mapping::new();
        map.insert(Value::String(key.clone()), inner);
        Value::Mapping(map)
    })
}

/// Create the document a set against contentless input produces: the
/// nested value, emitted in block style, after whatever bytes were there
/// (a leading comment survives). Only reached when the input parses to no
/// documents at all.
fn create_document(path: &Path, value: &Value, input: &str) -> Result<String> {
    if let Some(Segment::Index(index)) = path
        .segments()
        .iter()
        .find(|s| matches!(s, Segment::Index(_)))
    {
        return Err(YqrError::eval(format!(
            "cannot set '{}' in an empty document: a level is addressed by \
             index [{index}], and only mapping levels are created",
            path.display(),
        )));
    }
    let document = nest(path.segments(), value.clone());
    let rendered = crate::render(std::slice::from_ref(&document), false)?;
    if input.is_empty() {
        return Ok(rendered);
    }
    let separator = if input.ends_with('\n') { "" } else { "\n" };
    Ok(format!("{input}{separator}{rendered}"))
}

/// Mapping lookup by string key.
fn node_get<'v>(map: &'v Mapping, key: &str) -> Option<&'v Value> {
    map.get(&Value::String(key.to_string()))
}

/// Sequence lookup with yqr's negative-index convention.
fn seq_get(items: &[Value], index: i64) -> Option<&Value> {
    let resolved = if index < 0 {
        items
            .len()
            .checked_sub(index.unsigned_abs().try_into().ok()?)?
    } else {
        usize::try_from(index).ok()?
    };
    items.get(resolved)
}

/// Parse `input` into the value view the missing-prefix resolution walks:
/// the first document (`None` when the stream holds no documents — empty,
/// whitespace, or comments only) and how many documents the stream holds.
fn load_root(input: &str) -> Result<(Option<Value>, usize)> {
    let config = crate::parser_config();
    let docs = ::noyalib::document::load_all_with_config(input, &config)
        .map_err(|e| YqrError::io(format!("failed to parse YAML input: {e}")))?;
    let mut first = None;
    let mut count = 0usize;
    for doc in docs {
        let doc = doc.map_err(|e| YqrError::io(format!("failed to parse YAML input: {e}")))?;
        if count == 0 {
            first = Some(Value::from(doc));
        }
        count += 1;
    }
    Ok((first, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(path: Path, value: Value) -> Op {
        Op::Set { path, value }
    }

    #[test]
    fn an_empty_change_set_is_byte_identical() {
        for input in [
            "# comment\ntitle: 'Hello'   # trailing\n\ntags: [a, b]\n",
            "title: CRLF\r\n",
            "",
        ] {
            assert_eq!(apply_all(&[], input).unwrap(), input);
        }
    }

    #[test]
    fn the_first_refusal_names_the_op_index() {
        let ops = [
            set(Path::root().key("a"), Value::Int(1)),
            Op::Append {
                path: Path::root().key("a"),
                value: Value::Int(2),
            },
        ];
        let err = apply_all(&ops, "a: 0\n").unwrap_err();
        let message = err.to_string();
        assert!(message.contains("op 1 (append)"), "{message}");
    }

    #[test]
    fn the_empty_path_is_refused() {
        let err = apply_all(&[set(Path::root(), Value::Int(1))], "a: 0\n").unwrap_err();
        assert!(err.to_string().contains("empty path"), "{err}");
    }

    #[test]
    fn a_missing_mapping_level_nests_the_value() {
        let (root, _) = load_root("title: x\n").unwrap();
        let (target, nested) = resolve_set(
            &Path::root().key("menu").key("nav").key("visible"),
            &Value::Bool(false),
            &root.unwrap(),
        )
        .unwrap();
        assert_eq!(target, Path::root().key("menu"));
        let expected = {
            let mut inner = Mapping::new();
            inner.insert(Value::String("visible".into()), Value::Bool(false));
            let mut outer = Mapping::new();
            outer.insert(Value::String("nav".into()), Value::Mapping(inner));
            Value::Mapping(outer)
        };
        assert_eq!(nested, expected);
    }

    #[test]
    fn a_missing_level_addressed_by_index_is_refused() {
        let err = apply_all(
            &[set(Path::root().key("items").index(0), Value::Int(1))],
            "title: x\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("[0]"), "{err}");
    }

    #[test]
    fn a_scalar_in_the_way_is_a_refused_reshape() {
        // Mid-path, the typed layer names the re-shape; on the final
        // segment the write tier's own refusal already says what is wrong.
        let err = apply_all(
            &[set(
                Path::root().key("title").key("deep").key("deeper"),
                Value::Int(1),
            )],
            "title: x\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("re-shape"), "{err}");

        let err = apply_all(
            &[set(Path::root().key("title").key("deep"), Value::Int(1))],
            "title: x\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("cannot index string"), "{err}");
    }

    #[test]
    fn parent_creation_on_a_multi_document_stream_refuses() {
        // doc1 lacks `menu`, doc2 has one with other keys: a shortened
        // assign with a nested value would clobber doc2's siblings, so the
        // op refuses rather than resolving against doc1 alone.
        let input = "title: x\n---\nmenu:\n  nav:\n    a: 1\n  other: 2\n";
        let err = apply_all(
            &[set(
                Path::root().key("menu").key("nav").key("visible"),
                Value::Bool(false),
            )],
            input,
        )
        .unwrap_err();
        assert!(err.to_string().contains("2 documents"), "{err}");

        // A full-path set that exists everywhere it applies stays fine.
        let out = apply_all(&[set(Path::root().key("title"), Value::Int(1))], input).unwrap();
        assert!(out.contains("title: 1"), "{out}");
        assert!(out.contains("other: 2"), "{out}");
    }

    // -- Feature f051: the scalar-to-collection re-shape ------------------

    #[test]
    fn a_collection_over_a_scalar_reshapes_the_entry_in_place() {
        let out = apply_all(
            &[set(
                Path::root().key("tags"),
                Value::Sequence(vec![Value::String("a".into()), Value::String("b".into())]),
            )],
            "tags: solo\nname: x\n",
        )
        .unwrap();
        assert_eq!(out, "tags:\n  - a\n  - b\nname: x\n");
    }

    #[test]
    fn the_reshape_keeps_a_trailing_comment_on_the_keys_line() {
        let out = apply_all(
            &[set(
                Path::root().key("tags"),
                Value::Sequence(vec![Value::String("a".into())]),
            )],
            "tags: solo # keep\n",
        )
        .unwrap();
        assert_eq!(out, "tags: # keep\n  - a\n");
    }

    #[test]
    fn a_mapping_value_reshapes_and_loads_back() {
        let mut inner = Mapping::new();
        inner.insert(Value::String("x".into()), Value::Int(1));
        inner.insert(Value::String("y".into()), Value::String("yes".into()));
        let value = Value::Mapping(inner);
        let out = apply_all(
            &[set(Path::root().key("m"), value.clone())],
            "m: old\nafter: 1\n",
        )
        .unwrap();
        assert!(out.ends_with("after: 1\n"), "{out}");
        assert_eq!(crate::eval_str(".m", &out).unwrap(), vec![value]);
    }

    #[test]
    fn a_collection_over_a_sequence_item_still_refuses() {
        let err = apply_all(
            &[set(
                Path::root().key("xs").index(0),
                Value::Sequence(vec![Value::Int(1)]),
            )],
            "xs:\n  - solo\n",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("writes a collection only where"),
            "{err}"
        );
    }

    #[test]
    fn a_set_on_a_comments_only_document_keeps_the_comment() {
        let out = apply_all(
            &[set(Path::root().key("title"), Value::String("New".into()))],
            "# generated\n",
        )
        .unwrap();
        assert_eq!(out, "# generated\ntitle: New\n");
    }

    #[test]
    fn an_out_of_range_sequence_index_mid_path_names_the_range() {
        let err = apply_all(
            &[set(
                Path::root().key("items").index(5).key("name"),
                Value::Int(1),
            )],
            "items:\n  - a\n",
        )
        .unwrap_err();
        let message = err.to_string();
        assert!(message.contains("no index [5]"), "{message}");
        assert!(message.contains("len 1"), "{message}");
    }

    /// The serialized op shape is an interface; this test pins it.
    #[cfg(feature = "serde")]
    #[test]
    fn ops_round_trip_through_json() {
        let ops = vec![
            set(
                Path::root().key("spec").index(-1).key("image"),
                Value::String("nginx".into()),
            ),
            Op::Rename {
                path: Path::root().key("lead"),
                to: "summary".into(),
            },
        ];
        let json = serde_json::to_string(&ops).unwrap();
        assert_eq!(
            json,
            r#"[{"Set":{"path":[{"Key":"spec"},{"Index":-1},{"Key":"image"}],"value":"nginx"}},{"Rename":{"path":[{"Key":"lead"}],"to":"summary"}}]"#
        );
        let back: Vec<Op> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ops);
    }
}
