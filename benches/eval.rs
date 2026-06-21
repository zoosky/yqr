//! Criterion benchmarks for the core yqr pipeline.
//!
//! Covers the two hot paths: compiling a filter (`parser::parse`) and the
//! end-to-end `eval_str` (parse filter + load YAML + evaluate). Run with
//! `cargo bench`; results feed the Continuous Benchmarking workflow.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// A small, representative document for scalar/field-access benchmarks.
const SMALL_DOC: &str = "\
user:
  name: ada
  roles:
    - admin
    - dev
version: 1
";

/// Builds a YAML document with `n` mapping entries in a sequence.
fn seq_doc(n: usize) -> String {
    let mut s = String::from("items:\n");
    for i in 0..n {
        s.push_str(&format!("  - id: {i}\n    name: item-{i}\n"));
    }
    s
}

fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse/nested_path", |b| {
        b.iter(|| yqr::parser::parse(black_box(".user.roles[0]")).unwrap());
    });
}

fn bench_eval_field(c: &mut Criterion) {
    c.bench_function("eval_str/field_access", |b| {
        b.iter(|| yqr::eval_str(black_box(".user.name"), black_box(SMALL_DOC)).unwrap());
    });
}

fn bench_eval_iterate(c: &mut Criterion) {
    let doc = seq_doc(100);
    c.bench_function("eval_str/iterate_100", |b| {
        b.iter(|| yqr::eval_str(black_box(".items[].name"), black_box(&doc)).unwrap());
    });
}

criterion_group!(benches, bench_parse, bench_eval_field, bench_eval_iterate);
criterion_main!(benches);
