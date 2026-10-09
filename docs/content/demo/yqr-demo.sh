#!/usr/bin/env bash
#
# yqr demo -- a YAML editor with a fidelity guarantee, jq-style.
#
# A showcase that runs real queries and real edits against the sample files
# sitting next to this script (deploy.yaml, config.yaml, services.yaml).
# Open those files to see exactly what each command reads. Every mutating
# command works on a copy in a scratch directory, so re-running the demo is
# idempotent and leaves nothing behind.
#
# Usage:  bash yqr-demo.sh          (from anywhere -- paths resolve to this dir)
#
set -euo pipefail

# Resolve this script's own directory so the demo works from any cwd and the
# input files are read in place rather than regenerated in a temp dir.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY="$SCRIPT_DIR/deploy.yaml"
CONFIG="$SCRIPT_DIR/config.yaml"
SERVICES="$SCRIPT_DIR/services.yaml"

# --- pretty-printing helpers -------------------------------------------------
bold=$(printf '\033[1m'); dim=$(printf '\033[2m'); cyan=$(printf '\033[36m')
green=$(printf '\033[32m'); reset=$(printf '\033[0m')

section() { printf '\n%s>> %s%s\n' "$bold$cyan" "$1" "$reset"; }
# run <description> <filter> [extra yqr args...]
run() {
  local desc=$1; shift
  printf '%s# %s%s\n' "$dim" "$desc" "$reset"
  printf '%s$ yqr %s%s\n' "$green" "$*" "$reset"
  yqr "$@"
  echo
}

command -v yqr >/dev/null || { echo "yqr not found on PATH"; exit 1; }
printf '%s' "$bold"; yqr --version | head -1; printf '%s\n' "$reset"

# All mutating sections work in here, on copies of the samples.
WORKDIR="$(mktemp -d -t yqr-demo.XXXXXX)"
trap 'rm -rf "$WORKDIR"' EXIT

# =============================================================================
section "1. Navigate nested structure -- dotted paths & array indexing"
run "Deployment kind"                 '.kind'                     "$DEPLOY"
run "First container's image"         '.spec.containers[0].image' "$DEPLOY"
run "Last label (negative index)"     '.metadata.labels[-1]'      "$DEPLOY"

section "2. Iterate collections -- '[]' streams every element"
run "Every container name"            '.spec.containers[].name'   "$DEPLOY"
run "Every container image"           '.spec.containers[].image'  "$DEPLOY"

section "3. Compose with pipes -- '|' feeds one filter into the next"
run "First container, then its ports" '.spec.containers[0] | .ports[]' "$DEPLOY"

section "4. Raw output -- '-r' drops YAML quoting for shell scripting"
run "Quoted (default)"                '.spec.containers[0].image'      "$DEPLOY"
run "Raw string"                      '.spec.containers[0].image' -r   "$DEPLOY"

section "5. Reads from stdin too -- pipe YAML straight in"
printf '%s# %s%s\n' "$dim" "echo 'a: {b: [10, 20, 30]}' | yqr '.a.b[1]'" "$reset"
echo 'a: {b: [10, 20, 30]}' | yqr '.a.b[1]'
echo

section "6. Fidelity by default -- 'yqr .' keeps bytes & comments exactly"
printf '%s# By default, comments & formatting survive byte-for-byte:%s\n' "$dim" "$reset"
printf '%s$ yqr %s%s\n' "$green" "'.' config.yaml" "$reset"
yqr '.' "$CONFIG"; echo
printf '%s# Proof -- identity read is byte-identical to the source file:%s\n' "$dim" "$reset"
printf '%s$ yqr %s | diff - config.yaml%s\n' "$green" "'.' config.yaml" "$reset"
if yqr '.' "$CONFIG" | diff - "$CONFIG"; then
  printf '%sIDENTICAL -- zero bytes changed.%s\n\n' "$green$bold" "$reset"
fi
printf '%s# Opt into the classic pipeline with --normalize (drops comments, re-serializes):%s\n' "$dim" "$reset"
printf '%s$ yqr %s%s\n' "$green" "--normalize '.' config.yaml" "$reset"
yqr --normalize '.' "$CONFIG"; echo

section "7. Edit without reformatting -- the diff is the line you changed"
cp "$CONFIG" "$WORKDIR/config.yaml"
printf '%s# A write replaces the value and touches nothing else -- the\n# comment two spaces to its right stays exactly where it was:%s\n' "$dim" "$reset"
printf '%s$ yqr -i %s config.yaml && diff config.yaml.orig config.yaml%s\n' "$green" "'.replicas = 5'" "$reset"
cp "$WORKDIR/config.yaml" "$WORKDIR/config.yaml.orig"
yqr -i '.replicas = 5' "$WORKDIR/config.yaml"
diff "$WORKDIR/config.yaml.orig" "$WORKDIR/config.yaml" || true
echo
printf '%s# Append to a sequence; delete an entry structurally. Every\n# surviving byte is identical:%s\n' "$dim" "$reset"
printf '%s$ yqr %s config.yaml%s\n' "$green" "'.ports += 9090'" "$reset"
yqr '.ports += 9090' "$WORKDIR/config.yaml"; echo
printf '%s$ yqr %s config.yaml%s\n' "$green" "'del(.ports)'" "$reset"
yqr 'del(.ports)' "$WORKDIR/config.yaml"; echo

