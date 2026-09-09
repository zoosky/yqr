//! What a write refuses, and what it silently skips.
//!
//! Three kinds of decision, grouped because they answer one question — should
//! these bytes change at all? — rather than because one type owns them:
//!
//! - the **no-op guards**, which skip a write that would re-spell a value or a
//!   comment without changing it (`yqr-b018`, `yqr-b019`);
//! - the **type-change refusals**, which decline the two shapes the engine has
//!   no correct route for, each naming a remedy that was measured running
//!   (`yqr-f025`, `yqr-f032`, `yqr-b029`);
//! - the **post-write integrity check**, which compares the document against
//!   itself on two properties no YAML parser complains about, and so catches
//!   what every re-parse guard is blind to (`yqr-b029`, `yqr-b030`).
//!
//! The last two need the document, so they are `NoyalibWriter` methods. They
//! live here anyway: the pre-check gives a refusal its wording and the
//! post-check is the net behind it, and separating them by receiver is what
//! made them hard to follow.

// Features f006, f032; bugs b018, b019, b024, b029, b030. Moved here by f033.

use super::{NoyalibWriter, rebased, to_noyalib_path, type_name};
use crate::Value;
use crate::error::{Result, YqrError};
use crate::fidelity::write::seam::{CommentKind, FidelityWriter};
use crate::fidelity::{Path, PathSeg};

/// Append `item` to the sequence at `target`, refusing in yqr's own words when
/// it is not one.
///
/// **The engine's refusals here are not fit to show a user.** Every one of them
/// arrives as a *parse* error over a document that parsed fine, which sends the
/// reader to look at their file instead of their filter, and two of the three
/// name an internal: `swap_items` for a target that is not a sequence (a
/// primitive from the *reorder* path, shown to someone who asked to append) and
/// `push_back_value` for an empty one, whose advice is to *"use `set` with a
/// fragment"* — an API yqr does not expose. `set_value` already carries a guard
/// for exactly this reason; the append path never got one.
///
/// So yqr checks its own preconditions against the typed model first: the
/// target must be a sequence, and it must have an item to anchor indentation
/// on. What is left that can still fail is genuinely the engine's business —
/// flow layout — and its wording there is accurate and names nothing internal.
///
/// Each refusal names a remedy that **works**, which is `yqr-f025`'s rule, and
/// the remedy differs by what is actually there: `|=` computes a new value in
/// place of a scalar, and a mapping grows by assignment, not by append.
// Bug b024.
pub(super) fn append_item(
    writer: &mut dyn FidelityWriter,
    doc: usize,
    target: &Path,
    current: &Value,
    item: &Value,
) -> Result<()> {
    let path_str = to_noyalib_path(target);
    let remedy = |v: &Value| match v {
        // Only the numeric arm carries an arithmetic example, because only
        // there does one work: `(. + 1)` over a string is a type error, and a
        // remedy that errors is worse than no remedy (`yqr-f025`).
        Value::Int(_) | Value::Float(_) => {
            format!("Use `|=` to compute a new value in place, as in `.{path_str} |= (. + 1)`")
        }
        // An implicit null has no value to compute from, so `|=` is not the
        // way in; `=` writes one (`yqr-b021`).
        Value::Null => format!("Use `=` to write a value there, as in `.{path_str} = 1`"),
        _ => "Use `|=` to compute a new value in place".to_string(),
    };
    match current {
        Value::Sequence(items) if !items.is_empty() => writer.append(doc, target, item),
        Value::Sequence(_) => Err(YqrError::eval(format!(
            "cannot append at {path_str:?}: the sequence is empty, and yqr places a new item by \
             following the indentation of the one above it. Write the first item into the file \
             yourself, and `+=` will extend it"
        ))),
        Value::Mapping(_) => Err(YqrError::eval(format!(
            "cannot append at {path_str:?}: `+=` appends an item to a sequence, and this is a \
             mapping. Assign the entry you want instead, as in `.{path_str}.<key> = <value>`"
        ))),
        scalar => Err(YqrError::eval(format!(
            "cannot append at {path_str:?}: `+=` appends an item to a sequence, and this is \
             {}. {}",
            type_name(scalar),
            remedy(scalar)
        ))),
    }
}

