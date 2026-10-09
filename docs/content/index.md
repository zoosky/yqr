---
# `title` drives <title>, og:title and twitter:title, so it carries the words
# people actually search for rather than a section label. The brand is supplied
# separately -- base.html.jinja appends " - yqr", and og:site_name is "yqr".
# Home page -- the pitch, the three outcome cards, and the worked examples.
# The body is markdown with Markdoc components (f042); the structures the
# page repeats (hero, chart, outcomes, section heads, recipes, cards) are
# theme components in themes/default/shortcodes/, and the design still
# lives in themes/default/templates/home.html.jinja's style block.
# Traceability: yqr-b001, yqr-f003, yqr-m002; features f006 (the write tier)
# and f012 (validate) are the two the worked examples demonstrate.
title: jq for YAML -- query and edit without reformatting
lead: >-
  A jq-style query and editing tool for YAML: reads give back your own bytes, edits change only what you name.
description: >-
  yqr is a jq-style command-line tool for YAML. Query any field, edit a file in
  place, and keep every comment, anchor, quote style and blank line
  byte-for-byte.
menu:
  title: Home
  order: 1
template: home
---

{% hero title="Query and edit YAML from the command line." lede="Chart a path to any field in a manifest — and edit it without reformatting the file." %}
`yqr` is a jq-style filter for YAML. Point it at a manifest file, a `kubectl get -o yaml` dump, or a Helm-rendered bundle, and it walks straight to the field you asked for — as a value, not as JSON you have to decode back.
{% /hero %}

{% chart /%}

