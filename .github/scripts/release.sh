#!/usr/bin/env bash
# The mechanical half of a yqr release (yqr-m001 section 3).
#
# Two phases, because the release PR's review and merge sit between them:
#
#   release.sh prepare X.Y.Z [--dry-run]
#     Rolls the changelog, bumps the version stamps, refreshes the
#     lockfile, runs the full local gate, and opens the release PR.
#
#   release.sh tag X.Y.Z [--dry-run]
#     After that PR merges: tags the release commit on main and pushes
#     the tag, which triggers release.yml (binaries, checksums,
#     installer, the GitHub release itself, the Homebrew formula).
#
# What this script deliberately does NOT do:
#   - Write the changelog. The [Unreleased] lead paragraph and entries
#     are editorial; prepare aborts if the section is empty.
#   - Choose the version. Pre-1.0, a breaking CLI or library change
#     bumps the minor; purely additive is a patch (yqr-m001 section 3).
#   - Re-measure the docs. Console blocks on the measured pages are
#     re-run by hand against the release build; prepare reminds you.
#   - cargo publish. Irreversible, separately authorized (yqr-m004);
#     this script never runs it.
#
# --dry-run prints every mutating command instead of executing it.
# Read-only checks always run for real.

set -euo pipefail

usage() {
  echo "usage: $0 {prepare|tag} X.Y.Z [--dry-run]" >&2
  exit 2
}

phase="${1:-}"
version="${2:-}"
dry_run="${3:-}"

case "$phase" in prepare | tag) ;; *) usage ;; esac
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || usage
case "$dry_run" in "" | --dry-run) ;; *) usage ;; esac

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# Mutating commands route through run(); --dry-run prints them instead.
run() {
  if [[ "$dry_run" == "--dry-run" ]]; then
    echo "[dry-run] $*"
  else
    "$@"
  fi
}

die() {
  echo "release.sh: $*" >&2
  exit 1
}

current_version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
[[ -n "$current_version" ]] || die "could not read version from Cargo.toml"

