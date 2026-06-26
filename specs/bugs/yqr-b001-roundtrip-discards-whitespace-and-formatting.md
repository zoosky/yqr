# Bug b001 — Round-trip through `rust-yaml` discards whitespace, comments, and formatting

**Status:** Open (confirmed, reproducible)
**Severity:** High — violates the core product guarantee ratified in `yqr-a001`
**Owner:** yqr maintainers
**Last updated:** 2026-06-26
**Affects:** every `yqr` invocation (read/query *and* identity); blocks `yqr-a001` §2
**Related:** `yqr-a001` (Fidelity-First architecture), `yqr-r001` §5 (YAML-native gaps), `yqr.f001` §2 (Goals)
**Component:** `rust-yaml` 1.1.0 (load/compose/emit pipeline) as consumed by `src/lib.rs`

## 1. Summary

`yqr` loads YAML through `rust_yaml::Yaml::load_str` into a `rust_yaml::Value`
tree, then re-serializes with `dump_str` (`src/lib.rs:27-56`). `Value` is a
**purely semantic tree** — it records the *data* but none of the *source form*.
As a result the round trip **rewrites the entire file**: comments, blank lines,
indentation width, quote style, block-scalar style, flow style, line endings,
and trailing whitespace are all normalized away, and several constructs
(multi-document streams, anchors/aliases, out-of-range integers, BOM) are
**silently corrupted or dropped**.

This directly breaks the non-negotiable guarantee in `yqr-a001` §1:

> Comments, key ordering, and invisible characters … present in the input are
> preserved in the output. yqr never rewrites bytes it did not change.

The north-star acceptance test from `yqr-a001` §2 — `yqr '.' f` is byte-for-byte
identical to `f` — **fails for 13 of 14 files** in the reproduction corpus
below. The one "passing" file (BOM) only passes by accident and is actually
corrupted (see §5.3).

## 2. Impact

- **The product promise is unmet today.** `yqr-a001` makes fidelity the top
  priority for Cohort B (Kubernetes / Helm / Ansible / CI config wranglers). A
  tool that reformats a Helm `values.yaml` or a K8s manifest on every read is
  unusable for that cohort — it vandalizes diffs and destroys human structure.
- **It affects reads, not just `yqr .`.** Because the *read/query* path also
  re-serializes (`render` calls `dump_str` per result — `src/lib.rs:49-52`),
  even a projection such as `yqr '.spec.template' deploy.yaml` strips the
  comments and formatting of the selected subtree (verified, §5.2).
- **Some failures are data loss, not just cosmetic reformatting:**
  - Multi-document input (`---`): only the **first** document survives; the rest
    vanish with no error (§5.1 case 13).
  - Anchors / aliases / merge keys: the `&anchor`/`*alias` relationship is
    destroyed and `<<` merges are eagerly materialized (§5.1 case 09).
  - Integers outside `i64` are coerced to `f64`, losing precision **and**
    flipping the type (§5.2).
  - A leading BOM is folded into the first key, so `.a` no longer matches (§5.3).
- **No emitter setting fixes this.** The loss is structural — it happens during
  `load`/`compose` before the emitter ever runs, and the emitter reconstructs
  formatting from the `Value` alone. See §6 and §7.

## 3. Environment

- `yqr` `0.1.1`, branch baseline (commit `e234b55`).
- `rust-yaml` `1.1.0` — consumed from crates.io (`Cargo.lock`:
  `source = "registry+https://github.com/rust-lang/crates.io-index"`,
  `checksum = 3597280f…`). The local checkout at `../yqr-deps/rust-yaml`
  (HEAD `f2d1e3f`) is the **same 1.1.0 release** plus CI/docs/profiling-only
  commits; all source citations in §6 are against that tree.
- yqr does **not** pin a `[patch]`/`path` override — it ships the published
  crate. The local checkout is the matching source, used here for root-cause.

## 4. How to reproduce

```bash
cargo build
# Identity must be a no-op under yqr-a001 §2; it is not:
diff <(cat input.yaml) <(./target/debug/yqr '.' input.yaml)
```

The full reproduction corpus and driver used for §5 are reproducible with these
inputs (each isolates one fidelity dimension):

