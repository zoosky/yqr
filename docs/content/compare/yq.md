---
# Every command and its output on this page was run against yq v4.53.6 and a
# real yqr build (v0.7.1). Re-measure rather than re-assert when either moves;
# see yqr-k001 §7. Traceability: Feature f007 (the write tier the yqr examples
# use), f012 (validate), f017 (to_entries).
title: yqr and yq -- which tool for which job
lead: >-
  Two YAML tools with overlapping surfaces and different jobs. Here is what each one is good at, measured rather than argued.
description: >-
  When to reach for yqr and when to reach for yq: editing files that get
  reviewed, versus querying, building and converting them. Run against yq
  v4.53.6.
menu:
  title: yqr and yq
  order: 1
---

# yqr and yq

**yq is a good tool**, it is mature, and it does far more than yqr does. The
two are not really substitutes: yq is a YAML *processor*, and yqr is a YAML
*editor* with a fidelity guarantee. Most of the time the question is not
"which is better" but "which job am I doing right now".

This page answers that, with commands you can run.

## Which tool for which job

| The job | Reach for | Because |
|---|---|---|
| Change a value in a file that is checked into git | **yqr** | the diff is the line you changed, and nothing else |
| Read a value out of a file | **either** | they agree, including on `0640` and `1.10` |
| Answer a question across a document -- filter, count, select | **yq** | yqr has no `select`, `map`, or `length` |
| Build a document that does not exist yet | **yq** | yqr only edits documents that already do |
| Convert to JSON, XML, TOML | **yq** | yqr is YAML in, YAML out |
| Verify a file is correct before it ships | **yqr** | `validate --strict` catches what a parser accepts |
| Rename a key, edit a comment, reorder a list, in place | **yqr** | the rest of the file is not re-emitted |
| Do several things in one expression | **yq** | yqr applies one edit per run |

The rest of this page is the evidence for each row.

## What yqr is for

### Edits that a human is going to review

Take a Deployment with a comment aligned the way its author aligned it:

```yaml
spec:
  replicas: 3          # bumped for the Black Friday load test
```

Both tools bump it correctly. The difference is what else arrives in the
pull request:

```console
$ yq -i '.spec.replicas = 5' deploy.yaml && diff original.yaml deploy.yaml
9c9
<   replicas: 3          # bumped for the Black Friday load test
---
>   replicas: 5 # bumped for the Black Friday load test
```

```console
$ yqr -i '.spec.replicas = 5' deploy.yaml && diff original.yaml deploy.yaml
9c9
<   replicas: 3          # bumped for the Black Friday load test
---
>   replicas: 5          # bumped for the Black Friday load test
```

yq kept the comment, the quoting, the key order, the indentation and the
block style -- that is careful work, and most tools do far worse. The one
casualty is the comment's alignment.

On one line that is nothing. The reason it is on this page is that edits
like this arrive in batches: a script bumping an image tag across forty
manifests produces forty files whose untouched lines also moved, and the
review stops being a review.

### Reading a file back exactly as it was

Ask each tool to read a file and write it straight back out, changing
nothing. Here is one with an anchor, a merge key and a blank line doing
organisational work:

```yaml
defaults: &defaults
  mode: 0640      # octal, on purpose
  retries: 3

# Services below inherit the defaults.
web:
  <<: *defaults
  name: 'web'
```

```console
$ yqr '.' anchors.yaml | diff anchors.yaml - && echo identical
identical
```

```console
$ yq '.' anchors.yaml | diff anchors.yaml -
2c2
<   mode: 0640      # octal, on purpose
---
>   mode: 0640 # octal, on purpose
4d3
<
```

The comment gutter closed up and the blank line went. Neither is wrong
YAML -- both are decisions a printer made about a file nobody edited. yqr
has no printer: untouched nodes are emitted as the original bytes, sliced
out of your file.

### Verifying a file before it ships

A duplicate key is valid enough for a parser and almost never what anyone
meant. Later wins, silently:

