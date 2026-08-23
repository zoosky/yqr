# Bug b023 — `--version` prints an empty commit hash when yqr is installed from crates.io

> **Historical: resolved.** yqr no longer behaves as described below. The
> **Status** line records what fixed it and when; the rest is kept as the
> reproduction and the reasoning, written in the present tense of the time it
> was filed.

**Status:** Resolved — 2026-08-23, found by smoke-testing the published
`0.7.0` binary rather than a local build
**Severity:** Low — cosmetic, but it is the output a bug reporter pastes, and
it degrades to a malformed empty field rather than to the fallback that exists
**Component:** `build.rs`, the `GIT_HASH` stamp consumed by `src/cli.rs`
**Related:** `yqr-m001` (the release process), `yqr-m004` (crates.io posture)

## 1. Summary

Every yqr installed from crates.io reports an empty commit:

```console
$ cargo install yqr --version 0.7.0
$ yqr --version
yqr 0.7.0 (, built 2026-08-23 17:42:56 UTC)
target: aarch64-apple-darwin
```

The parentheses hold nothing. A build from a git checkout is fine, which is
why it survived every release to date — the binary anyone on the team runs is
built in the repository.

## 2. Cause: `output()` is `Ok` for a command that failed

`build.rs` asked git and never checked whether git succeeded:

```rust
let git_hash = Command::new("git")
    .args(["rev-parse", "--short", "HEAD"])
    .output()
    .ok()
    .and_then(|o| String::from_utf8(o.stdout).ok())
    .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string());
```

Outside a repository, `git rev-parse --short HEAD` exits **128** and writes
*"fatal: not a git repository"* to **stderr**, leaving stdout empty. Measured:

```console
$ cd /tmp/notgit && git rev-parse --short HEAD; echo "exit=$?"
fatal: not a git repository (or any of the parent directories): .git
exit=128
```

So `output()` is `Ok`, `.ok()` is `Some`, `String::from_utf8(vec![])` is
`Ok("")`, and the closure returns `""`. The `"unknown"` fallback is
**unreachable in practice** — `map_or_else`'s `None` arm needs the git binary
to be missing entirely, or stdout to be invalid UTF-8. The one case it was
written for is the one case it does not catch.

## 3. "unknown" was the wrong target anyway

The commit is not actually unavailable in a published crate. Cargo writes it
into the tarball when it packages one:

```console
$ tar xzf yqr-0.7.0.crate && cat yqr-0.7.0/.cargo_vcs_info.json
{
  "git": {
    "sha1": "bf703183954204133cde04168b6c9d0b7492d724"
  },
  "path_in_vcs": ""
}
```

`bf70318` is exactly the commit `v0.7.0` was cut from. So the fix is not to
degrade gracefully but to **read the answer that is already on disk**.

## 4. Fix

Two routes, tried in order, because a crate is built in two different places:

1. **`git`**, in a checkout — now gated on `status.success()` and a non-empty
   result, which is the check whose absence was the bug.
2. **`.cargo_vcs_info.json`**, from a published tarball. Scanned for the
   `sha1` field rather than parsed: it is Cargo's own file, machine-written
   and three keys deep, and a JSON dependency for one field would be paid for
   by every downstream build. Truncated to 7 characters so the two routes are
   indistinguishable in the output.

`"unknown"` remains for a source tree that is neither, and is now reachable.

## 5. Verification

Three builds, three outcomes:

| Build from | Reports |
|---|---|
| the git checkout | `yqr 0.7.0 (bf70318, …)` |
| the unpacked `0.7.0` tarball, no `.git` | `yqr 0.7.0 (bf70318, …)` |
| the same, with `.cargo_vcs_info.json` deleted | `yqr 0.7.0 (unknown, …)` |

The middle row is the bug, and it now agrees with the first — an installed
binary names the commit it was built from.

## 6. The same code is in accent, unexercised

`accentcms`'s `build.rs` carries the identical construct, including the
unreachable fallback. It has never been observed because `accentcms` is not
published to crates.io — every accent binary is built in its checkout, where
git answers. `accent --version` reporting `0.25.0 (56c084f)` is that path
working, not a different implementation.

Worth porting before accent's first publish, not after: the defect only
appears in the artefact strangers install, which is the worst place to find
out.

## 7. Reproduction

```console
$ cargo install yqr --version 0.7.0 --root /tmp/probe
$ /tmp/probe/bin/yqr --version        # yqr 0.7.0 (, built …)  -- wrong
$ cd /path/to/yqr && cargo run -- --version   # yqr 0.7.0 (bf70318, …)  -- fine
```
