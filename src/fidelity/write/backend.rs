//! `NoyalibWriter`: the write seam over noyalib's lossless CST.
//!
//! The one implementation of [`FidelityWriter`]. Its inherent methods here are
//! the ones every edit family needs — bounds-checked document access, and the
//! two questions a site is asked before it is written to. The guards that
//! decide *whether* to write live in [`super::guards`]; the three edit
//! families that need byte arithmetic of their own live in `anchor`, `delete`
//! and `reorder`.

// Feature f006; moved here by f033.

use super::{insertable, rebased, to_noyalib_path};
use crate::Value;
use crate::ast::ReorderOp;
use crate::error::{Result, YqrError};
use crate::fidelity::write::seam::{Borrowed, CommentKind, FidelityWriter};
use crate::fidelity::{Path, PathSeg};

pub(crate) struct NoyalibWriter {
    /// One editable CST document per logical YAML document.
    docs: Vec<::noyalib::cst::Document>,
}

impl NoyalibWriter {
    pub(crate) fn open(input: &str) -> Result<Self> {
        let docs = crate::fidelity::noyalib::parse_lossless_stream(input)?;
        // Same fidelity invariant the read engine asserts at open time: emit
        // concatenates each document, so a slice that diverged from the input
        // would corrupt an untouched document.
        crate::fidelity::noyalib::verify_stream_tiles_input(input, &docs)?;
        Ok(Self { docs })
    }

    pub(super) fn doc_mut(&mut self, doc: usize) -> Result<&mut ::noyalib::cst::Document> {
        let len = self.docs.len();
        self.docs
            .get_mut(doc)
            .ok_or_else(|| YqrError::eval(format!("document index {doc} out of range ({len})")))
    }

    fn comment_present(&self, doc: usize, path_str: &str, kind: CommentKind) -> Result<bool> {
        let d = self.doc_ref(doc)?;
        Ok(match kind {
            CommentKind::Line => d.comments_at(path_str).inline.is_some(),
            CommentKind::Head => crate::fidelity::noyalib::attached_head_len(d, path_str) > 0,
        })
    }

    fn check_comment_site(&self, doc: usize, path_str: &str, kind: CommentKind) -> Result<()> {
        let d = self.doc_ref(doc)?;
        if d.span_at(path_str).is_none() {
            // An entry written with nothing after its `:` has a key but no
            // value bytes, so there is nothing for a comment to sit beside.
            // That is a different fact from "this path reaches no entry", and
            // unlike that one it has a way out.
            //
            // The engine cannot simply be relaxed into allowing it:
            // `comments_at` reports an empty bundle for this shape, both
            // setters refuse it, and both removers return `Ok` having done
            // nothing. So yqr refuses too, and the only thing it owns here is
            // saying which case this is. Bug b031.
            if d.key_span(path_str).is_some() {
                return Err(YqrError::eval(format!(
                    "cannot address {}({path_str}): nothing is written after the `:`, so the \
                     entry has no value bytes for a comment to sit beside. Write a value first, \
                     as in `.{path_str} = \"\"`, then comment it",
                    kind.word()
                )));
            }
            return Err(YqrError::eval(format!(
                "cannot address {}({path_str}): the path does not resolve to a node",
                kind.word()
            )));
        }
        match kind {
            // An entry whose value begins on the next line has no line of its
            // own to comment. Upstream's guard looks at the value span, which
            // is single-line here, so it writes the comment onto the child's
            // line instead — and removal deletes the child's comment.
            CommentKind::Line => {
                if !crate::fidelity::noyalib::value_starts_on_key_line(d, path_str) {
                    return Err(YqrError::eval(format!(
                        "cannot address line_comment({path_str}): its value starts on the \
                         next line, so the entry has no line of its own to comment; \
                         comment one of its entries instead"
                    )));
                }
            }
            // Upstream's leading mutators rewrite whatever `comments_at`
            // reports above the entry, and that walk crosses blank lines. yqr
            // owns only the contiguous run, so a mismatch means the edit would
            // reach a comment that documents whatever came before.
            CommentKind::Head => {
                let owned = crate::fidelity::noyalib::attached_head_len(d, path_str);
                let upstream = d.comments_at(path_str).before.len();
                if owned != upstream {
                    // Deliberately generic. A blank line is the common cause
                    // and the one §4.1.1 describes, but not the only one — a
                    // differently-indented comment and an alias-valued entry
                    // both land here too, and naming a cause the check has not
                    // established would be a confident wrong answer.
                    return Err(YqrError::eval(format!(
                        "cannot address head_comment({path_str}): the comment block the YAML \
                         engine would rewrite is larger than the run directly above this \
                         entry — typically because a blank line separates part of it, so it \
                         documents what precedes the entry rather than the entry itself. \
                         Editing it here would rewrite bytes the path does not name"
                    )));
                }
            }
        }
        Ok(())
    }

