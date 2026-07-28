//! Benchmark half of the shared corpus.
//!
//! Every case in `tests/corpus` is measured here — the same data the
//! validation suite asserts against — plus scale variants built from the
//! `inventory` generator. Run with `cargo bench --bench corpus_bench`.

#[path = "../tests/corpus/mod.rs"]
mod corpus;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

use corpus::docs;

/// Compile every corpus filter — the parser hot path across the whole grammar.
fn bench_parse(c: &mut Criterion) {
    let classic = corpus::classic_cases();
    let engine = corpus::engine_cases();
    c.bench_function("corpus/parse_all", |b| {
        b.iter(|| {
            for case in &classic {
                let _ = black_box(yqr::parser::parse(black_box(case.filter)));
            }
            for case in &engine {
                let _ = black_box(yqr::parser::parse(black_box(case.filter)));
            }
        });
    });
}

/// Run the classic pipeline over every classic case (parse + load + eval).
fn bench_classic(c: &mut Criterion) {
    let cases = corpus::classic_cases();
    c.bench_function("corpus/classic_all", |b| {
        b.iter(|| {
            for case in &cases {
                let _ = black_box(yqr::eval_str(black_box(case.filter), black_box(case.doc)));
            }
        });
    });
}

/// Run every fidelity-engine case.
fn bench_engine(c: &mut Criterion) {
    let cases = corpus::engine_cases();
    c.bench_function("corpus/engine_all", |b| {
        b.iter(|| {
            for case in &cases {
                let _ = black_box(yqr::fidelity::run(
                    black_box(case.filter),
                    black_box(case.doc),
                    case.raw,
                ));
            }
        });
    });
}

/// Iterate-and-project over an inventory of growing size — the classic
/// pipeline's throughput on realistic list data.
fn bench_scale_classic(c: &mut Criterion) {
    let mut group = c.benchmark_group("corpus/scale_iterate");
    for &n in &[100usize, 1000] {
        let doc = docs::inventory(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &doc, |b, doc| {
            b.iter(|| black_box(yqr::eval_str(black_box(".hosts[] | .name"), black_box(doc))));
        });
    }
    group.finish();
}

/// Byte-for-byte identity over an inventory of growing size — the engine
/// path's throughput on realistic input.
fn bench_scale_engine(c: &mut Criterion) {
    let mut group = c.benchmark_group("corpus/scale_engine_identity");
    for &n in &[100usize, 1000] {
        let doc = docs::inventory(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &doc, |b, doc| {
            b.iter(|| black_box(yqr::fidelity::run(black_box("."), black_box(doc), false)));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_classic,
    bench_engine,
    bench_scale_classic,
    bench_scale_engine,
);
criterion_main!(benches);
