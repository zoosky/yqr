//! Sequence reorder: `swap(<path>; i; j)` and `move(<path>; from; to)`.
//!
//! The one edit in the write tier with no node to name — an ordering has no
//! bytes of its own — so the filter grammar spells it as a verb with arguments
//! rather than as a selector wrapping a path.
//!
//! Each verb is **one engine call**. That is only true because of what the
//! engine now does: noyalib's `swap_items` and `move_item` used to exchange the
//! two items' *value bytes* and nothing else, so every comment stayed with the
//! position and silently came to document whichever item landed in it — at
//! `Ok`, at exit 0, and past the engine's own integrity guard, which compares
//! typed values and therefore cannot observe a comment moving. yqr argued for
//! whole-entry semantics on the grounds that the same crate's `remove` already
//! treats the comment run above an entry as *the entry's*, wrote the fix, and
//! it shipped in noyalib 0.0.23. Since then an item's inline and head comments
//! travel with it, and the unit tests below pin that rather than assume it: a
//! property yqr sells and does not own is the one worth a regression test.
//!
//! What this module owns is everything around the call:
//!
//! - the **sequence length**, read from the typed view. A negative index
//!   resolves against it, so it is a precondition of addressing the items at
//!   all rather than something to learn from a refusal — and once it is in
//!   hand, forwarding the range check to the engine would produce a second
//!   message shape for the same mistake;
//! - the **index resolution**, through the same function `.[-1]` resolves
//!   through, so the two cannot drift apart;
//! - the **refusals**: an index outside the sequence, and a path naming
//!   something that is not one.
//!
//! A **flow** sequence has no per-item lines, so the engine exchanges value
//! spans there. That is the right answer rather than a gap — a flow member owns
//! no trivia to carry — and it is reordered rather than refused.

// Feature f007 (see specs/features/): write tier — structural edits.

use super::{FidelityWriter, NoyalibWriter, noyalib_path, type_name};
use crate::Value;
use crate::ast::ReorderOp;
use crate::error::{Result, YqrError};
use crate::eval::resolve_seq_index;
use crate::fidelity::Path;
use crate::fidelity::noyalib::walk_value;

impl NoyalibWriter {
    /// The length of the block sequence a reorder addresses.
    ///
    /// Read from the typed view rather than from the engine, because yqr needs
    /// it *before* the call: a negative index resolves against it, so the
    /// length is a precondition of addressing the items at all rather than
    /// something to discover from a refusal.
    // Feature f007; the index semantics are settled in yqr-a002 §4.5.
    fn reorder_len(&self, doc: usize, path: &Path, path_str: &str, op: ReorderOp) -> Result<usize> {
        let value = FidelityWriter::value(self, doc)?;
        match walk_value(&value, path.segments()) {
            Some(Value::Sequence(items)) => Ok(items.len()),
            Some(other) => Err(YqrError::eval(format!(
                "cannot {} the items of {}: it is {}, and a reorder addresses a sequence",
                op.word(),
                spelled(path_str),
                type_name(other)
            ))),
            // `apply_to_doc` resolved this path through the same typed view, so
            // an absent node here would mean the two walks disagree. Reported
            // rather than asserted: a read that panics is worse than any error
            // it could return.
            None => Err(YqrError::eval(format!(
                "cannot {} the items of {}: the path does not resolve to a node",
                op.word(),
                spelled(path_str)
            ))),
        }
    }

