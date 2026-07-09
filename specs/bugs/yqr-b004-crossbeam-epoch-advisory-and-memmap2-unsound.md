# Bug b004 — `cargo audit` flags RUSTSEC-2026-0204 (crossbeam-epoch) and RUSTSEC-2026-0186 (memmap2)

**Status:** Fixed
**Severity:** Low — both findings sit in transitive dependencies; one is a
build/test-only dev dependency and the other is an unsoundness warning, not an
exploited runtime path. Neither changes yqr's own code. The value is keeping the
advisory gate (`cargo audit` in `.github/scripts/local-ci.sh`) green.
**Owner:** yqr maintainers
**Last updated:** 2026-07-09
**Affects:** the dependency graph pinned by `Cargo.lock`; surfaced by the
`cargo audit` step of the local CI mirror (`.github/scripts/local-ci.sh`).
**Component:** `Cargo.lock` (transitive dependencies of `criterion` and
`rust-yaml`)
**Related:** `yqr-m001` (CI / release process)

## 1. Summary

Running the advisory gate reported one vulnerability and one warning:

```text
Crate:    crossbeam-epoch
Version:  0.9.18
Title:    Invalid pointer dereference in `fmt::Pointer` impl for `Atomic` and
          `Shared` when the underlying pointer is invalid
ID:       RUSTSEC-2026-0204
Solution: Upgrade to >=0.9.20

Crate:    memmap2
Version:  0.9.10
Warning:  unsound
Title:    Unchecked pointer offset in crate `memmap2`
ID:       RUSTSEC-2026-0186

error: 1 vulnerability found!
warning: 1 allowed warning found
```

## 2. Root cause / exposure

- **crossbeam-epoch 0.9.18** — pulled in only as a transitive dev dependency:
  `criterion -> rayon -> rayon-core -> crossbeam-deque -> crossbeam-epoch`. It
  never ships in the `yqr` binary; it affects the benchmark harness build only.
- **memmap2 0.9.10** — a runtime transitive dependency of `rust-yaml` (both the
  crates.io crate and the `feat/roundtrip-document` fork). The advisory is an
  unsoundness warning about an unchecked pointer offset, not a known exploited
  path in yqr's usage.

## 3. Fix

Lockfile-only bumps to the patched/newer versions; no source or manifest range
changes were required (both satisfy the existing semver constraints):

```bash
cargo update -p crossbeam-epoch --precise 0.9.20   # 0.9.18 -> 0.9.20
cargo update -p memmap2                             # 0.9.10 -> 0.9.11
```

`cargo update -p memmap2` to 0.9.11 clears the unsoundness warning as well.

## 4. Verification

```text
$ cargo audit
    Scanning Cargo.lock for vulnerabilities (97 crate dependencies)
    (no vulnerabilities, no warnings)

$ cargo build --all-targets --locked      # ok
$ cargo test --all-targets --all-features --locked   # all suites pass
```