{% outcomes %}
Kubernetes spells file permissions in octal — `defaultMode: 0640` on a Secret or ConfigMap volume. Read that field through yqr and the value comes back exactly as written, because yqr never re-typed it in the first place. Only if you opt into the classic [`--normalize`](#engines) pipeline is the leading zero lost: `640` is a different number.
{% /outcomes %}

{% home-section id="paths" %}
{% section-head eyebrow="installed paths" title="Where the binary actually lives" %}
Install from crates.io with `cargo install yqr`, or build any of the paths below from a source checkout.
{% /section-head %}

{% grid kind="paths" %}
{% path-card title="On your machine" %}
{% loc text="cargo install yqr" /%}

Pulls the published crate to `~/.cargo/bin/yqr` — keep that directory on `PATH` so plain `yqr` resolves from any shell, including one already piping `kubectl` output.

{% loc text="cargo build --release" /%}

Building from source instead? The binary lands at `target/release/yqr` (or `cargo install --path .` to put a local checkout on `PATH`).
{% /path-card %}

{% path-card title="Inside a container image" %}
{% loc text="/usr/local/bin/yqr" /%}

Build it in a multi-stage `Dockerfile` and copy just the binary into the runtime stage — no Rust toolchain, no source tree, in the image that actually ships.
{% /path-card %}
{% /grid %}

{% callout id="engines" title="Byte-preserving reads are the default." %}
Untouched nodes come back as their original source bytes — comments, quoting, indentation, and line endings survive, and the identity filter reproduces the input byte-for-byte, no flag required.

```console
$ yqr '.' pod.yaml
```

Pass `--normalize` (`-N`) to opt into the classic, re-serializing pipeline (comments dropped, scalars canonicalized). Byte-preserving reads are powered by noyalib's lossless CST — yqr's one and only YAML engine. See the [runnable demo](https://github.com/zoosky/yqr/tree/main/docs/content/demo) for an eight-step walkthrough of navigation, iteration, pipes, raw output, fidelity mode, and validation.
{% /callout %}

{% callout id="edits" title="It edits, too — and only the bytes you target." %}
Give it a mutating filter and yqr changes just that node, leaving every other byte — comments, indentation, quoting, key order — untouched, or refuses. Replace a value with `=`, append to a block sequence with `+=`, add a key, or drop an entry with `del(…)`:

```console
$ yqr '.spec.replicas = 5' deploy.yaml
$ yqr '.spec.ports += 9090' deploy.yaml
$ yqr 'del(.metadata.labels)' deploy.yaml
$ yqr 'del(.spec.template)' deploy.yaml   # a nested block, closed up cleanly
```

`del` removes multi-line and nested block entries as well as single-line ones, closing up the gap and leaving every surviving byte identical; removing the last entry of a block leaves the collection spelled out (`{}`), since a key with nothing under it reads back as null, and removing an item of an inline collection (`[a, b]`) takes exactly one separator with it. Add `-i` (`--in-place`) and the file is rewritten atomically — a `git diff` touches only the line you changed. An edit that would restructure the document is refused (exit 5) rather than emitted, and under `-i` the file is left untouched.

```console
$ yqr -i '.spec.replicas = 5' deploy.yaml
$ git diff deploy.yaml   # one line
```
{% /callout %}

{% callout id="validate" title="Validate after every edit." %}
One command answers whether a file is still correct YAML — and a pass certifies more than "parses": the parsed documents must reproduce the input byte-for-byte, the same invariant behind yqr's fidelity reads. Failures are compiler-style diagnostics with a stable code, a clickable location whenever a position is known, and a suggested fix, so humans and agents can act on them. Edit, then verify:

```console
$ yqr -i '.spec.replicas = 5' deploy.yaml   # edit
$ yqr validate --strict deploy.yaml        # verify -- silent, exit 0
```

When the file is not correct YAML — a hand edit gone wrong, a half-resolved merge, a truncated write — the verdict names the spot:

```console
$ yqr validate deploy.yaml
error[Y001]: expected a node but found StreamEnd
  --> deploy.yaml:3:7
  |
3 | b: [1,
  |       ^
```

`--strict` also flags duplicate mapping keys (`Y101`) — accepted last-wins by ordinary reads, so a bad edit silently drops data — reporting every duplicate, `<<` merge keys included, with the positions of both occurrences. Keys that collide after string conversion are refused outright (`Y102`), non-UTF-8 input is a coded finding (`Y003`), a mapping value that is not indented past its key is flagged by default (`Y103`) because yqr's engine reads such a file and other implementations refuse it, and a file containing unresolved merge-conflict markers gets a dedicated hint anchored at the first marker. A JSON Schema verdict is one flag away (`--schema`, codes `Y201`–`Y203`), with violations located in your file's own lines. Exit codes are scriptable: 0 all valid, 1 validation findings, 5 an input could not be read. Stdin is explicit (`yqr validate -`); an empty file list is a usage error, never a silent "all valid".
{% /callout %}
{% /home-section %}

{% home-section id="recipes" %}
{% section-head eyebrow="two ways to run it against a cluster" title="From an operator's shell, or from inside the image" /%}

{% grid kind="cards" %}
{% home-card title="Piped from kubectl" %}
Standard operator loop: dump a resource as YAML, pull one field out of it, move on.

{% recipe cmd="kubectl get pods -o yaml | yqr -r '.items[] | .metadata.name'" %}
One pod name per line.
{% /recipe %}

{% recipe cmd="kubectl get pod web-0 -o yaml | yqr -r '.spec.containers[0].image'" %}
The primary container's image, unquoted.
{% /recipe %}

{% recipe cmd="kubectl get pod web-0 -o yaml | yqr -r '.spec.initContainers[]? | .image'" %}
Init container images when the pod has any — the trailing `?` keeps pods with none from erroring the pipeline.
{% /recipe %}

{% recipe cmd="yqr validate --strict manifests/*.yaml" %}
A gate before `kubectl apply`: every manifest must parse, round-trip byte-for-byte, and carry no duplicate keys. Exit 1 with a located diagnostic if not — and an empty file list is a loud usage error, so a glob that matches nothing never passes as "all valid".
{% /recipe %}
{% /home-card %}

{% home-card title="Inside a container image" %}
Bake the binary in, then use it in an init container to read a mounted manifest or ConfigMap before the main container starts.

```
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

{% recipe cmd="yqr -r '.data.enableBeta' /config/values.yaml" %}
Read a flag out of a mounted ConfigMap and hand it to the next step — a common init-container job.
{% /recipe %}
{% /home-card %}
{% /grid %}
{% /home-section %}

{% home-section id="beyond" %}
{% section-head eyebrow="beyond the cluster" title="It's not just Kubernetes" %}
Anything that's YAML takes the same filters. Three more places yqr earns its keep.
{% /section-head %}

{% grid kind="trio" %}
{% home-card kind="mini-card" title="CI/CD pipelines" %}
GitHub Actions workflows are YAML. Audit what a job actually runs without opening the file — these two ran against this repo's own `ci.yml`.

{% recipe cmd="yqr -r '.jobs.test.[\"runs-on\"]' ci.yml" %}
ubuntu-latest — bracket syntax reaches keys a bareword can't spell, like `runs-on`.
{% /recipe %}

{% recipe cmd="yqr -r '.jobs.test.steps[1].with.toolchain' ci.yml" %}
1.97 — confirm the pinned Rust version without scrolling past the cache step.
{% /recipe %}
{% /home-card %}

{% home-card kind="mini-card" title="Docker Compose" %}
Check what a compose file is about to pull and expose before you run it.

{% recipe cmd="yqr -r '.services[] | .image' compose.yaml" %}
yqr-demo:latest, postgres:16 — every image referenced, one per line.
{% /recipe %}

{% recipe cmd="yqr -r '.services.web.environment.LOG_LEVEL' compose.yaml" %}
debug — one config value, no grep.
{% /recipe %}
{% /home-card %}

{% home-card kind="mini-card" title="Ansible playbooks" %}
A playbook is a YAML list of plays — walk it like any other sequence.

{% recipe cmd="yqr -r '.[0].tasks[] | .name' playbook.yml" %}
Install nginx, Start nginx — every task in the first play, at a glance.
{% /recipe %}

{% recipe cmd="yqr -r '.[0].hosts' playbook.yml" %}
web — which hosts that play targets.
{% /recipe %}
{% /home-card %}
{% /grid %}
{% /home-section %}

{% home-section id="further" %}
{% section-head eyebrow="further afield" title="Three more, shown in full" %}
Same grammar, different files — this time with the source shown, so nothing here has to be taken on faith.
{% /section-head %}

{% home-card kind="explicit-card" title="OpenAPI specs" %}
An OpenAPI document is plain YAML. Point yqr at it to pull a specific operation's details out of a spec someone else wrote, without loading it into an editor.

```
paths:
  /widgets/{id}:
    get:
      summary: Get a widget
      responses:
        "200":
          description: OK
```

{% grid kind="explicit-recipes" %}
{% recipe cmd="yqr -r '.paths.[\"/widgets/{id}\"].get.summary' openapi.yaml" out=["Get a widget"] %}
Path keys have slashes and braces, so a bareword can't spell them — bracket syntax reaches them anyway, the same way `.["runs-on"]` did for the CI workflow above.
{% /recipe %}

{% recipe cmd="yqr -r '.paths.[\"/widgets\"].get.responses.[\"200\"].description' openapi.yaml" out=["OK"] %}
Status codes are string keys, and an unquoted `200` in a filter would try to read a number. Bracket syntax reaches the string key `"200"` exactly as the spec wrote it.
{% /recipe %}
{% /grid %}
{% /home-card %}

{% home-card kind="explicit-card" title="Prometheus alerting rules" %}
Alerting rules are a YAML list of groups, each holding a list of rules. Reading one back tells you exactly what will page someone, and at what threshold.

```
groups:
  - name: api-slos
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status="5xx"}[5m]) > 0.05
        for: 10m
        labels:
          severity: page