    fn borrowed_site(&self, doc: usize, path: &Path) -> Result<Option<Borrowed>> {
        let d = self.doc_ref(doc)?;
        let path_str = to_noyalib_path(path);
        // A key the source does not contain is not the mapping's own — a `<<`
        // merge or an alias expansion produced it. Restricting this to paths
        // ending in a key is what keeps a sequence item and the root out: both
        // legitimately have no key. An implicit null keeps its key and so
        // stays out too, which matters — `a:` has no value span either, and
        // refusing `.a = null` there would be the very re-spell the no-op
        // guard exists to prevent.
        if matches!(path.segments().last(), Some(PathSeg::Key(_)))
            && d.key_span(&path_str).is_none()
        {
            return Ok(Some(Borrowed::Key));
        }
        // Everything below asks one question: do the resolved bytes start
        // before the earliest point this node's own bytes could? YAML requires
        // an anchor to precede every alias to it, so bytes ahead of that floor
        // are some other node's, reached by resolving an alias through.
        let Some((value_start, _)) = d.span_at(&path_str) else {
            return Ok(None);
        };
        let floor = match path.segments().last() {
            // A mapping entry's value cannot precede its own key.
            Some(PathSeg::Key(_)) => d.key_span(&path_str).map(|(start, _)| start),
            // A sequence item has no key, so the floor is the end of the item
            // before it — or, for the first, where the sequence itself starts.
            // Both are the item's own container, which is why an anchor
            // *outside* the sequence and one in an earlier sibling are caught
            // by the same comparison. An anchor reached through the sequence's
            // own parent is not, and cannot be: see `value_is_borrowed`.
            Some(PathSeg::Index(i)) => {
                let neighbour = match i.checked_sub(1) {
                    Some(prev) => rebased(path, Some(PathSeg::Index(prev))),
                    None => rebased(path, None),
                };
                let neighbour = to_noyalib_path(&neighbour);
                d.span_at(&neighbour).map(|(start, end)| {
                    if i.checked_sub(1).is_some() {
                        end
                    } else {
                        start
                    }
                })
            }
            // The root has nothing to precede.
            None => None,
        };
        Ok(floor
            .filter(|floor| value_start < *floor)
            .map(|_| Borrowed::Value))
    }

    pub(super) fn doc_ref(&self, doc: usize) -> Result<&::noyalib::cst::Document> {
        let len = self.docs.len();
        self.docs
            .get(doc)
            .ok_or_else(|| YqrError::eval(format!("document index {doc} out of range ({len})")))
    }
}

impl FidelityWriter for NoyalibWriter {
    fn doc_count(&self) -> usize {
        self.docs.len()
    }

    fn value(&self, doc: usize) -> Result<Value> {
        let len = self.docs.len();
        let doc = self
            .docs
            .get(doc)
            .ok_or_else(|| YqrError::eval(format!("document index {doc} out of range ({len})")))?;
        Ok(Value::from(&*doc.as_value()))
    }

