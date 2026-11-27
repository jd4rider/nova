#!/bin/sh
# Headless tests for the Nova Browser library, plus a real import run against
# an on-disk Chrome profile export.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "history: compiler missing" >&2; exit 1; }

echo "--- library model ---"
(cd "$HERE/library" && "$TOOL" build >/dev/null)
"$HERE/library/target/nova/nova-browser-library-test" | tee /tmp/nova-browser-history.txt
if grep -q '^FAIL ' /tmp/nova-browser-history.txt; then
    exit 1
fi
grep -q '^failures=0$' /tmp/nova-browser-history.txt

echo "--- google import (app CLI, temp profile) ---"
(cd "$ROOT/browser" && "$TOOL" build >/dev/null)
PROFILE=$(mktemp -d /tmp/nova-browser-profile.XXXXXX)
trap 'rm -rf "$PROFILE"' EXIT
OUT=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$PROFILE" \
    "$ROOT/browser/target/nova/nova-browser" --import "$HERE/fixtures/chrome-bookmarks.json" 2>&1)
echo "$OUT"
echo "$OUT" | grep -q 'import: files=1 found=2 new=2 bookmarks=2'
[ -s "$PROFILE/bookmarks.tsv" ] || { echo "history: bookmarks file missing" >&2; exit 1; }

# A second import must not duplicate anything.
OUT2=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$PROFILE" \
    "$ROOT/browser/target/nova/nova-browser" --import "$HERE/fixtures/chrome-bookmarks.json" 2>&1)
echo "$OUT2"
echo "$OUT2" | grep -q 'new=0 bookmarks=2'

# A Netscape export imports through the same entry point.
OUT3=$(cd "$ROOT" && NOVA_BROWSER_PROFILE="$PROFILE" \
    "$ROOT/browser/target/nova/nova-browser" --import "$HERE/fixtures/bookmarks-export.html" 2>&1)
echo "$OUT3"
echo "$OUT3" | grep -q 'found=1 new=1 bookmarks=3'