```

{% grid kind="explicit-recipes" %}
{% recipe cmd="yqr -r '.groups[0].rules[] | .alert' rules.yaml" out=["HighErrorRate", "HighLatency"] %}
Every alert name in the first group, without reading through a file's worth of PromQL to find them.
{% /recipe %}

{% recipe cmd="yqr -r '.groups[0].rules[0].expr' rules.yaml" out=["rate(http_requests_total{status=\"5xx\"}[5m]) > 0.05"] %}
The exact expression for that alert — useful when you just need to confirm the number that pages someone, not re-read the whole rules file.
{% /recipe %}
{% /grid %}
{% /home-card %}

{% home-card kind="explicit-card" title="Application config" %}
Most services ship a YAML config file alongside the binary — database targets, ports, feature flags. yqr reads it the same way it reads anything else.

```
database:
  host: db.internal
  port: 5432
featureFlags:
  newCheckout: true
  betaSearch: false
```

{% grid kind="explicit-recipes" %}
{% recipe cmd="yqr -r '.database.host' application.yaml" out=["db.internal"] %}
Confirm which database an environment's config actually points at before you run a migration against it.
{% /recipe %}

{% recipe cmd="yqr -r '.featureFlags.newCheckout' application.yaml" out=["true"] %}
Read a single feature flag's value straight out of the file that ships with the deploy, instead of grepping for it.
{% /recipe %}
{% /grid %}
{% /home-card %}
{% /home-section %}

{% home-section id="grammar" %}
{% section-head eyebrow="filter grammar" title="What yqr can walk today" %}
The whole grammar — every recipe on this page is built from it.
{% /section-head %}

| Filter | Meaning |
|---|---|
| `.` | Identity |
| `.foo` | Field access |
| `.a.b` | Nested field access |
| `.["a.b"]` | Field access for a key holding a `.`, `/` or a space |
| `."a.b"` | The same step, spelled the jq way; both forms read and edit the key |
| `.[n]` | Array index (`.[-1]` counts from the end) |
| `.[]` | Iterate sequence elements / mapping values |
| `a \| b` | Pipe |
| `f?` | Suppress runtime errors from `f` (e.g. iterating a field that turns out to be missing or the wrong shape) |
| `to_entries` | A mapping becomes `{key, value}` pairs, so the name travels with the value ([guide](guide/enumerate)) |
| `+ - * / %` | Arithmetic, with the usual precedence; `+` also joins strings. Numbers keep their type — `4 / 2` is `2`, `3 / 2` is `1.5` |

The write forms are the same paths with a verb: `=` replaces a value, `|=` computes a new one from the old (`.replicas |= (. + 1)`), `+=` appends to a sequence, `del(…)` removes an entry, `key(…)` renames, `line_comment(…)` and `head_comment(…)` edit comments, and `swap(…)` / `move(…)` reorder a list. The [Kubernetes guide](guide/kubernetes) works through each of them.

Not yet available: `select()`, `map()`, `length`, `keys`, comparisons and conditionals, and object/array construction — so a filter cannot yet pick entries by a condition or reshape them. Coming from jq? [What transfers and what does not](guide/from-jq) is the short version.
{% /home-section %}
