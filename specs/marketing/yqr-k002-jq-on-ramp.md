# Marketing k002 — An on-ramp page for people who already know jq

**Status:** Draft — filed 2026-08-25, measured before any prose was written
(`k001` §2's rule). The page is not built.
**Owner:** yqr maintainers
**Last updated:** 2026-08-25
**Related:** `yqr-k001` (the content plan this extends, and its measure-first
rule), `yqr-f001` (the grammar that bounds what the page may claim), `yqr-f008`
(arithmetic and `|=`), `yqr-f017` (`to_entries`), `yqr-f007` (the write forms
that have no jq counterpart)

## 1. Why

yqr sells itself in jq's vocabulary and never says where the resemblance
stops. The home page's `<title>` is literally **"jq for YAML"**. The
Kubernetes guide says *"Paths look like jq, because that is the idea"*. The
README, the demo script and the CLI's own `--help` all say "jq-style".

Nowhere does anything tell a jq user which of their habits survive. They find
the boundary by hitting it, one parse error at a time, and the first one they
hit is as likely to be `select` as `.a.b`.

The intent is already being sold. This page is what backs it.

## 2. Why this is not a `/compare/` page

**jq cannot read YAML.** There is no shared input, so there is nothing to run
both tools over and diff — and that is exactly what `/compare/` promises its
readers: *"every claim here comes from running both tools over the same file
and diffing the result."*

A comparison page against a tool that cannot do the job also reads as
strawmanning, which is the failure mode `k001` §2 spent its whole argument
avoiding. It would sit next to `/compare/yq`, which is measured, and cheapen
it.

The reader is not asking "which is better". They are asking **"I know jq —
what do I type?"** That is an on-ramp, and it belongs in `/guide/`.

## 3. The measurement

Run 2026-08-25 against **yqr v0.7.1** and **jq 1.8.2**, on a fixture with a
scalar, a sequence and a nested mapping. Every row below was executed.

### 3.1 Transfers with identical spelling (14)

`.` · `.name` · `.a.b.c` · `.["name"]` · `.tags[0]` · `.tags[-1]` · `.tags[]` ·
`.mapping[]` · `a | b` · `f?` · a missing field yielding `null` ·
`to_entries` · `del(.a)` · arithmetic `+ - * / %` including `+` on strings

Plus the assignment forms, which read the same way they do in jq:
`.a = 1`, `.a |= (. + 1)`, and a computed right-hand side (`.n = .n + 1`).

This is the part worth leading with. Someone who knows jq can already read
and edit a YAML file with yqr without learning anything new.

### 3.2 Same spelling, different meaning — the traps (1, and it is sharp)

**`+=`.** In jq it is arithmetic or concatenation, and the right-hand side of
a list append is a *list*. In yqr it means **append one element to a
sequence**, and the right-hand side is the element:

| | jq 1.8.2 | yqr 0.7.1 |
|---|---|---|
| `.tags += ["x"]` | appends `x` | **parse error** — no array literal |
| `.tags += "x"` | appends the *characters* | appends `x` as one item |
| `.replicas += 1` | `4` | **runtime error** — `+=` wants a sequence |

The yqr spelling for the last one is `.replicas |= (. + 1)` or
`.replicas = .replicas + 1`, both of which work.

A trap is worth more page space than a match. This is the one place where a
jq habit produces a *wrong result or a confusing error* rather than an honest
"unknown token", so it goes above the fold of the table.

### 3.3 No equivalent (18 measured)

`select` · `map` · `length` · `keys` · `from_entries` · `has` · `//` ·
`if/then/else` · comparisons (`==`) · `and`/`or`/`not` · object construction
`{}` · array construction `[]` · the comma operator · string interpolation
`"\(.x)"` · recursive descent `..` · `add` · `join` · `sort_by`

Most fail as `parse error: expected Dot but found Ident("...")`, which is a
serviceable message but does not tell a jq user *why*. Two do better and are
worth quoting on the page because they teach the model:

```console
$ yqr '.name, .replicas' f.yaml
yqr: lex error: unexpected character ',' at position 5: yqr has no ',' operator;
a function separates its arguments with ';', as in swap(.xs; 0; 1)
```

### 3.4 Where yqr goes past jq (6)

`key(...)` · `line_comment(...)` · `head_comment(...)` · `swap(...)` ·
`move(...)` · `-i` with a byte guarantee

**jq has no equivalent and cannot have one.** It processes data; a comment and
a source byte are not data. This is the page's punchline and the reason it is
not a list of yqr's shortcomings: the two languages stop overlapping in *both*
directions, and where yqr extends past jq is precisely what an editor needs
and a processor cannot express.

## 4. What the page says

Order matters, because the naive order — "here is everything we lack" — is
the version that should not ship.

1. **You already know most of this.** §3.1, as a table. First screen.
2. **One habit to unlearn.** §3.2, `+=`.
3. **What yqr does not have, and why.** §3.3 as a list, framed as scope: yqr
   walks and edits documents that exist; it does not compute over data. Name
   the escape hatch honestly — `yq -o=json … | jq` — and note that it routes
   through **yq**, not yqr, because `yqr --help` has no JSON output and should
   not grow one (`yqr-a001` §8).
4. **What jq cannot do at all.** §3.4. Comments, byte fidelity, in-place
   edits that survive review.

## 5. Placement

`/guide/from-jq`, a fifth guide page. Not a new nav section — `k001` §5 kept
the header to two new entries on purpose, and this sits under the existing
Guide menu.

## 6. Risks

- **Reading as an apology.** §3.3 is the longest section by item count and the
  least useful by value. Mitigated by order (§4) and by never listing a gap
  without the scope reason next to it.
- **Inviting the feature matrix.** `k001` §2's corollary applies unchanged:
  route by job, never by feature count. jq wins a feature count and it does
  not matter, because it cannot open the file.
- **Rot.** The §3 table is grammar-shaped, so every yqr release that adds a
  filter moves a row from §3.3 to §3.1. It joins the check `yqr-m001` §3 now
  carries.

## 7. Found while measuring: a wrong claim about jq on a live page

`/guide/enumerate` says:

> jq sorts object keys. yqr does not, and this is one of the places that
> difference is doing real work rather than being a footnote.

**Measured on jq 1.8.2, that is wrong**, and wrong in yqr's favour on the very
operation the paragraph is about:

```console
$ jq -c 'to_entries[] | .key' ord.json     # zebra apple mango -- insertion order
$ jq -c 'keys' ord.json                    # ["apple","mango","zebra"] -- sorted
$ jq -c 'keys_unsorted' ord.json           # ["zebra","apple","mango"]
```

jq's **`keys`** sorts. `to_entries` does not, and neither does `.[]`, and
neither does jq's output unless you pass `-S`. The paragraph is about
`to_entries` ordering, where jq behaves exactly as yqr does — so the sentence
claims a difference that is not there.

Corrected in the same change that files this spec. Recorded here rather than
only in the commit because it is the third competitor claim this content has
got wrong by asserting instead of measuring (`k001` §8 has the other two), and
that pattern is the finding.

## 8. Acceptance criteria

- [ ] `/guide/from-jq` exists, in the Guide menu, ordered after the existing
      four.
- [ ] Every filter shown was run against the released binary, jq claims
      against a named jq version, both stamped in the frontmatter.
- [ ] §3.1 is the first table on the page; §3.3 never appears without its
      scope reason.
- [ ] The `+=` trap is stated as a table of three shapes, not prose.
- [ ] No feature matrix.
- [x] The `/guide/enumerate` jq-sorting claim is corrected (§7).
