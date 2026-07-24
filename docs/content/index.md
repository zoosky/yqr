---
title: Chart a path to any field in a manifest
menu:
  title: Home
---

<!--
  yqr website home page, ported from the former docs/content/home.html.
  Every command and output on this page was run against a real yqr build;
  see the fidelity corpus this page borrows from.
  Feature/spec traceability: yqr-b001, yqr-f003, yqr-m002
-->

`yqr` is a jq-style filter for YAML. Point it at a manifest file, a
`kubectl get -o yaml` dump, or a Helm-rendered bundle, and it walks straight
to the field you asked for -- as a value, not as JSON you have to decode back.

## The filter

```
.spec.volumes[0].secret.defaultMode
```

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: web
spec:
  volumes:
    - name: tls
      secret:
        secretName: web-tls
        defaultMode: 0640    # <- the filter lands here
```

Default (byte-preserving):

```sh
$ yqr '.spec.volumes[0].secret.defaultMode' pod.yaml
0640
```

With `--normalize` (classic re-serializing pipeline):

```sh
$ yqr --normalize '.spec.volumes[0].secret.defaultMode' pod.yaml
640
```

Kubernetes spells file permissions in octal -- `defaultMode: 0640` on a Secret
or ConfigMap volume. Read that field through yqr and the value comes back
exactly as written, because yqr never re-typed it in the first place. Only if
you opt into the classic `--normalize` pipeline is the leading zero lost:
`640` is a different number.

## Where the binary actually lives

Install from crates.io with `cargo install yqr`, or build from a source
checkout.

### On your machine

```sh
cargo install yqr
```

Pulls the published crate to `~/.cargo/bin/yqr` -- keep that directory on
`PATH` so plain `yqr` resolves from any shell, including one already piping
`kubectl` output.

```sh
cargo build --release
```

Building from source instead? The binary lands at `target/release/yqr` (or
`cargo install --path .` to put a local checkout on `PATH`).

### Inside a container image

```
/usr/local/bin/yqr
```

Build it in a multi-stage `Dockerfile` and copy just the binary into the
runtime stage -- no Rust toolchain, no source tree, in the image that actually
ships.

[infobox type="info"]
**Byte-preserving reads are the default.** Untouched nodes come back as their
original source bytes -- comments, quoting, indentation, and line endings
survive, and the identity filter reproduces the input byte-for-byte, no flag
required.

```sh
$ yqr '.' pod.yaml
```

Pass `--normalize` (`-N`) to opt into the classic, re-serializing pipeline
(comments dropped, scalars canonicalized). `--engine <name>` selects which
backend parser performs the byte-preserving read (default `noyalib`); an
unknown engine name is rejected up front:

```sh
$ yqr --engine bogus '.' pod.yaml
yqr: io error: unknown engine "bogus" (available: noyalib, skald)
```

See the [runnable demo](/demo) for a seven-step walkthrough of navigation,
iteration, pipes, raw output, and fidelity mode.
[/infobox]

<!-- Feature f006: write tier v1 (value assignment + in-place edits) -->
[infobox type="success"]
**It edits, too -- and only the bytes you target.** Give it a mutating filter
and yqr changes just that node, leaving every other byte -- comments,
indentation, quoting, key order -- untouched, or refuses. Replace a value with
`=`, append to a block sequence with `+=`, add a key, or drop an entry with
`del(...)`:

```sh
$ yqr '.spec.replicas = 5' deploy.yaml
$ yqr '.spec.ports += 9090' deploy.yaml
$ yqr 'del(.metadata.labels)' deploy.yaml
$ yqr 'del(.spec.template)' deploy.yaml   # a nested block, closed up cleanly
```

`del` removes multi-line and nested block entries as well as single-line ones,
closing up the gap and leaving every surviving byte identical; deleting the
only entry of a block, or an item of a flow collection (`[a, b]`), is refused
with a clear message. Add `-i` (`--in-place`) and the file is rewritten
atomically -- a `git diff` touches only the line you changed. An edit that
would restructure the document is refused (exit 5) rather than emitted, and
under `-i` the file is left untouched.

```sh
$ yqr -i '.spec.replicas = 5' deploy.yaml
$ git diff deploy.yaml   # one line
```
[/infobox]

## Two ways to run it against a cluster

### Piped from kubectl

Standard operator loop: dump a resource as YAML, pull one field out of it,
move on.

```sh
$ kubectl get pods -o yaml | yqr -r '.items[] | .metadata.name'
```

One pod name per line.

```sh
$ kubectl get pod web-0 -o yaml | yqr -r '.spec.containers[0].image'
```

The primary container's image, unquoted.

```sh
$ kubectl get pod web-0 -o yaml | yqr -r '.spec.initContainers[]? | .image'
```

Init container images when the pod has any -- the trailing `?` keeps pods
with none from erroring the pipeline.

### Inside a container image

Bake the binary in, then use it in an init container to read a mounted
manifest or ConfigMap before the main container starts.

```dockerfile
# -- build --
FROM rust:1.97-slim AS build
WORKDIR /src
COPY . .
RUN cargo build --release

