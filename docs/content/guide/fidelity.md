---
# Traceability: Feature f009 made fidelity the default; f002 is the read
# floor. Bug b011 (wrapped flow collections) fixed in noyalib 0.0.25.
# The no-op write rule and the borrowed-entry refusal are bugs b018 and
# b019; the merged-key message is b020. Filling in a blank value is b021,
# fixed in noyalib 0.0.28.
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

A key someone left blank is an ordinary target. A key with nothing after it
is an implicit null: it reads as `null`, and writing to it fills the value in
on the line that is already there. Given this `image.yaml`:

```yaml
image: registry.example.com/web:1.4.2
replicas: 2
digest:          # filled by the release job
```

```console
$ yqr '.digest = "sha256:9f0a"' image.yaml
image: registry.example.com/web:1.4.2
replicas: 2
digest: sha256:9f0a          # filled by the release job
```

The value goes *before* the comment, and the gutter the author wrote is
still there. The same holds for an empty `-` item in a sequence.

If an edit *cannot* be made without restructuring the document, yqr refuses
it and exits 5 rather than emitting something surprising. With `-i` the
file is left untouched on refusal, so a failed edit never leaves you with a
half-written file.

An entry that a `<<` merge or an alias produced is one of those refusals.
You can *read* `.web.mode` -- it resolves to `0640` through the merge -- but
there is no `mode` entry under `web` to write to, so yqr declines rather
than inventing one:

```console
$ yqr '.web.mode = 416' config.yaml
yqr: runtime error: cannot assign at "web.mode": the mapping has no "mode" entry of its own to write; it is merged in from elsewhere, through a `<<` merge key or an alias. Assign where the key is defined instead
```

So `.defaults.mode = 416` is the edit, and it changes the value for
everything that inherits the anchor. Writing an entry under `web` that
overrides the merge for `web` alone is a different edit, and one yqr cannot
make yet.

## A write that changes nothing changes nothing

Assigning a value that is already there does not rewrite it. yqr writes
*values*, and a value does not carry its own spelling -- so re-emitting
`0640` would print `640`, exactly the way `--normalize` does. Instead the
write is skipped:

```console
$ yqr '.defaults.mode = .defaults.mode' config.yaml | sed -n 2p
  mode: 0640      # octal, on purpose
```

The same holds for comments. `#tight` and `# tight` say the same thing, so
writing a comment's own text back leaves the line alone -- which means you
can read a comment and feed it straight back:

```console
$ yqr 'line_comment(.defaults.mode) = "octal, on purpose"' config.yaml | sed -n 2p
  mode: 0640      # octal, on purpose
```

Anything yqr can tell apart still writes normally. This is a guard against
rewriting bytes you did not ask to change, not a limit on what you can
edit.

## Next

- [Editing Kubernetes manifests](kubernetes) -- this applied to a real
  workflow.
- [Validating YAML](validate) -- confirming a file is still correct
  afterwards.
