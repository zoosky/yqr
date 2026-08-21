---
# `title` drives <title>, og:title and twitter:title, so it carries the words
# people actually search for rather than a section label. The brand is supplied
# separately -- base.html.jinja appends " - yqr", and og:site_name is "yqr".
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
process:
  markdown: false
---
<!--
  Home page -- the pitch, the three outcome cards, and the worked examples.
  `process.markdown: false` in the frontmatter above passes this markup to
  the browser untouched, so what follows is hand-authored HTML rather than
  rendered markdown.
  The theme provides the header menu and footer; the page design lives in
  themes/default/templates/home.html.jinja.
  Feature/spec traceability: yqr-b001, yqr-f003, yqr-m002
-->
  <div class="hero">
    <h1>Query and edit YAML from the command line.</h1>
    <p class="hero-lede">Chart a path to any field in a manifest — and edit
      it without reformatting the file.</p>
    <p>
      <code>yqr</code> is a jq-style filter for YAML. Point it at a manifest file,
      a <code>kubectl get -o yaml</code> dump, or a Helm-rendered bundle, and it
      walks straight to the field you asked for — as a value, not as JSON
      you have to decode back.
    </p>
  </div>

  <div class="chart">
    <div class="chart-label">
      <span class="eyebrow">the filter</span>
      <code>.spec.volumes[0].secret.defaultMode</code>
    </div>
    <svg class="chart-route" viewBox="0 0 96 36" preserveAspectRatio="none" aria-hidden="true">
      <path class="chart-route-path" d="M2 18 H84"></path>
      <circle class="chart-route-dot" cx="88" cy="18" r="3.5"></circle>
    </svg>
    <pre class="chart-yaml"><code>apiVersion: v1
kind: Pod
metadata:
<span class="tok-key">  name:</span> web
spec:
  volumes:
    - name: tls
      secret:
        secretName: web-tls
<span class="hit">        <span class="tok-key">defaultMode:</span> <span class="tok-val">0640</span></span></code></pre>
  </div>

  <div class="outcomes">
    <div class="outcome exact">
      <span class="badge">default</span>
      <pre><span class="prompt">$</span> yqr '.spec.volumes[0].secret.defaultMode' pod.yaml
<span class="out">0640</span></pre>
    </div>
    <div class="outcome drift">
      <span class="badge">--normalize</span>
      <pre><span class="prompt">$</span> yqr --normalize '.spec.volumes[0].secret.defaultMode' pod.yaml
<span class="out">640</span></pre>
    </div>
  </div>

  <p style="color:var(--text-soft); font-size:15px; max-width:74ch; text-wrap:pretty; margin-top:-32px;">
    Kubernetes spells file permissions in octal — <code>defaultMode: 0640</code> on
    a Secret or ConfigMap volume. Read that field through yqr and the value comes
    back exactly as written, because yqr never re-typed it in the first place.
    Only if you opt into the classic
    <a href="#engines"><code>--normalize</code></a> pipeline is the leading zero
    lost: <code>640</code> is a different number.
  </p>

  <section id="paths">
    <div class="section-head">
      <p class="eyebrow">installed paths</p>
      <h2>Where the binary actually lives</h2>
      <p>Install from crates.io with <code class="mono">cargo install yqr</code>,
        or build any of the paths below from a source checkout.</p>
    </div>
    <div class="paths">
      <div class="path-card">
        <h3>On your machine</h3>
        <code class="loc">cargo install yqr</code>
        <p>Pulls the published crate to <code class="mono">~/.cargo/bin/yqr</code>
          — keep that directory on <code class="mono">PATH</code> so plain
          <code class="mono">yqr</code> resolves from any shell, including one
          already piping <code class="mono">kubectl</code> output.</p>
        <code class="loc">cargo build --release</code>
        <p>Building from source instead? The binary lands at
          <code class="mono">target/release/yqr</code> (or
          <code class="mono">cargo install --path .</code> to put a local
          checkout on <code class="mono">PATH</code>).</p>
      </div>
      <div class="path-card">
        <h3>Inside a container image</h3>
        <code class="loc">/usr/local/bin/yqr</code>
        <p>Build it in a multi-stage <code class="mono">Dockerfile</code> and
          copy just the binary into the runtime stage — no Rust
          toolchain, no source tree, in the image that actually ships.</p>
      </div>
    </div>

    <div class="callout" id="engines">
      <strong class="callout-title">Byte-preserving reads are the default.</strong>
      Untouched nodes come back as their original source bytes — comments,
      quoting, indentation, and line endings survive, and the identity filter
      reproduces the input byte-for-byte, no flag required.
      <pre><span class="prompt">$</span> yqr '.' pod.yaml</pre>
      Pass <code>--normalize</code> (<code>-N</code>) to opt into the classic,
      re-serializing pipeline (comments dropped, scalars canonicalized).
      Byte-preserving reads are powered by noyalib's lossless CST —
      yqr's one and only YAML engine.
      See the <a href="https://github.com/zoosky/yqr/tree/main/docs/content/demo">runnable demo</a>
      for an eight-step walkthrough of navigation, iteration, pipes, raw output,
      fidelity mode, and validation.
    </div>

    <!-- Feature f006: write tier v1 (value assignment + in-place edits) -->
    <div class="callout" id="edits">
      <strong class="callout-title">It edits, too — and only the bytes you target.</strong>
      Give it a mutating filter and yqr changes just that node, leaving every
      other byte — comments, indentation, quoting, key order —
      untouched, or refuses. Replace a value with <code>=</code>, append to a
      block sequence with <code>+=</code>, add a key, or drop an entry with
      <code>del(…)</code>:
      <pre><span class="prompt">$</span> yqr <span class="filter">'.spec.replicas = 5'</span> deploy.yaml
