# Changelog

All notable changes to `yqr` are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **Rename a key: `key(.a.b) = "new"`.** A path names a *value*, so until now
  there was no way to say "the key of this entry" -- renaming meant deleting
  the entry and writing it back, which loses its position and its comments.
  `key(...)` wraps a path and names the key instead. The rename rewrites the
  key token and nothing else: the value keeps its spelling, the entry keeps
  its place in the mapping, and the comments above and beside it are
  untouched.

  `key(.a.b)` also *reads* a key, and reads what the file says -- a key
  written `"a"` comes back `"a"`, quotes included, because the read slices the
  document rather than echoing back the path you typed. `-r` unquotes it, as
  it does for a string value.

  Reads never fail a batch: a sequence item has no key, and neither does one
  that arrived through a `<<` merge, so both read `null`. Writing to those is
  refused with the reason, as is a rename that would collide with an existing
  sibling, or one to a name the path syntax could not address afterwards.

  `key` is only a keyword directly before `(`, so `.key` still reads a field
  named `key` -- along with `.swap`, `.move`, `.del` and the comment words
  reserved for later.

- **Edit a comment: `line_comment(...)` and `head_comment(...)`.** The `#`
  after a value on its own line, and the block of comment lines above an
  entry. Both read, set, and delete:

  ```console
  $ yqr -i 'line_comment(.spec.replicas) = "tuned for peak"' deploy.yaml
  $ yqr -r 'line_comment(.spec.replicas)' deploy.yaml
  tuned for peak
  ```

  Setting and reading are exact inverses, leading spaces included, so a
  comment survives a round trip unchanged. An empty body writes a bare `#`
  rather than removing -- `del(...)` is how you remove, so both are
  reachable.

  Three cases are refused rather than guessed at, each because the obvious
  thing would be wrong: an entry whose value starts on the next line has no
  line of its own to comment (writing one would land it on the first child);
  a comment block separated by a blank line documents what came before it,
  not the entry below; and a comment block above a list item can be read but
  not edited. `foot_comment(...)` is refused with an explanation rather than
  a syntax error.

- **`del` now handles the last entry of a block, and items of inline
  collections.** Both used to be refused with a message explaining why.

  Removing the last entry writes the collection out as empty rather than
  leaving the key with nothing under it -- `spec:` on its own reads back as
  *null*, which is a type change rather than a removal, so `spec:` / `  {}`
  is what you get. A comment that documented the removed entry goes with it
  instead of being left behind describing an empty collection.

  For an inline collection like `ports: [80, 443]`, removing an item takes
  exactly one separator with it, so the result is never `[, 443]` or
  `[80, ]`.

### Changed

- **YAML engine upgraded to noyalib 0.0.23.** Reordering a list now moves each
  item's comments with it. Before, a swap exchanged the values and left every
  comment where it was, so a comment ended up describing whichever item landed
  beneath it -- silently, and reported as success. The fix is yqr's own,
  contributed upstream. yqr does not expose reordering yet, so nothing in the
  tool changes today; this is the engine being right before the feature that
  uses it ships.

- **YAML engine upgraded from noyalib 0.0.22.** The one functional change in
  that release is yqr's own contribution: an edit that adds a line now takes
  the file's own line ending instead of always writing a Unix one. 0.5.1 fixed
  that from yqr's side, by repairing the line endings after the fact; the
  engine now gets them right when it writes the line, so the repair pass is
  gone. Files are unchanged either way -- this removes a second mechanism doing
  the same job, not a behaviour. No new dependencies.

## [0.5.1] - 2026-08-14

A correctness release for the editing path. Adding a value that spanned more
than one line could damage the file while reporting success -- appending to a
list produced YAML that no longer parsed, and creating a key produced a value
that read back wrong. Adding any line to a Windows-style file left it with
mixed line endings. All three exited 0, so `--in-place` wrote the damage to
disk and said nothing. yqr now hands values to the YAML engine as values rather
than as pre-rendered text, so the engine places and spells them and rejects an
edit that would not read back as what was given. Reading files, replacing
existing values, and `del` were never affected.

### Fixed

- **Adding a multi-line string could corrupt the file.** Creating a new key or
  appending a list item whose value contained a newline -- `yqr '.s += "line
  one\nline two"'` -- wrote the value at the wrong indentation. Appending to a
  list produced a file that could no longer be parsed; creating a key produced
  a value that read back with a stray `|-` in it. Both reported success and
  exited 0, so `-i` wrote the damage to disk. Values are now handed to the
  engine as values rather than as pre-rendered text, so the engine places and
  spells them, and rejects the edit if the result would not read back as the
  value given. Replacing an *existing* key was never affected.
