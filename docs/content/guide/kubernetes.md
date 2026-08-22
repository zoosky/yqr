---
# Traceability: Feature f007 (structural edits -- key rename, comment
# editing, reorder). Bug b012, the dotted-key insert, was the sibling half
# of the limitation section; fixed in noyalib 0.0.25.
# "Computing a new value from the old one" is Feature f008.
title: Editing Kubernetes manifests without reformatting them
lead: >-
  How to bump an image tag or a replica count so the diff is one line, and which edits yqr refuses outright.
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
$ yqr -i 'swap(.spec.template.spec.containers; 0; 1)' deploy.yaml # reorder a list
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

## Computing a new value from the old one

`=` writes what you tell it. `|=` runs a filter on the value that is already
there and writes the result, so you can say "one more than this" without
knowing what "this" is:

```console
$ yqr -i '.spec.replicas |= (. + 1)' deploy.yaml
```

Inside the filter, `.` is the value at the path -- not the document. Arithmetic
is `+ - * / %`, with the usual precedence and parentheses.

Numbers keep their type. `replicas: 3` doubled is `6`, never `6.0`, and a
division only becomes a fraction when it genuinely is one:

```console
$ yqr '.n |= (. / 2)' <<< 'n: 4'      # n: 2
$ yqr '.n |= (. / 2)' <<< 'n: 3'      # n: 1.5
```

That is the same rule that keeps `0640` from becoming `640` on a read. An
integer result too large for 64 bits is an error rather than a silent widening
to a float, because widening is exactly the precision loss the rule prevents.

`|=` writes wherever `=` writes -- a scalar, in place. A filter returning a
list or a mapping is refused, the same way assigning one is.

## Editing a comment

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

## Reordering a list

An ordering is the one thing here with no node to name -- there is no path
that means "third". So it is a verb with arguments rather than a selector
wrapping a path, and the arguments are separated by `;`:

```console
$ yqr -i 'swap(.jobs.build.steps; 0; 2)' ci.yaml   # exchange two items
$ yqr -i 'move(.jobs.build.steps; 0; -1)' ci.yaml  # move one, shifting the rest
```

`swap` exchanges two items and leaves everything between them alone. `move`
takes one item out and puts it back at the destination, shifting the items
in between by one. Indices count from zero, and a negative index counts from
the end -- `-1` is the last item, the same as `.[-1]` in a path.

The reason this is worth having as its own verb is what travels with an
item. A step in a workflow is usually two or three lines with a comment
above it and another beside it, and all of that belongs to the step rather
than to the position:

```console
$ cat ci.yaml
jobs:
  build:
    steps:
      # check out first
      - uses: actions/checkout@v4  # pinned
      - name: test
        run:  cargo test
$ yqr -i 'swap(.jobs.build.steps; 0; 1)' ci.yaml
$ cat ci.yaml
jobs:
  build:
    steps:
      - name: test
        run:  cargo test
      # check out first
      - uses: actions/checkout@v4  # pinned
```

Both comments moved with the item they document, and the odd spacing in
`run:  cargo test` came through untouched, because nothing was re-emitted.

An inline list (`ports: [80, 443, 8080]`) reorders too. Its items have no
lines of their own, so there is no comment to carry -- just the values, in
their new order.

Two refusals, both exit 5 with the file left alone under `-i`: an index
outside the sequence, and a path that names something other than a
sequence. There is no partial reorder -- either the whole edit lands or none
of it does.

## Piping from kubectl

There is no file argument needed -- yqr reads stdin:

```console
$ kubectl get deploy web -o yaml | yqr -r '.spec.template.spec.containers[0].image'
```

Worth knowing: `-i` needs a real file, so it is an error to combine it with
stdin. That is deliberate; there is nothing to write back to.

## What is not here yet

Being straight about the edges, because finding them yourself is annoying:

- **The right-hand side must be a scalar.** Assigning a whole nested block
  is not supported yet.
- **Writing a key that contains `.` or `[`** -- the Kubernetes
  `app.kubernetes.io/name` style. Reading one works with the bracket form:

  ```console
  $ yqr '.metadata.labels["app.kubernetes.io/name"]' deploy.yaml
  web
  ```

  Changing, deleting or renaming it does not:

  ```console
  $ yqr '.metadata.labels["app.kubernetes.io/name"] = "api"' deploy.yaml
  yqr: runtime error: cannot address key "app.kubernetes.io/name": it uses
  characters the write path cannot express
  ```

  The edit is refused, so nothing is damaged. Adding a **plain-named key next
  to** dotted ones does work, which is the common case:

  ```console
  $ yqr '.metadata.labels.tier = "frontend"' deploy.yaml
  ```
- **No builtins beyond `to_entries`.** There is no `select`, no `map`, and no
  string interpolation, so a filter cannot yet pick entries by a condition or
  reshape them.

## Next

- [Validating YAML](validate) -- worth wiring into the same script.
- [Byte-for-byte, explained](fidelity) -- why the diff is one line.