| File | Construct exercised |
|------|---------------------|
| `01-comments` | leading / inline / section / nested comments + blank lines |
| `02-blanks` | blank lines between keys |
| `03-indent4` | 4-space indentation |
| `04-quotes` | bare / single / double / escaped-quote scalars |
| `05-blockscalars` | literal `\|` and folded `>` block scalars |
| `06-numbers` | leading-zero, float, large `i64`, negative |
| `07-flow` | flow mappings/sequences `{}` / `[]` |
| `08-ordering` | non-alphabetical key order |
| `09-anchors` | `&anchor`, `*alias`, `<<` merge key |
| `10-crlf` | CRLF line endings |
| `11-bom` | UTF-8 BOM prefix |
| `12-trailing` | trailing spaces / tabs |
| `13-multidoc` | multiple `---` documents |
| `14-k8s` | realistic manifest (comments + blanks + nesting) |

## 5. Observed behavior (verified empirically)

`./target/debug/yqr '.' <file>` was run over every corpus file and the output
diffed byte-for-byte against the input. **13 / 14 differ.**

### 5.1 Symptom matrix

| Dimension | Input | `yqr '.'` output | Verdict |
|-----------|-------|------------------|---------|
| Leading/inline/section/nested comments | `# header` / `x: 1  # note` | all comment text removed | LOST |
| Blank lines | `a: 1`⏎⏎`b: 2` | blank lines collapsed | LOST |
| Indent width | 4-space nested | re-emitted at 2 spaces | NORMALIZED |
| Single quotes | `single: 'hello world'` | `single: hello world` | NORMALIZED |
| Double quotes | `double: "hello world"` | `double: hello world` | NORMALIZED |
| Type-forcing quotes | `forced: "123"` | `forced: "123"` (kept) | preserved (incidental) |
| Escaped quote | `s: 'it''s a test'` | `s: it's a test` | NORMALIZED |
| Literal block scalar | `\|`⏎`  line one`⏎`  line two` | `"line one\nline two\n"` | REWRITTEN |
| Folded block scalar | `>`⏎`  this is`⏎`  folded text` | `"this is folded text\n"` | REWRITTEN |
| Leading-zero number | `zip: 007` | `zip: 7` | VALUE CHANGED |
| Flow collections | `{a: 1, b: 2}` / `[1, 2, 3]` | expanded to block style | NORMALIZED |
| Key order | `zebra/apple/mango` | unchanged | PRESERVED |
| Anchor + merge | `&defaults` … `<<: *defaults` | anchor dropped, merge expanded inline | CORRUPTED |
| CRLF | `a: 1\r\n` | `a: 1\n` | NORMALIZED |
| Trailing whitespace | `a: 1␠␠␠\n` | `a: 1\n` | STRIPPED |
| Multi-document | `---`/`a:1`/`---`/`b:2` | only `a: 1` emitted | **DATA LOSS** |
| BOM | `\xEF\xBB\xBFa: 1` | bytes survive, but `.a` → `null` | CORRUPTED (see §5.3) |

### 5.2 Read/query path and value precision

The loss is not confined to identity. Projections re-serialize the selected
subtree:

```
$ yqr '.config' 01-comments.yaml      # drops the `# nested comment`
debug: true
level: info

$ yqr '.spec.template' 14-k8s.yaml     # drops `# pin the tag`, blank lines
spec:
  containers:
    - name: web
      image: nginx:1.25
