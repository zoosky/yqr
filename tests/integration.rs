//! End-to-end tests driving the public library API (`eval_str` + `render`).

use yqr::{Value, eval_str, render};

fn query(filter: &str, input: &str) -> Vec<Value> {
    eval_str(filter, input).expect("filter should evaluate")
}

fn query_rendered(filter: &str, input: &str, raw: bool) -> String {
    let values = eval_str(filter, input).expect("filter should evaluate");
    render(&values, raw).expect("render should succeed")
}

const SAMPLE: &str = "\
name: yqr
version: 1
tags:
  - cli
  - yaml
authors:
  - name: ada
    role: lead
  - name: linus
    role: contributor
";

#[test]
fn identity_round_trips() {
    let out = query(".", SAMPLE);
    assert_eq!(out.len(), 1);
}

#[test]
fn top_level_field() {
    assert_eq!(query(".name", SAMPLE), vec![Value::String("yqr".into())]);
    assert_eq!(query(".version", SAMPLE), vec![Value::Int(1)]);
}

#[test]
fn array_index_and_negative_index() {
    assert_eq!(query(".tags[0]", SAMPLE), vec![Value::String("cli".into())]);
    assert_eq!(
        query(".tags[-1]", SAMPLE),
        vec![Value::String("yaml".into())]
    );
}

#[test]
fn iterate_and_project() {
    let names = query(".authors[].name", SAMPLE);
    assert_eq!(
        names,
        vec![Value::String("ada".into()), Value::String("linus".into())]
    );
}

#[test]
fn pipe_composition() {
    let out = query(".authors | .[0] | .role", SAMPLE);
    assert_eq!(out, vec![Value::String("lead".into())]);
}

#[test]
fn optional_swallows_errors() {
    // `.name` is a string; iterating it would error, but `?` yields nothing.
    let out = query(".name[]?", SAMPLE);
    assert!(out.is_empty());
}

#[test]
fn missing_field_yields_null() {
    assert_eq!(query(".nope", SAMPLE), vec![Value::Null]);
}

#[test]
fn raw_output_strips_quotes() {
    let rendered = query_rendered(".name", SAMPLE, true);
    assert_eq!(rendered, "yqr\n");
}

#[test]
fn iterate_renders_each_on_its_own() {
    let rendered = query_rendered(".tags[]", SAMPLE, true);
    assert_eq!(rendered, "cli\nyaml\n");
}

#[test]
fn invalid_filter_is_an_error() {
    assert!(eval_str("foo", SAMPLE).is_err());
}

#[test]
fn invalid_yaml_is_an_error() {
    // Unterminated flow sequence is unambiguously malformed YAML.
    assert!(eval_str(".", "items: [1, 2, 3").is_err());
}

// -- Feature f006: write tier via the public API ------------------------------

use yqr::ast::Program;
use yqr::fidelity::write;

/// Parse a mutating filter and apply it, returning the emitted stream.
fn mutate(filter: &str, input: &str) -> String {
    match yqr::parser::parse_program(filter).expect("filter parses") {
        Program::Mutate(m) => write::apply(&m, input).expect("mutation applies"),
        Program::Query(_) => panic!("expected a mutation filter"),
    }
}

#[test]
fn assignment_preserves_comments_and_layout() {
    let input = "# top\nspec:\n  replicas: 3   # inline\n  image: web\n";
    let out = mutate(".spec.replicas = 5", input);
    assert_eq!(
        out,
        "# top\nspec:\n  replicas: 5   # inline\n  image: web\n"
    );
}

#[test]
fn idempotent_assignment_is_byte_identical() {
    // Setting a node to its existing value changes nothing at the byte level.
    let input = "a: 1\nb: two\n";
    assert_eq!(mutate(".a = 1", input), input);
}

#[test]
fn append_respects_existing_indentation() {
    let input = "spec:\n  ports:\n    - 8080\n";
    let out = mutate(".spec.ports += 9090", input);
    assert_eq!(out, "spec:\n  ports:\n    - 8080\n    - 9090\n");
}

