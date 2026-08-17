# Bug b011 — A multi-line flow collection is valid YAML that yqr cannot read at all

**Status:** Open (found 2026-08-17, not yet filed upstream)
**Severity:** Medium — an input yqr refuses outright, on the **read** path, so
no filter runs; not silent, but a hard "cannot open your file"
**Component:** noyalib's parser (upstream), reached through every yqr entry
point — `parse_stream` is the first thing both the read engine and the write
engine call
**Related:** `yqr-a001` (the fidelity contract, which presumes yqr can read the
file), `yqr-f016` §5 (found while reviewing the flow-delete work, which is
where the shape came up), `yqr-b004` (the upstream gap catalog)

## 1. Summary

A flow collection spread over several lines is ordinary, valid YAML. noyalib
refuses to parse it, so yqr cannot read such a document, let alone edit one:

```console
$ cat ports.yaml
ports: [
  80,
  443,
]
$ yqr '.' ports.yaml
yqr: io error: failed to parse YAML input: YAML parse error: flow content must
be indented more than the surrounding block
```

The identity filter is enough to trigger it — this is not about any particular
operation.

## 2. It is valid

PyYAML accepts it, with the closing bracket at column 0 and with it indented:

```console
$ python3 -c "import yaml; print(yaml.safe_load(open('ports.yaml')))"
{'ports': [80, 443]}
```

Per the YAML spec, flow content inside a block context must be more indented
than the block — which `  80,` and `  443,` are. The closing `]` is an
indicator rather than content, and every other implementation accepts it at the
parent's column.

The message names the right rule and applies it to the wrong token.

## 3. Why it matters more than the refusal suggests

It is loud, which is the good half: exit 5 with a message, no damage. But it is
a **whole-file** refusal on the read path, so:

- The `yqr-a001` promise — "reads never rewrite bytes" — is vacuous for these
  files, because there is no read.
- `yqr validate` reports the file as unreadable rather than valid, which is
  wrong in the one direction a validator must not be wrong.
- Multi-line flow sequences are common in hand-maintained config (a long
  `args:` or `ports:` list wrapped for width), so this is not an exotic shape.

## 4. Route

Upstream, on the `yqr-b004` §5 `PR-with-fix` precedent. Not yet filed. The fix
is a parser indentation check that exempts the closing indicator, so it is
plausibly small — but this is the parser rather than a mutator, which is a part
of noyalib yqr has not contributed to before, so the estimate is worth
treating as a guess.

## 5. Provenance

Found while reviewing the `yqr-f016` §5 flow-delete work: the reviewer noted
that flow deletes only work on single-line flow collections, and the reason
turned out to be that yqr cannot open a multi-line one. So the delete
limitation is a symptom, and this is the defect.

## 6. Acceptance

- [ ] Filed upstream with the §2 evidence.
- [ ] A released noyalib parses a multi-line flow collection, closing indicator
      at the parent's column included.
- [ ] `yqr '.'` round-trips such a file byte-for-byte, pinned in the `a001`
      fidelity harness.
- [ ] A corpus case covers a wrapped flow sequence, since the shape is common
      enough that its absence is why this went unnoticed.
