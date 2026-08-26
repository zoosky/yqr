---
# Traceability: yqr-k002. Every yqr filter here was run against v0.7.1 and
# every jq claim against jq 1.8.2; re-measure rather than re-assert when
# either moves (yqr-m001 §3 carries the release-time check).
# Features: f001 (the grammar), f008 (arithmetic and |=), f017 (to_entries),
# f007 (the write forms jq has no counterpart for).
title: Coming to yqr from jq
lead: >-
  Most of your jq muscle memory works unchanged. Here is the part that transfers, the one habit to unlearn, and where the two languages stop.
description: >-
  A guide for jq users picking up yqr: which jq idioms work unchanged, the
  one operator that means something different, and what each tool can do
  that the other cannot. Measured against yqr 0.7.1 and jq 1.8.2.
menu:
  title: Coming from jq
  order: 5
---

# Coming to yqr from jq

yqr borrows jq's shape on purpose. If you can write a jq path, you can
already read and edit a YAML file with yqr -- most of what you know
transfers with the same spelling and the same meaning.

This page is about the edges: the one operator that means something
different, the parts of jq that are not here, and the parts of yqr that jq
has no way to express.

Examples run against this file:

```yaml
# Edge services. Owned by the platform team.
name: web
replicas: 3
tags:
  - search
  - export
services:
  api:
    port: 8080      # public
  cache:
    port: 6379
```

## What transfers unchanged

| Your jq habit | In yqr |
|---|---|
| `.` | same |
| `.name`, `.a.b.c` | same |
| `.["name"]` | same |
| `.tags[0]`, `.tags[-1]` | same |
| `.tags[]` | same |
| `.services[]` | same |
| `a \| b` | same |
| `f?` | same |
| a missing field yielding `null` | same |
| `to_entries` | same |
| `del(.a)` | same |
| `+ - * / %`, and `+` on strings | same |
| `.a = 1` | same |
| `.a \|= (. + 1)` | same |
| `.n = .n + 1` | same |

So this is a yqr session, and there is nothing new in it:

```console
$ yqr -r '.name' config.yaml
web
$ yqr '.services.api.port' config.yaml
8080
$ yqr -r '.tags[-1]' config.yaml
export
$ yqr -r '.services[] | .port' config.yaml
8080
6379
```

Ordering is the same too, which is worth saying because it is the kind of
thing that bites silently: `to_entries` and `.[]` keep the order the
document was written in, exactly as jq's do. (jq's `keys` sorts -- that is
why `keys_unsorted` exists -- and yqr has neither.)

## One habit to unlearn: `+=`

This is the only operator that is spelled the same and means something
else, so it is the only thing on this page you have to actively remember.

In jq, `+=` is addition or concatenation, and appending to a list takes a
**list** on the right. In yqr, `+=` means **append one element to a
sequence**, and the right-hand side is the **element**:

| | jq 1.8.2 | yqr 0.7.1 |
|---|---|---|
| `.tags += ["x"]` | appends `x` | parse error -- yqr has no array literal |
| `.tags += "x"` | appends the characters `x` | appends `x` as one item |
| `.replicas += 1` | `4` | error -- `+=` wants a sequence |

To increment a number, use the update form or write the sum out. Both work:

```console
$ yqr '.replicas |= (. + 1)' config.yaml | sed -n 3p
replicas: 4
$ yqr '.replicas = .replicas + 1' config.yaml | sed -n 3p
replicas: 4
```

And to append:

```console
$ yqr '.tags += "audit"' config.yaml | sed -n '4,7p'
tags:
  - search
  - export
  - audit
```

## What jq has that yqr does not

yqr walks and edits documents that already exist. It does not compute over
data, and almost everything missing follows from that one line.

**No filtering or aggregation.** `select`, `map`, `length`, `keys`, `has`,
`add`, `join`, `sort_by`, `from_entries` -- none of them. `to_entries` is
the only builtin.

**No expressions that build values.** No object or array construction
(`{}`, `[]`), no string interpolation, no comma operator, no `//`, no
`if/then/else`, no comparisons, no `and`/`or`/`not`.

**No recursive descent.** `..` has no equivalent.

**No format conversion.** yqr is YAML in and YAML out. There is no `-o=json`,
and there should not be: the guarantee it exists to make is about the bytes
of a YAML file, and there is nothing to preserve once you leave the format.

**One edit per run.** jq composes a whole program; yqr applies one mutation.
Two changes are two commands.

Some of those fail with a message that teaches the model rather than just
refusing:

```console
$ yqr '.name, .replicas' config.yaml
yqr: lex error: unexpected character ',' at position 5: yqr has no ',' operator;
a function separates its arguments with ';', as in swap(.xs; 0; 1)
```

**When you need any of it, reach for jq** -- convert on the way in and pipe:

```console
$ yq -o=json '.' config.yaml | jq -c '[.tags[] | select(. == "export")]'
["export"]
```

That routes through [yq](../compare/yq), not yqr, because yq is the tool
that converts formats. Nothing about using yqr commits you to using it for
everything.

## What jq cannot do at all

The trade has another side, and it is the reason yqr exists.

jq reads JSON into a data model and prints the model back. Comments, quote
style, blank lines and alignment are not in that model -- they are not data,
so no jq filter can address them. yqr reads a lossless tree over your actual
bytes, so they are addressable:

```console
$ yqr -r 'line_comment(.services.api.port)' config.yaml
public
$ yqr 'line_comment(.replicas) = "bumped for the load test"' config.yaml | sed -n 3p
replicas: 3  # bumped for the load test
```

Keys and order are addressable the same way, without rebuilding the
collection they live in:

```console
$ yqr 'key(.tags) = "labels"' config.yaml | sed -n 4p
labels:
$ yqr 'swap(.tags; 0; 1)' config.yaml | sed -n '4,6p'
tags:
  - export
  - search
```

And the whole file survives a round trip, which no data-model tool can
promise:

```console
$ yqr '.' config.yaml | diff config.yaml - && echo identical
identical
```

That is the job yqr is for: [changing one value in a file that a human is
going to review](fidelity), and leaving every other byte where it was.

## Next

- [Byte-for-byte, explained](fidelity) -- the guarantee behind the last
  section.
- [yqr and yq](../compare/yq) -- the other YAML tool, and which jobs each
  one is for.
- [Enumerating mappings](enumerate) -- `to_entries`, the one builtin you
  already know.
