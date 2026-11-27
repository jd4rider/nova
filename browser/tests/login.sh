#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
DIR=$(mktemp -d /tmp/nova-login.XXXXXX)
PROFILE=$(mktemp -d /tmp/nova-login-profile.XXXXXX)
trap 'kill "$pid" 2>/dev/null; rm -rf "$DIR" "$PROFILE"; security delete-generic-password -s dev.nova.browser.login -a 127.0.0.1 >/dev/null 2>&1 || true' EXIT
cat > "$DIR/index.html" <<'HTML'
<!doctype html><title>login</title>
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

# A normal tab: the submit is captured, stored in the login keychain, read back
# and filled into the page's own form.
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_LOGIN_CHECK=1 NOVA_BROWSER_URL="$URL" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" 2>&1)
echo "$OUT" | grep 'login:' || true
match "a submitted login is stored and filled back" "$OUT" "login: captured=1 host=127.0.0.1 user=ada saved=1 filled=1 fields=ada|hunter2"
absent "the probe leaves no credential behind" ""

# A private tab captures nothing and fills nothing, even with the entry stored
# by a normal tab in the same keychain.
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_LOGIN_CHECK=1 NOVA_BROWSER_URL="$URL" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" --private 2>&1)
echo "$OUT" | grep 'login:' || true
match "a private tab neither stores nor fills a login" "$OUT" "login: captured=0 host= user= saved=0 filled=0 fields=ada|"
echo "login: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
