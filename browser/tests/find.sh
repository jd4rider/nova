#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
DIR=$(mktemp -d /tmp/nova-find.XXXXXX)
PROFILE=$(mktemp -d /tmp/nova-find-profile.XXXXXX)
trap 'kill "$pid" 2>/dev/null; rm -rf "$DIR" "$PROFILE"' EXIT
cat > "$DIR/index.html" <<'HTML'
<!doctype html><title>find</title><p>Nova find fixture: alpha beta gamma alpha.</p>
HTML
port=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$DIR" >/dev/null 2>&1 &
pid=$!
sleep 0.6
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_FIND_CHECK=1 NOVA_BROWSER_URL="http://127.0.0.1:$port/index.html" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" 2>&1)
echo "$OUT" | grep 'find:' || true
pass=0; fail=0
check(){ if echo "$OUT" | grep -q "$2"; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }
check "zoom applies through WebKit's pageZoom" "zoom=150"
check "zoom resets to 100" "reset=100"
check "a real page word is found" "hit=1"
check "a missing word reports no match" "miss=2"
check "closing the bar clears the find state" "cleared=0"
check "the find bar has a real rectangle" "panel=320x34"
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_FIND_OPEN=1 NOVA_BROWSER_AUTOCLOSE=1 "$BIN" 2>&1)
check "the bar opens with a seeded query" "find: open=1 panel=320x34"
echo "find: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
