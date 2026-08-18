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