#[test]
fn new_key_is_created_under_existing_mapping() {
    let input = "metadata:\n  name: app\n";
    let out = mutate(".metadata.env = \"prod\"", input);
    assert_eq!(out, "metadata:\n  name: app\n  env: prod\n");
}

#[test]
fn structural_delete_removes_a_nested_entry() {
    // A multi-line/nested delete removes the entry's owned lines and leaves the
    // sibling byte-identical.
    let m = match yqr::parser::parse_program("del(.outer)").unwrap() {
        Program::Mutate(m) => m,
        Program::Query(_) => unreachable!(),
    };
    let out = write::apply(&m, "outer:\n  inner: 1\nx: 2\n").unwrap();
    assert_eq!(out, "x: 2\n");
}

#[test]
fn sole_entry_delete_empties_the_collection() {
    // Previously refused. Removing the last entry writes the collection out
    // explicitly, because deleting its bytes would leave `only:` behind, which
    // re-parses as null — a type change rather than a removal.
    let out = yqr::fidelity::write::apply(
        &yqr::ast::Mutation::Delete {
            target: yqr::ast::Target::Value(yqr::parser::parse(".only.a").expect("valid")),
        },
        "only:\n  a: 1\nother: 2\n",
    )
    .expect("sole-entry delete now succeeds");
    assert_eq!(out, "only:\n  {}\nother: 2\n");
}

// -- Feature f007: sequence reorder via the public API -------------------------

#[test]
fn swap_moves_whole_entries_through_the_public_api() {
    // The public surface a library caller reaches for: parse the filter, apply
    // it, get the whole stream back with each item's comments carried along.
    let input = "steps:\n  # build it\n  - run: cargo build  # release\n  - run: cargo test\n";
    let out = mutate("swap(.steps; 0; 1)", input);
    assert_eq!(
        out,
        "steps:\n  - run: cargo test\n  # build it\n  - run: cargo build  # release\n"
    );
}

#[test]
fn move_is_expressible_as_a_mutation_value() {
    // `Mutation::Reorder` is public, so a caller can build one without going
    // through the filter grammar at all.
    let out = write::apply(
        &yqr::ast::Mutation::Reorder {
            path: yqr::parser::parse(".xs").expect("valid"),
            op: yqr::ast::ReorderOp::Move,
            from: -1,
            to: 0,
        },
        "xs:\n  - a\n  - b\n  - c\n",
    )
    .expect("move applies");
    assert_eq!(out, "xs:\n  - c\n  - a\n  - b\n");
}

// -- Bug b025: the alias-to-anchor ratio heuristic vs. real values files ------

/// A merge-key-heavy document in the shape of a Helm tenants values file:
/// `anchors` anchored default blocks, each reused round-robin until
/// `aliases` merges have been written.
fn anchor_heavy_doc(anchors: usize, aliases: usize) -> String {
    let mut doc = String::from("defaults:\n");
    for a in 0..anchors {
        doc.push_str(&format!("  d{a}: &a{a}\n    k: v{a}\n"));
    }
    doc.push_str("tenants:\n");
    for t in 0..aliases {
        doc.push_str(&format!("  t{t}:\n    <<: *a{}\n", t % anchors));
    }
    doc
}

#[test]
fn classic_pipeline_accepts_a_merge_heavy_document() {
    // 221 aliases over 22 anchors is ratio 10.05 — above the engine's default
    // heuristic cap of 10, and exactly the shape that tripped in the field.
    let doc = anchor_heavy_doc(22, 221);
    assert_eq!(
        query(".tenants.t0.k", &doc),
        vec![Value::String("v0".into())]
    );
    assert_eq!(
        query(".tenants.t220.k", &doc),
        vec![Value::String("v0".into())]
    );
}

#[test]
fn classic_pipeline_keeps_the_absolute_alias_budget() {
    // Disabling the ratio heuristic must not disable the real amplification
    // guard: the parser's absolute cap on total alias expansions still holds.
    let doc = anchor_heavy_doc(22, 1025);
    let err = eval_str(".tenants.t0.k", &doc).expect_err("absolute alias budget must hold");
    assert!(
        err.to_string().contains("alias expansion limit exceeded"),
        "unexpected error: {err}"
    );
}
