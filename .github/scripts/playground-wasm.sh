#!/usr/bin/env bash
# Build the wasm package the playground page runs and copy it into the
# theme's assets, where `accent build` picks it up as a static file.
#
# Produces (both gitignored):
#   yqr-wasm/pkg/                          wasm-pack's output
#   docs/themes/default/assets/wasm/       the two files the page loads
#
# The invocation matches ci.yml's wasm lane: --target web, release, and
# wasm-pack pinned to 0.15.1 there (it keys the wasm-opt it downloads to
# its own version). Locally any 0.15.x works; CI installs the pin itself.
#
# Run from anywhere; the script resolves the repo root.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
dest="${root}/docs/themes/default/assets/wasm"

wasm-pack build "${root}/yqr-wasm" --target web --release

mkdir -p "${dest}"
cp "${root}/yqr-wasm/pkg/yqr_wasm.js" \
   "${root}/yqr-wasm/pkg/yqr_wasm_bg.wasm" \
   "${dest}/"

echo "playground wasm assets:"
ls -l "${dest}"
