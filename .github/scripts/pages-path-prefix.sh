#!/usr/bin/env bash
# Rewrite root-absolute URLs in an `accent build` output directory so the
# site works when served under a sub-path (GitHub Pages project site,
# e.g. https://zoosky.github.io/yqr/).
#
# Accent emits root-absolute internal links (href="/specs/...",
# src="/theme/assets/...") and has no sub-path setting, so the prefix is
# grafted on after the build. Handled forms, from a survey of the output:
#   - HTML href/src attributes, both literal (href="/specs") and
#     HTML-escaped (href="&#x2f;specs") variants
#   - meta-refresh redirect stubs (content="0;url=/demo")
#   - the DocFind search assets fetched by JS ("/_search/...")
#   - search-result page URLs in _search/index.json ("u":"/specs/...")
#   - markdown links in llms.txt / llms-full.txt ("](/demo)")
# Full-URL artifacts (sitemap.xml, feed.xml, robots.txt, canonicals) already
# honor --base-url and need no rewrite.
#
# Usage: pages-path-prefix.sh <output-dir> [prefix]
#   prefix defaults to /yqr and must start with "/".

set -euo pipefail

DIR="${1:?usage: pages-path-prefix.sh <output-dir> [prefix]}"
PREFIX="${2:-/yqr}"

case "$PREFIX" in
  /*) ;;
  *) echo "error: prefix must start with '/': $PREFIX" >&2; exit 1 ;;
esac

# The prefix without its leading slash, for the HTML-escaped variant.
BARE="${PREFIX#/}"
ESC="\&#x2f;${BARE}\&#x2f;"

# Every substitution guards against values that already carry the prefix
# (\Q$BARE\E in a negative lookahead), so the rewrite is idempotent --
# running the script twice produces identical output, and an accidental
# second pass can never corrupt links into /yqr/yqr/... form.
# Known limitation: a genuine content path that itself starts with the
# prefix segment (a page at /yqr/...) is indistinguishable from
# already-rewritten output and would be left unprefixed; do not create one.

find "$DIR" -name '*.html' -print0 | xargs -0 perl -pi -e "
  s{(href|src)=\"/(?!/|\\Q${BARE}\\E/)}{\$1=\"${PREFIX}/}g;
  s{(href|src)=\"&#x2f;(?!\\Q${BARE}\\E&#x2f;)}{\$1=\"${ESC}}g;
  s{content=\"0;url=/(?!/|\\Q${BARE}\\E/)}{content=\"0;url=${PREFIX}/}g;
"

# Idempotent as written: the pattern requires \"_\" right after the slash,
# which no longer holds once the prefix is in place.
find "$DIR" -name '*.js' -print0 | xargs -0 perl -pi -e "
  s{\"/(_search/)}{\"${PREFIX}/\$1}g;
"

if [ -f "$DIR/_search/index.json" ]; then
  perl -pi -e "s{\"u\":\"/(?!\\Q${BARE}\\E/)}{\"u\":\"${PREFIX}/}g" "$DIR/_search/index.json"
fi

for f in "$DIR/llms.txt" "$DIR/llms-full.txt"; do
  [ -f "$f" ] && perl -pi -e "s{\]\(/(?!/|\\Q${BARE}\\E/)}{](${PREFIX}/}g" "$f"
done

# Fail loudly if any unprefixed root-absolute href/src survived.
LEFTOVER=$(find "$DIR" -name '*.html' -print0 | BARE="$BARE" xargs -0 \
  perl -ne 'print "$ARGV: $_" if m{(href|src)="/(?!/|$ENV{BARE}/)}' | head -5)
if [ -n "$LEFTOVER" ]; then
  echo "error: unprefixed root-absolute links remain after rewrite:" >&2
  echo "$LEFTOVER" >&2
  exit 1
fi

echo "Rewrote root-absolute URLs under ${DIR} with prefix ${PREFIX}"
