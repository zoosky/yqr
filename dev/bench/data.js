window.BENCHMARK_DATA = {
  "lastUpdate": 1787235378716,
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
          "id": "8ac57afd0fe8d361372b7bef6cbafb7fab29b60f",
          "message": "fix(b005): bump crossbeam-epoch to 0.9.20 to clear RUSTSEC-2026-0204 (#30)\n\n`cargo update -p crossbeam-epoch` moves the transitive pin 0.9.18 -> 0.9.20\n(Rust 1.97-compatible), clearing the advisory that reached yqr only through the\n`criterion` dev-dependency. No manifest or source change is needed. `cargo audit`\nnow exits 0 and the full suite stays green (187 tests); only one Cargo.lock line\nchanged. Marks b005 Resolved in the bug tracker.",
          "timestamp": "2026-07-11T08:59:11+02:00",
          "tree_id": "18ebc9968b686bc44068919442e46362241b69e3",
          "url": "https://github.com/zoosky/yqr/commit/8ac57afd0fe8d361372b7bef6cbafb7fab29b60f"
        },
        "date": 1783753230462,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 511,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5178,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 253928,
            "range": "± 760",
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
          "id": "9de05cb9fbdb3ba55e580150b05fda1936aa1a51",
          "message": "feat(f007): structural delete for multi-line and nested block entries (#31)\n\ndel() now removes multi-line and nested block entries, not just\nsingle-line ones. When noyalib 0.0.14's first-class `remove` refuses an\nentry, yqr falls back to a `replace_span`-based delete that computes the\nentry's owned source lines and commits only if the re-parsed document\nequals the original value minus the target — the structural-integrity\nguard yqr must own, since `replace_span` guarantees only valid YAML, not\nstructure preservation (b004 2.4/2.5).\n\nThe deletion span is always the entry's own whole lines (key/`-` line\nthrough the last deeper-indented content line), so it can never eat a\npreceding comment or a following sibling; the guard backstops any\nresidual case by refusing rather than corrupting. Sole-entry and\nflow-collection deletes are refused with a clear message. Surviving bytes\n(comments, quoting, indentation, CRLF, key order) stay verbatim; `-i`\nwrites the closed-up document back atomically.\n\n- src/fidelity/write/delete.rs: guarded structural-delete fallback\n- src/fidelity/write.rs: hybrid dispatch (remove first, fallback on refusal)\n- tests: unit + integration + cli coverage; flip the old \"refused\"\n  multi-line tests to assert success, keep sole-entry/flow refusals\n- specs: flesh out f007 (delete shipped; comment/rename/reorder deferred),\n  update the status tracker, record the interim fallback in b004\n- docs: README and home.html reflect multi-line/nested delete",
          "timestamp": "2026-07-11T11:27:09+02:00",
          "tree_id": "28b240eb6948d28aae77b86199e6603d1d402c5f",
          "url": "https://github.com/zoosky/yqr/commit/9de05cb9fbdb3ba55e580150b05fda1936aa1a51"
        },
        "date": 1783762102088,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 374,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 3826,
            "range": "± 124",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 207016,
            "range": "± 9533",
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
          "id": "dbf43e518a014fc7888220833f9b8edc422458cf",
          "message": "fix(b006): correct structural-delete trivia handling and byte fidelity (#32)\n\nThe f007 structural-delete fallback derived the deleted byte range from an\nindentation walk backed only by a semantic Value-equality guard. Because the\nValue model carries no comments or blank lines, the guard was blind to trivia\nand several edits committed a byte-corruption at exit 0.\n\n- Derive the owned range from noyalib's authoritative value span (span_at)\n  instead of an indentation heuristic: a following sibling's comment survives,\n  an interleaved comment goes with its entry, and a keep-chomped (|+) scalar's\n  trailing blank lines are owned (no stray blank line).\n- Recover a same-column block sequence's end from its last item, so the common\n  K8s / GitHub Actions / Ansible list style deletes cleanly instead of being\n  refused.\n- Fold a contiguous same-indent head comment into the delete, so a comment is\n  never silently re-attributed to the following sibling; a blank-detached\n  comment is left in place.\n- Commit via the byte-preserving replace_span (in-place buffer splice) so an\n  untouched node can never be normalized by a parse->emit round-trip.\n- Detect a root-level flow collection for a clear message; thread the wrapped\n  remove error into the fallback's generic message; share walk_value from\n  noyalib.rs; consume the Value in remove_at_path instead of cloning the doc.\n\nAdds regression tests for each case and documents b006 (Resolved).",
          "timestamp": "2026-07-11T11:49:13+02:00",
          "tree_id": "25007a27e8a78dfc961c5623f0b42f444ad76daf",
          "url": "https://github.com/zoosky/yqr/commit/dbf43e518a014fc7888220833f9b8edc422458cf"
        },
        "date": 1783763430389,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 528,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5040,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 260140,
            "range": "± 1725",
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
          "id": "8cccae7487c026346ffabd36b13ce0146ba754a3",
          "message": "feat(f009): byte fidelity by default; replace --preserve with --normalize (#33)\n\n* feat(f009): byte fidelity by default; replace --preserve with --normalize\n\nInvert the read default so byte-preserving fidelity is what users get with\nno flag, and move the classic re-serializing pipeline behind an opt-in\n--normalize. Remove the --preserve/-p flag entirely.\n\nThis aligns the read path with the write tier (already fidelity-by-default)\nand with the product's core promise, closing the lossy-default bug b001 for\nthe default read. The classic pipeline's losses (dropped comments, scalar\ncanonicalization such as 007 -> 7) are now opt-in via --normalize.\n\nThe flip introduces no new error surface: the classic and fidelity paths\nshare the same noyalib value model, so the narrow whole-document refusals\n(e.g. 1 vs \"1\" key collisions) already fail identically under both; per-node\nnon-representability (merges, aliases, special-char keys) still degrades\nvisibly to typed rendering. --engine now selects the backend for the default\nbyte-preserving read; it is inert under --normalize beyond name validation.\n\n- src/cli.rs: drop preserve field, add --normalize; update help + unit tests\n- src/main.rs: flip query dispatch (default fidelity, --normalize opts out)\n- tests/cli.rs: rewrite preserve tests around the new default; add --normalize\n  and --preserve-rejected regression coverage\n- specs: add f009, supersede f005, resolve b001 for the default read, update\n  the feature/bug status trackers\n- docs: README fidelity section, home.html hero + callout, demo, CHANGELOG\n\n* feat(f009): add -N short flag for --normalize",
          "timestamp": "2026-07-11T13:30:51+02:00",
          "tree_id": "58a6578aa175ee2c86a989d65b6cdd151ab0759f",
          "url": "https://github.com/zoosky/yqr/commit/8cccae7487c026346ffabd36b13ce0146ba754a3"
        },
        "date": 1783769529646,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 502,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5111,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 250519,
            "range": "± 887",
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
          "id": "3ff092a3727fa8a581ae2a333acfcfb774d59ab9",
          "message": "chore: release v0.4.0 + reposition around the fidelity write tier (#34)\n\n* chore: release v0.4.0\n\n* docs: reposition around the fidelity write tier (read+edit)\n\nLead the README, crate description, and --help with yqr's differentiator:\nbyte-preserving reads by default plus surgical, guaranteed-clean edits, rather\nthan the read/query-only framing. Move the query-filter reference below the\nfidelity read and surgical-edit sections and rename it from the internal\n'Supported filters (M0)' heading.\n\n* docs: drop skald backend mention from README --engine notes\n\n* docs: drop skald backend mention from --engine doc comment\n\n* docs: state 0.4.0 facts plainly, drop prior-version framing in README",
          "timestamp": "2026-07-11T13:57:02+02:00",
          "tree_id": "bcd8270a8df4be48b5e8aa525f40c9bb95cd492e",
          "url": "https://github.com/zoosky/yqr/commit/3ff092a3727fa8a581ae2a333acfcfb774d59ab9"
        },
        "date": 1783771098920,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 470,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 4620,
            "range": "± 181",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 247869,
            "range": "± 708",
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
          "id": "f4c889cd2405404c09ddbfdaaa99f5032e1f732c",
          "message": "chore: pin Rust toolchain to 1.97.1 (#35)",
          "timestamp": "2026-07-24T07:55:36+02:00",
          "tree_id": "43bd28ddc8442bc55cb1723ba3387856241173b2",
          "url": "https://github.com/zoosky/yqr/commit/f4c889cd2405404c09ddbfdaaa99f5032e1f732c"
        },
        "date": 1784872628231,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 503,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5004,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 252480,
            "range": "± 1250",
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
          "id": "4763f48ba57cb079f3129e3300f0705cd40c7246",
          "message": "chore(deps): upgrade noyalib 0.0.14 -> 0.0.17 (#40)\n\nRelease content (crates.io / upstream GitHub releases):\n- 0.0.15: loader-parity fixes (DoS-budget parity, key-collision guard\n  re-landed) plus test-coverage hardening\n- 0.0.16: build fix, MSRV 1.86 (yqr pins Rust 1.97.1), dependency refresh\n- 0.0.17: lockstep republish, no core change\n\nThe v0.0.14...v0.0.17 diff touches no cst/ source file, so the CST edit\nAPI is unchanged: the b004 mutation-API gaps remain open; its spec and\nthe bug tracker now reference 0.0.17 as the verified-current upstream.\n\nLockfile delta is noyalib alone (no transitive churn). Full local CI\nmirror passes (fmt, clippy all-features -D warnings, build, test\n--all-features --locked, bench compile, doc, audit); the fidelity\nharness and corpus validation pass, preserving the byte round-trip\nproperty.",
          "timestamp": "2026-07-28T21:27:15+02:00",
          "tree_id": "bcfe0b3aa7565d8611a4deea71bfa281df0558a4",
          "url": "https://github.com/zoosky/yqr/commit/4763f48ba57cb079f3129e3300f0705cd40c7246"
        },
        "date": 1785266926993,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 499,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5107,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 255709,
            "range": "± 905",
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
          "id": "32c13431c6062eae7b06bea96254e72c1edd6b85",
          "message": "feat(f011): remove --engine; noyalib is the only engine (#41)\n\nyqr has settled on noyalib -- alternate engine approaches are retired.\nThe --engine flag had exactly one valid value, so its only observable\nbehaviors were a no-op and an error; it is now rejected by clap like any\nunknown flag, mirroring the retired --preserve.\n\nRemoved end to end:\n\n- CLI: the --engine <ENGINE> flag and its up-front name validation.\n- Library API (breaking): fidelity::BackendId is deleted; fidelity::open,\n  run, run_ast, and write::apply lose their backend parameter, and the\n  FidelityEngine trait drops backend_id(). The object-safe\n  FidelityEngine/FidelityWriter traits remain as the internal boundary\n  (yqr-m002), no longer a runtime choice point.\n- skald: the placeholder BackendId arm, its branch-pointer error message,\n  and the Cargo.toml note are gone; the name is no longer recognized.\n- Corpus/benches: EngineCase loses the per-backend engines field; the\n  corpus Engine enum and backend-mapping helpers are removed. Benchmark\n  ids are unchanged so the gh-pages baseline history stays comparable.\n- Docs: README, the site fidelity callout, and the demo (README +\n  yqr-demo.sh section 6) no longer mention --engine; CHANGELOG records\n  the removal under Unreleased.\n\nSpecs: new yqr-f011 (Done), f004 status notes the flag removal, m005 §6\nrecords the seam collapse, tracker totals updated.\n\nLocal CI mirror passes (fmt, clippy all-features -D warnings, build,\ntest all-features --locked, bench compile, doc, audit); the demo script\nruns clean against the rebuilt binary.",
          "timestamp": "2026-07-28T21:27:55+02:00",
          "tree_id": "e54703a4e146200704a89ed92a433dce1efd142f",
          "url": "https://github.com/zoosky/yqr/commit/32c13431c6062eae7b06bea96254e72c1edd6b85"
        },
        "date": 1785266952757,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 369,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 3772,
            "range": "± 387",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 207262,
            "range": "± 10601",
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
          "id": "c7d1b225fa07a83d965e4f9ac82350f851f8f643",
          "message": "feat(f012): yqr validate — YAML correctness checking with compiler-style diagnostics (#43)\n\n* feat(f012): yqr validate — YAML correctness checking with compiler-style diagnostics\n\nyqr's first subcommand closes the editing loop: after a surgical,\nhand-made, or agent-made edit, 'yqr validate [--strict] [FILES]...'\nanswers whether a file is still correct YAML, with diagnostics humans\nand agents can act on.\n\nChecks: every document parses on the noyalib CST, and the parsed\ndocuments must reproduce the input byte-for-byte (the a001 fidelity\ninvariant), so a pass certifies parses AND round-trips losslessly.\n--strict adds duplicate mapping keys via DuplicateKeyPolicy::Error.\n\nDiagnostics are rustc-style on stderr: stable codes (Y001 syntax, Y002\nstream integrity, Y101 duplicate key, Y102 stringified-key collision),\n'--> file:line:col' when the parser reports a location, the offending\nsource line with a caret, and '= help:' suggestions -- unresolved\nmerge-conflict markers get a dedicated hint. Exit codes: 0 all valid,\n1 findings, 5 unreadable input; highest wins, every input checked in\none run. Success is silent. The renderer is hand-rolled over noyalib's\ncore error API; no new dependencies.\n\nTwo spec amendments discovered during implementation (recorded in the\nspec's §3.3): the parser refuses stringified-key collisions outright,\nso Y102 is a default finding, not strict-only; and noyalib exposes no\nkey spans, so Y101/Y102 name the key and document instead of a source\nposition (upgradeable when upstream exposes spans).\n\nCLI: the filter form stays the default via\nargs_conflicts_with_subcommands; the filter positional becomes\nOption-typed with the binary enforcing presence (bare 'yqr' remains a\nusage error, exit 2). 'yqr validate <word>' was a filter parse error\nbefore, so no valid invocation changes meaning.\n\nTests: unit tests with golden renderings, black-box CLI tests for every\ncode and exit path, and a corpus guard requiring every corpus document\nto validate cleanly in both modes. Docs: README section, site callout,\nCHANGELOG. Spec f012 set to Done; tracker updated (6 features Done).\n\n* fix(f012): resolve all 15 code-review findings on the validate command\n\nFalse-valid verdicts eliminated:\n- An empty file list is now a usage error (exit 2), not a silent stdin\n  fallback -- a gate whose glob expands to nothing fails loudly.\n- Stdin ('-') is accepted at most once; a second '-' no longer re-reads\n  an exhausted stream as a vacuously valid empty input.\n- Strict mode now walks noyalib's lossless green tree instead of the\n  value layer's duplicate-key policy: every duplicate mapping key is\n  reported in one run -- nested, flow, quoted respellings, and duplicate\n  '<<' merge keys included (the policy exempted merge keys and stopped\n  at the first offence) -- each with the positions of both occurrences.\n\nContract fixes:\n- Non-UTF-8 input is a coded finding (new Y003, exit 1) pointing one\n  past the valid prefix, instead of an exit-5 environment error.\n- clap's auto 'help' subcommand is disabled: 'yqr help' stays an invalid\n  filter (exit 3) instead of becoming an exit-0 success.\n- A flag typed before 'validate' now gets a usage hint naming the\n  subcommand instead of a baffling filter parse error.\n- The filter positional renders as required <FILTER> again\n  (subcommand_negates_reqs + required), matching the error text.\n\nDiagnostics hardened:\n- Merge-conflict markers are detected anywhere in the file on any syntax\n  error; the diagnostic names the first marker and anchors there when\n  the parser reports no location -- full three-marker conflict blocks\n  now hit the advertised hint, not just a marker on the error line.\n- Every position is derived from the error's byte index through a\n  CR-aware line model (\\r\\n, \\n, lone \\r), fixing wrong line numbers\n  and garbled windows for CR-only files.\n- End-of-input errors clamp their source window to the last line, so a\n  truncated file shows context instead of pointing past the file.\n- Located error variants no longer repeat 'at line L, column C' inside\n  the message; unknown anchors surface noyalib's did-you-mean suggestion\n  as help.\n- Tabs in the offending line are expanded before the caret column is\n  computed, keeping the caret aligned; parse-level Y102 collisions name\n  the affected document of a multi-document stream when unambiguous.\n\nvalidate splits into a directory module (mod/render/scan) under the\n500-line rule. Docs (README, site callout, CHANGELOG) now promise\npositions only where they exist and document Y003, explicit stdin, and\nthe strict guarantees; the spec's §3.1/§3.3/§3.4 record the amendments\nand all acceptance criteria are re-checked against the fixed behavior.\n\nFull local CI mirror passes; every review scenario re-verified against\nthe built binary.",
          "timestamp": "2026-07-28T23:17:33+02:00",
          "tree_id": "00e4026e6f8e82dd44c5eb625fb6628f5b8b62cd",
          "url": "https://github.com/zoosky/yqr/commit/c7d1b225fa07a83d965e4f9ac82350f851f8f643"
        },
        "date": 1785273537203,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 505,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5115,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 257749,
            "range": "± 1636",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "zoosky@gmail.com",
            "name": "Zoo Sky",
            "username": "zoosky"
          },
          "committer": {
            "email": "127824+zoosky@users.noreply.github.com",
            "name": "Zoo Sky",
            "username": "zoosky"
          },
          "distinct": true,
          "id": "6dc124c033cc5fa2619d5d9d003c01d5927ba173",
          "message": "chore: release v0.5.0\n\nMinor bump rather than patch: the --engine removal is breaking for both\nthe CLI and the library API (fidelity::BackendId is gone and the\nfidelity entry points lost their backend argument), which under 0.x\nsemver moves the minor.\n\nCHANGELOG gains the 0.5.0 section -- the validate subcommand, the\n--engine removal, the noyalib 0.0.14 -> 0.0.17 engine upgrade, the clap\nand transitive dependency refresh, and the toolchain point release.",
          "timestamp": "2026-07-29T14:09:14+02:00",
          "tree_id": "7ef5261393b772d860b4410a7b022e45f8579b46",
          "url": "https://github.com/zoosky/yqr/commit/6dc124c033cc5fa2619d5d9d003c01d5927ba173"
        },
        "date": 1785327054589,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 534,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5150,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 257082,
            "range": "± 4759",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "zoosky@gmail.com",
            "name": "Zoo Sky",
            "username": "zoosky"
          },
          "committer": {
            "email": "127824+zoosky@users.noreply.github.com",
            "name": "Zoo Sky",
            "username": "zoosky"
          },
          "distinct": true,
          "id": "64db2cedb5e7ed1219cdcb5e77dda5837ea96243",
          "message": "docs(b004,f013): record noyalib#226, the PR fixing the remove()-trivia ask\n\nnoyalib#225 is answered by a PR the same day: remove() now derives its\nentry range from the same value-span boundary span_at reports, so the\nhead comment, the keep-chomped blanks, and the following sibling's\ncomment all land on the right side of the deletion. Nine tests cover the\nthree fixes and the preserved behaviours; the decisive check is that\nyqr's own suite passes with del routed back through upstream remove\nagainst the patched crate — the four tests that found the divergence\nincluded.\n\nAdopting it is deliberately not part of f013: option (b) becomes correct\nonly once the fix ships in a release yqr can pin.",
          "timestamp": "2026-08-02T13:27:07+02:00",
          "tree_id": "0b1405072ed0cc576698d72b6ac51e2560835bb3",
          "url": "https://github.com/zoosky/yqr/commit/64db2cedb5e7ed1219cdcb5e77dda5837ea96243"
        },
        "date": 1785670107037,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 509,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5045,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 258731,
            "range": "± 3046",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "zoosky@gmail.com",
            "name": "Zoo Sky",
            "username": "zoosky"
          },
          "committer": {
            "email": "127824+zoosky@users.noreply.github.com",
            "name": "Zoo Sky",
            "username": "zoosky"
          },
          "distinct": true,
          "id": "fad673e1b000c374a2411feb78fe29fe02fddf13",
          "message": "fix(b009): keep CRLF documents CRLF, and correct the write-tier docs\n\nReview of the previous commit turned up a second silent-corruption case in the\nsame two functions, plus several claims in the new docs that do not hold.\n\nb009: the insertion mutators terminate a new line with '\\n' whatever the\ndocument uses, so new-key assignment and '+=' gave a CRLF file one bare LF per\nadded line -- at exit 0, so '-i' wrote it. b004 2.5 had recorded the upstream\nbehaviour but it was never filed, which is how this branch came within one\nreview of setting the bug tracker to \"Open: none\" while shipping it.\n\nemit() now restores the convention for documents that were wholly CRLF at open\ntime. That is exact rather than heuristic: such a document has no bare '\\n' of\nits own, so every bare '\\n' in the output is one the edit added, and an\nuntouched document has none. A mixed-ending document is left alone -- there is\nno convention to restore and guessing one would be its own unasked-for rewrite.\nRead, set_value and del were never affected; five byte-exact tests.\n\nDoc and consistency corrections, each verified against the upstream source or\nthe built binary:\n\n- The module doc called all three typed mutators \"oracle-guarded\". set_value is\n  write_span -> format_value_for_site -> replace_span with no load-back oracle;\n  only the two insertion mutators have one. The previous commit's own CHANGELOG\n  proves the gap -- '.k = \"\\n\"' wrote a wrong value through set_value with\n  nothing catching it. Now stated per mutator, with the weaker guarantee called\n  out on the trait method too.\n- delete()'s comment justified not delegating to remove() with three trivia\n  divergences that 0.0.19 fixed -- via yqr's own noyalib#226. The same commit\n  updated four spec files to say exactly that and left the code contradicting\n  them. Restated: the reason is churn, not semantics.\n- set_value bypassed insertable(), so a collection RHS on an existing key\n  surfaced the engine's own refusal naming 'set' and \"fragment\" -- APIs yqr does\n  not expose -- and labelled it a parse error for input that parses. All three\n  write paths now report the scalar-only limit in yqr's words.\n- Rule 19: \"(bug b008)\" appeared in a /// doc comment, which renders in cargo\n  doc. Moved to a plain // comment. It was the only such hit in src/.\n- The insert_key key guard's comment gave the fragment-era reason. The typed\n  tier can splice a dotted key; what yqr cannot do is address one afterwards.\n  Restated, and filed in f007 6 as worth lifting -- it is the Kubernetes\n  label/annotation case.\n- Stale test comment claiming a collection has no expressible form.\n\nTests the review found owed: round-trip cases for the two Emit fixes the\nCHANGELOG advertises (they arrived via Cargo.toml, so nothing in yqr would\ncatch their return), a byte assertion on the quoting test whose spec text\nalready claimed one, and the collection-refusal wording.\n\nb008 6 no longer claims the corpus gap is closed -- the unit tests close it at\nthe unit level; m003's byte-exact EngineCase tier still has no multi-line\ninsert. Tracked in f007 6 alongside the two lifting decisions.\n\n163 lib tests pass; full local CI mirror green including cargo audit.",
          "timestamp": "2026-08-13T19:29:44+02:00",
          "tree_id": "791d5f48e2d6e9beabf8e21941343b366886fbf4",
          "url": "https://github.com/zoosky/yqr/commit/fad673e1b000c374a2411feb78fe29fe02fddf13"
        },
        "date": 1786642270649,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 363,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 3732,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 199953,
            "range": "± 2976",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "zoosky@gmail.com",
            "name": "Zoo Sky",
            "username": "zoosky"
          },
          "committer": {
            "email": "127824+zoosky@users.noreply.github.com",
            "name": "Zoo Sky",
            "username": "zoosky"
          },
          "distinct": true,
          "id": "991a67241cb8804c2e2ec168ce025b6d720d3251",
          "message": "chore: release v0.5.1",
          "timestamp": "2026-08-14T07:22:42+02:00",
          "tree_id": "83cd4a64155b25d39f0e77a546e83e4bea8e8413",
          "url": "https://github.com/zoosky/yqr/commit/991a67241cb8804c2e2ec168ce025b6d720d3251"
        },
        "date": 1786685040376,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 513,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5209,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 265663,
            "range": "± 845",
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
          "id": "e8759cf006757ffedac4d7b47dfaff7aac02c15c",
          "message": "chore(deps): adopt noyalib 0.0.22 and delete the b009 CRLF workaround (#62)\n\n0.0.22 carries one functional change, yqr's own noyalib#261 (merged\nunmodified 2026-08-14): a splice now takes the document's own line break\ninstead of assuming \\n, the same way it already took the indentation.\n\nThat subsumes the workaround f014 added -- a pass over the emitted string\nthat re-terminated the lines an edit introduced for documents that were\nwholly CRLF at open time. NoyalibWriter loses the crlf field, is_all_crlf,\nrestore_crlf, and the branch in emit, which is now a plain concatenation.\n\nKeeping both mechanisms would only add a place for them to disagree. The\ndeletion is measured, not assumed: with the workaround removed, three of\nthe five CRLF tests fail against a temporary 0.0.21 pin and pass on\n0.0.22. Those five survive unchanged and now pin the engine -- neither\nthe corpus nor the fidelity harness edits a CRLF document, so nothing\nelse here would catch a regression.\n\nSpecs: f015 (this adoption), b009 resolved and its delete-when-upstream-\nlands instruction carried out, f014 4 records that the #221 correction\nwas accepted upstream.",
          "timestamp": "2026-08-15T22:55:45+02:00",
          "tree_id": "0d91bb5446eeab170d35a4619783f53a2ee57dfb",
          "url": "https://github.com/zoosky/yqr/commit/e8759cf006757ffedac4d7b47dfaff7aac02c15c"
        },
        "date": 1786827425498,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 518,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5070,
            "range": "± 226",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 247594,
            "range": "± 1435",
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
          "id": "41041d40cbcf41ba71a6885887d52ad6ef633267",
          "message": "docs(specs): settle the delete-delegation question, measured on 0.0.22 (#63)\n\nf013 3.2, f014 3.4 and b004 6.4 each carried 'call upstream remove\ninstead of yqr's delete path' as cheap to revisit. Revisited on the\n0.0.22 pin, on its own rather than inside another change.\n\nMeasured: with delete routed to Document::remove, 161 of 163 lib tests\npass and every integration suite passes. Both failures are flow cases\nwhere upstream also refuses -- only the diagnostic wording differs. Every\nb006 case agrees, so 'churn concentrates here', the reason previously on\nrecord, no longer describes the semantics. yqr's own noyalib#226 is why.\n\nThe decision stands on grounds that do not depend on upstream being\nbehind: the independent implementation is the differential oracle that\nmade those divergences measurable in the first place, and swapping\nimplementations is a different trade from f015's deletion of a redundant\npass over the engine's own output. No longer an open item -- reopen on a\nnew argument, not on upstream improving further.\n\nThe delete() comment said 'revisit deliberately' and would otherwise\ncontradict the specs, so it carries the settled reasoning now.",
          "timestamp": "2026-08-15T23:11:04+02:00",
          "tree_id": "e5187a15980eaeb75001664d0da0f53af20f6484",
          "url": "https://github.com/zoosky/yqr/commit/41041d40cbcf41ba71a6885887d52ad6ef633267"
        },
        "date": 1786828344945,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 514,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5174,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 263671,
            "range": "± 1314",
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
          "id": "dfaf994c6d790b33e407095ddebaebb86881cd4c",
          "message": "feat(write): rename a key with key(<path>) = \"new\" (a002 slice 1) (#67)\n\nyqr's path grammar addresses value nodes and only value nodes, so until now\nthere was no filter that could say \"the key of this entry\". Renaming meant\ndeleting the entry and writing it back, which loses its position in the\nmapping and its comments. a002 settled the grammar; this is its first\nslice, and the whole new path -- selector, Target, one upstream call --\nlands under the operation with the fewest cases.\n\n    key(.metadata.name)              read the key token\n    key(.metadata.name) = \"title\"    rename it in place\n\nThe rename rewrites the key token and nothing else: the value keeps its\nspelling, the entry keeps its position, and the head and inline comments\nstay put. Routed to noyalib's rename_key, which matches the neighbouring\nquote style and carries a re-parse guard.\n\nThe read deliberately does *not* come from the resolved Path's last\nsegment, which is the obvious shortcut and reports the wrong thing.\nPathSeg::Key is stored decoded, so it is the string the filter named rather\nthan the bytes the document holds: a key authored \"a\" would read back a,\nand a key reached through a << merge would read back a token that appears\nnowhere in the file. It goes through Document::key_span and the existing\nread seam instead, so key(...) prints the bytes that are there like every\nother read -- and key_span returning None is both the source of the bytes\nand the test for whether there are any.\n\nReads stay total (a002 4.4): a sequence item, a merge-produced key, an\nalias site and an absent path all read null rather than failing a batch,\neven where the matching `=` refuses.\n\nGrammar notes. `key` is recognised only in function position, so .key is\nstill a field access -- along with .swap, .move, .del and the three comment\nwords, all seven in one test, because swap and move are ordinary YAML field\nnames. del's argument is unchanged and still takes a pipeline, so\ndel(.a | .b) parses and deletes as before; what is new is that it takes a\nTarget, which is what makes del(key(...)) expressible and therefore\nrefusable with a reason rather than a syntax error. The unimplemented\ncomment selectors are recognised too, only so the error names them.\n\nOne trap found while building it, recorded in a002 9 and f007 7.2: value\nassignment resolves through resolve_assign_target, whose absent-leaf branch\n*creates* a mapping key. That is right for .a.b = 1 and wrong for a rename,\nwhere an absent path means there is no key to rename -- so the rename uses\nthe plain resolver and skips the document, as del does. The comment slice\nmeets the same fork.\n\nOne edge documented rather than solved: a key holding `.` or `[` is\nunaddressable, so key(...) on one reads null -- the same answer as \"no\nkey\". Reads are total and there is no correct typed fallback, so the guide\nsays so plainly. It resolves when the dotted-key item does.\n\nCoverage: 13 write-path unit tests, 4 on key_bytes, 9 on the grammar, 10\nblack-box CLI tests including -i and the refusal-leaves-the-file-untouched\ncontract, and 3 shared-corpus EngineCases -- one a merge-produced key, the\ncase that separates a document read from an echo. Local CI green.\n\nPublic API: Program::Query and Mutation::Assign/Delete now carry a Target,\na breaking change to the library surface (minor bump pre-1.0, m001 3).",
          "timestamp": "2026-08-17T22:34:28+02:00",
          "tree_id": "4d4a427bd43e6b268966b4b87b8b345432818e8d",
          "url": "https://github.com/zoosky/yqr/commit/dfaf994c6d790b33e407095ddebaebb86881cd4c"
        },
        "date": 1786998939286,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 393,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 4012,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 199418,
            "range": "± 718",
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
          "id": "4c827984ca4d01ae4d0de79fe3d930b6e913d4d4",
          "message": "chore(deps): adopt noyalib 0.0.23; measure the delete-delegation question (#69)\n\n0.0.23 published 2026-08-17. The pin moves, the lockfile shows that one\ncrate and nothing else, and the whole suite passes untouched.\n\nb010 is fixed, and it is yqr's own commit: the reorder trivia change\nlanded as d397330 via upstream #271 and ships here, so an item's inline\nand head comments now travel with it. Verified against the published\ncrate rather than the branch, across all seven shapes including the\nno-final-newline case. That unblocks a002 slice 3, which leaves slices 2\nand 3 as implementation with nothing waiting on upstream.\n\nThe measurement f007 6 has owed since it settled \"delegate delete to\nupstream remove: no\" is run here, per test rather than in aggregate,\nbecause the aggregate is what hid the shape last time. Delete was routed\nto Document::remove on this branch, the suite run, and the patch\nreverted; the tree contains only the pin bump.\n\nSeven failures, all one shape: yqr refuses, upstream now succeeds. Five\nsole-entry, two flow-member. Not one is a trivia or fidelity divergence,\nand every other test in every suite passes. That kills the premise the\n0.0.22 record rested on -- it said the only two failures were flow cases\nwhere upstream also refused and only the diagnostic differed.\n\nBut the two halves are not one decision, which is the new information.\nThe flow-member output is exactly what a001 would want: one separator\ngoes with the member, from the correct side. The sole-entry output is\ncorrect in value and wrong in trivia -- the span it replaces begins below\nthe entry's head comment, so a comment that documented the removed entry\nsurvives and now documents an empty `{}`. Measured in every shape:\nsingle comment, contiguous run, and a document-level comment above a\nsingle-key document. An inline comment is correctly removed. That is the\nb006/b010 failure class a third time, found the same way each time.\n\nSo 5 stays open deliberately, with the evidence in front of it and a\nfourth option the original framing did not have: delegate the flow class,\nimplement the sole-entry class, each half going to whichever\nimplementation is already correct for it. Also flagged there as a scope\nquestion that survives any of them: whether `del` of a sole entry should\nwrite `{}` at all, given it turns a single-key document into `{}`.\n\nRe-checked because resolve_span and entry_line_span both changed in this\nrelease: all six b006 trivia cases still agree with delete_entry,\nblank-detached comment included. The trailing-newline defect upstream\nfound while building 0.0.23 does not reach yqr.\n\nb010 and b000 move to Resolved; the Open section is empty again. 4.4 is\nworth filing upstream regardless of what yqr decides, and is tracked in\n7's criteria.",
          "timestamp": "2026-08-17T22:55:21+02:00",
          "tree_id": "76f8adae5f193592f162d8ad0e03db333735a528",
          "url": "https://github.com/zoosky/yqr/commit/4c827984ca4d01ae4d0de79fe3d930b6e913d4d4"
        },
        "date": 1787000206107,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 527,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5178,
            "range": "± 88",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 251520,
            "range": "± 1482",
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
          "id": "e21a9dc34d52e1a8e3090eff7f527420b13b9d6a",
          "message": "feat(write): support sole-entry and flow-collection deletes (f016 §5) (#70)\n\n* feat(write): support sole-entry and flow-collection deletes (f016 5)\n\nThe two classes delete refused. f016 4 measured what delegating each\nwould buy, and the answer differed per class, so each half goes to\nwhichever implementation was already correct for it.\n\nFlow members are delegated to noyalib's `remove`. Upstream owns the\nseparator arithmetic -- exactly one comma, from the correct side -- and\nit measured clean. yqr has no flow implementation of its own, so\nre-deriving it would produce a second copy rather than a second opinion.\nThat is f007 6's differential-oracle argument applied in the direction it\nusually is not.\n\nSole entries are implemented here. Deleting the bytes would leave a\ndangling `a:`, which re-parses as null -- a type change, not a removal --\nso the collection is written out explicitly at the entry's own\nindentation and line terminator. Kept in this module because the range\nthat has to be replaced includes the entry's head-comment run, and\nupstream's own sole-entry path replaces the *collection's* span, which\nbegins below it. Delegating would have stranded a comment describing the\nremoved entry above an empty `{}`. Filed upstream as noyalib#280; yqr\ngets it right by construction, since owned_line_span already computes\nthat range.\n\nThe change turned out to be small because the range was already correct:\nthe same span is spliced, only the replacement text differs -- empty for\nan ordinary delete, `<indent>{}<nl>` for a sole entry. The existing\nre-parse guard needed nothing: remove_at_path already yields an empty\ncollection for this case, so the oracle compares against the right\ndocument unchanged.\n\nOn the scope question, whether `del` of a sole entry should write `{}` at\nall: yes. The objection was that it injects flow syntax into a block\ndocument, and that does not survive scrutiny -- an empty block collection\nhas no block spelling, so `{}` is not a style being chosen, it is the only\nthing that can be written. jq and yq both do it. The refusal was a\ncapability gap protecting the user from nothing. The cost is accepted and\nrecorded: del(.only) on a single-key file rewrites it to `{}`.\n\nFour refusal tests became behaviour tests, and thirteen more cover the\nshapes: flow member first/middle/last, root-level flow sequence, sole\nmember of a flow collection, sole mapping entry, sole sequence item,\nsingle-key document, head-comment travel, blank-detached comment\nsurvival, CRLF, and a file with no final newline. The one CLI test that\nused sole-entry as its example of a refusal now uses the dangling-alias\ncase, which still refuses.\n\nGuide updated, both examples run as printed. Local CI green.\n\n* fix(write): indent the emptied collection under its key; detect root flow properly\n\nReview of the f016 5 work found two real defects in it, one of them the\nfailure class yqr exists to refuse.\n\n**A sole-item delete of a same-column block sequence emitted invalid\nYAML.** The empty collection took its indentation from the deleted entry's\nown line, and a block sequence written at its key's own column -- `on:` /\n`- push`, the GitHub Actions idiom -- has an indent equal to the key's. So\n`del(.on[0])` produced `on:` / `[]`, where `[]` is a block-mapping value at\nits key's column. noyalib accepts that and PyYAML rejects it\n(\"could not find expected ':'\"), which means the re-parse guard could not\ncatch it and `yqr validate` called the result clean. At exit 0, with -i\nwriting a file the rest of the toolchain cannot read.\n\nThe empty collection is a block-mapping value, so it must sit strictly\ndeeper than its key. It now does: the entry's own indent is used when it\nalready qualifies, and otherwise the parent key's column plus a step. Every\nexisting shape is unchanged.\n\n**Root-level flow detection was defeated by `---` or a leading comment.**\nThe check sniffed raw source bytes, so an explicit-start document fell into\nthe block path and refused with a misleading message. It now asks the\nparsed document for the parent node's bytes, root included -- `get(\"\")`\nreturns the root node and skips markers and comments -- which also collapses\nthe special case, since the root is now just the empty parent path.\n\nThree smaller items from the same review: docs/content/index.md still said\nboth classes were refused (rule 15 -- the guide was updated, the landing\npage was not); the changelog had grown a second `### Added` under\n[Unreleased]; and two comments still described delete as never routing to\nupstream `remove`, which stopped being true when flow items were delegated.\n\nOne further finding, filed rather than fixed here: the reviewer noted flow\ndeletes only work on single-line flow collections, and the reason turns out\nto be that noyalib cannot **parse** a multi-line one at all -- valid YAML\nthat PyYAML accepts and yqr refuses to open, identity filter included. That\nis a read-path defect rather than a delete limitation, and it is now b011.\n\nFive regression tests: same-column sequence at root and nested, root flow\nbehind `---` and behind a comment, and the ordinary shapes re-asserted.\nLocal CI green; both new guide examples still print what the docs claim.",
          "timestamp": "2026-08-18T20:17:05+02:00",
          "tree_id": "6a620e60f98cb29e61d37ef0c12f587e7a317e27",
          "url": "https://github.com/zoosky/yqr/commit/e21a9dc34d52e1a8e3090eff7f527420b13b9d6a"
        },
        "date": 1787077103787,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 529,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5138,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 261665,
            "range": "± 1536",
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
          "id": "8b998fa2ca8fc0f3858459990660e9d3d9dd094f",
          "message": "feat(write): edit comments with line_comment / head_comment (a002 slice 2) (#71)\n\n* feat(write): edit comments with line_comment / head_comment (a002 slice 2)\n\nThe second slice of the addressing grammar. Both selectors read, set and\ndelete, and both compose with del() the same way key() does.\n\n    line_comment(.spec.replicas) = \"tuned\"    the # after the value\n    head_comment(.spec) = \"why\\nthis\"          the block above the entry\n    del(line_comment(.spec.replicas))\n\nReading strips the # and exactly one leading space, which is what makes\nset and read exact inverses -- a002 4.3's round-trip property, pinned as\nan executable CLI test over bodies with inner spaces, leading spaces, a\n'#' and a ':'. An empty body writes a bare '#'; only del() removes, so\nboth remain reachable.\n\nThe work is the pre-checks, not the calls. Three measured cases where\nupstream's guard answers a different question from the one the filter\nasked, all of them silent wrong results:\n\n  - An entry whose value starts on the next line has a single-line *value\n    span*, so upstream's guard does not fire and the comment lands on the\n    child's line; removal deletes the child's comment. Refused both ways,\n    on one comparison between key_span and span_at.\n  - A head block detached by a blank line is reported by comments_at, so\n    delegating would replace or delete a comment documenting whatever came\n    before. Refused; read is null.\n  - A *partly* detached block generalizes that, and one check covers both:\n    yqr computes the contiguous same-indent run itself and refuses when it\n    differs from comments_at().before, which means the edit would reach\n    bytes the path does not name.\n\nThe removal no-op is detected rather than enumerated, and that is the\ndesign decision worth flagging. Upstream's removers return Ok(()) on an\nunresolved path, a missing comment, and every shape their setters refuse.\nListing those shapes would put an upstream-owned list in yqr's source,\nfree to drift silently. Instead a removal checks afterwards that the\ncomment is gone and refuses when it is not -- which covers any such shape\nwithout naming them, and needs no rollback, since the document is\nuntouched in exactly that case.\n\nfoot_comment now parses and is refused with its reason (a002 8) rather\nthan reporting an unexpected token; the selector words are table-driven so\nthe three of them, and key, share one recognition path. None is a\nreserved word: .line_comment, .head_comment and .foot_comment still read\nfields, with a test.\n\nOne bug found on the way, recorded in a002 9 and f007 8.4: the attached-run\nwalk measured a sequence item's indent from its value, which sits past\n'- ', so a head comment aligned with the dash never matched and every item\nread null. delete_entry had this right already; this path had to learn it\nseparately.\n\nTwelve write-path unit tests, three parser tests, nine CLI tests and three\ncorpus cases. Guide updated; every example in it was run and prints what\nthe docs claim. Local CI green.\n\n* fix(read): a total read must not panic; extend the line guard to sequence items\n\nReview of the slice-2 work found two defects in it, both mine, and both in\nthe part the slice exists for.\n\n**A documented-total read panicked.** attached_head_len counts source lines\nwhile comments_at().before comes from the CST, and an alias-valued entry\nmakes the two disagree -- `a: &b 1` / `# c` / `c: *b` reports fewer\ncomments than there are lines above the entry. Taking a tail longer than\nthe list overflowed the subtraction (exit 101 in debug; an out-of-range\nslice panic in release). a002 4.4 says a read must never fail a batch, and\na panic is worse than any error it could have returned. A disagreement now\nreads null, because it means yqr cannot establish ownership. The write path\nwas already safe here, since check_comment_site compares the counts before\nusing either.\n\n**The value-starts-on-the-next-line guard was bypassed for sequence\nitems.** It asked only whether the entry had a key token on the value's\nline, and returned true for every sequence item -- I reasoned that an item\nalways sits on its own line. A bare `-` with the value below it is the\ncounterexample, and it is the mapping defect exactly:\n\n    xs:\n      -\n        a: 1  # child\n\nset rewrote the child's comment, del deleted it, and read reported it, all\nat exit 0. So the guard this slice was written to add was missing for half\nits cases. Both the guard and the head-comment walk now resolve an entry's\nmarker the same way -- key token, or `-` indicator via dash_before -- which\nis the same lesson twice in one file.\n\nThree smaller items from the same review. The head-comment refusal asserted\n\"separated by a blank line\" for any count mismatch, including an\nindentation difference or the alias case above, so it named a cause it had\nnot established; it now describes what the check actually knows. CommentKind\nwas inserted between the FidelityWriter trait's doc block and the trait,\nsilently reassigning five paragraphs to the enum -- the same displacement\nthe noyalib reorder PR had, so the lesson evidently needed repeating. And\nthe guide and changelog claimed setting and reading are \"exact inverses\",\nwhich holds write-then-read but not the other way: an authored `#note`\nreads as `note` and writes back as `# note`.\n\nFour regression tests, and f007 8.4 now records all three bugs, since each\nwas found by something disagreeing rather than by a test.",
          "timestamp": "2026-08-18T20:57:14+02:00",
          "tree_id": "9a931ba00989a1f6659a732f553d64f1322f2b68",
          "url": "https://github.com/zoosky/yqr/commit/8b998fa2ca8fc0f3858459990660e9d3d9dd094f"
        },
        "date": 1787079517472,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 524,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5249,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 266611,
            "range": "± 2169",
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
          "id": "f8f3df0430b7e56061ae62f2c067f3364686a692",
          "message": "feat(write): reorder a sequence with swap / move (a002 slice 3) (#72)\n\nAn ordering is the one edit a path cannot name -- there is no path that\nmeans \"third\" -- so it ships as a verb with arguments rather than as a\nselector wrapping a path:\n\n    swap(<path>; i; j)        exchange two items\n    move(<path>; from; to)    move one, shifting the rest\n\nCosts the grammar one token, `;` (jq's separator; `,` stays reserved for\nthe stream operator) and no reserved words: `swap` and `move` are only\nverbs directly before `(`, so `.swap` and `.move` still read fields.\nNegative indices resolve through the same function `.[-1]` resolves\nthrough, so the two cannot drift apart.\n\nEach verb is one engine call, and only because of b010: noyalib's\nreorder mutators used to exchange value bytes only, leaving every\ncomment to document whichever item landed beneath it -- at Ok, at exit\n0, and past the engine's guard by construction, since it compares typed\nvalues. yqr argued the semantics, wrote the fix, and it shipped in\n0.0.23. The property is still a yqr test: one yqr sells and does not own\nis the one worth pinning against engine drift.\n\nWhat yqr owns around the call is the sequence length (a negative index\nresolves against it, so it is a precondition rather than something to\nlearn from a refusal), the index resolution, and two refusals -- an\nindex outside the sequence, and a path naming something that is not one.\nDiagnostics spell the path as a filter does, since the engine's root\npath is the empty string.\n\nImplementation in src/fidelity/write/reorder.rs, a sibling of delete.rs\nunder the same rule 9 split; m002 section 8 records the further split\nwrite.rs still owes. Also drops the five internal spec references that\nhad accumulated in Rust doc comments, which rule 19 forbids.",
          "timestamp": "2026-08-18T21:47:08+02:00",
          "tree_id": "3f57c77c3f95243c5e45a7c225391b365a4f18c9",
          "url": "https://github.com/zoosky/yqr/commit/f8f3df0430b7e56061ae62f2c067f3364686a692"
        },
        "date": 1787082500169,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 308,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 3111,
            "range": "± 139",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 162054,
            "range": "± 1610",
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
          "id": "da9a33976bccad4591ae55d2b2663a206e5e86fa",
          "message": "test(corpus): add a write tier to the shared corpus (m003) (#73)\n\nThe corpus had two case tiers and both were read-only by construction:\nEngineCase runs through fidelity::run, which reaches parser::parse, which\nrejects a mutation by design. So no edit was covered at all -- b008 §6\nfiled this as a missing multi-line-insert case, but the gap was a level up.\n\nAdds WriteCase / WriteExpect and write_cases(): 31 cases over the genuine\ndocuments, at least one per shipped write operation (assign, insert, +=,\ndel, key rename, comment set/remove, swap/move, CRLF, multi-doc), plus\nseven refusals, one per integrity guard. A case states the spans it\nrewrites; the checker builds the expected document from the input, so\nevery byte the case does not name is asserted unchanged. Each successful\noutput is additionally run through validate in strict mode.\n\nBoth consumers pick the tier up: corpus_validation gains\nwrite_corpus_edits_only_the_targeted_bytes and folds the write docs and\nids into the two cross-cutting tests; corpus_bench gains corpus/write_all\nand corpus/scale_write, and routes write filters through parse_program in\ncorpus/parse_all -- parse rejects them, so timing them any other way times\nthe error path.\n\nTwo real documents were needed and added: a workflow written with flow\ncollections (block documents cannot reach the flow member paths) and a\nCRLF config (a block-string literal would spell the terminator \\n and\nquietly test nothing).\n\nThe tier found two upstream defects on its first run, both filed and\npinned as they behave:\n\n- b012: a new key cannot be inserted into a mapping whose keys all hold a\n  `.` -- the standard Kubernetes label block -- and the refusal blames a\n  `<<` merge the file does not contain. Upstream composes each candidate\n  anchor key back into a path string and re-parses it.\n- b013: an inserted scalar takes the document's dominant quote style, a\n  vote in which plain scalars do not count, so one quoted line anywhere\n  decides the spelling at every later edit site. This is the b008 shape\n  exactly: the unit test pinning the multi-line append passes only\n  because its toy document happens to contain no quoted scalar.\n\nThe Kubernetes guide promised the b012 edit in \"Other edits you can make\ntoday\"; its limitations section now says what actually happens.\n\nAlso drops a sentence from write::apply's doc comment that contradicted\nthe paragraph below it (and the behaviour): a mutation matching no\ndocument is a no-op, not an error.",
          "timestamp": "2026-08-19T06:40:18+02:00",
          "tree_id": "e2dce508611d0b944eb286226120469da09ae024",
          "url": "https://github.com/zoosky/yqr/commit/da9a33976bccad4591ae55d2b2663a206e5e86fa"
        },
        "date": 1787114500090,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 509,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5122,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 257075,
            "range": "± 2794",
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
          "id": "03796bf7e155be9becd94ae16b0362953822a510",
          "message": "chore(deps): adopt noyalib 0.0.24; re-measure the sole-entry delegation (f018) (#74)\n\n0.0.24 carries one functional change, and it is yqr's report: remove()\nnow takes a sole entry's head comment (noyalib#280, filed by yqr with the\nmechanism diagnosed; the patch is the maintainer's). The pin moves, the\nsuite is green untouched, and the lockfile loses a crate -- noyalib was\nthe last holder of hashbrown 0.15.5.\n\nThe fix is verified against the published crate rather than taken from\nthe release notes: probed on all four shapes f016 §4.4 measured, upstream\nnow matches delete_entry exactly, blank-detached exclusion included.\n\nThat removes the reason f016 §5 gave for keeping sole-entry delete in\nyqr's own code, so the delegation was re-run rather than assumed. 242 of\n244 lib tests pass under it. The two failures are one shape: the sole\nitem of a block sequence written at its key's own column, where upstream\nwrites\n\n    on:\n    []\n\nand yqr writes the empty collection one level deeper. PyYAML and Ruby's\nPsych both reject the first and accept the second, so delegation would\nwrite a file the ecosystem cannot read -- at exit 0, past upstream's\nguard, past yqr's re-parse guard (it re-parses with noyalib), and past\nyqr validate --strict.\n\nSo the class stays in delete_entry, on a new and sharper finding rather\nthan on the retired one, and the finding is filed as b014. Its live half\nis the validator: yqr's own strict mode calls that document clean,\nbecause validate walks noyalib's green tree and inherits the parser's\nleniency. That is b011 seen from the other side.",
          "timestamp": "2026-08-19T06:50:30+02:00",
          "tree_id": "dec6c4012764c9a3f9567d94890b05ec88f41a6b",
          "url": "https://github.com/zoosky/yqr/commit/03796bf7e155be9becd94ae16b0362953822a510"
        },
        "date": 1787115111836,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 520,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5253,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 250593,
            "range": "± 2161",
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
          "id": "fa7f568f15555a712a418c37e7cccb723ce07e87",
          "message": "fix(validate): report a block value not indented past its key (b014 §3.2) (#75)",
          "timestamp": "2026-08-19T07:04:58+02:00",
          "tree_id": "58ad87d488805182343dd7cecf0586a86e45d0a5",
          "url": "https://github.com/zoosky/yqr/commit/fa7f568f15555a712a418c37e7cccb723ce07e87"
        },
        "date": 1787115975837,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 417,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 4100,
            "range": "± 173",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 213808,
            "range": "± 8795",
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
          "id": "24f7a03aba115eec235b58e8b413d643516770db",
          "message": "chore: release v0.6.0 (#79)",
          "timestamp": "2026-08-20T08:36:50+02:00",
          "tree_id": "30b363ff8e763ade1edbf672835373918647a929",
          "url": "https://github.com/zoosky/yqr/commit/24f7a03aba115eec235b58e8b413d643516770db"
        },
        "date": 1787207889969,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 555,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5174,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 256322,
            "range": "± 1356",
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
          "id": "21e91118dfd4259c7d11cd87227b19aed0335a20",
          "message": "fix(release): keep the website out of the published crate (m004 s6) (#80)\n\nexclude in Cargo.toml is a denylist, so a new top-level directory ships by\ndefault. The Accent site landed after 0.5.1, making 0.6.0 the first release\nto carry it: 84 files and 340 KB, 40% of the package. The agent guide went\ntoo, since exclude names AGENT.md and not the CLAUDE.md symlink to it.\n\nAdd docs/ and CLAUDE.md to exclude, and gate it in local-ci.sh: ci.yml\nfilters on Rust-relevant paths, so the change that trips this is exactly the\none CI is configured to skip. local-ci.sh already sits on the release path\n(m001 s3), so the gate runs before every tag.\n\nAlso bumps softwareVersion in the site JSON-LD to 0.6.0, missed in the\nrelease, and adds that step to AGENT.md's release summary -- it was in\nm001 s3 but not in the summary that gets read.",
          "timestamp": "2026-08-20T13:23:41+02:00",
          "tree_id": "7cbb267255c30a940b0bb47fec7dee7a8a8cbfa7",
          "url": "https://github.com/zoosky/yqr/commit/21e91118dfd4259c7d11cd87227b19aed0335a20"
        },
        "date": 1787225094086,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 349,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 3134,
            "range": "± 170",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 161056,
            "range": "± 7004",
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
          "id": "f16be0c9d4b86010607c332ca48bf6a5544425e4",
          "message": "chore(deps): adopt noyalib 0.0.25; close b011, b012, b013, b014 (f019) (#81)\n\nnoyalib 0.0.25 carries all four of yqr's then-open engine bugs, filed\nupstream on 2026-08-19 and released the next morning as noyalib#287. Three\nare yqr's own commits, cherry-picked with authorship intact; the fourth is\nb013, the one filed deliberately without a patch because the dominance\nheuristic has a public API attached and what it counts was the maintainer's\ncall. The second of the two options the issue offered was taken: the quote\nvote is now scored at the edit site.\n\nEach fix is verified against the published crate on the reproduction its bug\nstates, not from the release notes (f019 §3). b014's writer half is not\nreachable from yqr, so it is measured by calling Document::remove directly,\nwith the BOM, CRLF, head-comment and nested variants, and the output checked\nagainst PyYAML and Psych.\n\nThe two m003 write-tier cases that pinned b012 and b013 as-they-behaved are\nflipped, and the three bugs that had no yqr-side test gain one: a wrapped-flow\nfidelity case, and five CLI tests covering the wrapped-flow read and edit, the\nstill-refused under-indented content, the dotted-key insert, and the quote\nstyle in all three positions.\n\nf019 §4 discharges what f018 §5 deferred to this release: the sole-entry\ndelegation revisit comes back zero divergence -- 382 tests, both f018 §4.1\nfailures gone -- and the class stays in delete_entry anyway, for the first\ntime on the standing f007 §6 argument alone. All four divergences to date were\nfound by having a second implementation to disagree, so deleting one ends that\nexactly when the disagreements stop.\n\nVerifying b011 walked the write verbs over the shape it unblocked and found\nb015: deleting a member of a wrapped flow collection leaves the removed\nmember's indentation behind as a whitespace-only line. Upstream's, through the\nflow class yqr does delegate. Filed as a spec, not yet upstream.",
          "timestamp": "2026-08-20T16:14:50+02:00",
          "tree_id": "6de07e0d629c0082b5611ebafce26828303a0a39",
          "url": "https://github.com/zoosky/yqr/commit/f16be0c9d4b86010607c332ca48bf6a5544425e4"
        },
        "date": 1787235376028,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/nested_path",
            "value": 496,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/field_access",
            "value": 5100,
            "range": "± 153",
            "unit": "ns/iter"
          },
          {
            "name": "eval_str/iterate_100",
            "value": 255882,
            "range": "± 3365",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}