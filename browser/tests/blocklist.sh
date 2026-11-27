#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
DIR=$(mktemp -d /tmp/nova-block.XXXXXX)
PROFILE=$(mktemp -d /tmp/nova-block-profile.XXXXXX)
RULES="$DIR/rules.json"
trap 'kill "$pid" 2>/dev/null; rm -rf "$DIR" "$PROFILE"' EXIT
cat > "$RULES" <<'JSON'
[{"trigger":{"url-filter":"tracker.js"},"action":{"type":"block"}}]
JSON
cat > "$DIR/tracker.js" <<'JS'
window.__novaTracker = 1;
JS
cat > "$DIR/index.html" <<'HTML'
<!doctype html><title>block</title><script src="/tracker.js"></script><p>page</p>
HTML
port=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$DIR" >/dev/null 2>&1 &
pid=$!
sleep 0.6
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_BLOCKLIST="$RULES" NOVA_BROWSER_BLOCK_CHECK=1 NOVA_BROWSER_URL="http://127.0.0.1:$port/index.html" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" 2>&1)
echo "$OUT" | grep 'block:' || true
pass=0; fail=0
check(){ if echo "$OUT" | grep -q "$2"; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }
check "WebKit compiled the rule list" "block: compiled=1"
check "the tracker script never ran" "tracker=undefined"
echo "blocklist: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