```console
$ yq '.' dup.yaml
port: 8080
host: localhost
port: 9090
$ echo $?
0
```

```console
$ yqr validate --strict dup.yaml
error[Y101]: duplicate mapping key "port"
  --> dup.yaml:3:1
  |
3 | port: 9090
  | ^
  = note: first occurrence at line 1, column 1
  = help: later occurrences silently override earlier ones; remove or rename one
$ echo $?
1
```

Exit 1 and a location, which is what a CI gate needs. See
[Validating YAML](../guide/validate).

## What yq is for

### Asking questions across a document

Filtering, counting, selecting -- yqr has none of it, and this is the most
common reason to reach past it:

```console
$ yq '.items[] | select(.kind == "Service") | .metadata.name' pods.yaml
web
cache
$ yq '.items | length' pods.yaml
3
```

yqr can walk to a value and iterate a collection, but it cannot ask a
question about one. `to_entries` is its only builtin.

### Building documents that do not exist yet

```console
$ yq -n '.name = "web" | .spec.port = 8080'
name: web
spec:
  port: 8080
```

yqr has no object or array construction and no string interpolation. It
edits documents that already exist; there is nothing for it to preserve in
a file that is not there yet.

### Moving between formats

```console
$ yq -o=json '.metadata' deploy.yaml
{
  "name": "web",
  "labels": {
    "app": "web"
  }
}
```

JSON, XML, TOML, properties, CSV -- all yq, in both directions. yqr is YAML
in and YAML out, on purpose: the fidelity guarantee is about bytes in a
YAML file, and there is nothing to preserve once you leave the format.

### Doing several things at once

yq composes. One expression can filter, transform and write in a single
pass, and it can chain edits:

```console
$ yq -i '.a = 1 | .b = 2' config.yaml
```

yqr applies one edit per run, so that is two runs. Computed right-hand
sides do work -- `.n = .n + 1` and `.n |= (. + 1)` both do what they look
like, along with `+ - * / %` -- but comparisons, conditionals and chained
edits are on yq's side of the line.

## The same operations, spelled differently

Both tools rename keys, edit comments and reorder lists. If you move
between them, these are the shapes that differ.

yq spells a rename `(.a | key) = "z"` and a comment edit
`.a line_comment="hi"` -- two shapes, and the comment one is a silent no-op
when you point it at a key. yqr uses one shape for both, so learning either
teaches the other:

```console
$ yqr 'key(.a) = "z"' y.yaml
$ yqr 'line_comment(.a) = "hi"' y.yaml
$ yqr 'head_comment(.a) = "why this exists"' y.yaml
```

`head_comment` puts the block **above** the entry you addressed. yq's puts
it below, where in a file with siblings it ends up documenting the next
entry instead.

Reordering has no yq builtin at all -- no `swap`, no `move` -- so the
nearest equivalent rebuilds the sequence, and rebuilding re-emits the file:

```console
$ yq '.jobs.build.steps |= reverse' ci.yaml
name: ci                      # was 'name:    ci'
      - uses: actions/checkout@v4 # pinned    # was two spaces before '#'
$ yqr 'swap(.jobs.build.steps; 0; 1)' ci.yaml
name:    ci
      - uses: actions/checkout@v4  # pinned
```

Same reordering; in the yqr run every byte outside the two items is the
byte that was there before.

## Using both

The obvious pattern, and a good one:

```console
$ yq '.spec.template.spec.containers[] | .image' deploy.yaml   # explore
$ yqr -i '.spec.replicas = 5' deploy.yaml                      # commit the edit
```

Explore with the richer query language. Make the edit that lands in git
with the tool that will not touch anything else.

## Next

- [Byte-for-byte, explained](../guide/fidelity) -- what yqr is actually
  doing, and the one flag that turns it off.
- [Editing Kubernetes manifests](../guide/kubernetes) -- this applied to a
  real workflow.
