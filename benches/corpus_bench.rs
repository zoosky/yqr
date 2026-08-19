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
use yqr::ast::Program;

/// Compile every corpus filter — the parser hot path across the whole grammar.
fn bench_parse(c: &mut Criterion) {
    let classic = corpus::classic_cases();
    let engine = corpus::engine_cases();
    let write = corpus::write_cases();
    c.bench_function("corpus/parse_all", |b| {
        b.iter(|| {
            for case in &classic {
                let _ = black_box(yqr::parser::parse(black_box(case.filter)));
            }
            for case in &engine {
                let _ = black_box(yqr::parser::parse(black_box(case.filter)));
            }
            // A mutating filter is a parse error for `parse`, which is the
            // read-only entry point; the write path compiles it through
            // `parse_program`, so timing it any other way times the error path.
            for case in &write {
                let _ = black_box(yqr::parser::parse_program(black_box(case.filter)));
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

/// Run every write-tier case: compile the mutation and apply it, which is the
/// span arithmetic plus the re-parse integrity guard on every edit.
fn bench_write(c: &mut Criterion) {
    let cases = corpus::write_cases();
    c.bench_function("corpus/write_all", |b| {
        b.iter(|| {
            for case in &cases {
                if let Ok(Program::Mutate(mutation)) =
                    yqr::parser::parse_program(black_box(case.filter))
                {
                    let _ = black_box(yqr::fidelity::write::apply(
                        black_box(&mutation),
                        black_box(case.doc),
                    ));
                }
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

/// A single targeted edit in an inventory of growing size — the write path's
/// cost profile is dominated by the re-parse guard, which is O(document), not
/// by the splice itself.
fn bench_scale_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("corpus/scale_write");
    for &n in &[100usize, 1000] {
        let doc = docs::inventory(n);
        let Ok(Program::Mutate(mutation)) =
            yqr::parser::parse_program(".hosts[0].role = \"leader\"")
        else {
            unreachable!("the benchmark's own filter must compile to a mutation")
        };
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &doc, |b, doc| {
            b.iter(|| {
                black_box(yqr::fidelity::write::apply(
                    black_box(&mutation),
                    black_box(doc),
                ))
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_classic,
    bench_engine,
    bench_write,
    bench_scale_classic,
    bench_scale_engine,
    bench_scale_write,
);
criterion_main!(benches);
