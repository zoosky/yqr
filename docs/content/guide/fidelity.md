---
title: Byte-for-byte YAML editing, explained
lead: >-
  Why `yqr '.' f` reproduces `f` exactly, what survives a read, and when you want `--normalize` instead.
description: >-
  What yqr preserves when it reads and edits YAML, how to test it with the
  identity filter, and when you want the --normalize pipeline instead.
menu:
  title: Byte-for-byte
  order: 1
---

# Byte-for-byte, explained

Most YAML tools work by parsing your file into data, doing something to the
data, and printing the data back out. That last step is where formatting
goes to die: the printer has opinions about quoting and indentation, and
your file comes back wearing them.

<!-- Feature f009 made fidelity the default; f002 is the read floor. -->

yqr does something different. Nodes you did not touch are emitted as **the
original bytes from your file**, sliced straight out of the source. There is
no printer involved, so there are no opinions to apply.

## The identity test

The clearest way to see it is to ask for everything and change nothing:

```console
$ yqr '.' config.yaml | diff config.yaml -
$ echo $?
0
```

No output, exit 0. The file went through yqr and came back identical, down
to the byte. That holds for the awkward cases too -- CRLF line endings, a
byte-order mark, trailing whitespace, tabs inside strings, a flow collection
wrapped over several lines, multiple documents in one file.

It is a good thing to try on your own gnarliest config file. If it comes
back clean, everything below is safe.

## What survives

Take this file, which has a few things that normally do not survive a round
trip:

```yaml
defaults: &defaults
  mode: 0640      # octal, on purpose
  retries: 3

# Services below inherit the defaults.
web:
  <<: *defaults
  name: 'web'
```

Read it back with yqr and you get exactly that, including:

- **Comments**, and the whitespace that aligns them.
- **Blank lines**, which are doing real organisational work here.
- **Anchors, aliases, and merge keys**, in the spelling you wrote them.
- **Quote style** -- `'web'` stays single-quoted rather than becoming
  `"web"` or bare `web`.
- **Scalar spelling** -- `0640` stays `0640`.
<!-- Bug b011, fixed in noyalib 0.0.25. -->
- **Line breaks inside a flow collection** -- a `ports:` or `args:` list
  wrapped for width keeps its wrapping, closing bracket included.

That last one is worth dwelling on.

## Why `0640` is not `640`

Kubernetes spells file permissions in octal. If a tool re-types that scalar
as a number and prints it back, you get `640`, which is a different
permission. Same story for a version pinned at `1.10`, which becomes `1.1`
the moment something treats it as a float.

yqr never re-types the value, so the question never arises:

```console
$ yqr -r '.mode' config.yaml
0640
$ yqr -r '.ver' config.yaml
1.10
```

## When you want the opposite

Sometimes you genuinely want canonical output -- comparing two files that
are semantically equal but formatted differently, or feeding something
downstream that wants predictable shapes. That is what `--normalize` (`-N`)
is for.

```console
$ yqr --normalize '.' config.yaml
defaults:
  mode: 640
  retries: 3
web:
  name: web
  mode: 640
  retries: 3
```

Look at what that did, because it is instructive. Comments are gone. The
blank line is gone. The anchor and merge key have been resolved, so `web`
now carries its own copies of `mode` and `retries`. And `0640` has become
`640`.

Every one of those is correct as *data*. None of them is what you want
landing in a pull request. That is the trade, and it is why the byte-exact
path is the default and `--normalize` is the flag you have to ask for.

```console
$ yqr -rN '.mode' config.yaml
640
$ yqr -rN '.ver' config.yaml
1.1
```

## Editing works the same way

The guarantee extends to edits. Change one value and everything else is
untouched:

```console
$ yqr '.spec.replicas = 5' deploy.yaml
```

Only the bytes of that one scalar are replaced. The comment two spaces to
its right, the indentation, the key order, the quoting elsewhere in the
file -- all still the original bytes.

If an edit *cannot* be made without restructuring the document, yqr refuses
it and exits 5 rather than emitting something surprising. With `-i` the
file is left untouched on refusal, so a failed edit never leaves you with a
half-written file.

## Next

- [Editing Kubernetes manifests](kubernetes) -- this applied to a real
  workflow.
- [Validating YAML](validate) -- confirming a file is still correct
  afterwards.