/// Write `new` at `path`, unless it is already what is there.
///
/// **A write that changes nothing must not rewrite anything.** `set_value`
/// re-emits a scalar from the typed model, and the model cannot carry a
/// number's spelling — so writing `0640` back as the same `Int` emits `640`,
/// and a `1.10` version pin comes back `1.1`. On an edit that changed nothing,
/// that is `yqr-a001` §1's own counter-example: *yqr never rewrites bytes it
/// did not change*.
///
/// The comparison is the typed model's, and that is **why** it works rather
/// than a limitation to work around: the model cannot tell `0640` from `640`,
/// so equal-by-value is exactly the set of cases where re-emitting would lose
/// a spelling. Where the model *can* tell two values apart, they are not
/// equal and the write proceeds.
///
/// A skip is a **success**. `.n = .n` is a no-op, not a refusal, matching the
/// absent-path rule.
///
/// That premise fails where the value is **borrowed** — an alias reference, or
/// an entry a `<<` merge produced. There the model compares equal to a literal
/// the entry only points at, so skipping would swallow a write that had real
/// work to do: `.b = 1` over `b: *x` leaves a reference behind, and a later
/// edit to the anchor moves `b` with it. So the borrowed check runs *first*
/// and a borrowed site falls through to the writer, which refuses it in its
/// own words rather than in a copy of them kept here.
///
/// Shared by `=` and `|=`, which reach it by different resolvers and would
/// otherwise carry a copy of this rule each — the shape that let the `=` half
/// go unguarded while `|=` was fixed (`yqr-b018`).
// Feature f006 / f008; bugs b018, b019.
pub(super) fn set_value_unless_unchanged(
    writer: &mut dyn FidelityWriter,
    doc: usize,
    path: &Path,
    new: &Value,
    current: &Value,
) -> Result<()> {
    if new == current && !writer.value_is_borrowed(doc, path)? {
        return Ok(());
    }
    refuse_scalar_to_collection(path, new, current)?;
    writer.set_value(doc, path, new)
}

/// Refuse replacing a scalar with a collection, in yqr's words.
///
/// The engine writes a collection over a collection and a scalar over a
/// scalar; it has no typed route from one to the other, and says so by naming
/// `set` and "fragment", an API yqr does not expose.
///
/// Checked here rather than in the backend because this is the one caller
/// holding both the current value and the new one, and both are needed to
/// name the shape that is wrong.
///
/// The remedy differs by shape, and each is one this refusal was measured
/// running — `yqr-f025`'s rule, which a single remedy sentence broke in two
/// ways. Removing the entry and assigning it again works for a mapping entry,
/// because a *new key* is the insertion path and that one spells a collection.
/// It does **not** work for a sequence item: `del` shifts the items up, so the
/// same path then names the next one and the refusal repeats. A sequence takes
/// a collection through `+=` instead.
///
/// An entry left empty was a third case until `yqr-b031` taught `del` to
/// remove one; the mapping remedy covers it now.
// Feature f032; the per-shape remedies are the code review of that feature.
fn refuse_scalar_to_collection(path: &Path, new: &Value, current: &Value) -> Result<()> {
    let new_is_collection = matches!(new, Value::Sequence(_) | Value::Mapping(_));
    let current_is_collection = matches!(current, Value::Sequence(_) | Value::Mapping(_));
    if !new_is_collection || current_is_collection {
        return Ok(());
    }
    let path_str = to_noyalib_path(path);
    let remedy = match path.segments().last() {
        Some(PathSeg::Index(_)) => {
            let seq = to_noyalib_path(&rebased(path, None));
            format!(
                "There is no in-place route for a sequence item: removing it shifts the rest up, \
                 so the same index then names the next one. `+=` is a different edit and it does \
                 work, appending to the end: `.{seq} += <path>`"
            )
        }
        // The null arm this used to carry is gone: it hedged only because
        // `del` refused an entry left empty, which `yqr-b031` fixed.
        _ => format!(
            "Remove the entry and write it again, as in `del(.{path_str})` then \
             `.{path_str} = <path>`, which places it at the end of its mapping"
        ),
    };
    Err(YqrError::eval(format!(
        "cannot assign at {path_str:?}: the value there is {}, and yqr writes a collection \
         only where one already is. {remedy}",
        type_name(current)
    )))
}

