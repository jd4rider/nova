#!/bin/sh
# Tabs show the site's favicon, or a letter tile when it has none.
#
# The page is a real HTTP page on loopback, so this exercises the fetch path:
# the app requests /favicon.ico once per host, caches it in the profile under
# favicons/ and draws it in the tab. NOVA_BROWSER_NO_FAVICONS=1 turns it off.
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="$ROOT/target/nova/nova-browser"
if [ ! -x "$BIN" ]; then
    echo "favicons: missing $BIN (build the browser first)"
    exit 1
fi

pass=0
fail=0
check() {
    if [ "$1" = "1" ]; then
        echo "PASS $2"
        pass=$((pass + 1))
    else
        echo "FAIL $2"
        fail=$((fail + 1))
    fi
}

dir=$(mktemp -d "/tmp/nova-favicon-site.XXXXXX")
profile=$(mktemp -d "/tmp/nova-favicon-profile.XXXXXX")
off=$(mktemp -d "/tmp/nova-favicon-off.XXXXXX")
server=""
cleanup() {
    [ -n "$server" ] && kill "$server" 2>/dev/null
    rm -rf "$dir" "$profile" "$off"
}
trap cleanup EXIT

python3 - "$dir" <<'PY'
import base64, os, sys
png = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==")
open(os.path.join(sys.argv[1], "favicon.ico"), "wb").write(png)
open(os.path.join(sys.argv[1], "index.html"), "w").write(
    "<!doctype html><title>Favicon Fixture</title><h1>ok</h1>")
PY

port=$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$dir" >/dev/null 2>&1 &
server=$!
ready=0
i=0
while [ "$i" -lt 40 ]; do
    if curl -sf "http://127.0.0.1:$port/index.html" >/dev/null 2>&1; then
        ready=1
        break
    fi
    i=$((i + 1))
    sleep 0.1
done
check "$ready" "the loopback site answers"

NOVA_BROWSER_PROFILE="$profile" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_URL="http://127.0.0.1:$port/index.html" \
    "$BIN" >"$profile/run.log" 2>&1
status=$?
check "$([ "$status" -eq 0 ] && echo 1 || echo 0)" "the app loads the page (status=$status)"
check "$(grep -q 'browser: state=ready' "$profile/run.log" && echo 1 || echo 0)" "the page reached ready"

icon="$profile/favicons/127_0_0_1.img"
check "$([ -s "$icon" ] && echo 1 || echo 0)" "the favicon is cached in the profile"
check "$(cmp -s "$icon" "$dir/favicon.ico" && echo 1 || echo 0)" "the cached bytes are the site's icon"

NOVA_BROWSER_PROFILE="$off" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_NO_FAVICONS=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_URL="http://127.0.0.1:$port/index.html" \
    "$BIN" >"$off/run.log" 2>&1
check "$([ ! -e "$off/favicons/127_0_0_1.img" ] && echo 1 || echo 0)" "NOVA_BROWSER_NO_FAVICONS=1 fetches nothing"

echo "favicons: pass=$pass fail=$fail"
[ "$fail" -eq 0 ] || exit 1
exit 0
