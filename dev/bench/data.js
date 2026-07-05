window.BENCHMARK_DATA = {
  "lastUpdate": 1783281039478,
  "repoUrl": "https://github.com/zoosky/yqr",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "127824+zoosky@users.noreply.github.com",
            "name": "Zoo Sky",
            "username": "zoosky"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "40b255beca1606dd3f17ee98a6323638ee360dfb",
          "message": "feat(fidelity): add rust-yaml fork RoundTripDocument backend A (f003) (#9)\n\nAdd backend A of the fidelity engine seam over our rust-yaml fork's\nsource-preserving RoundTripDocument, exactly parallel to the noyalib\nbackend (f002):\n\n- src/fidelity/rustyaml.rs: RustYamlEngine adapter (parse_all multi-doc,\n  byte-offset rebasing, value lowering, resolve Found/Synthetic/Absent)\n- BackendId::RustYamlRoundTrip, open() dispatch, CLI --engine rust-yaml\n  (feature-gated behind backend-rust-yaml; default build untouched)\n- Cargo: rust-yaml-rt optional git dependency (fork feat/roundtrip-document)\n- tests/fidelity.rs comparison backend + tests/fidelity_engine_rustyaml.rs\n\nThe fork keeps full typed mapping keys, so backend A is strictly better than\nnoyalib on several axes: no key-collision guard, special-character keys\naddressable, last-wins duplicates emit real bytes. IDENTICAL across every\nb001 corpus dimension.\n\nAdversarial review hardening:\n- verified_found now prefers the line-start-extended slice for block\n  collections; the fork's lenient loader accepted first-line-dedented slices\n  that stricter parsers (PyYAML/Psych/go-yaml) reject\n- b003: fork parse_all errors on a trailing '...' after a block collection;\n  documented + pinned as a known limitation, tracked for an upstream fork fix",
          "timestamp": "2026-07-04T13:08:51+02:00",
          "tree_id": "73d1a0e1d89e9020036a83bdd70f90c6175e17ff",
          "url": "https://github.com/zoosky/yqr/commit/40b255beca1606dd3f17ee98a6323638ee360dfb"
        },
        "date": 1783280906671,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 432,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 9064,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 900328,
            "range": "± 9597",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "127824+zoosky@users.noreply.github.com",
            "name": "Zoo Sky",
            "username": "zoosky"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e5675d51a4bd5eb396224759763ff6780472cb85",
          "message": "fix(fidelity): bump noyalib to 0.0.13, consume dup-key last-wins fix (b002 2.1) (#10)\n\nnoyalib 0.0.13 resolves b002 deficiency 2.1: span_at now resolves duplicate\nmapping keys last-wins, matching the typed view. Our submission (noyalib#143,\nclosed) was folded into the 0.0.13 release via PR #145 with author credit.\n\n- Cargo: noyalib 0.0.12 -> 0.0.13 (Cargo.lock updated)\n- noyalib backend: a duplicate-key projection now emits the last occurrence's\n  real bytes instead of degrading to Synthetic; the re-parse guard is retained\n  for the residual cases (implicit-null indicators, keep-chomped block scalars,\n  aliases) and now verifies the correct slice\n- tests: duplicate_keys_resolve_to_last_occurrence /\n  duplicate_collection_keys_resolve_to_last_occurrence pin the new behavior\n- specs/docs: b002 2.1 marked Resolved; b000 tracker, f002, README reconciled\n\nAll quality gates green (fmt, clippy -D warnings default + all-features, tests\nall profiles, doc, bench compile).",
          "timestamp": "2026-07-05T21:44:14+02:00",
          "tree_id": "9a196bf3065558c5f32ee2e2f459ed36e3c74936",
          "url": "https://github.com/zoosky/yqr/commit/e5675d51a4bd5eb396224759763ff6780472cb85"
        },
        "date": 1783281039164,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 449,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 8959,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 896921,
            "range": "± 4560",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}