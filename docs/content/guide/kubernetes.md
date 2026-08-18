---
title: Editing Kubernetes manifests without reformatting them
description: >-
  Bump an image tag or replica count in a Kubernetes manifest so the git
  diff is exactly one line, with comments and formatting left alone.
menu:
  title: Kubernetes manifests
  order: 2
---

# Editing Kubernetes manifests

Manifests are checked in, reviewed, and argued over. So the useful property
in a tool that edits them is not how much it can do -- it is how little it
changes.

Here is a manifest with the things real ones have: an ownership comment, a
comment explaining a number, and a quoted image reference.

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

## Reading a field

Paths look like jq, because that is the idea:

```console
$ yqr '.spec.template.spec.containers[0].image' deploy.yaml
"registry.example.com/web:1.4.2"
```

The quotes are there because that is how the value is written in the file.
When you want the bare string -- to pass to `docker pull`, say -- use `-r`:

```console
$ yqr -r '.spec.template.spec.containers[0].image' deploy.yaml
registry.example.com/web:1.4.2
```

## Bumping an image tag

```console
$ yqr '.spec.template.spec.containers[0].image = "registry.example.com/web:1.5.0"' deploy.yaml
```

That prints the whole file to stdout with one value changed. Add `-i` to
write it back in place instead:

```console
$ yqr -i '.spec.template.spec.containers[0].image = "registry.example.com/web:1.5.0"' deploy.yaml
```

The write is atomic -- a temporary file and a rename -- so an interrupted
run cannot leave you with half a manifest.

## The part that matters

```console
$ yqr -i '.spec.replicas = 5' deploy.yaml
$ git diff deploy.yaml
```
```diff
-  replicas: 3          # bumped for the Black Friday load test
+  replicas: 5          # bumped for the Black Friday load test
```

One line. The comment is still aligned where it was, the quoting elsewhere
is untouched, the blank lines are where you left them. A reviewer reads one
line and moves on.

Run that across forty manifests in a release script and you get forty
one-line diffs, which is a review someone can actually do.

## Other edits you can make today

```console
$ yqr -i '.spec.ports += 9090' service.yaml          # append to a sequence
$ yqr -i '.metadata.labels.tier = "frontend"' deploy.yaml   # add a key
$ yqr -i 'del(.spec.template.metadata.annotations)' deploy.yaml   # remove an entry
$ yqr -i 'key(.metadata.labels.app) = "application"' deploy.yaml  # rename a key
```

`del` handles nested blocks and multi-line values, not just single lines,
and closes the gap cleanly afterwards. It also handles the two cases that
used to be refused:

- **The last entry of a block.** The collection is written out explicitly,
  because deleting the bytes would leave a dangling `spec:` -- and a key
  with nothing under it reads back as `null`, which is a type change rather
  than a removal:

  ```console
  $ yqr 'del(.spec.replicas)' one.yaml
  spec:
    {}
  ```

  A comment documenting the removed entry goes with it, rather than being
  left behind describing an empty collection.

- **An item of a flow collection** like `ports: [80, 443]`. Exactly one
  separator goes with the item, so you never get `[, 443]` or `[80, ]`.

## Editing a comment

<!-- Feature f007 -->

Two selectors, wrapping a path the same way `key(...)` does:

```console
$ yqr 'line_comment(.spec.replicas) = "tuned for peak"' deploy.yaml
$ yqr 'head_comment(.spec) = "why this exists"' deploy.yaml
$ yqr -i 'del(line_comment(.spec.replicas))' deploy.yaml
```

`line_comment` is the `# ...` after the value on the entry's own line;
`head_comment` is the block of comment lines directly above it. Reading
either gives the body without the `#`:

```console
$ yqr -r 'line_comment(.spec.replicas)' deploy.yaml
tuned for peak
```

What you write is what you read back, including leading spaces, so a
comment survives being set and read again unchanged. The reverse is not a
byte-level identity: a comment authored `#note`, with no space, reads as
`note`, and writing that back renders `# note`.

An empty body writes a bare `#` rather than removing anything. Removal has
its own spelling, `del(...)`, so both are reachable.

Three cases are refused rather than guessed at, each because the obvious
thing to do would be wrong:

- **An entry whose value starts on the next line** has no line of its own,
  so there is nowhere to put an inline comment. Writing one would land it
  on the first *child* instead, where it would look like it documents that.
- **A comment block separated from the entry by a blank line** documents
  whatever came before it, not the entry below. yqr will not rewrite or
  delete it.
- **A comment block above a sequence item** can be read but not edited --
  the YAML engine attaches leading comments to mapping keys only.

And `foot_comment(...)` is refused with an explanation rather than a syntax
error: a comment *below* an entry belongs to whatever follows it about as
often as to the entry itself, so there is no unambiguous block to address.

## Renaming a key

<!-- Feature f007 -->

A path names a *value*, so there is no path that means "the key of this
entry". `key(...)` wraps one and names the key instead:

```console
$ yqr 'key(.metadata.name)' deploy.yaml
name
$ yqr -i 'key(.metadata.name) = "title"' deploy.yaml
```

The rename rewrites the key token and nothing else. The value keeps its
spelling, the entry keeps its position in the mapping -- a rename is not a
delete followed by an insert -- and the comments stay where they were:

```console
$ cat deploy.yaml
metadata:
  # names the app
  name: web  # required
$ yqr -i 'key(.metadata.name) = "title"' deploy.yaml
$ cat deploy.yaml
metadata:
  # names the app
  title: web  # required
```

Reading a key gives you the token as the file spells it, quotes included,
because the read slices the document rather than echoing back the path you
typed. `-r` unquotes it, the same way it unquotes a string value:

```console
$ yqr 'key(.["retry count"])' config.yaml
"retry count"
$ yqr -r 'key(.["retry count"])' config.yaml
retry count
```

`key(...)` reads are total: a sequence item has no key, and neither does a
key that arrived through a `<<` merge, so both read `null` rather than
failing a batch. Writing to those is refused with the reason, as is a
rename that would collide with an existing sibling, or one to a name the
path syntax could not address afterwards.

One edge worth knowing, because it reads as a wrong answer rather than an
error: a key containing `.` or `[` -- the `app.kubernetes.io/name` style --
cannot be addressed by the path syntax at all, so `key(...)` on one reports
`null` like any other keyless node. That limitation is not specific to
renames; it is the same one listed below.

## Piping from kubectl

There is no file argument needed -- yqr reads stdin:

```console
$ kubectl get deploy web -o yaml | yqr -r '.spec.template.spec.containers[0].image'
```

Worth knowing: `-i` needs a real file, so it is an error to combine it with
stdin. That is deliberate; there is nothing to write back to.

## What is not here yet

Being straight about the edges, because finding them yourself is annoying:

- **Reordering a sequence** is not in the released grammar. Values, new
  keys, appends, deletes, key renames, and comment edits are.
- **The right-hand side must be a scalar.** Assigning a whole nested block
  is not supported yet.
- **Keys containing `.` or `[`** -- the Kubernetes
  `app.kubernetes.io/name` style -- cannot be addressed by the path syntax,
  so yqr will tell you so rather than guess.
- **No arithmetic or builtins.** You cannot say "increment the replica
  count"; you say what it should become.

## Next

- [Validating YAML](validate) -- worth wiring into the same script.
- [Byte-for-byte, explained](fidelity) -- why the diff is one line.
