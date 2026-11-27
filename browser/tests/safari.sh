#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
SAFARI=$(mktemp -d /tmp/nova-safari.XXXXXX)
PROFILE=$(mktemp -d /tmp/nova-safari-profile.XXXXXX)
trap 'rm -rf "$SAFARI" "$PROFILE"' EXIT
cat > "$SAFARI/Bookmarks.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Children</key><array><dict>
<key>URIDictionary</key><dict><key>title</key><string>Safari Doc</string></dict>
<key>URLString</key><string>https://safari.example/docs</string>
<key>WebBookmarkType</key><string>WebBookmarkTypeLeaf</string>
</dict></array>
<key>Title</key><string>BookmarksBar</string>
<key>WebBookmarkType</key><string>WebBookmarkTypeList</string>
</dict></plist>
PLIST
plutil -convert binary1 "$SAFARI/Bookmarks.plist"
sqlite3 "$SAFARI/History.db" "create table history_items(id integer primary key, url text); create table history_visits(id integer primary key, history_item integer, title text, visit_time real); insert into history_items values(1,'https://safari.example/history'); insert into history_visits values(1,1,'Safari History',700000000);"
OUT=$(env "$PENV=$PROFILE" "$BIN" --import "$SAFARI" 2>&1)
echo "$OUT" | grep 'import' || true
pass=0; fail=0
check(){ if echo "$OUT" | grep -q "$2"; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }
check "the Safari folder is detected" "import: files=1"
check "one bookmark imported" "bookmarks=1 history=1"
grep -q "Safari Doc" "$PROFILE/bookmarks.tsv"; check "the bookmark title survived" $?
grep -q "safari.example/history" "$PROFILE/history.tsv"; check "the visit survived" $?
echo "safari: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
