# Feature f023 — Adopt noyalib 0.0.27: the last open engine bug

**Status:** Done — 0.0.27 adopted, `b016` verified fixed against the published
crate and closed (2026-08-21)
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f020` (the 0.0.26 adoption this succeeds), `yqr-b016` (the
bug this closes), `yqr-f017` (whose output made `b016` routine and whose test
pinned it), `yqr-m003` (the pin-what-it-does rule that made this bump legible)

## 1. Scope

Bump `noyalib = "0.0.26"` to `0.0.27` and close `b016`.

0.0.27 carries three functional changes. One is yqr's (noyalib#298, `b016`);
two are the maintainer's and land in territory yqr tests, so they are verified
rather than assumed:

| upstream | whose | what |
|---|---|---|
| #298 | yqr | the serializer writes no trailing whitespace that is not string content |
| #304 | maintainer | aliases resolve on the streaming replay branch |
| #305 | maintainer | only a plain `<<` scalar is a merge key |

## 2. Verification, run 2026-08-21 against the published crate

### 2.1 `b016`, both faces

The dangling indicator, which `f017`'s output made routine:

```console
$ printf 'm:\n  a:\n    x: 1\n' | yqr '.m | to_entries' | sed -n l
- key: a$
  value:$
    x: 1$
```

The empty block-scalar line, reachable through `--normalize` on any document
with one:

```console
$ printf 'k: |\n  a\n\n  b\n' | yqr --normalize '.' | sed -n l
k: |$
  a$
$
  b$
```

Both were a trailing space before this release. The controls hold: a pair whose
value is an inline scalar or an empty collection is byte-identical to 0.0.26,
which is the line the fix had to draw.

### 2.2 The two changes that are not yqr's

`b016`'s fix is in the serializer; #304 and #305 are in the loader, and yqr has
merge-key and alias tests that would notice:

```console
$ yqr '.' merge.yaml | diff - merge.yaml    # byte-exact
$ yqr -r '.s.t' merge.yaml                  # 30   (inherited through <<)
$ yqr -r '.s.n' merge.yaml                  # web  (the entry's own)
```

Whole suite green untouched, including `engine/key/merge-produced-key-is-null`
— the corpus case that asserts a key reaching a mapping through `<<` owns no
token in the file. Nothing moved.

## 3. What the pin did

`f017` pinned `b016` **as it behaved** rather than working around it, on
`yqr-m003`'s rule. This bump is what that rule is for: the suite came back with
exactly one failure, `to_entries_output_carries_the_emitters_trailing_space`,
which is the pin saying *the bump changed this*.

Without it the fix would have arrived silently, and the guide would still be
carrying a paragraph apologising for a wart that no longer exists.

The test is now flipped and renamed to what it asserts, with two more added:
the block-scalar face, and the control the fix turned on — that whitespace a
*string* owns survives, which is why the fix keys on what the string holds
rather than on how the emitted line looks.

## 4. `b016` closes, and with it the engine backlog

Every bug yqr has filed against noyalib is now fixed in a **published**
release:

| bug | fixed in | filed as |
|---|---|---|
| `b011` wrapped flow collection could not be parsed | 0.0.25 | noyalib#285 / #286 |
| `b012` no key could be inserted beside a dotted one | 0.0.25 | noyalib#288 / #289 |
| `b013` inserted scalar took a document-wide quote style | 0.0.25 | noyalib#290 |
| `b014` sole-entry replacement at its key's own column | 0.0.25 | noyalib#283 / #284 |
| `b015` wrapped-flow delete left a whitespace-only line | 0.0.26 | noyalib#294 / #296 |
| `b016` serializer wrote trailing whitespace | 0.0.27 | noyalib#297 / #298 |

Six bugs, three releases, four days. Five of the six were fixed by yqr's own
commits upstream.

Worth recording because the same six were, three weeks ago, the reason the
write tier kept its own implementation of things upstream also implemented
(`yqr-f007` §6). That argument is unchanged — `yqr-f019` §4 re-ran it and kept
`delete_entry` on the standing reason — but the *evidence* it rests on is now
a closed set rather than an open one.

## 5. Acceptance criteria

- [x] 0.0.27 published; the pin moves and `Cargo.lock` shows noyalib moving
      and nothing else.
- [x] Both faces of `b016` verified against the **published** crate, with the
      controls checked byte-identical to 0.0.26 (§2.1).
- [x] #304 and #305 verified not to move yqr's merge-key or alias behaviour
      (§2.2), rather than assumed harmless because they are not yqr's.
- [x] The `b016` pin flipped, renamed to what it asserts, and joined by the
      block-scalar face and the string-owns-whitespace control (§3).
- [x] The guide's wart paragraph removed; `llms-full.txt` still clean.
- [x] `b016` moved to Resolved; `yqr-b000` shows one open bug, and it is
      accent's.
- [x] Full suite green on the new pin with yqr's own code unchanged;
      `local-ci.sh` clean.
