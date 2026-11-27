#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
DIR=$(mktemp -d /tmp/nova-passwords.XXXXXX)
PROFILE=$(mktemp -d /tmp/nova-passwords-profile.XXXXXX)
trap 'kill "$pid" 2>/dev/null; rm -rf "$DIR" "$PROFILE"; security delete-generic-password -s dev.nova.browser.login -a 127.0.0.1 >/dev/null 2>&1 || true' EXIT
cat > "$DIR/index.html" <<'HTML'
<!doctype html><title>passwords</title>
<form onsubmit="return false">
  <input id="u" type="text" autocomplete="username">
  <input id="p" type="password">
  <button type="submit">Sign in</button>
</form>
HTML
port=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$DIR" >/dev/null 2>&1 &
pid=$!
sleep 0.6
URL="http://127.0.0.1:$port/index.html"
pass=0; fail=0
match(){ if echo "$2" | grep -q "$3"; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }
absent(){ if security find-generic-password -s dev.nova.browser.login -a 127.0.0.1 >/dev/null 2>&1; then echo "FAIL $1"; fail=$((fail+1)); else echo "PASS $1"; pass=$((pass+1)); fi }

# One capture feeds the panel's list, which must report the host and user the
# keychain holds; removing it from the panel path deletes the item again.
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_PASSWORDS_CHECK=1 NOVA_BROWSER_URL="$URL" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" 2>&1)
echo "$OUT" | grep 'passwords:' || true
match "the panel lists a saved login by host and user" "$OUT" "passwords: seeded=1 rows=1 host=127.0.0.1 user=ada panel=380x100 removed=1 remaining=0"
absent "removing the row deletes the keychain item" ""
echo "passwords: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