    /// Reorder the items of the block sequence at `path` — the whole of
    /// [`FidelityWriter::reorder`](super::FidelityWriter::reorder), which
    /// delegates here.
    ///
    /// # Errors
    ///
    /// Errors when the path is unaddressable, does not name a sequence, either
    /// index falls outside it, or the engine refuses the splice.
    pub(super) fn reorder_items(
        &mut self,
        doc: usize,
        path: &Path,
        op: ReorderOp,
        from: i64,
        to: i64,
    ) -> Result<()> {
        let path_str = noyalib_path(path)?;
        let len = self.reorder_len(doc, path, &path_str, op)?;
        let i = reorder_index(from, len, &path_str, op, true)?;
        let j = reorder_index(to, len, &path_str, op, false)?;
        let d = self.doc_mut(doc)?;
        // Whole entries move, comments included, since noyalib 0.0.23 — the
        // release that took `remove`'s ownership rule into the reorder path
        // (`yqr-b010`). Before it, a reorder exchanged value bytes only and
        // left every comment annotating whichever item landed beneath it, at
        // `Ok` and past the engine's own guard, which compares typed values
        // and cannot see a comment move. That is why this slice is a call and
        // not byte arithmetic, and why the tests below pin the behaviour
        // rather than assume it.
        match op {
            ReorderOp::Swap => d.swap_items(&path_str, i, j),
            ReorderOp::Move => d.move_item(&path_str, i, j),
        }
        .map_err(|e| {
            YqrError::eval(format!(
                "cannot {} the items of {}: {e}",
                op.word(),
                spelled(&path_str)
            ))
        })
    }
}

/// Spell a lowered engine path the way the filter that produced it does:
/// `xs[0]` reads back `.xs[0]`, and the document root reads `.`.
///
/// Diagnostics quote the path the user typed. The engine's root path is the
/// empty string, which in a message reads as a missing argument rather than as
/// the whole document.
// Feature f007.
fn spelled(path_str: &str) -> String {
    format!(".{path_str}")
}