section "8. Anchors, aliases, merge keys -- editable, both ways"
printf '%s# services.yaml shares a defaults block via &defaults / <<: / *defaults:%s\n' "$dim" "$reset"
printf '%s$ yqr %s%s\n' "$green" "'.' services.yaml" "$reset"
yqr '.' "$SERVICES"; echo
printf '%s# A key inherited through <<: takes an explicit override entry --\n# only this mapping changes, the defaults block is untouched:%s\n' "$dim" "$reset"
printf '%s$ yqr %s services.yaml%s\n' "$green" "'.web.retries = 5'" "$reset"
yqr '.web.retries = 5' "$SERVICES"; echo
printf '%s# An entry whose value IS an alias edits through its own *name token --\n# assigning replaces the reference, del removes the entry:%s\n' "$dim" "$reset"
printf '%s$ yqr %s services.yaml%s\n' "$green" "'.alert = 9'" "$reset"
yqr '.alert = 9' "$SERVICES"; echo
printf '%s$ yqr %s services.yaml%s\n' "$green" "'del(.backup)'" "$reset"
yqr 'del(.backup)' "$SERVICES"; echo
printf '%s# Writing the value the alias already resolves to is a no-op that\n# keeps your *t spelling -- a save that changes nothing re-spells nothing:%s\n' "$dim" "$reset"
printf '%s$ yqr %s services.yaml | grep alert%s\n' "$green" "'.alert = 5'" "$reset"
yqr '.alert = 5' "$SERVICES" | grep alert
echo

section "9. Comments are addressable -- read them, write them"
run "Read the comment on a value"  'line_comment(.replicas)' -r "$CONFIG"
run "Write one"                    'line_comment(.name) = "renamed 2026-10"' "$CONFIG"

section "10. jq-style exit codes -- scriptable error handling"
printf '%s# Parse errors exit 3; runtime errors exit 5 -- so you can branch in scripts:%s\n' "$dim" "$reset"
printf '%s$ echo '\''x: 1'\'' | yqr '\''.x.y'\''  %s# index a number -> runtime error\n' "$green" "$reset"
if echo 'x: 1' | yqr '.x.y'; then :; else printf '%s-> exit %s%s\n' "$dim" "$?" "$reset"; fi
echo

section "11. Validate after editing -- a verdict humans and agents can act on"
# Run from the scratch directory so the paths in the diagnostics are
# identical to the commands printed above them.
cp "$CONFIG" "$WORKDIR/config.yaml"
cd "$WORKDIR"
printf '%s# Edit in place, then ask whether the file is still correct YAML:%s\n' "$dim" "$reset"
printf '%s$ yqr -i %s config.yaml%s\n' "$green" "'.replicas = 5'" "$reset"
yqr -i '.replicas = 5' config.yaml
printf '%s$ yqr validate --strict config.yaml%s\n' "$green" "$reset"
if yqr validate --strict config.yaml; then
  printf '%sVALID -- exit 0, silent. A pass also proves the file still\n' "$green$bold"
  printf 'round-trips byte-for-byte, not merely that it parses.%s\n\n' "$reset"
fi
printf '%s# Now break it the way a careless edit does:%s\n' "$dim" "$reset"
printf 'ports: [8080,\n' >> config.yaml
printf '%s$ yqr validate config.yaml%s\n' "$green" "$reset"
if yqr validate config.yaml; then :; else printf '%s-> exit %s -- coded, located, with the offending line.%s\n' "$dim" "$?" "$reset"; fi
echo
printf '%s# --strict also catches a duplicate key, which ordinary reads accept\n' "$dim"
printf '# silently (last one wins -- a bad edit quietly drops data):%s\n' "$reset"
cp "$CONFIG" config.yaml; printf 'replicas: 9\n' >> config.yaml
printf '%s$ yqr validate --strict config.yaml%s\n' "$green" "$reset"
if yqr validate --strict config.yaml; then :; else printf '%s-> exit %s%s\n' "$dim" "$?" "$reset"; fi
echo
printf '%s# And --schema holds the values to a JSON Schema, with the finding\n# located in YOUR file, not in an abstract instance path:%s\n' "$dim" "$reset"
cp "$CONFIG" config.yaml
printf '{"type":"object","properties":{"replicas":{"type":"integer","minimum":5}}}\n' > schema.json
printf '%s$ yqr validate --schema schema.json config.yaml%s\n' "$green" "$reset"
if yqr validate --schema schema.json config.yaml; then :; else printf '%s-> exit %s%s\n' "$dim" "$?" "$reset"; fi
echo

printf '%sThat is yqr: jq ergonomics, YAML-native, fidelity by default -- the\ndiff is the line you changed -- and a validate pass that tells you when\nan edit went wrong.%s\n' "$bold" "$reset"