/// Write the `kind` comment at `path`, unless it already says that.
///
/// The value guard's rule, on the other thing a write can re-spell. `#tight`
/// and `# tight` carry the same body, and the body is all a comment mutation
/// is given, so `set_comment` re-emits the canonical spacing and a write that
/// changed no content rewrites the line — `yqr-a001` §1 again, on a comment
/// instead of a scalar.
///
/// The comparison is the read path's spelling of the body, which is what makes
/// reading a comment and writing it straight back a no-op.
// Feature f007; bug b019.
pub(super) fn set_comment_unless_unchanged(
    writer: &mut dyn FidelityWriter,
    doc: usize,
    path: &Path,
    kind: CommentKind,
    text: &str,
) -> Result<()> {
    if writer.current_comment(doc, path, kind)?.as_deref() == Some(text) {
        return Ok(());
    }
    writer.set_comment(doc, path, kind, text)
}

/// [`FidelityWriter`] backed by noyalib's editable `cst::Document` stream.
/// The structural facts [`NoyalibWriter::check_integrity`] compares on either
/// side of a write. Counts rather than positions: a write is refused when it
/// *adds* a violation, so a document that already had one stays editable.
// Bugs b029, b030.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Integrity {
    /// Block mapping values sitting at or before their key's column (`Y103`).
    under_indented: usize,
    /// `\r\n` pairs.
    crlf: usize,
    /// Line feeds with no carriage return before them.
    bare_lf: usize,
}

/// `bytes` with any leading `&anchor` and `!tag` properties removed.
///
/// A node's properties precede its value, so a flow collection carrying one
/// does not start with `[` or `{` — and reading it as a block collection is
/// how the `yqr-b029` guard came to refuse `.k = 5` over `k: &an {a: 1}`, an
/// edit that works and that `write::anchor` has its own accurate message for.
/// Both spellings are skipped, in either order, since a node may carry both.
///
/// A property ends at whitespace **or** at a flow indicator, because YAML
/// forbids those characters in an anchor name or a tag and so does not
/// require a space between the two: `k: &an{a: 1}` is an anchored flow
/// mapping, which noyalib parses and the first version of this function
/// stopped short of. Splitting on whitespace alone left `1}` behind, which
/// kept the answer on the refusing side and was still the wrong answer.
// Bug b029, found in code review; the no-space spelling in the round after.
fn after_properties(bytes: &str) -> &str {
    let mut rest = bytes.trim_start();
    while rest.starts_with(['&', '!']) {
        let end = rest
            .find(|c: char| c.is_whitespace() || "[]{},".contains(c))
            .unwrap_or(rest.len());
        // A property with nothing after it is the whole slice; stop rather
        // than loop on an unchanged `rest`.
        if end == 0 {
            break;
        }
        rest = rest[end..].trim_start();
    }
    rest
}

/// Line feeds in `source` that no carriage return precedes.
// Bug b030.
fn bare_line_feeds(source: &str) -> usize {
    let bytes = source.as_bytes();
    bytes
        .iter()
        .enumerate()
        .filter(|(i, b)| **b == b'\n' && (*i == 0 || bytes[i - 1] != b'\r'))
        .count()
}

impl NoyalibWriter {
    pub(super) fn refuse_block_collection_to_scalar(
        &self,
        doc: usize,
        path: &Path,
        path_str: &str,
        new: &Value,
    ) -> Result<()> {
        if matches!(new, Value::Sequence(_) | Value::Mapping(_)) {
            return Ok(());
        }
        if !matches!(path.segments().last(), Some(PathSeg::Key(_))) {
            return Ok(());
        }
        let current = self.value(doc)?;
        if !matches!(
            crate::fidelity::noyalib::walk_value(&current, path.segments()),
            Some(Value::Sequence(_) | Value::Mapping(_))
        ) {
            return Ok(());
        }
        let d = self.doc_ref(doc)?;
        if d.get(path_str)
            .is_some_and(|bytes| after_properties(bytes).starts_with(['[', '{']))
        {
            return Ok(());
        }
        Err(YqrError::eval(format!(
            "cannot assign at {path_str:?}: the value there is a block collection, and writing a \
             scalar over one would leave it at its key's own column, which this engine reads \
             back but other YAML parsers reject. Remove the entry and write it again, as in \
             `del(.{path_str})` then `.{path_str} = <value>`, which places it at the end of its \
             mapping"
        )))
    }

