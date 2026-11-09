//! The accent contract floor for the typed edit API.
//!
//! accentcms's lossless frontmatter editor encodes the guarantees its admin
//! save, MCP tools, and CLI depend on in its own test suite
//! (`src/content/frontmatter/edit/tests.rs`). This file ports every case
//! that is about *YAML editing* — comments, blank lines, quoting style, and
//! key order survive; a no-op change set is a no-op on the bytes — so the
//! convergence cannot silently weaken what accent ships today. The cases
//! that stay behind are accent-domain: the `---` block split (markdown, not
//! YAML), the mapping `diff`, and the CST path-grammar spelling (yqr paths
//! are typed segments, so there is nothing to misread). The
//! scalar-to-collection case, accent's last hand-rendered fragment, ports
//! here too now that the typed re-shape exists.

// Features f050, f051.

use yqr::Value;
use yqr::edit::{Op, Path, apply_all};
use yqr::value::Mapping;

fn s(text: &str) -> Value {
    Value::String(text.into())
}

fn seq(items: &[&str]) -> Value {
    Value::Sequence(items.iter().map(|i| s(i)).collect())
}

fn set(path: Path, value: Value) -> Op {
    Op::Set { path, value }
}

fn key(name: &str) -> Path {
    Path::root().key(name)
}

#[test]
fn an_empty_change_set_reproduces_the_input_byte_for_byte() {
    for input in [
        "title: Hello\n",
        "# a leading note\ntitle: 'Hello'   # trailing\n\ntags: [a, b]\n",
        "",
        "title: CRLF\r\nlead: x\r\n",
        "title: no trailing newline",
    ] {
        assert_eq!(apply_all(&[], input).unwrap(), input, "input {input:?}");
    }
}

#[test]
fn a_comment_and_the_quoting_style_survive_a_set() {
    let input = "# why this page exists\ntitle: 'Old Title'  # keep me\nweight: 10\n";
    let out = apply_all(&[set(key("title"), s("New Title"))], input).unwrap();
    assert!(out.contains("# why this page exists"), "{out}");
    assert!(out.contains("# keep me"), "{out}");
    assert!(out.contains("New Title"), "{out}");
    assert!(out.contains("weight: 10"), "{out}");
}

#[test]
fn only_the_touched_line_changes() {
    let input = "title: Old\nlead: 'unchanged'\ntags: [a, b]\n";
    let out = apply_all(&[set(key("title"), s("New"))], input).unwrap();
    assert_eq!(out, "title: New\nlead: 'unchanged'\ntags: [a, b]\n");
}

#[test]
fn setting_an_equal_value_does_not_touch_the_bytes() {
    // The property accent's agent write path depends on: a re-put of
    // unchanged content is a no-op on the bytes, whatever the author's
    // formatting was.
    let input = "title: 'Quoted'\nratio: 1.10\n";
    let out = apply_all(&[set(key("title"), s("Quoted"))], input).unwrap();
    assert_eq!(out, input);
}

#[test]
fn a_missing_parent_level_is_created() {
    // accent: `--set menu.visible=false` on a page with no `menu:`.
    let out = apply_all(
        &[set(key("menu").key("visible"), Value::Bool(false))],
        "title: x\n",
    )
    .unwrap();
    assert_eq!(out, "title: x\nmenu:\n  visible: false\n");
}

#[test]
fn a_flow_sequence_stays_flow_and_a_block_sequence_stays_block() {
    let out = apply_all(&[set(key("tags"), seq(&["a", "c"]))], "tags: [a, b]\n").unwrap();
    assert_eq!(out, "tags: [a, c]\n");

    let out = apply_all(
        &[set(key("tags"), seq(&["a", "c"]))],
        "tags:\n  - a\n  - b\n",
    )
    .unwrap();
    assert_eq!(out, "tags:\n  - a\n  - c\n");
}

/// The accent case this convergence existed to retire: its editor rendered
/// a YAML fragment by hand and spliced it through the CST's text `set`
/// whenever a scalar had to grow into a collection. The typed re-shape
/// replaces that fallback; the assertions are accent's own — the result
/// loads back as what was written and the sibling key survives — plus the
/// exact bytes, which accent could not promise.
// Feature f051; accent's a_scalar_grows_into_a_collection_through_the_fragment_fallback.
#[test]
fn a_scalar_grows_into_a_collection() {
    let out = apply_all(&[set(key("tags"), seq(&["a", "b"]))], "tags: solo\nx: 1\n").unwrap();
    assert_eq!(out, "tags:\n  - a\n  - b\nx: 1\n");
    assert_eq!(
        yqr::eval_str(".tags", &out).unwrap(),
        vec![seq(&["a", "b"])]
    );
    assert!(out.contains("x: 1"), "the sibling key survives: {out}");
}

#[test]
fn a_set_on_an_empty_document_creates_it() {
    let out = apply_all(&[set(key("title"), s("New"))], "").unwrap();
    assert_eq!(out, "title: New\n");

    // Nested creation from nothing.
    let out = apply_all(&[set(key("menu").key("visible"), Value::Bool(false))], "").unwrap();
    assert_eq!(out, "menu:\n  visible: false\n");
}