- **`.k = "a:"` failed, and `.k = "\n"` wrote the wrong value.** Two spelling
  defects in the engine's value emitter: a string ending in a colon was
  rejected as invalid, and a string that is a single newline was written as an
  empty block scalar and read back as `"|"`. Both fixed by the engine upgrade
  below.
- **Adding a line to a CRLF file no longer mixes line endings.** Creating a key
  or appending a list item wrote the new line with a Unix ending regardless of
  the file's own convention, so a Windows-style file silently ended up with
  both -- again at exit 0, so `-i` saved it that way. Files that consistently
  use one convention keep it; a file that already mixes endings is left as it
  is rather than being rewritten to a guess. Reading, replacing an existing
  value, and `del` were never affected.

### Changed

- **YAML engine upgraded from noyalib 0.0.17 to 0.0.21.** 0.0.18 brought the
  CST mutation API yqr had been missing -- comment setters, `rename_key` /
  `key_span`, `swap_items` / `move_item`, a `remove` that accepts multi-line
  and nested values, and a typed insertion tier that quotes and escapes on the
  caller's behalf; two of those are yqr's own upstream contributions, and the
  typed tier is what fixes the corruption above. The releases after it fix
  defects rather than add surface: 0.0.19 carries yqr's own upstream fix for
  how `remove()` treats the trivia around an entry, plus a scalar-resolution
  bug where bare `nan` / `inf` spellings destroyed a key's original text;
  0.0.21 fixes three cases where an edit could damage a document while
  reporting success, and the two emitter defects noted above. The remaining new
  operations still need filter grammar, so they are groundwork rather than
  user-facing features. Two transitive dependencies are added (`hashbrown`,
  `libm`), both from the engine's bare-metal support work. Byte fidelity is
  unaffected -- the round-trip and corpus harnesses pass untouched.
- **`del(...)` no longer delegates to the engine's `remove`.** 0.0.18's
  `remove` accepts the shapes it used to refuse, but it scopes a deletion to
  the entry's own key and value lines, where yqr treats an entry as owning the
  trivia around it. Left to the engine, deleting an entry would strand the
  comment documenting it (silently re-attaching it to the next entry), leave
  behind the blank lines a `|+` block scalar deliberately keeps, and swallow a
  trailing comment that belongs to the *following* entry. yqr keeps its own
  deletion path so `del` continues to remove exactly what a reader would say
  the entry is. Behaviour is unchanged from 0.5.0.

## [0.5.0] - 2026-07-29

Verification joins the editing loop: `yqr validate` answers whether a file is
still correct YAML after an edit -- surgical, hand-made, or agent-made -- with
compiler-style diagnostics a human or an agent can act on. In the same release
yqr settles on noyalib as its one and only YAML engine, so the `--engine` flag
and the runtime backend seam behind it are gone.

### Added

- **`yqr validate [--strict] FILES...` -- YAML correctness checking with
  compiler-style diagnostics.** yqr's first subcommand closes the editing
  loop: after a surgical, hand-made, or agent-made edit, one command answers
  whether a file is still correct YAML. A pass certifies that every document
  parses *and* that the parsed documents reproduce the input byte-for-byte
  (the fidelity invariant). Failures are rustc-style diagnostics on stderr
  with stable codes (`Y001` syntax, `Y002` stream integrity, `Y003`
  non-UTF-8 input, `Y101` duplicate key under `--strict`, `Y102`
  stringified-key collision), a `file:line:column` location whenever a
  position is known, the offending source line with a caret, and a
  suggested fix. `--strict` reports every duplicate mapping key in one run
  -- nested, flow, quoted respellings, and duplicate `<<` merge keys
  included -- with the positions of both occurrences (found by walking the
  lossless CST). A file containing unresolved merge-conflict markers gets a
  dedicated hint anchored at the first marker, end-of-input errors clamp
  their source window to the last line, and CR-only line endings render
  correctly. Exit codes: 0 all inputs valid, 1 validation findings, 5 an
  input could not be read (highest wins; every input is checked in one
  run). Stdin is explicit (`-`, at most once); an empty file list is a
  usage error rather than a silent stdin fallback, so a CI gate whose glob
  expands to nothing fails loudly. clap's auto-generated `help` subcommand
  is disabled: `yqr help` keeps failing as an invalid filter instead of
  becoming a success, and `validate` stays the only word the subcommand
  namespace claims. The library gains the `validate` module (`check_str` /
  `encoding_diagnostic` / `render`).

### Removed

