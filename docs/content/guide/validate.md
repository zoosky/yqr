---
title: Validating YAML from the command line
lead: >-
  How to check a file is still correct after an edit, what each exit code means, and how duplicate keys are reported.
description: >-
  Check a YAML file is still correct after an edit, with compiler-style
  diagnostics -- and use --strict to catch duplicate keys that silently
  drop data.
menu:
  title: Validating YAML
  order: 3
---

# Validating YAML

An edit went in -- by hand, by a script, or by an agent. Is the file still
correct?

<!-- Feature f012: the validate subcommand. -->

```console
$ yqr validate deploy.yaml
$ echo $?
0
```

Silence and exit 0. That is the whole success case, which makes it easy to
put in a script or a pre-commit hook.

A pass here means more than "it parsed". yqr checks that the parsed
documents reproduce the input byte-for-byte, so a file that parses but
round-trips differently is reported rather than waved through.

## When something is wrong

```console
$ yqr validate broken.yaml
error[Y001]: inconsistent indentation: token at a column that does not match any open block scope
  --> broken.yaml
$ echo $?
1
```

Diagnostics are compiler-shaped on purpose: an error code, the file, and
where possible the line, column, and a pointer into the source. That is as
readable for a person as it is parseable for whatever is running it.

Several files at once works, and the exit code covers all of them:

```console
$ yqr validate deploy.yaml service.yaml configmap.yaml
```

Exit 0 when every input is valid, 1 when any input fails, and 5 when an
input cannot be read at all -- a missing file is a different problem from a
malformed one, so it gets a different code.

## The file your engine reads and nobody else does

<!-- Bug b014 §3.2: the Y103 check. -->

A mapping value on its own line has to be indented past its key. Some
parsers -- yqr's included -- read one that is not, which is worse than
refusing it: the file works for you and fails for everyone else.

```console
$ cat workflow.yaml
on:
[]
jobs: {}
$ yqr validate workflow.yaml
error[Y103]: block mapping value is not indented past its key
  --> workflow.yaml:2:1
  |
2 | []
  | ^
  = note: its key is at line 1, column 1, so the value must start at column 2 or deeper
  = help: indent the value, or write it on the key's own line; noyalib reads this file but other YAML implementations reject it
```

This one is not a `--strict` opinion, so it is on by default: the document
is invalid, not merely unusual. Python's PyYAML and Ruby's Psych both refuse
that file.

Two layouts look like this and are perfectly fine, so they are never
flagged. A block sequence may sit at its key's own column -- the GitHub
Actions and Ansible style:

```yaml
on:
- push
- pull_request
```

And a block scalar sets its own indentation, so its `|` may sit anywhere:

```yaml
script:
|
  make build
```

## `--strict`, and the bug it catches

Here is a file that is perfectly legal YAML and almost certainly a mistake:

```yaml
name: web
port: 8080
name: api
```

`name` appears twice. The YAML spec says a mapping should not have
duplicate keys, but almost every parser accepts it anyway and resolves
last-wins. So does yqr:

```console
$ yqr validate dup.yaml
$ echo $?
0

$ yqr -r '.name' dup.yaml
api
```

The first `name` is simply gone. No warning, no error -- your data quietly
lost a field. This is exactly the sort of thing a careless merge or a
templating bug produces.

`--strict` turns it into an error:

```console
$ yqr validate --strict dup.yaml
error[Y101]: duplicate mapping key "name"
  --> dup.yaml:3:1
  |
3 | name: api
  | ^
  = note: first occurrence at line 1, column 1
  = help: later occurrences silently override earlier ones; remove or rename one
$ echo $?
1
```

It points at the later occurrence, tells you where the first one was, and
says what will happen if you leave it. The two are kept separate because
they answer different questions: plain `validate` asks "will this file
load", `--strict` asks "will it load the way you think it will".

**Use `--strict` in CI.** The cost is one flag; the thing it catches is
silent data loss, which is the failure mode you find out about in
production.

## In a script

The pattern that works:

```bash
yqr -i '.spec.replicas = 5' deploy.yaml
yqr validate --strict deploy.yaml || exit 1
```

Edit, then check. yqr already refuses edits that would restructure the
document, so this is a second net rather than the only one -- but it also
catches problems that were in the file before you touched it, which is
worth knowing before you ship it.

In a pre-commit hook, over everything that changed:

```bash
git diff --cached --name-only --diff-filter=ACM -- '*.yaml' '*.yml' \
  | xargs -r yqr validate --strict
```

## Reading from stdin

Same as everywhere else -- omit the file, or pass `-`:

```console
$ kubectl get deploy web -o yaml | yqr validate --strict
$ helm template ./chart | yqr validate --strict -
```

The second one is a genuinely useful check: chart templating produces YAML
through string interpolation, which is exactly the process most likely to
emit a duplicate key.

## Next

- [Editing Kubernetes manifests](kubernetes) -- the edits worth validating
  after.
- [Byte-for-byte, explained](fidelity) -- what "reproduces the input"
  means.
