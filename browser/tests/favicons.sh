#!/bin/sh
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/browser/target/nova/nova-browser"
PENV=$(grep -oh 'NOVA_BROWSER_[A-Z_]*PROFILE[A-Z_]*' "$ROOT/browser/tests/history.sh" | head -1)
DIR=$(mktemp -d /tmp/nova-icons.XXXXXX)
PROFILE=$(mktemp -d /tmp/nova-icon-profile.XXXXXX)
trap 'kill "$pid" 2>/dev/null; rm -rf "$DIR" "$PROFILE" "$P2"' EXIT
python3 - "$DIR" <<'PY'
import base64, os, sys
d = sys.argv[1]
png = base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==")
open(os.path.join(d, "site-icon.png"), "wb").write(png)
open(os.path.join(d, "favicon.ico"), "wb").write(b"not the declared icon")
open(os.path.join(d, "index.html"), "w").write('<!doctype html><title>icon</title><link rel="icon" href="/site-icon.png"><p>icon</p>')
PY
port=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$DIR" >/dev/null 2>&1 &
pid=$!
sleep 0.6
OUT=$(env "$PENV=$PROFILE" NOVA_BROWSER_URL="http://127.0.0.1:$port/index.html" NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_FAVICON_TRACE=1 "$BIN" 2>&1)
CACHED="$PROFILE/favicons/127_0_0_1.img"
pass=0; fail=0
check(){ if [ "$2" = 0 ]; then echo "PASS $1"; pass=$((pass+1)); else echo "FAIL $1"; fail=$((fail+1)); fi }
[ -f "$CACHED" ]; check "the declared icon is cached" $?
cmp -s "$CACHED" "$DIR/site-icon.png"; check "the cache is the bytes the page named" $?
grep -q "site-icon.png" "$CACHED.src"; check "the source records the declared href" $?
P2=$(mktemp -d /tmp/nova-nofav.XXXXXX)
env "$PENV=$P2" NOVA_BROWSER_NO_FAVICONS=1 NOVA_BROWSER_URL="http://127.0.0.1:$port/index.html" NOVA_BROWSER_AUTOCLOSE=1 "$BIN" >/dev/null 2>&1
[ ! -e "$P2/favicons/127_0_0_1.img" ]; check "NOVA_BROWSER_NO_FAVICONS writes nothing" $?
echo "favicons: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