- **The `--engine` flag is gone.** yqr has settled on noyalib as its one and
  only YAML engine, so there is no backend to select: byte-preserving reads
  and surgical edits always run on noyalib's lossless CST. `--engine noyalib`
  now fails argument parsing instead of being accepted as a no-op; drop the
  flag from any invocation that used it (the behavior is unchanged without
  it). The experimental `skald` backend is retired with the seam: its name is
  no longer recognized anywhere.
- The library API lost `fidelity::BackendId`; `fidelity::open`,
  `fidelity::run`, `fidelity::run_ast`, and `fidelity::write::apply` no
  longer take a backend argument.

### Changed

- The YAML engine was upgraded from noyalib 0.0.14 to **0.0.17** (loader-parity
  fixes and a key-collision guard in 0.0.15, a build fix and MSRV 1.86 in
  0.0.16, a lockstep republish in 0.0.17). The CST edit API is unchanged, so
  the mutation-surface gaps yqr tracks upstream still stand.
- `clap` was upgraded from 4.6.1 to 4.6.4, and the remaining 29 transitive
  dependencies were refreshed (serde 1.0.229, serde_json 1.0.151, regex
  1.13.1, libc 0.2.189, zerocopy 0.8.55, and friends). `cargo audit` reports
  no advisories across the resulting 94-crate graph.
- The pinned Rust toolchain was updated from 1.97 to 1.97.1 (point release;
  no MSRV change -- `rust-version` stays 1.97).

## [0.4.0] - 2026-07-11

The fidelity write tier arrives: surgical, byte-preserving edits that change only
the bytes a filter targets and leave every other byte -- comments, indentation,
quoting, key order -- untouched, or refuse. In the same release, byte-preserving
reads become the default and the classic re-serializing pipeline moves behind
`--normalize`.

### Added

- **Write tier: surgical value edits.** yqr can now mutate a document through the
  fidelity engine, changing only the targeted bytes: assignment `.a.b = <rhs>`
  (scalar literal or a `.`-rooted path), append `.xs += <item>`, new-key assign
  `.a.new = <rhs>`, and `del(.a.b)`. Each edit passes through the engine's
  re-parse guard -- an edit that would restructure the document is refused
  (exit 5) rather than emitted, and scalar writes are quoted to match the
  neighbouring style. A filter is either a read-only query or a single mutation;
  mixing them is a parse error.
- **`-i` / `--in-place` flag** writes the mutated document back to the input file
  atomically (temp file + rename, `fsync` before rename, symlinks followed,
  owner-only temp permissions). Using `-i` with stdin or with a read-only filter
  is an error, diagnosed before any input is read. Without `-i`, the mutated
  document is printed to stdout (byte-exact except the edit).
- **Structural delete** of multi-line and nested block entries (e.g.
  `del(.spec.template)`), which the single-line delete path rejects. Flow-style
  and sole-entry deletes remain refused with a clear message.
- **`--normalize` / `-N` flag** for the classic re-serializing pipeline: it
  drops comments and canonicalizes scalars (e.g. `007` becomes `7`) -- the
  previous default behaviour.

### Changed

- **Byte-preserving reads are now the default.** `yqr '.' file.yaml` reproduces
  the input byte-for-byte -- comments, quoting, indentation, scalar spellings,
  and line endings survive -- with no flag. Untouched nodes are emitted as their
  original source bytes; computed, absent, and unaddressable nodes fall back to
  typed rendering per node.
- **`--engine` now selects the backend for the default (byte-preserving) read.**
  Under `--normalize` the classic pipeline runs and the engine choice has no
  observable effect beyond the up-front name validation.

### Breaking

- **`--preserve` / `-p` removed.** Byte preservation is now the default, so the
  flag is gone. Replace `yqr -p '.' f` with `yqr '.' f`; use
  `yqr --normalize '.' f` for the old re-serializing behaviour.

### Security

- Bumped the transitive `crossbeam-epoch` pin `0.9.18 -> 0.9.20` to clear
  RUSTSEC-2026-0204. It reaches the build only through the `criterion`
  dev-dependency (benchmarks), so released binaries were never affected; the
  change is lockfile-only.

## [0.3.0] - 2026-07-10

Byte/comment preservation becomes its own flag, decoupled from backend
selection.

### Added

- **`--preserve` / `-p` flag** for byte/comment-preserving reads. It turns on
  fidelity mode with the default backend, so `yqr -p '.' file.yaml` reproduces
  the input byte-for-byte — comments, quoting, indentation, and line endings
  survive.
- A runnable demo showcase under `docs/content/demo/` (`yqr-demo.sh` plus sample
  `deploy.yaml` / `config.yaml` inputs) walking through navigation, iteration,
  pipes, raw output, and preserve mode.

