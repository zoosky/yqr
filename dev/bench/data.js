window.BENCHMARK_DATA = {
  "lastUpdate": 1783529005217,
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
          "id": "900c60b2d1951f88095783bf8f6e8f0d64e6b1cc",
          "message": "chore(ci): bump checkout/cache to v5 (Node 24) + add benchmark dashboard link (#11)\n\n* chore(ci): bump checkout/cache to v5 (Node 24), add benchmark dashboard link\n\n- actions/checkout@v4 -> @v5 and actions/cache@v4 -> @v5 in ci.yml and\n  benchmark.yml. The v4 line targets the deprecated Node 20; v5 is the Node 24\n  migration, clearing the \"forced to run on Node.js 24\" CI warning. Both jobs\n  run on ubuntu-latest (Node 24 present), so the bump is a no-op behaviorally.\n- README: add a Benchmarks badge/button and a Benchmarks section linking to the\n  live criterion dashboard published to gh-pages via GitHub Pages\n  (https://zoosky.github.io/yqr/dev/bench/).\n\n* fix(tests): tolerate BrokenPipe when a CLI test child exits before reading stdin\n\ntests/cli.rs `run()` wrote to the child's stdin with `.expect(\"write stdin\")`,\nwhich panics with a BrokenPipe when the binary rejects its arguments and exits\nbefore reading stdin (e.g. an unknown `--engine`). That raced the child's exit\nand made `unknown_engine_is_an_io_error` flaky in CI (one event's run passed\nwhile the other failed on identical code). Ignore BrokenPipe on the stdin write\nand drop the handle to send EOF; the child's exit status and output are what the\nassertions inspect. Stress-run 30x locally with no failures.",
          "timestamp": "2026-07-05T22:14:57+02:00",
          "tree_id": "0507efecac3730b7f7b7c797f6e62a94f2cb3f83",
          "url": "https://github.com/zoosky/yqr/commit/900c60b2d1951f88095783bf8f6e8f0d64e6b1cc"
        },
        "date": 1783282576784,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 437,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 8813,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 925565,
            "range": "± 10669",
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
          "id": "7a9221280a2c28a346b435677650beb6f83e4fdb",
          "message": "feat(f004): ship both fidelity engines by default, runtime-switchable, from the zoosky forks (#14)",
          "timestamp": "2026-07-08T11:45:37+02:00",
          "tree_id": "6505029afc5156e005f0e28265dded712c0e2da6",
          "url": "https://github.com/zoosky/yqr/commit/7a9221280a2c28a346b435677650beb6f83e4fdb"
        },
        "date": 1783504034964,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 441,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 8721,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 829114,
            "range": "± 28495",
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
          "id": "fd92d2da47c2ce903fccfa5961b13dcf13ee04c3",
          "message": "test(m003): add shared real-world corpus for validation and benchmarks (#16)",
          "timestamp": "2026-07-08T18:19:24+02:00",
          "tree_id": "71b1fa18abba4d4d115cf25bffe72d008a757297",
          "url": "https://github.com/zoosky/yqr/commit/fd92d2da47c2ce903fccfa5961b13dcf13ee04c3"
        },
        "date": 1783527644213,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 446,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 8270,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 797036,
            "range": "± 18957",
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
          "id": "c9e432fced77dcec47d7d44f21241e7d80d9f86e",
          "message": "chore: release v0.2.0 (#17)\n\nBump version to 0.2.0 and document the changes since 0.1.1: the runtime\n--engine flag with the noyalib and rust-yaml fidelity backends (default-on\nand switchable in one binary), the fidelity round-trip harness, the shared\nreal-world corpus, and the Kubernetes usage guide.\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2026-07-08T18:42:10+02:00",
          "tree_id": "ebd5e7316a3f178b161b8aa32037f428323ed3e4",
          "url": "https://github.com/zoosky/yqr/commit/c9e432fced77dcec47d7d44f21241e7d80d9f86e"
        },
        "date": 1783529004703,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 340,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 6941,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 701618,
            "range": "± 15429",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}