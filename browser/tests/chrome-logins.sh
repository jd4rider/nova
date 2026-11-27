#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
DIR=$(mktemp -d /tmp/nova-chrome-login.XXXXXX)
CHROME=$(mktemp -d /tmp/nova-chrome-profile.XXXXXX)
IMPORT_PROFILE=$(mktemp -d /tmp/nova-chrome-import.XXXXXX)
RUN_PROFILE=$(mktemp -d /tmp/nova-chrome-run.XXXXXX)
trap 'kill "$pid" 2>/dev/null; rm -rf "$DIR" "$CHROME" "$IMPORT_PROFILE" "$RUN_PROFILE"; security delete-generic-password -s dev.nova.browser.login -a 127.0.0.1 >/dev/null 2>&1 || true' EXIT
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

# A Chrome-shaped profile: the Bookmarks file is what makes it a profile, and
# Login Data holds one v10 row encrypted the way Chrome encrypts on macOS.
echo '{"roots":{}}' > "$CHROME/Bookmarks"
keyhex=$(python3 -c "import hashlib;print(hashlib.pbkdf2_hmac('sha1',b'testkey',b'saltysalt',1003,16).hex())")
ivhex=$(python3 -c "print('20'*16)")
cipher=$(printf 'hunter2' | openssl enc -aes-128-cbc -K "$keyhex" -iv "$ivhex" | xxd -p | tr -d '\n')
python3 - "$CHROME/Login Data" "763130$cipher" <<'PY'
import sqlite3, sys
db = sqlite3.connect(sys.argv[1])
db.execute("create table logins (origin_url text, username_value text, password_value blob)")
db.execute("insert into logins values ('http://127.0.0.1/index.html', 'ada', X'%s')" % sys.argv[2])
db.commit()
PY

pass=0; fail=0
match(){ if echo "$2" | grep -q "$3"; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }

OUT=$(env "$PENV=$IMPORT_PROFILE" NOVA_BROWSER_CHROME_KEY=testkey "$BIN" --import "$CHROME" 2>&1)
echo "$OUT" | grep 'import' || true
match "a Chrome login is decrypted and stored" "$OUT" "logins=1"
match "the profile's bookmarks still import" "$OUT" "bookmarks=0"

# The imported credential is what a later visit fills.
OUT2=$(env "$PENV=$RUN_PROFILE" NOVA_BROWSER_AUTOFILL_CHECK=1 NOVA_BROWSER_URL="$URL" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" 2>&1)
echo "$OUT2" | grep 'autofill' || true
match "the imported password fills the site's form" "$OUT2" "autofill: host=127.0.0.1 filled=1 fields=ada|hunter2"

# A second import must not duplicate or overwrite the entry.
OUT3=$(env "$PENV=$IMPORT_PROFILE" NOVA_BROWSER_CHROME_KEY=testkey "$BIN" --import "$CHROME" 2>&1)
echo "$OUT3" | grep 'import' || true
match "a second import adds no login" "$OUT3" "logins=0"

# Without the Chrome key nothing is read, and the report says so instead of
# writing blank passwords.
OUT4=$(env "$PENV=$IMPORT_PROFILE" NOVA_BROWSER_NO_CHROME_KEY=1 "$BIN" --import "$CHROME" 2>&1)
echo "$OUT4" | grep 'import' || true
match "a locked Chrome key imports nothing" "$OUT4" "chrome key locked"
match "a locked import reports zero logins" "$OUT4" "logins=0"
echo "chrome-logins: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
