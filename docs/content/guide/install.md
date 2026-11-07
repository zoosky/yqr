---
# Traceability: the release pipeline and its artifacts are yqr-f044 and
# yqr-m001 §2.4-2.5; the wasm embedder package is yqr-k003 §7. Every
# console block was run against the v0.12.0 release assets on 2026-11-07:
# the archive and checksum were downloaded back from the release, the
# shasum verdict (including its trailing-blank-line warning) is the real
# output, and the --version and smoke-test blocks ran the released
# aarch64-apple-darwin binary.
title: Install yqr
lead: >-
  Homebrew, a shell installer, `cargo install`, or a prebuilt archive you verify yourself -- and how to confirm the binary works.
description: >-
  Install yqr with Homebrew, the release shell installer, cargo install, or
  a prebuilt archive with checksum verification, then confirm the binary
  runs.
menu:
  title: Install
  order: 1
---

# Install

Every path below ends with a `yqr` binary on your `PATH`. Pick the one
that matches how you manage tools; they all install the same program.

## Homebrew

```console
$ brew install zoosky/tap/yqr
```

Installs the prebuilt binary — Apple Silicon and Intel Macs, and x86_64
Linux. The release pipeline pushes each version's formula to the tap, so
`brew upgrade` tracks new releases.

## Shell installer

```console
$ curl --proto '=https' --tlsv1.2 -LsSf https://github.com/zoosky/yqr/releases/latest/download/yqr-installer.sh | sh
```

Detects your platform, downloads the matching archive from the latest
GitHub release, and installs the binary into `$CARGO_HOME/bin`
(`~/.cargo/bin` by default) — no Rust toolchain involved. Prefer not to
pipe a script into `sh`? The next section is the same download done by
hand.

## Prebuilt archive

Each [GitHub release](https://github.com/zoosky/yqr/releases/latest)
attaches an archive per platform, with a SHA-256 checksum beside it:

| Archive | Platform |
|---|---|
| `yqr-aarch64-apple-darwin.tar.xz` | macOS, Apple Silicon |
| `yqr-x86_64-apple-darwin.tar.xz` | macOS, Intel |
| `yqr-x86_64-unknown-linux-musl.tar.xz` | Linux x86_64 — static musl build, runs on any distribution |

Download, verify, unpack:

```console
$ curl -sLO https://github.com/zoosky/yqr/releases/latest/download/yqr-aarch64-apple-darwin.tar.xz
$ curl -sLO https://github.com/zoosky/yqr/releases/latest/download/yqr-aarch64-apple-darwin.tar.xz.sha256
$ shasum -a 256 -c yqr-aarch64-apple-darwin.tar.xz.sha256
yqr-aarch64-apple-darwin.tar.xz: OK
shasum: WARNING: 1 line is improperly formatted
$ tar xf yqr-aarch64-apple-darwin.tar.xz
```

`OK` is the verdict; the warning is `shasum` objecting to the checksum
file's trailing blank line and does not affect it. The archive holds the
`yqr` binary alongside the licenses, README, and changelog — copy the
binary anywhere on your `PATH`.

## cargo install

```console
$ cargo install yqr
```

Builds the [published crate](https://crates.io/crates/yqr) from source
with your own toolchain (Rust 1.99 or later) and installs it to
`~/.cargo/bin/yqr`. Using yqr as a Rust library rather than a binary?
Depend on the crate instead — the [home page](/#paths) shows the
`default-features = false` form that leaves the CLI layer out, and
[docs.rs/yqr](https://docs.rs/yqr) documents the API.

## From a source checkout

```console
$ git clone https://github.com/zoosky/yqr
$ cd yqr
$ cargo build --release
```

The binary lands at `target/release/yqr`; `cargo install --path .` puts
it on your `PATH` instead.

## Confirm it works

```console
$ yqr --version
yqr 0.12.0 (b484fc9, built 2026-11-07 10:30:44 UTC)
target: aarch64-apple-darwin
$ echo 'a: 1' | yqr '.a'
1
```

The version line names the release, the exact commit it was built from,
and the target — worth pasting into a bug report.

## No install at all

The [playground](../playground) runs the same engine in your browser,
compiled to WebAssembly: paste YAML, run filters, and watch the
byte-for-byte guarantee hold, without touching your machine. Embedding
that engine in your own page is the playground's closing note — every
release attaches the built wasm package as `yqr-wasm-pkg.tar.gz`.
