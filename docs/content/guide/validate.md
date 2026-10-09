---
# Traceability: Feature f012 (the validate subcommand); bug b014 §3.2 is
# the Y103 check. Every console block re-run against v0.9.0 on 2026-10-09
# (before that v0.8.0 on 2026-09-07, and the --schema blocks against the
# f040 build on 2026-10-08); the usage-error block now quotes the full
# output, clap's usage lines included. The indentation error gained its
# position with noyalib 0.0.36 (f029). The --schema section is Feature
# f040.
title: Validating YAML from the command line
lead: >-
  How to check a file is still correct after an edit, what each exit code means, how duplicate keys are reported, and how to validate against a JSON Schema.
description: >-
  Check a YAML file is still correct after an edit, with compiler-style
  diagnostics -- use --strict to catch duplicate keys that silently drop
  data, and --schema to validate against a JSON Schema with source
  positions.
menu:
  title: Validating YAML
  order: 3
---

# Validating YAML

An edit went in -- by hand, by a script, or by an agent. Is the file still
correct?

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
$ cat broken.yaml
a:
  b: 1
 c: 2
$ yqr validate broken.yaml
error[Y001]: inconsistent indentation: token at a column that does not match any open block scope
  --> broken.yaml:3:2
  |
3 |  c: 2
  |  ^
$ echo $?
1
```

Diagnostics are compiler-shaped on purpose: an error code, the file, the
line and column, and a pointer into the source. That is as readable for a
person as it is parseable for whatever is running it.

Several files at once works, and the exit code covers all of them:

```console
$ yqr validate deploy.yaml service.yaml configmap.yaml
```

Exit 0 when every input is valid, 1 when any input fails, and 5 when an
input cannot be read at all -- a missing file is a different problem from a
malformed one, so it gets a different code.

## The file your engine reads and nobody else does


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

## Validating against a schema

Correct YAML is not yet the YAML your system accepts. `--schema` checks
every document against a JSON Schema (draft 2020-12), written in YAML or
JSON:

```console
$ cat schema.yaml
type: object
required: [spec]
properties:
  spec:
    type: object
    required: [image]
    properties:
      replicas:
        type: integer
      image:
        type: string
$ cat deploy.yaml
name: web
spec:
  replicas: "three"
$ yqr validate --schema schema.yaml deploy.yaml
error[Y201]: "image" is a required property
  --> deploy.yaml:3:1
  |
3 |   replicas: "three"
  | ^
  = note: at instance path /spec
error[Y201]: "three" is not of type "integer"
  --> deploy.yaml:3:13
  |
3 |   replicas: "three"
  |             ^
  = note: at instance path /spec/replicas
$ echo $?
1
```

The difference from schema validators you may know: the position. A
violation names the line and column in your file, not just a JSON pointer
you then go hunting for. The pointer is still there, on the note line,
because scripts match on it.

Three codes belong to this check, and like every other code they are
stable: `Y201` is a violation in your document, `Y202` means the schema
file itself is unusable (not valid YAML, or not a valid 2020-12 schema --
the run stops rather than reporting a hollow pass), and `Y203` means the
document holds a value JSON cannot represent (such as `.nan`), so it
cannot be checked against a JSON Schema at all.

```console
$ yqr validate --schema badschema.yaml deploy.yaml
error[Y202]: schema does not compile as JSON Schema 2020-12: 5 is not valid under any of the schemas listed in the 'anyOf' keyword
  --> badschema.yaml
  = help: the schema is a single JSON Schema 2020-12 document, in YAML or JSON
$ echo $?
1
```

Two boundaries worth knowing. The validator never touches the network: a
schema that `$ref`s an external resource is refused as `Y202`, by
construction. And the dialect is 2020-12, which covers SchemaStore-style
schemas (GitHub Actions, docker-compose) well; Kubernetes CRD schemas are
an older OpenAPI dialect and remain a job for kubeconform.

The default checks still run first -- `--schema` adds to the verdict, it
never replaces it -- and `--strict` combines with it the way you would
expect.

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

Pass `-`. Unlike the filter form, `validate` will not fall back to stdin
when you give it nothing:

```console
$ kubectl get deploy web -o yaml | yqr validate --strict -
$ helm template ./chart | yqr validate --strict -
$ yqr validate --strict
error: no input files; pass one or more YAML files, or '-' to read stdin

Usage: yqr [OPTIONS] <FILTER> [FILE]
       yqr <COMMAND>

For more information, try '--help'.
```

That refusal is the point. A validation gate whose file list came up empty
would otherwise report "all valid" over nothing, which is the one answer a
gate must never give by accident.

The helm check is a genuinely useful one: chart templating produces YAML
through string interpolation, which is exactly the process most likely to
emit a duplicate key.

## Next

- [Editing Kubernetes manifests](kubernetes) -- the edits worth validating
  after.
- [Byte-for-byte, explained](fidelity) -- what "reproduces the input"
  means.
