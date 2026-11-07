#!/usr/bin/env bash
# Build the yqr-wasm package and tar it for the GitHub release.
#
# Runs as a dist extra-artifact build (Cargo.toml
# [[workspace.metadata.dist.extra-artifacts]]) on the release's global
# artifacts job, and works locally the same way. Produces, in the repo
# root:
#
#   yqr-wasm-pkg.tar.gz          wasm-pack's pkg/: module, JS glue,
#                                .d.ts, package.json
#   yqr-wasm-pkg.tar.gz.sha256   its checksum
#
# The tarball is the embedder artifact: a pinned, checksummed build of
# the bindings from the tagged commit, without an npm registry (the
# decision is recorded in specs/marketing/yqr-k003-wasm-playground.md).
# wasm-pack is pinned to the same 0.15.1 as ci.yml's wasm lane; the
# rust-toolchain.toml pin covers the compiler.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${root}"

if ! command -v wasm-pack >/dev/null; then
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) triple="x86_64-unknown-linux-musl" ;;
    Darwin-arm64) triple="aarch64-apple-darwin" ;;
    Darwin-x86_64) triple="x86_64-apple-darwin" ;;
    *) echo "wasm-pkg-tarball.sh: no wasm-pack and no known prebuilt for $(uname -s)-$(uname -m)" >&2; exit 1 ;;
  esac
  curl -sSfL "https://github.com/rustwasm/wasm-pack/releases/download/v0.15.1/wasm-pack-v0.15.1-${triple}.tar.gz" \
    | tar -xz --strip-components=1 -C "${CARGO_HOME:-$HOME/.cargo}/bin" --wildcards '*/wasm-pack'
fi
wasm-pack --version

rustup target add wasm32-unknown-unknown

wasm-pack build yqr-wasm --target web --release

# Stage under a top-level directory (extracting must not scatter files
# into the consumer's working directory) and drop wasm-pack's .gitignore.
stage="target/wasm-pkg-stage"
rm -rf "${stage}"
mkdir -p "${stage}/yqr-wasm-pkg"
find yqr-wasm/pkg -maxdepth 1 -type f ! -name .gitignore \
  -exec cp {} "${stage}/yqr-wasm-pkg/" \;
tar -czf yqr-wasm-pkg.tar.gz -C "${stage}" yqr-wasm-pkg
shasum -a 256 yqr-wasm-pkg.tar.gz > yqr-wasm-pkg.tar.gz.sha256

echo "wasm package artifact:"
ls -l yqr-wasm-pkg.tar.gz yqr-wasm-pkg.tar.gz.sha256
