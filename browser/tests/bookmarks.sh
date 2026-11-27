#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
PROFILE=$(mktemp -d /tmp/nova-bookmarks.XXXXXX)
trap 'rm -rf "$PROFILE"' EXIT
printf 'bookmark\thttps://example.com/a\tAlpha\nbookmark\thttps://example.com/b\tBeta\n' > "$PROFILE/bookmarks.tsv"
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_BOOKMARKS_OPEN=1 "$BIN" 2>&1)
echo "$OUT" | grep -E 'bookmarks:' || true
pass=0; fail=0
check(){ if echo "$OUT" | grep -q "$2"; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }
check "the panel opens with both bookmarks" "bookmarks: open=1 rows=2"
check "newest bookmark is listed first" "first=Beta"
check "the panel has a real rectangle" "panel=340x"
echo "bookmarks: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