<span class="prompt">$</span> yqr <span class="filter">'.spec.ports += 9090'</span> deploy.yaml
<span class="prompt">$</span> yqr <span class="filter">'del(.metadata.labels)'</span> deploy.yaml
<span class="prompt">$</span> yqr <span class="filter">'del(.spec.template)'</span> deploy.yaml   <span class="filter"># a nested block, closed up cleanly</span></pre>
      <code>del</code> removes multi-line and nested block entries as well as
      single-line ones, closing up the gap and leaving every surviving byte
      identical; removing the last entry of a block leaves the collection
      spelled out (<code>{}</code>), since a key with nothing under it reads
      back as null, and removing an item of an inline collection
      (<code>[a, b]</code>) takes exactly one separator with it.
      Add <code>-i</code> (<code>--in-place</code>) and the file is rewritten
      atomically — a <code>git diff</code> touches only the line you
      changed. An edit that would restructure the document is refused (exit 5)
      rather than emitted, and under <code>-i</code> the file is left untouched.
      <pre><span class="prompt">$</span> yqr -i <span class="filter">'.spec.replicas = 5'</span> deploy.yaml
<span class="prompt">$</span> git diff deploy.yaml   <span class="filter"># one line</span></pre>
    </div>

    <!-- Feature f012: the validate subcommand -->
    <div class="callout" id="validate">
      <strong class="callout-title">Validate after every edit.</strong>
      One command answers whether a file is still correct YAML — and a
      pass certifies more than "parses": the parsed documents must reproduce
      the input byte-for-byte, the same invariant behind yqr's fidelity
      reads. Failures are compiler-style diagnostics with a stable code, a
      clickable location whenever a position is known, and a suggested fix,
      so humans and agents can act on them. Edit, then verify:
      <pre><span class="prompt">$</span> yqr -i <span class="filter">'.spec.replicas = 5'</span> deploy.yaml   <span class="filter"># edit</span>
<span class="prompt">$</span> yqr validate --strict deploy.yaml        <span class="filter"># verify -- silent, exit 0</span></pre>
      When the file is not correct YAML — a hand edit gone wrong, a
      half-resolved merge, a truncated write — the verdict names the
      spot:
      <pre><span class="prompt">$</span> yqr validate deploy.yaml
error[Y001]: expected a node but found StreamEnd
  --> deploy.yaml:3:3
  |