```

Integer precision/type is silently corrupted for values beyond `i64`:

```
$ echo 'id: 123456789012345678901234567890' | yqr '.'
id: 123456789012345677877719597056.0     # coerced to f64: precision + type lost
```

### 5.3 The BOM case is corruption, not preservation

`11-bom` is the only file whose bytes round-trip identically, so a naive
byte-diff reports "IDENTICAL". It is not faithful — there is no BOM handling in
`rust-yaml` at all (§6.6), so the `U+FEFF` is absorbed into the first key. The
mapping key becomes `"\u{FEFF}a"`, and key access breaks:

```
$ yqr '.a' 11-bom.yaml
null        # the key is "﻿a", not "a"
```

So the BOM "survives" only because it was mis-parsed as ordinary content; it is
data corruption that happens to be byte-stable for the trivial identity filter.

### 5.4 Note: the mutate path does not exist yet

`yqr` has no assignment grammar today (`yqr '.a.b = 5'` is a lex error, exit 3;
confirmed in `yqr-r001` §4.2). The fidelity guarantee therefore **already fails
on pure reads**, before any in-place mutation feature is built. Fixing this is a
prerequisite for the surgical-edit/mutate path described in `yqr-a001` §4.2.

## 6. Root-cause analysis (`rust-yaml` 1.1.0 source)

yqr's pipeline is `text → load_str → Value → dump_str → text`. The format is
lost at three points along that path; the emitter is the *last* place it could be
recovered, but by then the information is already gone.

```
 input bytes
     │  Yaml::load_str            (yaml.rs)  ── reads FIRST document only
     ▼
 scanner Tokens  ── carry byte spans, QuoteStyle, block-scalar token kinds … [DISCARDED]
     │  parser → ScalarStyle (Literal/Folded/quoted) … [DISCARDED at compose]
     ▼
 compose_scalar (composer.rs:320-341)  ── quote style collapsed; type-coerced
     ▼
 Value  (value.rs:227-242)  ── purely semantic: no spans, no style, no comments
     │  Yaml::dump_str → emit_yaml_value (yaml.rs:315-322)
     ▼
 BasicEmitter::emit (emitter.rs)  ── reconstructs formatting from Value ALONE
     ▼
 output bytes  (normalized)
