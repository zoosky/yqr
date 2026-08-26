---
# Traceability: Feature f017 (to_entries).
# The jq ordering note was corrected 2026-08-25 against jq 1.8.2: `keys`
# sorts, `to_entries` does not. See yqr-k002 §7.
# Bug b016 -- the emitter's trailing space, fixed in noyalib 0.0.27; the wart
# paragraph that stood here is gone with it.
title: Enumerating a mapping without losing the keys
lead: >-
  How to report what each entry of a mapping is about, with `to_entries`, when iterating throws the keys away.
description: >-
  Iterating a mapping gives you the values and throws away the names.
  to_entries keeps both, so one filter can report what each value is about.
menu:
  title: Enumerating mappings
  order: 4
---

# Enumerating a mapping

The most ordinary shape in YAML is a mapping of named things -- services,
environments, jobs, hosts. Iterating one gives you the **values**:

```console
$ cat services.yaml
services:
  alpha:
    domain: alpha.example.com
  beta:
    domain: beta.example.com
  gamma:
    tier: core

$ yqr -r '.services[].domain' services.yaml
alpha.example.com
beta.example.com
null
```

That is the right data and it cannot say what it is about. The names are
gone by the time the filter can use them.

## `to_entries`

`to_entries` turns a mapping into a list of pairs, so the name travels with
the value:

```console
$ yqr '.services | to_entries' services.yaml
- key: alpha
  value:
    domain: alpha.example.com
- key: beta
  value:
    domain: beta.example.com
- key: gamma
  value:
    tier: core
```

It takes its input from the pipe rather than wrapping a path, so it is
`<path> | to_entries`, not `to_entries(<path>)`. Iterate the pairs and reach
into either half:

```console
$ yqr -r '.services | to_entries[] | .key' services.yaml
alpha
beta
gamma

$ yqr -r '.services | to_entries[] | .value.domain' services.yaml
alpha.example.com
beta.example.com
null
```

`key` and `value` are the field names jq uses. They are worth nothing if
they do not transfer, so they are the same here.

### The order is your file's

Pairs come out in the order the entries were written, never sorted. That
matters more than it sounds: the two streams above line up entry for entry,
and they only line up because both keep document order and because a missing
field yields `null` rather than being skipped. `gamma` has no `domain`, and
it still gets a line.

jq's `keys` sorts, which is why it has a `keys_unsorted` beside it. Its
`to_entries` keeps insertion order, the same as yqr's -- so if that is the
habit you are bringing, it transfers.

### It is a query, not a place to write

The pairs are a view yqr invents; they exist in no file, so there is nothing
to write back to. Every write form is refused, with the reason:

```console
$ yqr '.services | to_entries = 1' services.yaml
yqr: parse error: 'to_entries' computes a value rather than naming one in the
document, so it cannot appear on the left of '=': there is nothing to write
back to. Read it with a query, or address the entry itself by path
```

For the same reason its output is **normalized rather than byte-preserved**:
comments and quote styles inside the printed pairs are yqr's, not your
file's. Everywhere else yqr hands back your own bytes -- see
[byte-for-byte](fidelity) -- but that promise is about nodes your filter
names, and these pairs are not in the document to be named.

### `to_entries` on anything but a mapping

Refused, naming what it actually got:

```console
$ yqr '.services.gamma.tier | to_entries' services.yaml
yqr: runtime error: to_entries takes an object, but this is string; it turns a
mapping's entries into {key, value} pairs, so there is nothing for it to
enumerate here
```

A sequence is refused the same way. jq refuses both too.

## `key(...)` reads a key; `to_entries` reads a mapping

There are two ways to get at a key, and they are not the same thing.

```console
$ cat quoting.yaml
m:
  "quoted": 1
  plain: 2

$ yqr 'key(.m[])' quoting.yaml
"quoted"
plain

$ yqr '.m | to_entries[] | .key' quoting.yaml
quoted
plain
```

`key(...)` hands back the **key token from your file**, quotes and all,
because it is a read of the document's own bytes. `to_entries` hands back the
**decoded string**, because its pairs are computed and have no bytes to
show; yqr then spells that string however it needs to.

The rule is one line: `key(...)` is what your file says, `to_entries` is what
it means. Ask for raw output with `-r` and the difference disappears, since
`-r` is a request for the value rather than the spelling:

```console
$ yqr -r 'key(.m[])' quoting.yaml
quoted
plain
```

Reach for `key(...)` when you are working on the document -- renaming a key,
or reporting exactly how one is written. Reach for `to_entries` when you are
working on the data.

## What is not here yet

`from_entries` -- the inverse -- is deliberately absent. It is only useful
once a filter can *build* pairs, which needs object construction and `map`,
and until then its only possible input is `to_entries`' own unmodified
output. `with_entries`, `select` and `map` are the same story: `to_entries`
makes the pairing available, and transforming it is the next piece of
language work.

## Next

- [Byte-for-byte, explained](fidelity) -- why a query that names a node
  gives you your own bytes back.
- [Editing Kubernetes manifests](kubernetes) -- the write side.