3 | b: [1,
  |   ^</pre>
      <code>--strict</code> also flags duplicate mapping keys
      (<code>Y101</code>) — accepted last-wins by ordinary reads, so a
      bad edit silently drops data — reporting every duplicate,
      <code>&lt;&lt;</code> merge keys included, with the positions of both
      occurrences. Keys that collide after string conversion are refused
      outright (<code>Y102</code>), non-UTF-8 input is a coded finding
      (<code>Y003</code>), a mapping value that is not indented past its key
      is flagged by default (<code>Y103</code>) because yqr's engine reads
      such a file and other implementations refuse it, and a file containing
      unresolved merge-conflict markers gets a dedicated hint anchored at the
      first marker. Exit codes
      are scriptable: 0 all valid, 1 validation findings, 5 an input could
      not be read. Stdin is explicit (<code>yqr validate -</code>); an empty
      file list is a usage error, never a silent "all valid".
    </div>
  </section>

  <section id="recipes">
    <div class="section-head">
      <p class="eyebrow">two ways to run it against a cluster</p>
      <h2>From an operator's shell, or from inside the image</h2>
    </div>
    <div class="cards">

      <div class="card">
        <h3>Piped from kubectl</h3>
        <p>Standard operator loop: dump a resource as YAML, pull one field out
          of it, move on.</p>

        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> kubectl get pods -o yaml | yqr -r <span class="filter">'.items[] | .metadata.name'</span></pre>
          <p class="note">One pod name per line.</p>
        </div>

        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> kubectl get pod web-0 -o yaml | yqr -r <span class="filter">'.spec.containers[0].image'</span></pre>
          <p class="note">The primary container's image, unquoted.</p>
        </div>

        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> kubectl get pod web-0 -o yaml | yqr -r <span class="filter">'.spec.initContainers[]? | .image'</span></pre>
          <p class="note">Init container images when the pod has any —
            the trailing <code class="mono">?</code> keeps pods with none
            from erroring the pipeline.</p>
        </div>

        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr validate --strict manifests/*.yaml</pre>
          <p class="note">A gate before <code class="mono">kubectl apply</code>:
            every manifest must parse, round-trip byte-for-byte, and carry no
            duplicate keys. Exit 1 with a located diagnostic if not — and
            an empty file list is a loud usage error, so a glob that matches
            nothing never passes as “all valid”.</p>
        </div>
      </div>

      <div class="card">
        <h3>Inside a container image</h3>
        <p>Bake the binary in, then use it in an init container to read a
          mounted manifest or ConfigMap before the main container starts.</p>
        <pre class="dockerfile"><span class="rem"># -- build --</span>
<span class="kw">FROM</span> rust:1.97-slim <span class="kw">AS</span> build
WORKDIR /src
COPY . .
RUN cargo build --release

<span class="rem"># -- runtime --</span>
<span class="kw">FROM</span> debian:bookworm-slim
COPY --from=build /src/target/release/yqr /usr/local/bin/yqr
<span class="kw">ENTRYPOINT</span> [<span class="mono">"yqr"</span>]</pre>
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.data.enableBeta'</span> /config/values.yaml</pre>
          <p class="note">Read a flag out of a mounted ConfigMap and hand it
            to the next step — a common init-container job.</p>
        </div>
      </div>

    </div>
  </section>

  <section id="beyond">
    <div class="section-head">
      <p class="eyebrow">beyond the cluster</p>
      <h2>It's not just Kubernetes</h2>
      <p>Anything that's YAML takes the same filters. Three more places yqr
        earns its keep.</p>
    </div>
    <div class="trio">

      <div class="mini-card">
        <h3>CI/CD pipelines</h3>
        <p>GitHub Actions workflows are YAML. Audit what a job actually runs
          without opening the file — these two ran against this
          repo's own <code class="mono">ci.yml</code>.</p>
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.jobs.test.["runs-on"]'</span> ci.yml</pre>
          <p class="note">ubuntu-latest — bracket syntax reaches keys a
            bareword can't spell, like <code class="mono">runs-on</code>.</p>
        </div>
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.jobs.test.steps[1].with.toolchain'</span> ci.yml</pre>
          <p class="note">1.97 — confirm the pinned Rust version without
            scrolling past the cache step.</p>
        </div>
      </div>

      <div class="mini-card">
        <h3>Docker Compose</h3>
        <p>Check what a compose file is about to pull and expose before you
          run it.</p>
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.services[] | .image'</span> compose.yaml</pre>
          <p class="note">yqr-demo:latest, postgres:16 — every image
            referenced, one per line.</p>
        </div>
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.services.web.environment.LOG_LEVEL'</span> compose.yaml</pre>
          <p class="note">debug — one config value, no grep.</p>
        </div>
      </div>

      <div class="mini-card">
        <h3>Ansible playbooks</h3>
        <p>A playbook is a YAML list of plays — walk it like any other
          sequence.</p>
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.[0].tasks[] | .name'</span> playbook.yml</pre>
          <p class="note">Install nginx, Start nginx — every task in
            the first play, at a glance.</p>
        </div>
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.[0].hosts'</span> playbook.yml</pre>
          <p class="note">web — which hosts that play targets.</p>
        </div>
      </div>

    </div>
  </section>

  <section id="further">
    <div class="section-head">
      <p class="eyebrow">further afield</p>
      <h2>Three more, shown in full</h2>
      <p>Same grammar, different files — this time with the source
        shown, so nothing here has to be taken on faith.</p>
    </div>

    <div class="explicit-card">
      <h3>OpenAPI specs</h3>
      <p class="lede">An OpenAPI document is plain YAML. Point yqr at it to
        pull a specific operation's details out of a spec someone else
        wrote, without loading it into an editor.</p>
      <pre class="snippet"><code><span class="tok-key">paths:</span>
  /widgets/{id}:
    get:
      <span class="tok-key">summary:</span> Get a widget
      responses:
        "200":
          <span class="tok-key">description:</span> OK</code></pre>
      <div class="explicit-recipes">
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.paths.["/widgets/{id}"].get.summary'</span> openapi.yaml
Get a widget</pre>
          <p class="note">Path keys have slashes and braces, so a bareword
            can't spell them — bracket syntax reaches them anyway,
            the same way <code class="mono">.["runs-on"]</code> did for the
            CI workflow above.</p>
        </div>
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.paths.["/widgets"].get.responses.["200"].description'</span> openapi.yaml
OK</pre>
          <p class="note">Status codes are string keys, and an unquoted
            <code class="mono">200</code> in a filter would try to read a
            number. Bracket syntax reaches the string key
            <code class="mono">"200"</code> exactly as the spec wrote it.</p>
        </div>
      </div>
    </div>

    <div class="explicit-card">
      <h3>Prometheus alerting rules</h3>
      <p class="lede">Alerting rules are a YAML list of groups, each holding
        a list of rules. Reading one back tells you exactly what will page
        someone, and at what threshold.</p>
      <pre class="snippet"><code><span class="tok-key">groups:</span>
  - name: api-slos
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status="5xx"}[5m]) &gt; 0.05
        for: 10m
        labels:
          severity: page</code></pre>
      <div class="explicit-recipes">
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.groups[0].rules[] | .alert'</span> rules.yaml
HighErrorRate
HighLatency</pre>
          <p class="note">Every alert name in the first group, without
            reading through a file's worth of PromQL to find them.</p>
        </div>
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.groups[0].rules[0].expr'</span> rules.yaml
rate(http_requests_total{status="5xx"}[5m]) &gt; 0.05</pre>
          <p class="note">The exact expression for that alert — useful
            when you just need to confirm the number that pages someone,
            not re-read the whole rules file.</p>
        </div>
      </div>
    </div>

    <div class="explicit-card">
      <h3>Application config</h3>
      <p class="lede">Most services ship a YAML config file alongside the
        binary — database targets, ports, feature flags. yqr reads it
        the same way it reads anything else.</p>
      <pre class="snippet"><code><span class="tok-key">database:</span>
  host: db.internal
  port: 5432
<span class="tok-key">featureFlags:</span>
  newCheckout: true
  betaSearch: false</code></pre>
      <div class="explicit-recipes">
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.database.host'</span> application.yaml
db.internal</pre>
          <p class="note">Confirm which database an environment's config
            actually points at before you run a migration against it.</p>
        </div>
        <div class="recipe">
          <pre class="cmd"><span class="prompt">$</span> yqr -r <span class="filter">'.featureFlags.newCheckout'</span> application.yaml
true</pre>
          <p class="note">Read a single feature flag's value straight out of
            the file that ships with the deploy, instead of grepping for
            it.</p>
        </div>
      </div>
    </div>
  </section>

  <section id="grammar">
    <div class="section-head">
      <p class="eyebrow">filter grammar</p>
      <h2>What yqr can walk today</h2>
      <p>yqr is at milestone M0. This is the whole grammar — every
        recipe on this page is built from it.</p>
    </div>
    <table>
      <thead>
        <tr><th>Filter</th><th>Meaning</th></tr>
      </thead>
      <tbody>
        <tr><td><code>.</code></td><td class="meaning">Identity</td></tr>
        <tr><td><code>.foo</code></td><td class="meaning">Field access</td></tr>
        <tr><td><code>.a.b</code></td><td class="meaning">Nested field access</td></tr>
        <tr><td><code>.[n]</code></td><td class="meaning">Array index (<code>.[-1]</code> counts from the end)</td></tr>
        <tr><td><code>.[]</code></td><td class="meaning">Iterate sequence elements / mapping values</td></tr>
        <tr><td><code>a | b</code></td><td class="meaning">Pipe</td></tr>
        <tr><td><code>f?</code></td><td class="meaning">Suppress runtime errors from <code>f</code> (e.g. iterating a field that turns out to be missing or the wrong shape)</td></tr>
      </tbody>
    </table>
    <p class="not-yet">
      Not yet available: <code>select()</code>, <code>map()</code>,
      <code>keys</code>, arithmetic, and object/array construction. Reach for
      <code>kubectl</code>'s own <code>-o jsonpath</code> or a
      <code>grep</code> in the meantime if a recipe needs one of those.
    </p>
  </section>
