#!/usr/bin/env bash
# Regenerate .github/workflows/release.yml from the dist config in
# Cargo.toml, then re-apply the one local adjustment dist 0.33 has no
# config knob for: its template hardcodes "axo bot <admin+bot@axo.dev>"
# as the committer for Homebrew tap pushes, which would attribute the
# tap's release commits to a third party. The formula commits are made
# by CI, so they carry the github-actions bot identity instead.
#
# Always regenerate through this script, never with bare `dist init`
# and never by hand-editing release.yml.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

dist init --yes

perl -0pi -e '
  s/GITHUB_USER: "axo bot"/GITHUB_USER: "github-actions[bot]"/;
  s/GITHUB_EMAIL: "admin\+bot\@axo\.dev"/GITHUB_EMAIL: "41898282+github-actions[bot]\@users.noreply.github.com"/;
' .github/workflows/release.yml

# The override must have taken on both lines; dist moving the strings in
# a future version should fail loudly here, not ship axo's identity.
grep -q 'GITHUB_USER: "github-actions\[bot\]"' .github/workflows/release.yml \
  || { echo "dist-regen.sh: committer override did not take" >&2; exit 1; }
grep -q 'GITHUB_EMAIL: "41898282' .github/workflows/release.yml \
  || { echo "dist-regen.sh: committer email override did not take" >&2; exit 1; }

echo "release.yml regenerated with the committer override applied."
