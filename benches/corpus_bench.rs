//! Benchmark half of the shared corpus.
//!
//! Every case in `tests/corpus` is measured here — the same data the
//! validation suite asserts against — plus scale variants built from the
//! `inventory` and `tenants` generators, and a few end-to-end runs of the
//! compiled binary. Run with `cargo bench --bench corpus_bench`.

#[path = "../tests/corpus/mod.rs"]
mod corpus;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::process::{Command, Stdio};

use corpus::docs;
use corpus::values::{self, VALUES_YAML};
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

/// The production values file through the classic pipeline: every values
/// case, and the identity on its own. The classic pipeline is the one path
/// that reads the file on the shipped pin (`yqr-b025`); the byte-preserving
/// groups join once `yqr-f026` lands.
fn bench_values_classic(c: &mut Criterion) {
    let cases = values::classic_cases();
    let mut group = c.benchmark_group("corpus/values");
    group.throughput(Throughput::Bytes(VALUES_YAML.len() as u64));
    group.bench_function("classic_all", |b| {
        b.iter(|| {
            for case in &cases {
                let _ = black_box(yqr::eval_str(black_box(case.filter), black_box(case.doc)));
            }
        });
    });
    group.bench_function("classic_identity", |b| {
        b.iter(|| black_box(yqr::eval_str(black_box("."), black_box(VALUES_YAML))));
    });
    group.finish();
}

/// The tenants shape at growing sizes — identity on both paths, a read
/// through a merge, one targeted write, and strict validation — annotated
/// with byte throughput so the cost per kilobyte of values file is
/// comparable across sizes.
fn bench_scale_tenants(c: &mut Criterion) {
    let mut group = c.benchmark_group("corpus/scale_tenants");
    let Ok(Program::Mutate(mutation)) =
        yqr::parser::parse_program(".argo.tenants.t7.categories.weight |= . + 1")
    else {
        unreachable!("the benchmark's own filter must compile to a mutation")
    };
    for &n in &[100usize, 400, 1000] {
        let doc = values::tenants(n);
        group.throughput(Throughput::Bytes(doc.len() as u64));
        group.bench_with_input(BenchmarkId::new("engine_identity", n), &doc, |b, doc| {
            b.iter(|| black_box(yqr::fidelity::run(black_box("."), black_box(doc), false)));
        });
        group.bench_with_input(BenchmarkId::new("classic_identity", n), &doc, |b, doc| {
            b.iter(|| black_box(yqr::eval_str(black_box("."), black_box(doc))));
        });
        group.bench_with_input(BenchmarkId::new("merged_read", n), &doc, |b, doc| {
            b.iter(|| {
                black_box(yqr::fidelity::run(
                    black_box(".argo.tenants.t7.ops.DEFAULT_LANGUAGE"),
                    black_box(doc),
                    true,
                ))
            });
        });
        group.bench_with_input(BenchmarkId::new("write", n), &doc, |b, doc| {
            b.iter(|| {
                black_box(yqr::fidelity::write::apply(
                    black_box(&mutation),
                    black_box(doc),
                ))
            });
        });
        group.bench_with_input(BenchmarkId::new("validate_strict", n), &doc, |b, doc| {
            b.iter(|| black_box(yqr::validate::check_str(black_box(doc), true)));
        });
    }
    group.finish();
}

/// The compiled binary end to end — process start, argument parsing, the
/// read, the output — over the shape at four hundred tenants and the
/// production file. Noisier than the in-process groups, and the only
/// measurement of what a user waits for.
fn bench_cli(c: &mut Criterion) {
    let dir = std::env::temp_dir().join(format!("yqr-corpus-bench-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("bench scratch dir");
    let shape = dir.join("tenants-400.yaml");
    std::fs::write(&shape, values::tenants(400)).expect("write the shape");
    let file = dir.join("values.yaml");
    std::fs::write(&file, VALUES_YAML).expect("write the values file");
    let shape = shape.to_str().expect("utf-8 path");
    let file = file.to_str().expect("utf-8 path");

    let run = |args: &[&str]| {
        let status = Command::new(env!("CARGO_BIN_EXE_yqr"))
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("spawn yqr");
        black_box(status);
    };
    let mut group = c.benchmark_group("corpus/cli");
    group.sample_size(20);
    group.bench_function("identity_shape_400", |b| b.iter(|| run(&[".", shape])));
    group.bench_function("merged_read_shape_400", |b| {
        b.iter(|| run(&["-r", ".argo.tenants.t7.ops.DEFAULT_LANGUAGE", shape]));
    });
    group.bench_function("validate_shape_400", |b| {
        b.iter(|| run(&["validate", "--strict", shape]));
    });
    group.bench_function("normalize_scalar_values", |b| {
        b.iter(|| run(&["-N", ".preImage", file]));
    });
    group.finish();
    let _ = std::fs::remove_dir_all(&dir);
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
    bench_values_classic,
    bench_scale_tenants,
    bench_cli,
);
criterion_main!(benches);
