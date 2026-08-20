---
title: yqr and yq -- what actually changes in your file
description: >-
  A measured comparison of yqr and yq: both edit YAML well, but only one
  leaves every untouched byte alone. Run against yq v4.53.3.
menu:
  title: yqr and yq
  order: 1
---

# yqr and yq

Let's get the important part out of the way first: **yq is a good tool.** It
is mature, it does far more than yqr does, and if you need to build new
documents, evaluate expressions, or convert between formats, you should
reach for it and not for this.

<!-- Every command and its output on this page was run against yq v4.53.3
     and a real yqr build. Re-measure rather than re-assert when yq changes;
     see specs/marketing/yqr-k001-content-plan.md. -->

There is one thing yqr does that yq does not, and this page is about
exactly that thing.

## The short version

yq **normalizes** your file. yqr leaves it **byte-for-byte alone** apart
from the edit you asked for.

That difference does not show up as broken YAML -- yq's output is correct.
It shows up in code review, as lines nobody touched appearing in the diff.

## Where they agree

More than you might expect, so let's be concrete. Take a Deployment:

```yaml
# Web tier. Owned by the platform team.
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
  labels:
    app: web
spec:
  replicas: 3          # bumped for the Black Friday load test
  template:
    spec:
      containers:
        - name: web
          image: "registry.example.com/web:1.4.2"
          ports:
            - containerPort: 8080
```

Bump the replica count with each tool and diff against the original.

```console
$ yq -i '.spec.replicas = 5' deploy.yaml
$ diff original.yaml deploy.yaml
9c9
<   replicas: 3          # bumped for the Black Friday load test
---
>   replicas: 5 # bumped for the Black Friday load test
```

yq kept the comment, the quoting, the key order, the indentation, and the
block style. That is careful work. The only casualty is the alignment of
the trailing comment, which got collapsed to a single space.

```console
$ yqr -i '.spec.replicas = 5' deploy.yaml
$ diff original.yaml deploy.yaml
9c9
<   replicas: 3          # bumped for the Black Friday load test
---
>   replicas: 5          # bumped for the Black Friday load test
```

Same edit, and the alignment survives.

On **reads**, the two agree completely. Both give you `0640` for an octal
file mode and `1.10` for a version string -- neither one re-types your
scalars behind your back.

## Where they diverge

The gap widens on a whole-file round trip. Here is a file with an anchor, a
merge key, and a blank line doing organisational work:

```yaml
defaults: &defaults
  mode: 0640      # octal, on purpose
  retries: 3

# Services below inherit the defaults.
web:
  <<: *defaults
  name: 'web'
```

Ask each tool for the identity filter -- read it and write it back out,
changing nothing.

```console
$ yqr '.' anchors.yaml | diff anchors.yaml -
$ echo $?
0
```

Nothing. The output is byte-identical to the input.

```console
$ yq '.' anchors.yaml | diff anchors.yaml -
2c2
<   mode: 0640      # octal, on purpose
---
>   mode: 0640 # octal, on purpose
4d3
<
7c6
<   <<: *defaults
---
>   !!merge <<: *defaults
```

Three changes to a file nobody edited:

1. **Comment alignment collapsed.** Cosmetic, but it is your cosmetic.
2. **The blank line was deleted.** That line was separating two sections.
3. **`<<: *defaults` became `!!merge <<: *defaults`.** yq made an implicit
   merge key explicit by adding a tag that was not in your source.

None of that is wrong YAML. All of it lands in your pull request.

## Why this matters (and when it doesn't)

If you are generating YAML, piping it somewhere, or reading values out of
it, none of this matters and yq is the better tool by a mile.

It starts mattering when the file is **checked in and reviewed by humans**.
A one-line change should be a one-line diff. When a script bumps an image
tag across forty manifests and each one comes back with reshuffled comments
and stripped blank lines, the review stops being a review.

That is the whole of yqr's argument. It is a narrow argument, and it is the
only one being made here.

## What yq does that yqr does not

Plenty, and it is worth knowing before you switch:

- **Building new documents.** yqr has no object or array construction, no
  string interpolation, and no comma operator. It walks and edits documents
  that already exist.
- **Builtins and arithmetic.** No `map`, `select`, `length`, `+` on values.
- **Format conversion.** No JSON, XML, TOML, or properties in or out.
- **Everything above, in one expression.** yq composes; yqr applies one edit
  per run. Chaining, conditionals, and computed right-hand sides all live on
  the yq side of the line.

If you need any of those, use yq. The tools are not really substitutes;
one of them is a scalpel with a very short blade.

## Where the shapes differ on purpose

<!-- Feature f007 -->

Both tools can rename a key, edit a comment, and reorder a list. The
spellings are not the same, and the differences are deliberate.

yq spells a rename `(.a | key) = "z"` and a comment edit `.a
line_comment="hi"` -- two different shapes, and the one that works for
comments is a **silent no-op** for keys (measured on v4.53.3). yqr uses one
shape for both, so learning either teaches the other:

```console
$ yqr 'key(.a) = "z"' y.yaml
$ yqr 'line_comment(.a) = "hi"' y.yaml
$ yqr 'head_comment(.a) = "why this exists"' y.yaml
```

`head_comment` puts the block **above** the entry you addressed. yq's
`head_comment` puts it below, where in a file with siblings it comes to
document the next entry instead.

Reordering is the one operation with no yq builtin at all -- there is no
`swap` or `move` -- so the nearest equivalent rebuilds the sequence, and
rebuilding re-emits the document:

```console
$ yq '.jobs.build.steps |= reverse' ci.yaml
name: ci                      # was 'name:    ci'
      - uses: actions/checkout@v4 # pinned    # was two spaces before '#'
$ yqr 'swap(.jobs.build.steps; 0; 1)' ci.yaml
name:    ci
      - uses: actions/checkout@v4  # pinned
```

Same reordering, and in the yqr run every byte outside the two items is the
byte that was there before.

## Using both

Nothing stops you. A pattern that works well:

```console
$ yq '.spec.template.spec.containers[] | .image' deploy.yaml   # explore
$ yqr -i '.spec.replicas = 5' deploy.yaml                      # commit the edit
```

Read and explore with the tool that has the richer query language. Make the
edit that lands in git with the one that will not touch anything else.

## Next

- [Byte-for-byte, explained](../guide/fidelity) -- what yqr is actually
  doing, and the one flag that turns it off.
- [Editing Kubernetes manifests](../guide/kubernetes) -- the workflow this
  page keeps alluding to.
