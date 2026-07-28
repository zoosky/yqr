#!/usr/bin/env bash
#
# yqr demo -- a jq-style query & transform tool for YAML.
#
# A showcase that runs real queries against the sample files sitting next to
# this script (deploy.yaml, config.yaml). Open those files to see exactly what
# each query reads.
#
# Usage:  bash yqr-demo.sh          (from anywhere -- paths resolve to this dir)
#
set -euo pipefail

# Resolve this script's own directory so the demo works from any cwd and the
# input files are read in place rather than regenerated in a temp dir.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY="$SCRIPT_DIR/deploy.yaml"
CONFIG="$SCRIPT_DIR/config.yaml"

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

section "7. jq-style exit codes -- scriptable error handling"
printf '%s# Parse errors exit 3; runtime errors exit 5 -- so you can branch in scripts:%s\n' "$dim" "$reset"
printf '%s$ echo '\''x: 1'\'' | yqr '\''.x.y'\''  %s# index a number -> runtime error\n' "$green" "$reset"
if echo 'x: 1' | yqr '.x.y'; then :; else printf '%s-> exit %s%s\n' "$dim" "$?" "$reset"; fi
echo

printf '%sThat is yqr: jq ergonomics, YAML-native, with a byte-exact fidelity mode.%s\n' "$bold" "$reset"
