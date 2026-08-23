---
# Traceability: the "Related tools" section answers the recurring question of
# whether yqr should grow a `format` subcommand. It should not -- yqr-a001 §8
# lists reflowing as a non-goal, and noyafmt already wraps noyalib's
# cst::format_with_config. Measured against noyafmt 0.0.27.
title: How yqr compares to other YAML tools
lead: >-
  Where yqr differs from the YAML tools you already have, measured against real files rather than argued.
description: >-
  Honest comparisons between yqr and the other tools you might reach for,
  measured against real files rather than argued from feature lists.
menu:
  title: Compare
  order: 3
---

# How yqr compares

There are good YAML tools already. These pages are about where yqr differs
from them, measured rather than asserted -- every claim here comes from
running both tools over the same file and diffing the result.

If a comparison here ever stops matching what you see, the other tool has
probably improved and this page needs re-measuring. The versions used are
named on each page so you can tell how stale it is.

- [yqr and yq](yq) -- the closest neighbour, and the one most people arrive
  from.

## Related tools, not competitors

Not everything in this space is a comparison. **yqr edits; `noyafmt`
formats.** They are different jobs, and they share an engine -- both are built
on [noyalib](https://crates.io/crates/noyalib), so they agree about what your
file means.

The difference is which bytes they are allowed to touch. Run both over the
same file:

```console
$ cat config.yaml
# Production cluster
defaults: &defaults
  mode:   0640      # octal, on purpose
  retries: 3
web:
  <<: *defaults
  name:   'web'
```

```console
$ noyafmt config.yaml
# Production cluster
defaults: &defaults
  mode: 0640 # octal, on purpose
  retries: 3
web:
  <<: *defaults
  name: 'web'
```

`noyafmt` closed up the alignment padding, everywhere, on purpose -- that is
what a formatter is for. It kept the comment, the anchor, the merge key, the
single quotes, and the octal `0640`, because it works through the same
lossless CST yqr reads.

```console
$ yqr '.' config.yaml | diff - config.yaml && echo identical
identical
$ yqr '.defaults.retries = 5' config.yaml | diff config.yaml -
4c4
<   retries: 3
---
>   retries: 5
```

yqr changed one line, because one line is what the filter named. It has no
opinion about the padding and never will -- [that is the
guarantee](/guide/fidelity), and a `format` subcommand would contradict it.

So reach for `noyafmt` when you want the file tidied, and yqr when you want
one value changed and everything else left alone. They compose in a pipe, and
`noyafmt --check` is the CI gate that exits 1 and names the files that need
formatting.

Install both with `cargo install noya-cli --locked`, which also ships
`noyavalidate` -- a JSON Schema 2020-12 checker with autofix. That is a
different question from yqr's [`validate`](/guide/validate), which asks
whether a file is *correct YAML* rather than whether it matches a schema.

Measured against `noyafmt 0.0.27`.