/// Resolve one reorder index against the sequence length, or refuse.
///
/// Shares [`resolve_seq_index`] with `.[-1]`, which is what makes
/// "negatives count from the end, as `.[-1]` does" a fact about the code
/// rather than a claim about it.
// Feature f007.
fn reorder_index(
    idx: i64,
    len: usize,
    path_str: &str,
    op: ReorderOp,
    first: bool,
) -> Result<usize> {
    resolve_seq_index(idx, len).ok_or_else(|| {
        YqrError::eval(format!(
            "cannot {} the items of {}: {} is {idx}, but {}",
            op.word(),
            spelled(path_str),
            op.arg_name(first),
            match len {
                0 => "the sequence is empty".to_string(),
                1 => "the sequence has 1 item, so 0 and -1 are its only indices".to_string(),
                n => format!(
                    "the sequence has {n} items, so the valid range is -{n}..={}",
                    n - 1
                ),
            }
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Mutation;
    use crate::fidelity::write::apply;

    // -- Feature f007: sequence reorder (a002 slice 3) -------------------------

    /// Run `swap(<path>; i; j)` / `move(<path>; from; to)` over `input`.
    fn reorder(op: ReorderOp, path: &str, from: i64, to: i64, input: &str) -> Result<String> {
        apply(
            &Mutation::Reorder {
                path: crate::parser::parse(path).expect("valid path"),
                op,
                from,
                to,
            },
            input,
        )
    }

    fn swap(path: &str, i: i64, j: i64, input: &str) -> Result<String> {
        reorder(ReorderOp::Swap, path, i, j, input)
    }

    fn move_item(path: &str, from: i64, to: i64, input: &str) -> Result<String> {
        reorder(ReorderOp::Move, path, from, to, input)
    }

    #[test]
    fn swaps_two_block_items_and_leaves_every_other_byte() {
        let input = "# header\nxs:\n  - one\n  - two\n  - three\nafter: 1\n";
        assert_eq!(
            swap(".xs", 0, 2, input).unwrap(),
            "# header\nxs:\n  - three\n  - two\n  - one\nafter: 1\n"
        );
    }

    #[test]
    fn an_items_comments_travel_with_the_item() {
        // The b010 property, pinned rather than assumed: before noyalib 0.0.23
        // a reorder exchanged value bytes only, so every comment stayed with
        // the *slot* and silently came to document whatever landed in it —
        // at exit 0, and past the engine's own guard, which compares typed
        // values and cannot see a comment move. This test is what notices if
        // the engine ever drifts back.
        let input = "xs:\n  # about one\n  - one  # first\n  # about two\n  - two  # second\n";
        assert_eq!(
            swap(".xs", 0, 1, input).unwrap(),
            "xs:\n  # about two\n  - two  # second\n  # about one\n  - one  # first\n"
        );
    }

    #[test]
    fn moves_an_item_shifting_the_items_between() {
        // `move` is not `swap`: b and c keep their order and shift up by one.
        let input = "xs:\n  - a  # ca\n  - b  # cb\n  - c  # cc\n";
        assert_eq!(
            move_item(".xs", 0, 2, input).unwrap(),
            "xs:\n  - b  # cb\n  - c  # cc\n  - a  # ca\n"
        );
        assert_eq!(
            move_item(".xs", 2, 0, input).unwrap(),
            "xs:\n  - c  # cc\n  - a  # ca\n  - b  # cb\n"
        );
    }

    #[test]
    fn negative_indices_count_from_the_end() {
        // a002 §4.5: the same resolution `.[-1]` uses, and literally the same
        // function, so the two cannot drift apart.
        let input = "xs:\n  - a\n  - b\n  - c\n";
        assert_eq!(
            swap(".xs", 0, -1, input).unwrap(),
            "xs:\n  - c\n  - b\n  - a\n"
        );
        assert_eq!(
            move_item(".xs", -1, 0, input).unwrap(),
            "xs:\n  - c\n  - a\n  - b\n"
        );
    }

    #[test]
    fn reorders_a_root_sequence() {
        // The document root lowers to the engine's empty path; `.` is what the
        // filter spells it, and the diagnostics have to agree (`spelled`).
        assert_eq!(swap(".", 0, 1, "- a\n- b\n").unwrap(), "- b\n- a\n");
    }

    #[test]
    fn reorders_a_sequence_written_at_its_keys_own_column() {
        // The GitHub Actions / Ansible / Kubernetes list style, where the items
        // are not indented past their key.
        let input = "on:\n- push\n- pull_request\n";
        assert_eq!(
            swap(".on", 0, 1, input).unwrap(),
            "on:\n- pull_request\n- push\n"
        );
    }

    #[test]
    fn reorders_multi_line_items_whole() {
        let input =
            "steps:\n  - name: test\n    run: cargo test\n  - name: lint\n    run: cargo clippy\n";
        assert_eq!(
            swap(".steps", 0, 1, input).unwrap(),
            "steps:\n  - name: lint\n    run: cargo clippy\n  - name: test\n    run: cargo test\n"
        );
    }

    #[test]
    fn a_flow_sequence_is_reordered_rather_than_refused() {
        // A flow member owns no line of its own, so the engine exchanges value
        // spans there. That is the right answer, not a gap: there is no trivia
        // to carry along.
        assert_eq!(
            swap(".ports", 0, 2, "ports: [80, 443, 8080]\n").unwrap(),
            "ports: [8080, 443, 80]\n"
        );
        assert_eq!(swap(".", 0, 1, "[a, b]\n").unwrap(), "[b, a]\n");
    }

    #[test]
    fn reordering_an_index_with_itself_leaves_the_document_alone() {
        // Not the forbidden silent no-op: the request has a well-defined
        // result, and this is it. `del` of a missing comment is the other
        // case, and it refuses, because there the request cannot be honoured.
        let input = "xs:\n  - a  # first\n  - b\n";
        assert_eq!(swap(".xs", 1, 1, input).unwrap(), input);
        assert_eq!(move_item(".xs", 0, 0, input).unwrap(), input);
    }

    #[test]
    fn an_item_keeps_its_anchor_and_the_alias_still_resolves() {
        let input = "xs:\n  - &x a\n  - b\nref: *x\n";
        assert_eq!(
            swap(".xs", 0, 1, input).unwrap(),
            "xs:\n  - b\n  - &x a\nref: *x\n"
        );
    }

    #[test]
    fn a_blank_detached_header_is_not_part_of_the_first_item() {
        // The same ownership rule `delete_entry` draws: an entry owns the
        // contiguous run directly above it, and nothing across a blank line.
        let input = "# about the list\n\nxs:\n  - a\n  - b\n";
        assert_eq!(
            swap(".xs", 0, 1, input).unwrap(),
            "# about the list\n\nxs:\n  - b\n  - a\n"
        );
    }

    #[test]
    fn a_document_without_a_trailing_newline_keeps_its_lines_apart() {
        // Each position keeps its own line terminator while the bodies move;
        // carrying the break along with the body would splice `- b- a`.
        assert_eq!(swap(".", 0, 1, "- a\n- b").unwrap(), "- b\n- a");
    }

    #[test]
    fn crlf_line_endings_survive_a_reorder() {
        assert_eq!(
            swap(".xs", 0, 1, "xs:\r\n  - a\r\n  - b\r\n").unwrap(),
            "xs:\r\n  - b\r\n  - a\r\n"
        );
    }

    #[test]
    fn a_block_scalar_item_moves_with_its_continuation_lines() {
        let input = "xs:\n  - |\n    line one\n    line two\n  - b\n";
        assert_eq!(
            swap(".xs", 0, 1, input).unwrap(),
            "xs:\n  - b\n  - |\n    line one\n    line two\n"
        );
    }

    #[test]
    fn an_out_of_range_index_is_refused_naming_the_range() {
        let input = "xs:\n  - a\n  - b\n  - c\n";
        for (from, to, arg) in [(0_i64, 3_i64, "j"), (5, 0, "i"), (0, -4, "j")] {
            let err = swap(".xs", from, to, input).unwrap_err();
            let text = format!("{err}");
            assert!(text.contains("-3..=2"), "should name the range: {text}");
            assert!(text.contains(arg), "should name the argument: {text}");
        }
        // `move`'s arguments are not interchangeable, so it names them so.
        let err = format!("{}", move_item(".xs", 0, 9, input).unwrap_err());
        assert!(err.contains("to is 9"), "got: {err}");
    }

    #[test]
    fn an_index_into_an_empty_or_single_item_sequence_is_refused_readably() {
        let empty = format!("{}", swap(".xs", 0, 1, "xs: []\n").unwrap_err());
        assert!(empty.contains("the sequence is empty"), "got: {empty}");
        let one = format!("{}", swap(".xs", 0, 1, "xs:\n  - a\n").unwrap_err());
        assert!(one.contains("1 item"), "got: {one}");
    }

    #[test]
    fn a_path_that_is_not_a_sequence_is_refused() {
        for (input, want) in [
            ("xs:\n  a: 1\n", "a mapping"),
            ("xs: 1\n", "a number"),
            ("xs: hello\n", "a string"),
        ] {
            let err = format!("{}", swap(".xs", 0, 1, input).unwrap_err());
            assert!(err.contains(want), "got: {err}");
            assert!(err.contains("addresses a sequence"), "got: {err}");
        }
    }

    #[test]
    fn an_absent_path_leaves_its_document_untouched() {
        // The one no-op a mutation is allowed: a document in a stream whose
        // target does not resolve is skipped, exactly as `del` skips it.
        let input = "---\nxs:\n  - a\n  - b\n---\nother: 1\n";
        assert_eq!(
            swap(".xs", 0, 1, input).unwrap(),
            "---\nxs:\n  - b\n  - a\n---\nother: 1\n"
        );
        assert_eq!(swap(".nope", 0, 1, input).unwrap(), input);
    }

    #[test]
    fn a_reorder_applies_to_every_document_that_resolves() {
        let input = "xs:\n  - a\n  - b\n---\nxs:\n  - c\n  - d\n";
        assert_eq!(
            swap(".xs", 0, 1, input).unwrap(),
            "xs:\n  - b\n  - a\n---\nxs:\n  - d\n  - c\n"
        );
    }

    #[test]
    fn a_reorder_of_an_unaddressable_key_is_reported() {
        let err = format!(
            "{}",
            swap(r#".["a.b"]"#, 0, 1, "'a.b':\n  - x\n  - y\n").unwrap_err()
        );
        assert!(err.contains("cannot address"), "got: {err}");
    }
}
