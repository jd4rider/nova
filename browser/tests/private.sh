#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
DIR=$(mktemp -d /tmp/nova-private.XXXXXX)
PROFILE=$(mktemp -d /tmp/nova-private-profile.XXXXXX)
trap 'kill "$pid" 2>/dev/null; rm -rf "$DIR" "$PROFILE"' EXIT
cat > "$DIR/index.html" <<'HTML'
<!doctype html><title>private</title><p>private fixture</p>
HTML
port=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$DIR" >/dev/null 2>&1 &
pid=$!
sleep 0.6
URL="http://127.0.0.1:$port/index.html"
pass=0; fail=0
match(){ if echo "$2" | grep -q "$3"; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }
absent(){ if grep -qs "$3" "$2"; then echo "FAIL $1"; fail=$((fail+1)); else echo "PASS $1"; pass=$((pass+1)); fi }
present(){ if grep -qs "$3" "$2"; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }

# A private launch: WebKit's non-persistent store, no history row, no session.
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_PRIVATE_CHECK=1 NOVA_BROWSER_URL="$URL" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" --private 2>&1)
echo "$OUT" | grep 'private:' || true
match "the launch tab is private with WebKit's non-persistent store" "$OUT" "private: tabs=1 flag=1 persistent=0 visits=0"
absent "a private tab writes no session entry" "$PROFILE/session.tsv" "$URL"
absent "a private page writes no history" "$PROFILE/history.tsv" "$URL"

# The same probe on a normal tab must still record a visit and save the session.
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_PRIVATE_CHECK=1 NOVA_BROWSER_URL="$URL" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" 2>&1)
echo "$OUT" | grep 'private:' || true
match "a normal tab uses the persistent store and records the visit" "$OUT" "private: tabs=1 flag=0 persistent=1 visits=1"
present "a normal tab still saves the session" "$PROFILE/session.tsv" "$URL"
echo "private: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