#[test]
fn a_scalar_to_collection_set_on_a_sequence_item_is_refused_loudly() {
    // The mapping-entry crossing re-shapes now (the test above); what
    // stays refused is the sequence-item site, whose typed route is
    // Op::Append — and the refusal must be loud, not a silent no-op.
    let err = apply_all(
        &[set(key("xs").index(0), seq(&["a", "b"]))],
        "xs:\n  - solo\nx: 1\n",
    )
    .unwrap_err();
    let message = err.to_string();
    assert!(message.contains("op 0 (set)"), "{message}");
}

#[test]
fn remove_drops_the_key_and_leaves_its_neighbours() {
    let input = "# note\ntitle: x\ndraft: true\n";
    let out = apply_all(&[Op::Remove { path: key("draft") }], input).unwrap();
    assert_eq!(out, "# note\ntitle: x\n");
}

#[test]
fn removing_an_absent_key_is_a_no_op_not_an_error() {
    let input = "title: x\n";
    let ops = [
        Op::Remove { path: key("nope") },
        Op::Remove {
            path: key("deeply").key("nested").key("nope"),
        },
    ];
    assert_eq!(apply_all(&ops, input).unwrap(), input);

    // An empty document is the same no-op.
    assert_eq!(apply_all(&ops, "").unwrap(), "");
}

#[test]
fn rename_keeps_the_value_and_the_trailing_comment() {
    let out = apply_all(
        &[Op::Rename {
            path: key("lead"),
            to: "summary".into(),
        }],
        "lead: x  # keep\n",
    )
    .unwrap();
    assert_eq!(out, "summary: x  # keep\n");
}

#[test]
fn renaming_an_absent_key_is_a_no_op() {
    let input = "summary: x\n";
    let ops = [Op::Rename {
        path: key("lead"),
        to: "summary".into(),
    }];
    assert_eq!(apply_all(&ops, input).unwrap(), input);
    assert_eq!(apply_all(&ops, "").unwrap(), "");
}

#[test]
fn apply_runs_a_change_set_in_order() {
    let out = apply_all(
        &[
            set(key("title"), s("New")),
            Op::Remove { path: key("draft") },
            Op::Rename {
                path: key("lead"),
                to: "summary".into(),
            },
        ],
        "title: Old\ndraft: true\nlead: L\n",
    )
    .unwrap();
    assert_eq!(out, "title: New\nsummary: L\n");
}

#[test]
fn append_pushes_onto_a_block_sequence() {
    let out = apply_all(
        &[Op::Append {
            path: key("tags"),
            value: s("b"),
        }],
        "tags:\n  - a\n",
    )
    .unwrap();
    assert_eq!(out, "tags:\n  - a\n  - b\n");
}

#[test]
fn a_head_comment_write_lands_above_the_entry() {
    let out = apply_all(
        &[Op::SetHeadComment {
            path: key("title"),
            text: "the page name".into(),
        }],
        "title: x\n",
    )
    .unwrap();
    assert!(out.contains("# the page name"), "{out}");
    assert!(out.contains("title: x"), "{out}");
}

#[test]
fn a_key_with_filter_metacharacters_is_just_data() {
    // accent bracket-quotes segments its CST grammar would misread; yqr's
    // typed segments never pass through a grammar, so the same keys need
    // no spelling at all.
    let input = "normal: 1\n";
    let hostile = r#"a.b["x"] | del(.)"#;
    let out = apply_all(&[set(key(hostile), Value::Int(2))], input).unwrap();

    // The key reads back as itself and the sibling survives.
    let reparsed = apply_all(&[], &out).unwrap();
    assert_eq!(reparsed, out);
    assert!(out.contains("normal: 1"), "{out}");
    let values = yqr::eval_str(".", &out).unwrap();
    let Value::Mapping(map) = &values[0] else {
        panic!("a mapping: {values:?}")
    };
    assert_eq!(
        map.get(&Value::String(hostile.into())),
        Some(&Value::Int(2)),
        "{out}"
    );
}

#[test]
fn a_rename_to_a_hostile_key_round_trips() {
    let out = apply_all(
        &[Op::Rename {
            path: key("lead"),
            to: "a: b # not a comment".into(),
        }],
        "lead: x\n",
    )
    .unwrap();
    let values = yqr::eval_str(".", &out).unwrap();
    let Value::Mapping(map) = &values[0] else {
        panic!("a mapping: {values:?}")
    };
    assert_eq!(
        map.get(&Value::String("a: b # not a comment".into())),
        Some(&s("x")),
        "{out}"
    );
}

#[test]
fn a_collection_value_writes_through_set() {
    // accent's typed-collection case (its noyalib#328 parity): a sequence
    // written over a sequence, and a mapping created under a new key.
    let mut inner = Mapping::new();
    inner.insert(Value::String("visible".into()), Value::Bool(true));
    inner.insert(Value::String("weight".into()), Value::Int(3));
    let out = apply_all(&[set(key("menu"), Value::Mapping(inner))], "title: x\n").unwrap();
    let values = yqr::eval_str(".menu.weight", &out).unwrap();
    assert_eq!(values, vec![Value::Int(3)], "{out}");
}
