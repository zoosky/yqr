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
