# Feature f044 — Binary releases and Homebrew: the three-target set via dist

**Status:** Done — machinery shipped 2026-10-09; the first tag after this
merge produces the first binary release (§6)
**Epic:** Distribution (f044)
**Owner:** yqr maintainers
**Related:** `yqr-m001` §3 and §5.1 (the gap this closes), `yqr-m004`
(crates.io posture — unchanged by this feature)

## 1. Why, and why now

Since v0.1.0 every release carried no binary assets: users install with
`cargo install yqr` or build from source, both of which need a Rust
toolchain. `yqr-m001` §5.1 recorded the gap and required a spec before
closing it. This spec closes it with the minimal artifact set that makes
`brew install` and `curl | sh` work, chosen over both the zero-binary
rung (a source-building tap formula: works today, but every install
compiles) and the full matrix (Windows, ARM Linux: no one has asked).

## 2. The three targets

| Target | Why it is in the minimal set |
|--------|------------------------------|
| `aarch64-apple-darwin` | Every M-series Mac — the bulk of brew installs |
| `x86_64-apple-darwin` | Intel Macs, still supported by Homebrew |
| `x86_64-unknown-linux-musl` | One static binary for any distro; also Homebrew-on-Linux and the `curl \| sh` path |

musl rather than gnu is the deliberate choice for Linux: static linking
removes the glibc-version floor, so one asset serves old and new distros
alike. Deferred until someone asks: `aarch64-unknown-linux-musl` (ARM
servers, Raspberry Pi) and `x86_64-pc-windows-msvc` — each is one line in
`targets` plus a runner dist already knows.

## 3. The tool: dist 0.33.0, not a hand-rolled workflow

dist (formerly cargo-dist, `axodotdev/cargo-dist`) generates and owns
`.github/workflows/release.yml` from config in `Cargo.toml`
(`[workspace.metadata.dist]`). Per tag push matching a version, it: plans
the release, builds each target on its own runner (musl via
cargo-zigbuild, which the generated workflow installs itself), produces
`tar.xz` archives with per-archive `sha256` files plus a combined
`sha256.sum` and a machine-readable `dist-manifest.json`, **creates the
GitHub release** with notes extracted from the matching `CHANGELOG.md`
section (the Keep a Changelog format yqr already uses), attaches a
`yqr-installer.sh` for `curl | sh` installs, and pushes a generated
formula to the Homebrew tap. The workflow also runs its plan step on
pull requests as a dry-run check.

Why a generated workflow over writing the same thing by hand: the
hand-rolled equivalent is a cross-compile matrix, tarball and checksum
scripting, changelog extraction, formula templating, and tap-push
automation — all machinery dist maintains. The trade-offs, weighed:

- **Maintenance status.** The company behind it (axodotdev) wound down
  and its docs domain is gone (docs now at
  axodotdev.github.io/cargo-dist). Measured 2026-10-09: the repository
  is active (pushed that day, v0.33.0 released 2026-09-11, not
  archived), community-maintained, pre-1.0.
- **Shallow lock-in.** `cargo-dist-version = "0.33.0"` pins what CI
  runs. Worst case the project dies: the last generated `release.yml`
  is checked in and keeps working; maintain it by hand from there.
- **Generated code in-tree.** `release.yml` is dist's output, not
  hand-edited; regenerate with `dist init --yes` after config changes.

`cargo publish` is untouched: dist has no crates.io publish job
configured, so publishing stays the separately authorized manual step
(`yqr-m004`).

## 4. The tap

`zoosky/homebrew-tap` (created 2026-10-09, public, README only). dist
owns its contents: each release regenerates `yqr.rb` from the prebuilt
archives and pushes it, so the formula is binary-only (no source-build
fallback), covers macOS and Linux, and holds exactly one version — a
release replaces it. Users run `brew install zoosky/tap/yqr`.

**Prerequisite the repo owner must do once, before the next tag:**
create a GitHub personal access token (classic, `repo` scope) and store
it as the `HOMEBREW_TAP_TOKEN` secret on `zoosky/yqr`. Without it the
release still builds, uploads assets, and publishes release notes; only
the formula push fails (re-runnable after the secret exists).

## 5. Measured before merge

- `dist plan` on this tree names the full asset set: the three archives
  with per-archive checksums, `sha256.sum`, `source.tar.gz`,
  `yqr-installer.sh`, and `yqr.rb`.
- The host-target archive builds locally
  (`dist build --artifacts=local --target=aarch64-apple-darwin`); the
  binary inside reports `yqr 0.10.0 … target: aarch64-apple-darwin` and
  evaluates a filter. The musl target is not buildable on the dev
  machine without cargo-zigbuild; it is exercised by the pipeline's own
  runner.
- `install-updater = false`: no self-updater baked into the binary.

## 6. First live run

The pipeline first fires on the next version tag. Until then no formula
and no binaries exist, so this PR updates only the README's install
section, phrased as "from the next release on"; the site home page's
install card follows in the PR that accompanies the first binary
release, so the deployed site never advertises a command that fails.
Verify on that release: three archives plus checksums and installer
attached, release notes match the changelog section, `yqr.rb` appears in
the tap, and `brew install zoosky/tap/yqr` works on a Mac.

## 7. Acceptance criteria

- [x] `[workspace.metadata.dist]` pins dist 0.33.0, the three targets,
      shell and homebrew installers, and the tap; `release.yml` is the
      generated output of exactly that config.
- [x] `dist plan` names the §5 asset set.
- [x] The host-target archive builds locally and its binary runs.
- [x] `zoosky/homebrew-tap` exists with a README that says the formula
      is generated and where issues go.
- [x] `yqr-m001` §3 and §5.1 describe the new release flow; the
      workflow summaries in `CLAUDE.md`/`AGENT.md` list `release.yml`.
- [x] README documents brew, installer-script, and direct-download
      installs, dated from the next release.
- [ ] First live run verified per §6 (requires the next version tag and
      the `HOMEBREW_TAP_TOKEN` secret).
