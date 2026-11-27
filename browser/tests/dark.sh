#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
DIR=$(mktemp -d /tmp/nova-dark.XXXXXX)
PROFILE=$(mktemp -d /tmp/nova-dark-profile.XXXXXX)
trap 'kill "$pid" 2>/dev/null; rm -rf "$DIR" "$PROFILE"' EXIT
cat > "$DIR/index.html" <<'HTML'
<!doctype html><title>appearance</title><p>appearance fixture</p>
HTML
port=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$DIR" >/dev/null 2>&1 &
pid=$!
sleep 0.6
URL="http://127.0.0.1:$port/index.html"
pass=0; fail=0
check(){ if echo "$2" | grep -q "$3"; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }
DARK=$(env "$PENV=$PROFILE" NOVA_BROWSER_THEME=gruvbox NOVA_BROWSER_DARK_CHECK=1 NOVA_BROWSER_URL="$URL" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" 2>&1)
echo "$DARK" | grep 'dark:' || true
check "a dark theme asks the page for its dark styles" "$DARK" "dark: theme=gruvbox light=0 media=1"
LIGHT=$(env "$PENV=$PROFILE" NOVA_BROWSER_THEME=light NOVA_BROWSER_DARK_CHECK=1 NOVA_BROWSER_URL="$URL" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" 2>&1)
echo "$LIGHT" | grep 'dark:' || true
check "a light theme asks the page for its light styles" "$LIGHT" "dark: theme=light light=1 media=0"
echo "dark: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