### Changed

- **`--engine` now selects the backend parser only** and no longer implies
  preservation. It picks *which* library performs a `--preserve` read (default
  `noyalib`); *whether* to preserve is `--preserve`'s job. Without `--preserve`,
  `--engine` has no observable effect.

### Breaking

- `--engine noyalib '.'` no longer preserves bytes on its own — use
  `--preserve` (optionally with `--engine noyalib` to name the backend
  explicitly). This decouples backend choice from fidelity mode.

## [0.2.1] - 2026-07-10

The first crates.io release. yqr consolidates on a single YAML engine, which
removes every git dependency and makes the crate publishable.

### Changed

- **One YAML engine.** yqr now uses noyalib for both the standard pipeline and
  byte-preserving reads. `--engine noyalib` (the fidelity engine) still emits
  untouched nodes as their original source bytes — comments, quoting,
  indentation, and line endings survive, and the identity filter reproduces the
  input byte-for-byte. It is always built in, so there are no backend build
  features to toggle.
- yqr now has its own value type instead of re-exporting the YAML library's, so
  the parser is a swappable internal detail. Library users of `yqr::Value` get a
  stable type that does not change when the engine does.
- Minimum supported Rust version is now 1.97 (the pinned toolchain was updated
  from 1.96).
- **Non-string mapping keys.** Integer, boolean, and composite keys (`1:`,
  `? [a, b]:`) are preserved byte-for-byte via `--engine noyalib`, but the
  standard re-serializing pipeline now renders them as strings, and a document
  that mixes keys colliding as strings (`1` and `"1"`) is rejected rather than
  kept distinct. This is rare in typical Kubernetes/CI/config documents — and
  GitHub Actions' `on:` is unaffected (it stays the string `on`).

### Removed

- The `rust-yaml` fidelity backend and the `--engine rust-yaml` option, along
  with the `backend-noyalib` / `backend-rust-yaml` build features and the
  `--no-default-features` minimal build. The fidelity engine is now always
  compiled in.

### Notes

- First release published to crates.io — install with `cargo install yqr`. (The
  0.2.0 tag below was a GitHub-only release that predates this consolidation.)
- `--engine` remains pluggable; an experimental `skald` engine is recognized for
  future comparison but is not built into the released binary.

## [0.2.0] - 2026-07-08

### Added

- Fidelity engines for byte-preserving reads, selectable at runtime with the
  new `--engine` flag. With `--engine noyalib` or `--engine rust-yaml`,
  untouched nodes are emitted as their original source bytes — comments,
  quoting, indentation, and line endings survive, and the identity filter
  reproduces the input byte-for-byte.
- Both fidelity backends now ship in the default build, so `--engine noyalib`
  and `--engine rust-yaml` are switchable in one binary without recompiling.
  Build with `--no-default-features` for a minimal binary that carries neither
  backend (the standard re-serializing pipeline still works; `--engine` then
  reports the backend as unavailable).
- A backend-agnostic fidelity round-trip harness (`tests/fidelity.rs`) that
  checks the `parse -> emit == input` property across backends, one case per
  formatting dimension (comments, blank lines, indent, quote/block/flow style,
  CRLF, BOM, multi-doc, anchors, numbers, key order).
- A shared real-world corpus (`tests/corpus/`) driving both the validation
  suite and the Criterion benchmarks from one case table (Kubernetes, GitHub
  Actions, Docker Compose, Helm, application config).
- Kubernetes usage guide in the documentation.

### Changed

- An unknown `--engine` value is now diagnosed before any input is read, so a
  typo is reported immediately instead of after consuming stdin or the file.

## [0.1.1] - 2026-06-21

### Changed

- Packaging only: dev-only files (`.agent/`, `.github/`, `specs/`, `AGENT.md`)
  are now excluded from the published crate. No functional or API changes —
  the compiled code is identical to 0.1.0; the source tarball is just slimmer
  (21 files vs 36).

This release supersedes 0.1.0, which has been yanked from crates.io.

## [0.1.0] - 2026-06-21

### Added

- Initial release (M0 foundation): a jq-style processor for YAML, operating
  natively on YAML via `rust-yaml` (no JSON round-trip).
- Filters: identity `.`, field access (`.foo`, `.a.b`, `.["k"]`), array
  indexing (`.[n]`, negative from end), iteration (`.[]`), pipe (`a | b`),
  and optional error suppression (`f?`).
- CLI with `--raw-output`, file/stdin input, and jq-style exit codes.
- `--version` reports the git commit, build timestamp, and target triple.
