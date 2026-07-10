window.BENCHMARK_DATA = {
  "lastUpdate": 1783708638221,
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
          "id": "300ca39f618ba97b7f49f5162634799efe78f6b8",
          "message": "Experiment/noyalib only (#20)\n\n* chore(deps): pin noyalib to crates.io 0.0.14 release (drop git-dep)\n\nnoyalib's b002 fidelity fixes (deficiencies 2.2-2.7: five CST span_at\nfixes, the loader KeyCollision parity, and the scanner lone-CR fix)\nshipped upstream in the 0.0.14 release (noyalib#160). Re-pin from the\n`zoosky/noyalib` `feat/fidelity-span-fixes` git branch to the published\ncrates.io release:\n\n    noyalib = { version = \"0.0.14\", optional = true }\n\nThis resolves m004 unblock condition #1 (a crates.io-published noyalib\ncarrying the fixes); `rust-yaml-rt` is now the only git-dep still\nblocking the yqr crates.io publish.\n\nVerification (default features carry both fidelity backends):\n- fidelity harness `noyalib_cst_round_trip_is_faithful` passes -> the\n  0.0.14 CST backend still round-trips byte-for-byte (a001/r002).\n- fmt, clippy (--all-features and --no-default-features), full test\n  suite, and `cargo bench --no-run` all green.\n\nSpecs synced: m004 (condition 1 done, table + acceptance), b002 (now\nresolved via the released 0.0.14, not the fork branch), b000 tracker,\nand f004 (engine-sourcing note + acceptance annotation).\n\n* release 0.2.1",
          "timestamp": "2026-07-10T17:04:43+02:00",
          "tree_id": "f694273a173334650221e38743a125181e79e9ee",
          "url": "https://github.com/zoosky/yqr/commit/300ca39f618ba97b7f49f5162634799efe78f6b8"
        },
        "date": 1783695981583,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 444,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5075,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 253313,
            "range": "± 1365",
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
          "id": "b4d21e202569e5041e994dc5a22750d649941718",
          "message": "feat: decouple byte preservation from backend with --preserve (#21)\n\n* feat(f005): decouple byte preservation from backend with --preserve\n\n--engine conflated two orthogonal concerns: which YAML backend parses the\ninput, and whether untouched nodes are emitted as their original source\nbytes. Because noyalib is both the default parser and the only fidelity\nbackend, --engine noyalib was in practice just a verbose \"preserve\nformatting\" switch.\n\nSplit them into two flags:\n\n  --preserve / -p   turn on byte/comment-preserving reads (fidelity mode)\n  --engine <name>   select the backend parser for --preserve (default noyalib)\n\n--preserve alone uses the default backend (the common case); --engine names\nthe backend and defaults to noyalib. Unknown engine names are still\ndiagnosed before any input is read.\n\nClean break (pre-1.0): --engine noyalib no longer implies preservation. All\nfirst-party callers move to --preserve.\n\nAlso adds a runnable demo showcase under docs/content/demo/ (script plus the\ndeploy.yaml/config.yaml inputs it reads in place), and refreshes the README\nbyte-preserving section, which still referenced the removed rust-yaml engine.\n\n* fix(f005): address code-review findings on --preserve decoupling\n\nResolves five verified review findings:\n\n1. docs/content/home.html demonstrated byte-exact fidelity via `--engine\n   noyalib`, which f005 broke — those commands now re-serialize (0640 -> 640,\n   comments dropped). Switch the landing-page examples to `--preserve`, reframe\n   `--engine` as backend selection, fix the stale `available:` engine list, and\n   link the runnable demo (rule 15).\n2. Demo \"Proof\" line printed a command without the input file, so a copy-paste\n   hangs on stdin — show `config.yaml` in the displayed command.\n3. Demo raw-output section queried `.metadata.name` (`web`), which renders\n   unquoted by default, so `-r` showed no difference — query\n   `.spec.containers[0].image` (`\"nginx:1.27\"` vs `nginx:1.27`).\n4. `--engine` help claimed \"no effect\" without `--preserve` while an unknown\n   name still errors — reword so it states the backend choice does not affect\n   output but an unknown name is rejected.\n5. Demo explicit-backend line asserted byte-exactness from exit status alone —\n   make it actually `diff` against the source.\n\nAlso records the landing page in the f005 acceptance criteria.",
          "timestamp": "2026-07-10T20:36:02+02:00",
          "tree_id": "6d1e74668e4357877f7063f17dd31c1a3c7fc0c4",
          "url": "https://github.com/zoosky/yqr/commit/b4d21e202569e5041e994dc5a22750d649941718"
        },
        "date": 1783708637914,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 423,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5026,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 247071,
            "range": "± 3553",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}