    fn set_value(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()> {
        let path_str = to_noyalib_path(path);
        // A merged-in key is refused either way; what yqr owns here is the
        // *reason*. Only the anchor route is named, because it is the only one
        // that works: writing an *overriding* entry here is this very check,
        // and inserting a sibling is refused too unless the mapping already
        // owns one (both measured). Naming a remedy the tool declines would be
        // worse than naming none. Creating the override is `yqr-f025`. Upstream's resolver returns the same `None` for a key a
        // merge produced as for one that does not exist, so `set_value`
        // reports `path not found` for a path yqr had just read a value from —
        // one tool contradicting itself. Its own `rename_key` words this
        // correctly, so the wording is borrowed from there rather than
        // invented. The alias arm is *not* intercepted: upstream's message for
        // it is already accurate and names the way out.
        if let (Some(PathSeg::Key(key)), Some(Borrowed::Key)) =
            (path.segments().last(), self.borrowed_site(doc, path)?)
        {
            return Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: the mapping has no {key:?} entry of its own \
                 to write; it is merged in from elsewhere, through a `<<` merge key or an \
                 alias. Assign where the key is defined instead"
            )));
        }
        let ny = insertable(value);
        // A value led by a `&anchor` or `!tag` property is not `set_value`'s
        // to rewrite: its span starts at the property, so the edit would
        // delete the definition. Bug b026; handled by the guarded span
        // surgery in `write::anchor`, which also decides the tagged case.
        self.refuse_block_collection_to_scalar(doc, path, &path_str, value)?;
        let before = self.integrity(doc)?;
        if self.value_has_leading_property(doc, &path_str)? {
            self.assign_at_definition(doc, path, &path_str, value, &ny)?;
            return self.check_integrity(doc, before, &path_str);
        }
        match self.doc_mut(doc)?.set_value(&path_str, &ny) {
            Ok(()) => self.check_integrity(doc, before, &path_str),
            // Since noyalib 0.0.29 (#338) every mutator refuses a write into
            // a value that live alias sites share — the anchor's own
            // definition included, which is the one remedy the merged-key
            // refusal above names. The refusal is `Error::Parse` with no
            // variant of its own, so it is recognized by the API it points
            // at; a test pins the marker. The definition write goes through
            // the guarded span surgery instead.
            Err(e) if e.to_string().contains("materialise_aliases_of") => {
                self.assign_at_definition(doc, path, &path_str, value, &ny)?;
                self.check_integrity(doc, before, &path_str)
            }
            Err(e) => Err(YqrError::eval(format!(
                "cannot assign at {path_str:?}: {e}"
            ))),
        }
    }

    fn value_is_borrowed(&self, doc: usize, path: &Path) -> Result<bool> {
        // A node inside bytes that are not their owner's is not its own
        // either, so every ancestor counts, not just the addressed node.
        // `.b[0]` over `b: *x` is the case that needs it: the item clears its
        // own floor because the floor is measured inside the anchor's
        // sequence, and it is the *sequence* that is borrowed.
        let mut probe = path.clone();
        loop {
            if self.borrowed_site(doc, &probe)?.is_some() {
                return Ok(true);
            }
            if probe.is_root() {
                return Ok(false);
            }
            probe = rebased(&probe, None);
        }
    }

    fn insert_key(&mut self, doc: usize, parent: &Path, key: &str, value: &Value) -> Result<()> {
        let parent_str = to_noyalib_path(parent);
        let ny = insertable(value);
        self.doc_mut(doc)?
            .insert_entry_value(&parent_str, key, &ny)
            .map_err(|e| YqrError::eval(format!("cannot insert key {key:?}: {e}")))
    }

    fn append(&mut self, doc: usize, path: &Path, value: &Value) -> Result<()> {
        let path_str = to_noyalib_path(path);
        let ny = insertable(value);
        self.doc_mut(doc)?
            .push_back_value(&path_str, &ny)
            .map_err(|e| YqrError::eval(format!("cannot append at {path_str:?}: {e}")))
    }

    fn delete(&mut self, doc: usize, path: &Path) -> Result<()> {
        // Block entries are deliberately not routed to noyalib's `remove`
        // (an item of a *flow* collection is — see `delete_entry`), and the gap
        // has closed rather than narrowed: measured on 0.0.22, upstream agrees with this path on
        // every case the b006 tests pin, differing only in how it words the
        // flow-item refusal. Keeping this path is therefore not a claim that
        // upstream is behind. It is that two implementations are what make
        // either one checkable — this one is how upstream's trivia divergences
        // were found and fixed (noyalib#225/#226) — and that swapping
        // implementations is a different trade from deleting a redundant pass
        // over the engine's own output, which is what f015 removed. Settled;
        // reopen on a new argument, not on upstream improving further.
        self.delete_entry(doc, path)
    }

    fn set_comment(
        &mut self,
        doc: usize,
        path: &Path,
        kind: CommentKind,
        text: &str,
    ) -> Result<()> {
        let path_str = to_noyalib_path(path);
        self.check_comment_site(doc, &path_str, kind)?;
        let d = self.doc_mut(doc)?;
        match kind {
            CommentKind::Line => d.set_inline_comment(&path_str, text),
            CommentKind::Head => d.set_leading_comment(&path_str, text),
        }
        .map_err(|e| YqrError::eval(format!("cannot set {}({path_str}): {e}", kind.word())))
    }

    fn current_comment(
        &self,
        doc: usize,
        path: &Path,
        kind: CommentKind,
    ) -> Result<Option<String>> {
        self.doc_ref(doc)?;
        let path_str = to_noyalib_path(path);
        // Report nothing at a site `set_comment` would refuse, so an equal
        // body can never stand in for a refusal — the lesson `value_is_borrowed`
        // records on the value side.
        if self.check_comment_site(doc, &path_str, kind).is_err() {
            return Ok(None);
        }
        let bundle = self.doc_ref(doc)?.comments_at(&path_str);
        Ok(match kind {
            CommentKind::Line => bundle
                .inline
                .map(|c| crate::fidelity::noyalib::comment_body(&c.text).to_string()),
            // `check_comment_site` passed, which for a head comment means the
            // run yqr owns *is* `before` — so no tail-slicing is needed here,
            // unlike on the read side where the two can disagree.
            CommentKind::Head if !bundle.before.is_empty() => Some(
                bundle
                    .before
                    .iter()
                    .map(|c| crate::fidelity::noyalib::comment_body(&c.text))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            CommentKind::Head => None,
        })
    }

    fn remove_comment(&mut self, doc: usize, path: &Path, kind: CommentKind) -> Result<()> {
        let path_str = to_noyalib_path(path);
        self.check_comment_site(doc, &path_str, kind)?;
        // Upstream's removers refuse nothing, so "there is no comment here" has
        // to be yqr's own check: `del` is a mutation, and a mutation refuses
        // rather than no-ops (`yqr-a002` §4.4).
        if !self.comment_present(doc, &path_str, kind)? {
            return Err(YqrError::eval(format!(
                "cannot remove {}({path_str}): the entry has no {} to remove",
                kind.word(),
                match kind {
                    CommentKind::Line => "comment on its own line",
                    CommentKind::Head => "comment block above it",
                }
            )));
        }
        let d = self.doc_mut(doc)?;
        match kind {
            CommentKind::Line => d.remove_inline_comment(&path_str),
            CommentKind::Head => d.remove_leading_comment(&path_str),
        }
        .map_err(|e| YqrError::eval(format!("cannot remove {}({path_str}): {e}", kind.word())))?;

        // Upstream's removers report `Ok(())` for shapes they do not handle —
        // a leading block on a sequence item is the measured case — so `Ok` is
        // not evidence that anything happened. Checking the comment is
        // actually gone turns that into the refusal `yqr-a002` §4.4 requires,
        // and does it for any such shape rather than for an enumerated list.
        // The document is already unchanged in exactly this case, so there is
        // nothing to roll back.
        if self.comment_present(doc, &path_str, kind)? {
            return Err(YqrError::eval(format!(
                "cannot remove {}({path_str}): the YAML engine does not support removing \
                 it from this kind of entry, and reported success without removing it",
                kind.word()
            )));
        }
        Ok(())
    }

    fn rename_key(&mut self, doc: usize, path: &Path, new_key: &str) -> Result<()> {
        // The addressable set is closed under rename without a check here:
        // every string is a key some filter can name, the empty one included
        // (`.[""]`), so no rename can produce an entry yqr cannot reach again.
        // Upstream still refuses `<<` and a non-printable character, and yqr
        // forwards both.
        let path_str = to_noyalib_path(path);
        self.doc_mut(doc)?
            .rename_key(&path_str, new_key)
            .map_err(|e| YqrError::eval(format!("cannot rename key at {path_str:?}: {e}")))
    }

    fn reorder(
        &mut self,
        doc: usize,
        path: &Path,
        op: ReorderOp,
        from: i64,
        to: i64,
    ) -> Result<()> {
        self.reorder_items(doc, path, op, from, to)
    }

    /// Concatenate the document stream, byte-for-byte as each document now
    /// stands.
    ///
    /// Nothing is post-processed here. yqr used to re-terminate the lines an
    /// edit added, because the mutators ended an inserted line with `\n`
    /// whatever the document used; noyalib 0.0.22 derives the terminator from
    /// the document the same way it already derived the indentation, so the
    /// bytes arrive correct and second-guessing the engine's line endings would
    /// only be a rewrite waiting to disagree with it.
    fn emit(&self) -> String {
        self.docs.iter().map(ToString::to_string).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::apply;
    use super::super::testutil::*;
    use super::NoyalibWriter;
    use crate::Value;
    use crate::ast::{Mutation, Rhs, Target};
    use crate::fidelity::write::seam::{Borrowed, FidelityWriter};

    // -- Feature f007: comment editing (a002 slice 2) --------------------------

    #[test]
    fn sets_and_changes_an_inline_comment_byte_exactly() {
        let input = "# header\nspec:\n  replicas: 3\n  image: web\n";
        let once = set_comment_on(line_of(".spec.replicas"), "tuned", input).unwrap();
        assert_eq!(
            once,
            "# header\nspec:\n  replicas: 3  # tuned\n  image: web\n"
        );
        // Changing replaces the body in place, keeping the separator.
        let twice = set_comment_on(line_of(".spec.replicas"), "again", &once).unwrap();
        assert_eq!(
            twice,
            "# header\nspec:\n  replicas: 3  # again\n  image: web\n"
        );
    }

    #[test]
    fn an_empty_body_writes_a_bare_hash_rather_than_removing() {
        // a002 §4.2: upstream distinguishes the two and yqr already owns an
        // unambiguous removal spelling, so conflating them would make one of
        // them unreachable.
        let out = set_comment_on(line_of(".a"), "", "a: 1  # note\n").unwrap();
        assert_eq!(out, "a: 1  #\n");
        assert_eq!(
            del_comment(line_of(".a"), "a: 1  # note\n").unwrap(),
            "a: 1\n"
        );
    }

    #[test]
    fn head_comment_lands_above_the_entry_at_its_own_indent() {
        let out =
            set_comment_on(head_of(".spec.replicas"), "why", "spec:\n  replicas: 3\n").unwrap();
        assert_eq!(out, "spec:\n  # why\n  replicas: 3\n");
    }

    #[test]
    fn a_multi_line_head_comment_is_one_line_per_segment() {
        let out = set_comment_on(head_of(".a"), "one\ntwo", "a: 1\n").unwrap();
        assert_eq!(out, "# one\n# two\na: 1\n");
    }

    #[test]
    fn a_crlf_document_gets_crlf_comment_lines() {
        let out = set_comment_on(head_of(".a"), "why", "a: 1\r\nb: 2\r\n").unwrap();
        assert_eq!(out, "# why\r\na: 1\r\nb: 2\r\n");
    }

    #[test]
    fn an_entry_whose_value_starts_on_the_next_line_is_refused_both_ways() {
        // a002 §4.1.2. Upstream's guard looks at the value span, which is
        // single-line here, so it writes onto the *child's* line — and the
        // remover deletes the child's comment. Neither may happen.
        let input = "a:\n  b: 1  # child\nc: 2\n";
        for err in [
            set_comment_on(line_of(".a"), "x", input).unwrap_err(),
            del_comment(line_of(".a"), input).unwrap_err(),
        ] {
            assert!(
                format!("{err}").contains("no line of its own"),
                "got: {err}"
            );
        }
    }

    #[test]
    fn a_sequence_item_whose_value_starts_below_the_dash_is_refused_too() {
        // The §4.1.2 guard originally asked only "does this entry have a key
        // token on the value's line", and answered `true` for every sequence
        // item on the reasoning that an item always sits on its own line. A
        // bare `-` with the value below it is the counterexample: the shape is
        // the mapping case exactly, and the comment lands on the child.
        let input = "xs:\n  -\n    a: 1  # child\n";
        for err in [
            set_comment_on(line_of(".xs[0]"), "mine", input).unwrap_err(),
            del_comment(line_of(".xs[0]"), input).unwrap_err(),
        ] {
            assert!(
                format!("{err}").contains("no line of its own"),
                "got: {err}"
            );
        }
    }

    #[test]
    fn an_ordinary_sequence_item_still_takes_an_inline_comment() {
        // The guard above must not refuse the common shape.
        let out = set_comment_on(line_of(".xs[0]"), "first", "xs:\n  - one\n").unwrap();
        assert_eq!(out, "xs:\n  - one  # first\n");
    }

    #[test]
    fn a_blank_detached_block_is_never_rewritten() {
        // a002 §4.1.1: `comments_at().before` walks past blank lines, so
        // delegating would replace (or delete) a comment documenting whatever
        // came before the entry.
        let input = "# detached\n\na: 1\n";
        for err in [
            set_comment_on(head_of(".a"), "new", input).unwrap_err(),
            del_comment(head_of(".a"), input).unwrap_err(),
        ] {
            assert!(
                format!("{err}").contains("rewrite bytes the path does not name"),
                "got: {err}"
            );
        }
    }

    #[test]
    fn a_partially_detached_block_is_refused_too() {
        // Upstream reports both comments as the entry's; yqr owns only the
        // contiguous tail. Editing here would reach `# far`, which the path
        // does not name.
        let input = "# far\n\n# near\na: 1\n";
        assert!(set_comment_on(head_of(".a"), "new", input).is_err());
        assert!(del_comment(head_of(".a"), input).is_err());
    }

    #[test]
    fn removing_a_comment_that_is_not_there_refuses() {
        // a002 §4.4: `del` is a mutation, and a mutation refuses rather than
        // no-ops. Upstream's removers return Ok for this.
        assert!(del_comment(line_of(".a"), "a: 1\n").is_err());
        assert!(del_comment(head_of(".a"), "a: 1\n").is_err());
    }

    #[test]
    fn removing_a_sequence_items_head_comment_refuses_instead_of_no_opping() {
        // Upstream returns Ok having done nothing, which is the shape the
        // post-removal check exists to catch.
        let input = "xs:\n  # about one\n  - one\n  - two\n";
        let err = del_comment(head_of(".xs[0]"), input).unwrap_err();
        assert!(
            format!("{err}").contains("reported success without removing it"),
            "got: {err}"
        );
    }

    #[test]
    fn a_comment_on_an_absent_path_is_a_skip_not_an_error() {
        let input = "a: 1\n";
        assert_eq!(set_comment_on(line_of(".nope"), "x", input).unwrap(), input);
        assert_eq!(del_comment(line_of(".nope"), input).unwrap(), input);
    }

    #[test]
    fn a_non_string_comment_body_is_refused() {
        let err = apply(
            &Mutation::Assign {
                target: line_of(".a"),
                rhs: Rhs::Literal(Value::Int(5)),
            },
            "a: 1\n",
        )
        .unwrap_err();
        assert!(format!("{err}").contains("must be a string"), "got: {err}");
    }

    // -- Feature f007: key rename ---------------------------------------------

    #[test]
    fn rename_rewrites_the_key_token_and_nothing_else() {
        // The whole property in one case: the value keeps its spelling, the
        // inline comment keeps its column, the head comment stays above, the
        // sibling and the document header are untouched, and the entry keeps
        // its position in the mapping.
        let input = "# header\nmetadata:\n  # names the app\n  name:   app   # why\n  tier: web\n";
        let out = rename(".metadata.name", "title", input).unwrap();
        assert_eq!(
            out,
            "# header\nmetadata:\n  # names the app\n  title:   app   # why\n  tier: web\n"
        );
    }

    #[test]
    fn rename_keeps_key_order() {
        // A rename is not a delete plus an insert; `b` must not move to the end.
        let out = rename(".b", "z", "a: 1\nb: 2\nc: 3\n").unwrap();
        assert_eq!(out, "a: 1\nz: 2\nc: 3\n");
    }

    #[test]
    fn rename_matches_the_neighbouring_quote_style() {
        let out = rename(".b", "new", "\"a\": 1\n\"b\": 2\n").unwrap();
        assert_eq!(out, "\"a\": 1\n\"new\": 2\n");
    }

    #[test]
    fn rename_preserves_a_multi_line_value_verbatim() {
        let input = "notes: |\n    line one\n    line two\nafter: 1\n";
        let out = rename(".notes", "text", input).unwrap();
        assert_eq!(out, "text: |\n    line one\n    line two\nafter: 1\n");
    }

    #[test]
    fn rename_of_an_absent_path_is_a_noop_not_a_new_key() {
        // `resolve_assign_target`'s create-a-key branch would be the wrong
        // answer here: there is no key to rename, so the document is left
        // alone rather than growing an entry the filter never named.
        let input = "a: 1\n";
        assert_eq!(rename(".absent", "x", input).unwrap(), input);
    }

    #[test]
    fn rename_skips_documents_where_the_path_is_absent() {
        let input = "a: 1\n---\nb: 2\n";
        assert_eq!(rename(".a", "z", input).unwrap(), "z: 1\n---\nb: 2\n");
    }

    #[test]
    fn rename_refuses_a_sequence_item() {
        let input = "xs:\n  - one\n";
        let err = rename(".xs[0]", "k", input).unwrap_err();
        assert!(
            format!("{err}").contains("sequence item"),
            "message should name the reason, got: {err}"
        );
    }

    #[test]
    fn rename_refuses_a_sibling_collision() {
        let err = rename(".a", "b", "a: 1\nb: 2\n").unwrap_err();
        assert!(
            format!("{err}").contains("already has an entry"),
            "message should name the collision, got: {err}"
        );
    }

    #[test]
    fn rename_refuses_a_merge_key() {
        let input = "base: &b\n  x: 1\nuse:\n  <<: *b\n  y: 2\n";
        let err = rename(".use.x", "z", input).unwrap_err();
        assert!(
            format!("{err}").contains("merge key"),
            "message should name the merge, got: {err}"
        );
    }

    #[test]
    fn rename_refuses_a_non_string_key() {
        let err = apply(
            &Mutation::Assign {
                target: Target::Key(crate::parser::parse(".a").expect("valid")),
                rhs: Rhs::Literal(Value::Int(5)),
            },
            "a: 1\n",
        )
        .unwrap_err();
        assert!(format!("{err}").contains("must be a string"), "got: {err}");
    }

    #[test]
    fn a_refused_rename_leaves_the_document_untouched() {
        // The `-i` contract: a refusal must not have half-applied anything.
        let input = "a: 1\nb: 2\n";
        assert!(rename(".a", "b", input).is_err());
        // Re-running a *valid* rename over the same input still produces the
        // clean result, so nothing was mutated in place on the way out.
        assert_eq!(rename(".a", "z", input).unwrap(), "z: 1\nb: 2\n");
    }

    // -- Bug b031: the comment face of an entry left empty -----------------

    #[test]
    fn commenting_an_entry_left_empty_says_which_case_it_is() {
        // The refusal stands: `comments_at` reports an empty bundle for this
        // shape, both setters refuse it, and both removers return `Ok` having
        // done nothing, so relaxing yqr's check would trade a clear refusal
        // for a worse one. What yqr owns is naming the case and a way out.
        for target in [line_of(".k"), head_of(".k")] {
            let err = set_comment_on(target, "why", "k:   # todo\nafter: 1\n").unwrap_err();
            let text = format!("{err}");
            assert!(text.contains("nothing is written after the `:`"), "{text}");
            assert!(text.contains(r#"`.k = """#), "no remedy named: {text}");
        }

        // The remedy has to work (`yqr-f025`). `.k = null` would not: an
        // equal value is a no-op, so it writes no bytes.
        let filled = apply(
            &assign(".k", Rhs::Literal(Value::String(String::new()))),
            "k:   # todo\nafter: 1\n",
        )
        .unwrap();
        assert_eq!(filled, "k: \"\"   # todo\nafter: 1\n");
        assert_eq!(
            set_comment_on(line_of(".k"), "why", &filled).unwrap(),
            "k: \"\"   # why\nafter: 1\n"
        );
    }

    #[test]
    fn a_path_that_reaches_no_entry_keeps_the_old_wording() {
        // The other half of the split: no value span *and* no key span still
        // means the entry is not the mapping's own.
        let err =
            set_comment_on(line_of(".c.k"), "why", "base: &m\n  k: 1\nc:\n  <<: *m\n").unwrap_err();
        assert!(
            format!("{err}").contains("does not resolve to a node"),
            "{err}"
        );
    }

    #[test]
    fn a_borrowed_site_says_which_way_it_is_borrowed() {
        // The two arms drive different refusals -- `Key` is yqr's own message
        // (`yqr-b020`), `Value` falls through to upstream's -- so collapsing
        // them to a bool at the source would lose the distinction the
        // diagnostics depend on.
        let cases: &[(&str, &str, Option<Borrowed>)] = &[
            ("m: &m\n  k: 1\nc:\n  <<: *m\n", ".c.k", Some(Borrowed::Key)),
            ("m: &m\n  k: 1\nc: *m\n", ".c.k", Some(Borrowed::Key)),
            ("a: &x 1\nb: *x\n", ".b", Some(Borrowed::Value)),
            ("a: &x 1\nb:\n  c: *x\n", ".b.c", Some(Borrowed::Value)),
            // A sequence item measured against its floor rather than a key.
            ("b:\n  - &x 1\n  - *x\n", ".b[1]", Some(Borrowed::Value)),
            ("a:\n  - &x 1\nb:\n  - *x\n", ".b[0]", Some(Borrowed::Value)),
            ("a:\n  - &x 1\n  - 2\n", ".a[0]", None),
            ("a:\n  - 1\n  - 2\n", ".a[1]", None),
            ("n: 0640\n", ".n", None),
            ("a:\nb: 1\n", ".a", None),
        ];
        for (src, filter, want) in cases {
            let writer = NoyalibWriter::open(src).expect("valid YAML");
            let value = writer.value(0).expect("one document");
            let ast = crate::parser::parse(filter).expect("a valid path");
            let path = crate::eval::resolve_target(&ast, &value)
                .expect("resolves")
                .expect("to a node");
            assert_eq!(
                writer.borrowed_site(0, &path).expect("in range"),
                *want,
                "{filter} over {src:?}"
            );
        }
    }

    #[test]
    fn borrowed_is_the_value_living_somewhere_else() {
        // The predicate the no-op guard consults. Its exactness is what keeps
        // `yqr-b018`'s skip intact while restoring `yqr-b019`'s refusal, so
        // both directions are pinned: a false positive re-spells a scalar that
        // needed no write, a false negative swallows a refusal.
        let borrowed: &[(&str, &str, &str)] = &[
            ("an alias-valued entry", "a: &x 1\nb: *x\n", ".b"),
            ("nested under one", "a: &x 1\nb:\n  c: *x\n", ".b.c"),
            (
                "a key a merge produced",
                "m: &m\n  k: 1\nc:\n  <<: *m\n",
                ".c.k",
            ),
            ("a key an alias expanded", "m: &m\n  k: 1\nc: *m\n", ".c.k"),
            // Not borrowed by its *own* measurement — it clears the floor,
            // because the floor is inside the anchor's sequence. This is the
            // case the ancestor walk exists for: the borrowing is at `b`.
            (
                "an item of an aliased sequence",
                "a: &x\n  - 1\nb: *x\n",
                ".b[0]",
            ),
        ];
        let own: &[(&str, &str, &str)] = &[
            ("a plain scalar", "n: 0640\n", ".n"),
            // The anchor is where the value actually lives.
            ("the anchor itself", "a: &x 1\nb: *x\n", ".a"),
            // No value span, but the key is the mapping's own -- refusing
            // here would re-spell `a:` on `.a = null`.
            ("an implicit null", "a:\nb: 1\n", ".a"),
            // No key span, and legitimately so.
            ("a sequence item", "a:\n  - 1\n", ".a[0]"),
            ("the document root", "1\n", "."),
            (
                "a merge's own sibling",
                "m: &m\n  k: 1\nc:\n  <<: *m\n  z: 2\n",
                ".c.z",
            ),
        ];
        for (what, src, filter) in borrowed.iter().chain(own) {
            let writer = NoyalibWriter::open(src).expect("valid YAML");
            let value = writer.value(0).expect("one document");
            let ast = crate::parser::parse(filter).expect("a valid path");
            let path = crate::eval::resolve_target(&ast, &value)
                .expect("resolves")
                .expect("to a node");
            assert_eq!(
                writer.value_is_borrowed(0, &path).expect("in range"),
                borrowed.iter().any(|(w, _, _)| w == what),
                "{what}"
            );
        }
    }

    // Feature f030: a dotted key is addressed through a bracket-quoted
    // segment, so the write lands on it like on any other key.
}
