#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
DIR=$(mktemp -d /tmp/nova-permissions.XXXXXX)
PROFILE=$(mktemp -d /tmp/nova-permissions-profile.XXXXXX)
trap 'kill "$pid" 2>/dev/null; rm -rf "$DIR" "$PROFILE"' EXIT
cat > "$DIR/index.html" <<'HTML'
<!doctype html><title>permissions</title>
<p>microphone probe</p>
HTML
port=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$DIR" >/dev/null 2>&1 &
pid=$!
sleep 0.6
URL="http://127.0.0.1:$port/index.html"
pass=0; fail=0
match(){ if echo "$2" | grep -q "$3"; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }

# The page asks for the microphone; the delegate must hold the request for
# Nova, report the origin and kind, and answer deny without touching TCC.
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_PERMISSION_CHECK=1 NOVA_BROWSER_URL="$URL" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" 2>&1)
echo "$OUT" | grep 'permission:' || true
match "a microphone request reaches the permission card" "$OUT" "permission: pending=1 origin=127.0.0.1 kind=microphone remaining=0"
echo "permissions: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
