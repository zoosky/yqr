window.BENCHMARK_DATA = {
  "lastUpdate": 1783751899378,
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
          "id": "5f5362226482b99382773c986346bf7e5cd2e271",
          "message": "chore: release v0.3.0 (#25)\n\nByte/comment preservation moves to its own --preserve/-p flag, decoupled from\n--engine (which now only selects the backend parser). Breaking: --engine\nnoyalib no longer implies preservation.\n\n- CHANGELOG: [Unreleased] -> [0.3.0] - 2026-07-10 (adds the demo showcase)\n- Cargo.toml/Cargo.lock: 0.2.1 -> 0.3.0",
          "timestamp": "2026-07-10T21:04:56+02:00",
          "tree_id": "b1b2e8dd7a68cfcd14c9d1904af8c27cbd686cea",
          "url": "https://github.com/zoosky/yqr/commit/5f5362226482b99382773c986346bf7e5cd2e271"
        },
        "date": 1783710369432,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 332,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 3845,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 191655,
            "range": "± 4444",
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
          "id": "7bf75038c988928572138fb9a8ca6f115ffddae3",
          "message": "feat(f006): fidelity write tier v1 — assignment, +=, del, -i (#29)\n\n* feat(f006): fidelity write tier v1 (assignment, +=, del, -i)\n\nAdd a mutation surface to yqr routed entirely through noyalib 0.0.14's\nfirst-class, re-parse-guarded mutators, so edits change only the bytes the\nfilter targets and leave every other byte untouched, or refuse.\n\n- lexer: `=`, `+=`, `|=`, `(`, `)` tokens plus float literals\n- ast: a `Program` layer (`Query` | `Mutate`) with `Mutation`/`Rhs`\n- parser: `parse_program` parses `<path> = rhs`, `<path> += rhs`, `del(<path>)`;\n  read-only `parse` rejects mutations; `|=` is a clear \"not yet supported\" error\n- eval: `resolve_target` / `resolve_assign_target` / `resolve_rhs` locate the\n  single target node (new mapping keys route to the parent for insertion)\n- fidelity: a `FidelityWriter` seam (`src/fidelity/write.rs`) with a noyalib\n  backend mapping `=`/new-key/`+=`/`del` to set_value/insert_entry/push_back/\n  remove; each mutator's Result is the structural-integrity guard\n- cli/main: `-i`/`--in-place` writes back atomically (temp file + rename),\n  errors on stdin or a read-only filter; mutations always use the write path\n\nScalar writes match neighbouring quote style; += / new-key fragments render\nthrough the same Value -> noyalib emission (never a raw user string). A\nmulti-document edit applies to each document whose path resolves and leaves the\nothers byte-identical.\n\nCovers every f006 acceptance criterion (unit + integration + black-box CLI\ntests). Deferred structural edits (f007) and `|=` computed updates (f008) each\nfail with an actionable message. Also files b005 for a pre-existing\n`crossbeam-epoch` advisory reached only through the `criterion` dev-dependency.\n\n* fix(f006): evaluate RHS after target-skip; preserve file mode on -i\n\nCode-review follow-ups:\n\n- apply_to_doc resolved the RHS before checking whether the target resolves\n  in the current document. In a multi-document stream a path RHS that is\n  absent in a document which should be skipped (its target does not resolve)\n  turned that skip into a hard error. Resolve the target first and return\n  early on a skip, then evaluate the RHS.\n- write_in_place created the temp file with default (umask) permissions and\n  renamed it over the original, silently relaxing a restrictive mode (a 0600\n  secret became 0644). Carry the original file's permissions onto the temp\n  before the rename.\n\nRegression tests for both.\n\n* fix(f006): harden write path per multi-agent review\n\nAddress the verified findings from the xhigh code review of the write tier.\n\nCorrectness / security (the `-i` path and RHS):\n- No-match mutation is now a successful no-op (returns input unchanged),\n  matching jq/yq, so `del(.x)` across a batch no longer fails files lacking .x.\n- Reject a float RHS that overflows f64 (`1e999`) at lex time instead of\n  silently emitting the bare token `inf` (which reloads as the string \"inf\").\n- Reject a collection RHS for `+=` / new-key inserts with a clear message\n  rather than splicing mis-shaped multi-line YAML at exit 0.\n- Validate `-i`+stdin / `-i`+read-only-filter BEFORE reading input or applying\n  the mutation, so misuse fails fast instead of hanging or doing throwaway work.\n- write_in_place: resolve symlinks (edit the real file, keep the link); create\n  the temp with owner-only perms before writing so a 0600 secret is never\n  briefly world-readable; fsync the temp before the atomic rename so a crash\n  cannot leave a truncated file. Owner/SELinux/ACL/xattr/hardlink limits of\n  temp+rename are documented.\n\nCleanup:\n- Read queries no longer parse the filter twice: main threads the already\n  parsed Ast via new eval_ast_str / fidelity::run_ast.\n- Extract shared verify_stream_tiles_input and offending_key helpers (were\n  duplicated across the read engine and write adapter).\n- Add PathSeg::key_is_plain so insert_key tests a &str key without allocating.\n\nRegression tests for every fix; README + spec §12.1 updated. The one refuted\ncandidate (emit() fidelity) needed no change: noyalib's Display re-tiles\nverbatim source, not a re-serializer.\n\n* test(f006): cover the temp-file security property and -i stdin guard\n\nAdd binary unit tests for the write-back path:\n- write_private_synced creates the temp with no group/other access, so a\n  restricted file's contents are never exposed via the temp during the write\n  (directly tests the security fix, not just the final file mode).\n- in_place_path rejects stdin / '-' and accepts a real file path.\n\nCloses the test gap for finding #3 (the transient-exposure property was only\ncovered end-to-end via final file mode before).",
          "timestamp": "2026-07-11T08:36:57+02:00",
          "tree_id": "cfc3f32a0d9fcd2b06d160838c4bca6ac2ce03b7",
          "url": "https://github.com/zoosky/yqr/commit/7bf75038c988928572138fb9a8ca6f115ffddae3"
        },
        "date": 1783751898524,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 501,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5037,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 254926,
            "range": "± 9968",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}