# -- runtime --
FROM debian:bookworm-slim
COPY --from=build /src/target/release/yqr /usr/local/bin/yqr
ENTRYPOINT ["yqr"]
```

```sh
$ yqr -r '.data.enableBeta' /config/values.yaml
```

Read a flag out of a mounted ConfigMap and hand it to the next step -- a
common init-container job.

## It's not just Kubernetes

Anything that's YAML takes the same filters. Three more places yqr earns its
keep.

### CI/CD pipelines

GitHub Actions workflows are YAML. Audit what a job actually runs without
opening the file -- these two ran against this repo's own `ci.yml`.

```sh
$ yqr -r '.jobs.test.["runs-on"]' ci.yml
```

ubuntu-latest -- bracket syntax reaches keys a bareword can't spell, like
`runs-on`.

```sh
$ yqr -r '.jobs.test.steps[1].with.toolchain' ci.yml
```

1.97.1 -- confirm the pinned Rust version without scrolling past the cache
step.

### Docker Compose

Check what a compose file is about to pull and expose before you run it.

```sh
$ yqr -r '.services[] | .image' compose.yaml
```

yqr-demo:latest, postgres:16 -- every image referenced, one per line.

```sh
$ yqr -r '.services.web.environment.LOG_LEVEL' compose.yaml
```

debug -- one config value, no grep.

### Ansible playbooks

A playbook is a YAML list of plays -- walk it like any other sequence.

```sh
$ yqr -r '.[0].tasks[] | .name' playbook.yml
```

Install nginx, Start nginx -- every task in the first play, at a glance.

```sh
$ yqr -r '.[0].hosts' playbook.yml
```

web -- which hosts that play targets.

## Three more, shown in full

Same grammar, different files -- this time with the source shown, so nothing
here has to be taken on faith.

### OpenAPI specs

An OpenAPI document is plain YAML. Point yqr at it to pull a specific
operation's details out of a spec someone else wrote, without loading it into
an editor.

```yaml
paths:
  /widgets/{id}:
    get:
      summary: Get a widget
      responses:
        "200":
          description: OK
```

```sh
$ yqr -r '.paths.["/widgets/{id}"].get.summary' openapi.yaml
Get a widget
```

Path keys have slashes and braces, so a bareword can't spell them -- bracket
syntax reaches them anyway, the same way `.["runs-on"]` did for the CI
workflow above.

```sh
$ yqr -r '.paths.["/widgets"].get.responses.["200"].description' openapi.yaml
OK
```

Status codes are string keys, and an unquoted `200` in a filter would try to
read a number. Bracket syntax reaches the string key `"200"` exactly as the
spec wrote it.

### Prometheus alerting rules

Alerting rules are a YAML list of groups, each holding a list of rules.
Reading one back tells you exactly what will page someone, and at what
threshold.

```yaml
groups:
  - name: api-slos
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status="5xx"}[5m]) > 0.05
        for: 10m
        labels:
          severity: page
```

```sh
$ yqr -r '.groups[0].rules[] | .alert' rules.yaml
HighErrorRate
HighLatency
```

Every alert name in the first group, without reading through a file's worth
of PromQL to find them.

```sh
$ yqr -r '.groups[0].rules[0].expr' rules.yaml
rate(http_requests_total{status="5xx"}[5m]) > 0.05
```

The exact expression for that alert -- useful when you just need to confirm
the number that pages someone, not re-read the whole rules file.

### Application config

Most services ship a YAML config file alongside the binary -- database
targets, ports, feature flags. yqr reads it the same way it reads anything
else.

```yaml
database:
  host: db.internal
  port: 5432
featureFlags:
  newCheckout: true
  betaSearch: false
```

```sh
$ yqr -r '.database.host' application.yaml
db.internal
```

Confirm which database an environment's config actually points at before you
run a migration against it.

```sh
$ yqr -r '.featureFlags.newCheckout' application.yaml
true
```

Read a single feature flag's value straight out of the file that ships with
the deploy, instead of grepping for it.

## What yqr can walk today

yqr is at milestone M0. This is the whole grammar -- every recipe on this
page is built from it.

| Filter | Meaning |
|--------|---------|
| `.` | Identity |
| `.foo` | Field access |
| `.a.b` | Nested field access |
| `.[n]` | Array index (`.[-1]` counts from the end) |
| `.[]` | Iterate sequence elements / mapping values |
| `a \| b` | Pipe |
| `f?` | Suppress runtime errors from `f` (e.g. iterating a field that turns out to be missing or the wrong shape) |

Not yet available: `select()`, `map()`, `keys`, arithmetic, and object/array
construction. Reach for `kubectl`'s own `-o jsonpath` or a `grep` in the
meantime if a recipe needs one of those.

Browse the [spec tree](/specs) for the full feature, bug, and architecture
history behind everything on this page.