```

### 6.1 `Value` is a purely semantic tree (no place to store form)

`value.rs:227-242` — the enum has no span, style, comment, or anchor field:

```rust
pub enum Value {
    Null, Bool(bool), Int(i64), Float(f64), String(String),
    Sequence(Vec<Value>),
    Mapping(IndexMap<Value, Value>),   // IndexMap → key ORDER is the one thing preserved
}
```

Because `Value` cannot represent source form, **everything downstream of
`compose` is reconstructed**, and everything upstream of it is thrown away.

### 6.2 The compose step collapses quote style and coerces types

`composer.rs:320-341` `compose_scalar` is the precise drop point for yqr's load
path (the main composer, not the comment composer):

```rust
match style {
    ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted =>
        return Ok(Value::String(value)),   // single vs double collapsed to bare String
    _ => {}
}
Ok(match resolve_plain_scalar(&value, self.yaml_version) { … })  // plain → type-coerced
```

- Single- and double-quoted scalars both become a bare `Value::String` — the
  quote *style* is gone (explains `'hello world'`/`"hello world"` → bare).
- Plain **and** block (`Literal`/`Folded`) scalars fall through to
  `resolve_plain_scalar` (`resolver.rs`), which type-coerces:
  - `resolver.rs:68` `value.parse::<i64>()` → `007` becomes `Int(7)`
    (leading-zero loss; `06-numbers`).
  - `resolver.rs:104` collapses `~` / `null` / empty into one `Value::Null`
    spelling.
  - Block scalars are *mis-routed* here too: a literal block whose text happens
    to read as `true`/`123` can be silently retyped, because `compose_scalar`
    only special-cases the quoted styles, not the block styles.
- The block-scalar **header and content** (the `|`/`>`, chomping `+`/`-`, indent
  indicator, trailing-newline policy) are already gone before compose: the
  parser dedents/chomps the content (`parser/mod.rs:1559-1607`) and only the
  `ScalarStyle` tag (`parser/events.rs:90-101`, incl. `Literal`/`Folded`)
  survives — and that tag is dropped at compose.

### 6.3 The emitter reconstructs formatting from `Value` alone

yqr's dump path is `dump_str` → `emit_yaml_value` (`yaml.rs:315-322`), which
builds a `BasicEmitter` from only the configured indent and anchor flags and
calls `emit(&Value)`. The emitter struct (`emitter.rs:36-48`) has **no
source-text, `Position`, or byte-offset field** — it cannot consult the original
bytes even in principle. Consequently:

| Output trait | Emitter behavior | Citation |
|--------------|------------------|----------|
| Quote style | `needs_quoting` type-ambiguity heuristic; defaults to **double** quotes when quoting is required, bare otherwise — so `"123"` stays quoted (parses as int) but `'hello world'` goes bare | `emitter.rs:351-360,365,380-382` |
| Block scalars | **no `\|`/`>` emission exists**; any string with `\n`/`\r`/`\t` is double-quoted and the newline escaped to `\n` | `emitter.rs:391-394,446` |
| Flow vs block | collections are **always block**; flow only for complex (collection) keys, never ordinary values | `emitter.rs:482-489,700-713` |
| Indent width | from the `indent` config field (default `IndentStyle::Spaces(2)`), never from source | `emitter.rs:225-232`; `value.rs:65-68` |
| Line endings | always `\n` via `writeln!`; no CRLF path | `emitter.rs:937` |
| Trailing whitespace / blank lines | no representation anywhere | (absent) |

### 6.4 The comment-preserving API is insufficient and unused

`rust-yaml` does ship a comment path (`CommentedValue` + the comment composer),
but it cannot deliver byte fidelity and yqr does not use it:

- `CommentedValue.value` is a **plain `Value`** tree (`value.rs:127-134`) — so
  it inherits every loss in §6.1–6.3 for the data itself.
- Comments are stored as **trimmed semantic strings** attached via a fuzzy
  `HashMap` line-proximity heuristic (`composer_comments.rs:73-99`), not as
  byte spans; blank lines are not captured.
- Every node's style is hard-coded to `Style::default()`
  (`composer_comments.rs:184,244,304`).
- It is **single-document and non-recursive**; there is **no
  `load_all_str_with_comments`**.
- yqr's `load_str` does not invoke it at all (`src/lib.rs:29-31`).

There **is** an opt-in round-trip path —
`Yaml::with_loader(LoaderType::RoundTrip)` with `config.preserve_comments`
(and `preserve_quotes`), via `load_str_with_comments` → `dump_str_with_comments`.
It is not the default (`Yaml::new()` is the lossy `Safe` loader, and
`with_loader(RoundTrip)` does **not** auto-enable `preserve_comments`), and it
does not close the gap. Tested against this bug's corpus, the round-trip path:
preserves leading/standalone comments and key order, but still loses inline
comments, blank lines, quote style (despite `preserve_quotes`), block/flow style,
indentation width, CRLF, trailing whitespace, the trailing newline, number
lexemes, multi-document, and anchors — **0 of 14 dimensions round-trip
byte-for-byte**. Worse, `dump_str_with_comments` currently **duplicates and
relocates comments**: it strips inline comments from the body and appends a
trailing block repeating each comment several times (minimal repro: input
`# header\nname: app   # inline\nport: 8080\n`). Reported upstream as
[elioetibr/rust-yaml#72](https://github.com/elioetibr/rust-yaml/issues/72)
(alongside the maintainer's existing #29 quote-style and #40 comment-edit issues).

So even adopting the comment API would, at best, re-emit normalized comments on a
still-reformatted document — and today it mis-emits them outright — not the
byte-faithful output `yqr-a001` requires.

### 6.5 Multi-document loss is a `load_str` API choice

`Yaml::load_str` returns only the first document; `load_all_str` /
`dump_all_str` exist (`yaml.rs:182,211`) but yqr calls the single-document form
(`src/lib.rs:29`). This one is **partially mitigable** by switching APIs — but
`dump_all_str` emits only a bare `---` between documents and ignores explicit
`---`/`...` markers and directives (`yaml.rs:218-226`), and there is no
multi-document *comment* path, so multi-doc + comments together remain
impossible via the `Value` pipeline.

### 6.6 No BOM handling exists

Input is read with `read_to_string` (`yaml.rs:173-174`) and the scanner builds
its char cache via `input.chars().collect()` (`scanner/mod.rs:132`) with no BOM
strip anywhere in the source (grep for `bom`/`feff` is empty). A leading
`U+FEFF` is therefore swallowed into the first token (the corruption shown in
§5.3), not handled as a document BOM.

### 6.7 Dead configuration knobs

`YamlConfig` (`yaml.rs:11-44`) exposes ~12 fields that the emitter never reads,
and there is **no** knob for blank lines, CRLF, BOM, trailing whitespace,
flow-vs-block, block scalars, or anchor-name preservation. Tuning config cannot
close the gap.

## 7. Why no emitter/config change can fix this

The information needed to reproduce the input is destroyed during
`load`/`compose`, before the emitter runs, and `Value` has nowhere to carry it
(§6.1–6.2). A semantic round-trip (`text → Value → text`) is **structurally
incapable** of byte fidelity for untouched regions — exactly the conclusion of
`yqr-a001` §3. Polishing `BasicEmitter` (block-scalar support, quote-style
hints, indent inheritance) would reduce *some* normalization but can never reach
the §2 byte-for-byte property, because the emitter is reconstructing from a lossy
model.

## 8. The raw material for a fix exists (and is public)

This bug is fixable without forking `rust-yaml`. The scanner/token layer already
carries the form information that `Value` drops, and it is part of the public
API yqr can consume:

- Tokens carry **byte-accurate** start/end `Position` (`scanner/tokens.rs:18-26`);
  `Position.index` is a 0-based **byte** offset (`position.rs:12-13`) advanced
  via `len_utf8` (`position.rs:41-47`) — correct for multi-byte text.
- Tokens carry `QuoteStyle` (`scanner/tokens.rs:8-15`) and distinct
  `BlockScalarLiteral` / `BlockScalarFolded` / `Comment` token kinds.
- The parser computes the full `ScalarStyle` set including `Literal`/`Folded`
  (`parser/events.rs:90-101`).
- The scanner even detects the source indent width
  (`scanner/mod.rs:99,243,601`, accessor `detected_indent_style()`).
- `Scanner`, `BasicScanner`, `Token`, `TokenType`, `Position`, and `QuoteStyle`
  are all re-exported (`lib.rs:118-119,142`).

This is the substrate for the **source-preserving, span-based** model in
`yqr-a001` §4: build a `path → byte-span` index from the token stream, **slice**
original bytes for unchanged/selected regions, and **splice** only the bytes that
actually change. That makes untouched regions byte-identical *by construction*.

## 9. Corrections to `yqr-a001` assumptions

The source review confirmed `a001`'s core thesis but corrected four specifics
(worth folding back into `a001` when it is next revised):

1. **`Position::advance_by` multi-byte hazard (a001 §7) is not live.** The
   raw-count `advance_by` is effectively dead code on the load path; the
   exercised path uses `advance` with `len_utf8`. Span precision risk is lower
   than `a001` feared (still validate against non-ASCII before relying on it).
2. **Indent width is not "never recorded" (a001 §3/§4).** The scanner *detects*
   it (`scanner/mod.rs:601`) but uses it only for validation and then discards
   it. It is loss-on-discard, and yqr can read the accessor.
3. **Block-scalar style is represented internally.** `ScalarStyle`
   (`parser/events.rs:90-101`) has `Literal`/`Folded`; the loss is at
   composition/emission, not for lack of a style vocabulary. (The 3-variant
   scanner `QuoteStyle` is not the whole story.)
4. **BOM is not preserved.** `a001`/early notes implied BOM pass-through; there
   is no BOM handling at all — it is corruption (§5.3, §6.6).

## 10. Adjacent value-fidelity losses (related; may warrant separate bugs)

These share the same root cause (`Value` as a lossy semantic tree) but are about
*value/typing* rather than whitespace. Filed here for visibility; split out if
they need independent tracking:

- **Numeric form normalization** beyond `007`→`7`: hex/octal/binary
  (`resolver.rs:111+`), `+5`, `1e3`, `.inf`/`.nan`, sexagesimal, etc. each
  re-emit in a single canonical form.
- **Out-of-`i64` integers → `f64`** (precision + type loss, §5.2).
- **`null` spelling** (`~` / `null` / empty → one form, `resolver.rs:104`).
- **Tab indentation → spaces** (tabs detected but the bare dump path emits
  configured spaces).
- **Document markers / directives** (`%YAML`, `%TAG`, explicit `---`/`...`) not
  round-tripped (`yaml.rs:218-226`).
- **Duplicate keys** undefined (`allow_duplicate_keys` is a dead knob).

## 11. Remediation options

| Option | Closes | Verdict |
|--------|--------|---------|
| A. Tune `BasicEmitter` / set `YamlConfig` | a few normalization cases | **Insufficient** — cannot reach §2 (see §7); most knobs are dead (§6.7) |
| B. Adopt `CommentedValue` / `RoundTrip` loader | normalized comments only | **Insufficient (and buggy today)** — still reformats; single-doc; lossy strings; `dump_str_with_comments` duplicates/relocates comments ([rust-yaml#72](https://github.com/elioetibr/rust-yaml/issues/72)) (§6.4) |
| C. Switch `load_str`→`load_all_str` + `dump_all_str` | multi-document survival | **Partial** — fixes data loss in §5.1/13 only; everything else still normalized |
| D. **Source/span layer over the scanner token stream** (slice unchanged bytes, splice only edits) | the whole guarantee | **Recommended** — the `yqr-a001` §4 architecture; the only path to §2 |

**Recommendation:** pursue Option D (tracked by the `f002` source-preserving
read-path spec proposed in `yqr-r001` §9). Option C is a cheap, independent win
for the multi-document data-loss case and can land first.

## 12. Acceptance criteria (definition of done)

This bug is resolved when:

- [ ] `yqr '.' f` is **byte-for-byte identical** to `f` across the entire §4
      corpus (the `yqr-a001` §2 north-star property), enforced by a fidelity
      test corpus committed under `tests/` (per `yqr-a001` §5).
- [ ] Comments (leading/inline/section/nested) and blank lines survive a read
      and an identity round trip.
- [ ] Indentation width, quote style, block-scalar style (`|`/`>`), and
      flow-vs-block style are preserved for untouched nodes.
- [ ] CRLF line endings and trailing whitespace are preserved.
- [ ] Multi-document streams round-trip (no silent document drop).
- [ ] Anchors/aliases/merge keys round-trip without eager expansion, or the
      limitation is explicitly documented and errors loudly (per `yqr-a001` §7).
- [ ] A leading BOM is preserved as a BOM and does not corrupt the first key.
- [ ] Integers within range stay `Int`; out-of-range integers do not silently
      become lossy `Float` (covered by the `yqr-a001` §6 number model).
- [ ] A regression test asserts the §2 property in CI.

## 13. Appendix — citation index (`rust-yaml` 1.1.0)

| Concern | File:line |
|---------|-----------|
| `Value` semantic enum (no spans/style) | `value.rs:227-242` |
| `IndentStyle` default `Spaces(2)` | `value.rs:65-68` |
| `CommentedValue` wraps plain `Value` | `value.rs:127-134` |
| `LoaderType` (default `Safe`); `RoundTrip` variant | `yaml.rs:48-57,75` |
| `with_loader` does not enable `preserve_comments` | `yaml.rs:145-149` |
| Opt-in comment round-trip API | `yaml.rs:229-249` (`load_str_with_comments`/`dump_str_with_comments`) |
| Quote-style collapse + type coercion (yqr's path) | `composer.rs:320-341` |
| `007`→`Int(7)`; null spellings; hex/oct/bin | `resolver.rs:68,104,111+` |
| Block-scalar dedent/chomp at parse | `parser/mod.rs:1559-1607` |
| Full `ScalarStyle` incl. Literal/Folded | `parser/events.rs:90-101` |
| dump path builds emitter from config only | `yaml.rs:197-201,315-322` |
| Emitter struct has no source/position field | `emitter.rs:36-48` |
| Quote heuristic / default double | `emitter.rs:351-360,365,380-382` |
| No block-scalar emission; `\n` escaped | `emitter.rs:391-394,446` |
| Always block; flow only for complex keys | `emitter.rs:482-489,700-713` |
| Indent from config | `emitter.rs:225-232` |
| Always-LF final newline | `emitter.rs:937` |
| Comment heuristic / hard-coded `Style::default()` | `composer_comments.rs:73-99,184,244,304` |
| `load_all_str` / `dump_all_str`; bare `---` | `yaml.rs:182,211,218-226` |
| No BOM handling (read_to_string / char cache) | `yaml.rs:173-174`; `scanner/mod.rs:132` |
| Dead `YamlConfig` knobs | `yaml.rs:11-44` |
| Byte-accurate token spans | `scanner/tokens.rs:18-26`; `position.rs:12-13,41-47` |
| Scanner detects indent width | `scanner/mod.rs:99,243,601` |
| Public re-exports (Scanner/Token/Position/QuoteStyle) | `lib.rs:118-119,142` |

*Method: root cause was produced by reading the `rust-yaml` 1.1.0 source across
the load/compose/emit layers; every claim above was independently
re-verified against the source (0 of 64 claims refuted). The empirical symptom
matrix (§5) was obtained by running the compiled `yqr` binary over the §4
corpus.*