    pub(super) fn integrity(&self, doc: usize) -> Result<Integrity> {
        let d = self.doc_ref(doc)?;
        Ok(Integrity {
            under_indented: crate::validate::scan::under_indented_values(d, 0).len(),
            crlf: d.source().matches("\r\n").count(),
            bare_lf: bare_line_feeds(d.source()),
        })
    }

    pub(super) fn check_integrity(
        &self,
        doc: usize,
        before: Integrity,
        path_str: &str,
    ) -> Result<()> {
        let after = self.integrity(doc)?;
        if after.under_indented > before.under_indented {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: the result would leave a block mapping's value at \
                 its key's own column, which this engine reads back but other YAML parsers \
                 reject. Replacing a block collection with a scalar is what does this; remove the \
                 entry and write it again, as in `del(.{path_str})` then `.{path_str} = <value>`, \
                 which places it at the end of its mapping"
            )));
        }
        // Only a document that is wholly CRLF is held to it. One already
        // mixing the two is not made worse by this rule, and an all-LF
        // document gains a bare line feed per added line, legitimately.
        if before.crlf > 0 && before.bare_lf == 0 && after.bare_lf > 0 {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: this file's lines end with CRLF, and the \
                 replacement's own lines would end with LF, leaving the file with mixed line \
                 endings. yqr refuses rather than change bytes the edit does not name"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::apply;
    use super::super::testutil::*;
    use super::after_properties;
    use crate::Value;
    use crate::ast::{Mutation, Rhs, Target};

    // -- Feature f032: collection right-hand sides ------------------------

    #[test]
    fn a_collection_becomes_a_new_key() {
        let out = assign_path(".m.new", ".src", "m:\n  a: 1\nsrc:\n  x: 1\n  y: two\n").unwrap();
        assert_eq!(
            out,
            "m:\n  a: 1\n  new:\n    x: 1\n    y: two\nsrc:\n  x: 1\n  y: two\n"
        );
    }

    #[test]
    fn a_sequence_becomes_a_new_key() {
        let out = assign_path(".m.new", ".src", "m:\n  a: 1\nsrc:\n  - one\n  - two\n").unwrap();
        assert_eq!(
            out,
            "m:\n  a: 1\n  new:\n    - one\n    - two\nsrc:\n  - one\n  - two\n"
        );
    }

    #[test]
    fn a_collection_is_appended_as_a_sequence_item() {
        let out = apply(
            &Mutation::Append {
                path: crate::parser::parse(".xs").expect("valid"),
                rhs: Rhs::Path(crate::parser::parse(".src").expect("valid")),
            },
            "xs:\n  - a: 1\nsrc:\n  a: 2\n  b: 3\n",
        )
        .unwrap();
        assert_eq!(
            out,
            "xs:\n  - a: 1\n  - a: 2\n    b: 3\nsrc:\n  a: 2\n  b: 3\n"
        );
    }

    #[test]
    fn a_collection_replaces_a_collection_at_the_sites_own_indent() {
        let out = assign_path(
            ".top.k",
            ".src",
            "top:\n  k:\n    old: 1\n  after: keep   # tail\nsrc:\n  new: 2\n",
        )
        .unwrap();
        assert_eq!(
            out,
            "top:\n  k:\n    new: 2\n  after: keep   # tail\nsrc:\n  new: 2\n"
        );
    }

    #[test]
    fn writing_a_collection_over_an_equal_one_is_a_no_op() {
        // The no-op guard compares values, so it holds for a collection the
        // same way it holds for a scalar: nothing is re-spelled.
        let input = "k:\n  a: 1      # kept\n  b: 'two'\n";
        assert_eq!(assign_path(".k", ".k", input).unwrap(), input);
    }

    #[test]
    fn a_collection_over_a_scalar_names_a_remedy_that_works() {
        let input = "c: 1\nsrc:\n  a: 1\n";
        let err = assign_path(".c", ".src", input).unwrap_err();
        let text = format!("{err}");
        assert!(
            text.contains("yqr writes a collection only where one already is"),
            "got: {text}"
        );
        assert!(text.contains("del(.c)"), "no remedy named: {text}");
        // The remedy has to work (`yqr-f025`).
        let removed = apply(
            &Mutation::Delete {
                target: Target::Value(crate::parser::parse(".c").expect("valid")),
            },
            input,
        )
        .unwrap();
        assert_eq!(
            assign_path(".c", ".src", &removed).unwrap(),
            "src:\n  a: 1\nc:\n  a: 1\n"
        );
    }

    // -- Bug b029: a scalar must not be written over a block collection -----

    #[test]
    fn a_scalar_over_a_block_collection_is_refused() {
        // Upstream writes the scalar at column 0 (`k:` then `5`), which its
        // own parser reads back and PyYAML and Psych reject, so neither the
        // re-parse guard nor upstream's own guard sees it.
        for (path, input) in [
            (".k", "k:\n  a: 1\nafter: 1\n"),
            (".k", "k:\n  - 1\n  - 2\nafter: 1\n"),
            (".top.k", "top:\n  k:\n    a: 1\n  after: 2\n"),
        ] {
            let err = apply(&assign(path, Rhs::Literal(Value::Int(5))), input).unwrap_err();
            assert!(
                format!("{err}").contains("at its key's own column"),
                "{input:?} was not refused in yqr's words: {err}"
            );
        }
    }

    #[test]
    fn a_scalar_over_a_flow_collection_or_a_sequence_item_still_writes() {
        // The guard is the Y103 property, not a ban on the type change: a
        // flow value has no line of its own to under-indent, and a sequence
        // item is not a mapping entry.
        assert_eq!(
            apply(
                &assign(".k", Rhs::Literal(Value::Int(5))),
                "k: {a: 1}\nafter: 1\n"
            )
            .unwrap(),
            "k: 5\nafter: 1\n"
        );
        assert_eq!(
            apply(
                &assign(".xs[0]", Rhs::Literal(Value::Int(5))),
                "xs:\n  - a: 1\n  - b: 2\n"
            )
            .unwrap(),
            "xs:\n  - 5\n  - b: 2\n"
        );
    }

    #[test]
    fn an_absent_right_hand_path_does_not_flatten_a_block() {
        // `.k = .missing` resolves to null, a scalar, so this is the b029
        // shape reached without naming a scalar at all.
        let err = assign_path(".k", ".missing", "k:\n  a: 1\n").unwrap_err();
        assert!(format!("{err}").contains("block collection"), "{err}");
    }

    // Found in code review of f032: a flow collection carrying a property
    // does not start with `[`, so the b029 guard read it as a block one and
    // refused an edit that works.
    #[test]
    fn a_scalar_over_a_flow_collection_with_a_property_still_writes() {
        assert_eq!(
            apply(
                &assign(".k", Rhs::Literal(Value::Int(5))),
                "k: &an {a: 1}\nafter: 1\n"
            )
            .unwrap(),
            "k: &an 5\nafter: 1\n"
        );
        // A tag is refused, but by the guard that knows why: replacing the
        // scalar under a tag can change what the tag makes of it.
        let err = apply(
            &assign(".k", Rhs::Literal(Value::Int(5))),
            "k: !!map {a: 1}\nafter: 1\n",
        )
        .unwrap_err();
        let text = format!("{err}");
        assert!(text.contains("carries the tag"), "{text}");
        assert!(
            !text.contains("block collection"),
            "wrong diagnosis: {text}"
        );
    }

    // Review round two: YAML forbids a flow indicator in an anchor name, so a
    // property need not be followed by a space. Splitting on whitespace alone
    // left `1}` behind and the value read as a block collection again.
    #[test]
    fn a_property_with_no_space_before_a_flow_collection_is_not_a_block() {
        let err = apply(
            &assign(".k", Rhs::Literal(Value::Int(5))),
            "k: &an{a: 1}\nafter: 1\n",
        )
        .unwrap_err();
        let text = format!("{err}");
        // Still refused — the anchor writer cannot splice a scalar where the
        // property runs into the value, and its own guard catches that. What
        // this pins is the *diagnosis*: not the block-collection one.
        assert!(
            !text.contains("block collection"),
            "wrong diagnosis: {text}"
        );
        assert!(text.contains("does not re-parse"), "{text}");
    }

    #[test]
    fn after_properties_stops_at_a_flow_indicator() {
        assert_eq!(after_properties("&an {a: 1}"), "{a: 1}");
        assert_eq!(after_properties("&an{a: 1}"), "{a: 1}");
        assert_eq!(after_properties("!!map {a: 1}"), "{a: 1}");
        assert_eq!(after_properties("&an !!map {a: 1}"), "{a: 1}");
        assert_eq!(after_properties("&an [1, 2]"), "[1, 2]");
        assert_eq!(after_properties("a: 1"), "a: 1");
        assert_eq!(after_properties("&an"), "");
    }

    #[test]
    fn an_anchored_block_collection_is_still_refused() {
        // The property skip must not turn the guard off for a block value.
        let err = apply(
            &assign(".k", Rhs::Literal(Value::Int(5))),
            "k: &an\n  a: 1\nafter: 1\n",
        )
        .unwrap_err();
        assert!(format!("{err}").contains("block collection"), "{err}");
    }

    // Each refusal names the remedy for its own shape, and the remedy is run
    // here rather than asserted to exist (`yqr-f025`). One sentence for all
    // three was wrong for two of them.
    #[test]
    fn the_collection_over_a_scalar_remedy_fits_the_shape() {
        // A sequence item: `del` would shift the items up, so the remedy is
        // `+=`, which appends.
        let seq = "xs:\n  - 1\n  - 2\nsrc:\n  a: 1\n";
        let err = assign_path(".xs[0]", ".src", seq).unwrap_err();
        let text = format!("{err}");
        assert!(text.contains("`.xs += <path>`"), "{text}");
        assert!(
            !text.contains("end of its mapping"),
            "no mapping here: {text}"
        );
        assert_eq!(
            apply(
                &Mutation::Append {
                    path: crate::parser::parse(".xs").expect("valid"),
                    rhs: Rhs::Path(crate::parser::parse(".src").expect("valid")),
                },
                seq,
            )
            .unwrap(),
            "xs:\n  - 1\n  - 2\n  - a: 1\nsrc:\n  a: 1\n"
        );

        // A mapping entry, including one left empty: `del` then assign. That
        // second shape needed a hedge until `yqr-b031` taught `del` to remove
        // an entry with no value of its own; it takes the ordinary remedy now,
        // and this runs it.
        for input in ["c: 1\nsrc:\n  a: 1\n", "c:\nsrc:\n  a: 1\n"] {
            let err = assign_path(".c", ".src", input).unwrap_err();
            let text = format!("{err}");
            assert!(text.contains("`del(.c)`"), "{text}");
            let removed = apply(
                &Mutation::Delete {
                    target: Target::Value(crate::parser::parse(".c").expect("valid")),
                },
                input,
            )
            .unwrap();
            assert_eq!(
                assign_path(".c", ".src", &removed).unwrap(),
                "src:\n  a: 1\nc:\n  a: 1\n",
                "the remedy did not work for {input:?}"
            );
        }
    }

    // -- Bug b030: a write must not give a CRLF file mixed line endings -----

    #[test]
    fn a_multi_line_write_into_a_crlf_document_is_refused() {
        // The engine joins the replacement's own lines with LF while the
        // file's are CRLF. Shipped since the multi-line string write existed;
        // the collection arm would have inherited it.
        let err = apply(
            &assign(".a", Rhs::Literal(Value::String("one\ntwo".into()))),
            "a: 1\r\nb: 2\r\n",
        )
        .unwrap_err();
        assert!(format!("{err}").contains("mixed line endings"), "{err}");
    }

    #[test]
    fn the_crlf_guard_leaves_the_paths_that_are_correct_alone() {
        // The insertion mutators derive the terminator from the document
        // (`yqr-b009`, fixed upstream), so they are untouched by the guard —
        // and a single-line write never trips it.
        assert_eq!(
            assign_path(".m.new", ".src", "m:\r\n  a: 1\r\nsrc:\r\n  x: 1\r\n").unwrap(),
            "m:\r\n  a: 1\r\n  new:\r\n    x: 1\r\nsrc:\r\n  x: 1\r\n"
        );
        assert_eq!(
            apply(
                &assign(".a", Rhs::Literal(Value::Int(9))),
                "a: 1\r\nb: 2\r\n"
            )
            .unwrap(),
            "a: 9\r\nb: 2\r\n"
        );
    }

    #[test]
    fn an_all_lf_document_may_still_grow_lines() {
        // The rule keys on a document that is wholly CRLF, so an LF file
        // gaining a line per inserted entry is not a violation.
        let out = assign_path(".m.new", ".src", "m:\n  a: 1\nsrc:\n  x: 1\n  y: 2\n").unwrap();
        assert!(!out.contains('\r'), "{out:?}");
    }
}