require_clean_main() {
  [[ -z "$(git status --porcelain)" ]] || die "working tree is not clean"
  [[ "$(git branch --show-current)" == "main" ]] || die "run from main (on $(git branch --show-current))"
  git fetch --quiet origin main
  [[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] \
    || die "main is not up to date with origin/main; git pull --rebase first"
}

# The local tag store is not the truth: a tag published from another
# machine may never have been fetched here, and tagging past it strands
# a divergent local tag when the push is rejected. Ask origin directly.
require_tag_free() {
  git rev-parse -q --verify "refs/tags/v$version" >/dev/null && die "tag v$version already exists locally"
  if git ls-remote --exit-code --tags origin "refs/tags/v$version" >/dev/null 2>&1; then
    die "tag v$version already exists on origin"
  fi
}

prepare() {
  require_clean_main

  [[ "$version" != "$current_version" ]] || die "version $version is already current"
  highest="$(printf '%s\n%s\n' "$current_version" "$version" | sort -V | tail -1)"
  [[ "$highest" == "$version" ]] || die "$version is lower than the current $current_version"
  require_tag_free

  # The [Unreleased] section must hold the release's content already:
  # a lead paragraph in the style of previous entries, and the entries
  # themselves. That is writing, not mechanics, so it happens first.
  unreleased="$(awk '/^## \[Unreleased\]/{found=1; next} /^## \[/{exit} found' CHANGELOG.md)"
  grep -q '[^[:space:]]' <<<"$unreleased" \
    || die "CHANGELOG.md [Unreleased] is empty -- write the lead paragraph and entries first"
  grep -q '^### ' <<<"$unreleased" \
    || die "CHANGELOG.md [Unreleased] has no '### ' entry sections -- is the content finished?"

  # Branch before touching any file: every later failure (a red gate,
  # gh not authenticated) then strands its edits on the release branch,
  # never on main.
  branch="chore/release-v$version"
  echo "==> creating branch $branch"
  run git checkout -b "$branch"

  today="$(date +%Y-%m-%d)"
  echo "==> rolling CHANGELOG.md: [Unreleased] -> [$version] - $today"
  run perl -0pi -e "s/^## \\[Unreleased\\]\$/## [Unreleased]\n\n## [$version] - $today/m" CHANGELOG.md

  echo "==> bumping Cargo.toml $current_version -> $version"
  run perl -0pi -e "s/^version = \"\Q$current_version\E\"\$/version = \"$version\"/m" Cargo.toml

  jsonld="docs/themes/default/templates/home.html.jinja"
  echo "==> bumping softwareVersion in $jsonld"
  run perl -0pi -e "s/\"softwareVersion\": \"\Q$current_version\E\"/\"softwareVersion\": \"$version\"/" "$jsonld"

  if [[ "$dry_run" != "--dry-run" ]]; then
    # Each stamp must have moved exactly where intended; a pattern that
    # silently matched nothing would ship a half-bumped release.
    grep -q "^version = \"$version\"\$" Cargo.toml || die "Cargo.toml bump did not take"
    grep -q "\"softwareVersion\": \"$version\"" "$jsonld" || die "JSON-LD bump did not take"
    grep -q "^## \[$version\] - $today\$" CHANGELOG.md || die "changelog roll did not take"
  fi

  echo "==> cargo check (refreshes Cargo.lock)"
  run cargo check --quiet

  echo "==> full local gate"
  run bash .github/scripts/local-ci.sh

  echo "==> commit, push, PR"
  run git add CHANGELOG.md Cargo.toml Cargo.lock "$jsonld"
  run git commit -m "chore: release v$version"
  run git push -u origin "$branch"
  run gh pr create --title "chore: release v$version" \
    --body "Cuts v$version per the release checklist (yqr-m001 section 3): changelog rolled, version stamps bumped, lockfile refreshed, local gate green. After merge: \`release.sh tag $version\`."

  # The release-process spec is the source of truth for the manual
  # steps this reminder compresses.
  cat <<EOF

prepare done. Before merging the PR, by hand:
  - Re-measure the docs pages against this build if the release changes
    behavior: re-run the console blocks, do not re-assert them, and
    push any corrections to this branch.
  - Wait for CI and review.

Then: $0 tag $version
EOF
}

tag() {
  require_clean_main

  [[ "$current_version" == "$version" ]] \
    || die "main's Cargo.toml says $current_version, not $version -- has the release PR merged?"
  require_tag_free
  grep -q "^## \[$version\] " CHANGELOG.md \
    || die "CHANGELOG.md has no [$version] section for release.yml to extract notes from"

  # The tag belongs on the release commit, not on whatever HEAD happens
  # to be: a feature PR merged after the release PR must not be swept
  # into the release. Find the commit by its subject and tag that.
  release_commit="$(git rev-list -n1 --grep="^chore: release v${version//./\\.}\b" HEAD)"
  [[ -n "$release_commit" ]] \
    || die "no 'chore: release v$version' commit found on main -- has the release PR merged?"
  if [[ "$release_commit" != "$(git rev-parse HEAD)" ]]; then
    echo "note: main has moved on since the release commit; tagging the release commit itself:"
    git log -1 --format='  %h %s' "$release_commit"
  fi

  echo "==> tagging v$version at the release commit and pushing the tag"
  run git tag -a "v$version" -m "Release v$version" "$release_commit"
  run git push origin "v$version"

  # Publishing to crates.io is deliberately absent here: it is
  # irreversible and separately authorized (see the crates.io posture
  # spec), so no script runs it.
  cat <<EOF

tag pushed. release.yml now builds the binaries, attaches archives and
checksums, creates the GitHub release from the changelog section, and
pushes the Homebrew formula. Watch it:
  gh run watch \$(gh run list --workflow=release.yml --limit 1 --json databaseId -q '.[0].databaseId')

cargo publish is a separate, explicitly authorized step that this
script never runs.
EOF
}

"$phase"
