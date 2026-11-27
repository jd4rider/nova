#!/bin/sh
# End-to-end download evidence for Nova Browser.
#
# Serves a real file with Content-Disposition: attachment from a loopback
# server that trickles the body and honours Range requests, lets WKWebView turn
# the response into a WKDownload, and checks that the bytes land in
# NOVA_BROWSER_DOWNLOAD_DIR. The first run pauses mid-transfer and resumes from
# WebKit's resume data; the second run into the same directory checks the
# collision name. No fake download state.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "downloads: compiler missing" >&2; exit 1; }

echo "--- build ---"
(cd "$ROOT/browser" && "$TOOL" build >/dev/null)
BIN="$ROOT/browser/target/nova/nova-browser"
[ -x "$BIN" ] || { echo "downloads: browser missing" >&2; exit 1; }

TMP=$(mktemp -d /tmp/nova-browser-download.XXXXXX)
SERVER_PID=""
cleanup() {
    [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
    rm -rf "$TMP"
}
trap cleanup EXIT INT TERM

mkdir -p "$TMP/downloads"

echo "--- server ---"
python3 - "$TMP/port" <<'PY' &
import http.server
import socketserver
import sys
import time

port_file = sys.argv[1]
payload = b"0123456789abcdef" * 3200  # 51200 bytes


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        if self.path != "/payload.bin":
            self.send_response(404)
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        start = 0
        header = self.headers.get("Range")
        if header and header.startswith("bytes="):
            start = int(header.split("=", 1)[1].split("-", 1)[0])
        body = payload[start:]
        if start:
            self.send_response(206)
            self.send_header(
                "Content-Range", "bytes %d-%d/%d" % (start, len(payload) - 1, len(payload))
            )
        else:
            self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Disposition", 'attachment; filename="nova-fixture.bin"')
        self.send_header("Accept-Ranges", "bytes")
        # A validator is what lets WebKit accept the partial file as resumable.
        self.send_header("ETag", '"nova-fixture-v1"')
        self.send_header("Last-Modified", "Wed, 26 Nov 2025 10:00:00 GMT")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        for offset in range(0, len(body), 2048):
            self.wfile.write(body[offset:offset + 2048])
            self.wfile.flush()
            time.sleep(0.02)

    def log_message(self, *args):
        pass


socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("127.0.0.1", 0), Handler) as server:
    with open(port_file, "w") as handle:
        handle.write(str(server.server_address[1]))
        handle.flush()
    server.serve_forever()
PY
SERVER_PID=$!

i=0
while [ ! -s "$TMP/port" ] && [ "$i" -lt 100 ]; do
    sleep 0.1
    i=$((i + 1))
done
[ -s "$TMP/port" ] || { echo "downloads: server did not start" >&2; exit 1; }
PORT=$(cat "$TMP/port")
URL="http://127.0.0.1:$PORT/payload.bin"

echo "--- pause / resume ---"
OUT=$(NOVA_BROWSER_PROFILE="$TMP/profile" \
    NOVA_BROWSER_DOWNLOAD_DIR="$TMP/downloads" \
    NOVA_BROWSER_DOWNLOAD_PAUSE=1 \
    "$BIN" --download "$URL" 2>&1)
echo "$OUT"
echo "$OUT" | grep -q 'download: paused at'
echo "$OUT" | grep -q 'resumable=1'
echo "$OUT" | grep -q 'download: resumed'
echo "$OUT" | grep -q 'download: state=done'
echo "$OUT" | grep -q 'bytes=51200'

FILE="$TMP/downloads/nova-fixture.bin"
[ -s "$FILE" ] || { echo "downloads: file missing" >&2; exit 1; }
SIZE=$(wc -c < "$FILE" | tr -d ' ')
[ "$SIZE" = "51200" ] || { echo "downloads: size $SIZE, expected 51200" >&2; exit 1; }
echo "downloads: resumed file size $SIZE bytes"

echo "--- collision name ---"
OUT2=$(NOVA_BROWSER_PROFILE="$TMP/profile" \
    NOVA_BROWSER_DOWNLOAD_DIR="$TMP/downloads" \
    "$BIN" --download "$URL" 2>&1)
echo "$OUT2"
echo "$OUT2" | grep -q 'download: state=done'
echo "$OUT2" | grep -q 'name=nova-fixture-1.bin'
[ -s "$TMP/downloads/nova-fixture-1.bin" ] || { echo "downloads: collision file missing" >&2; exit 1; }
echo "--- history across restart ---"
OUT3=$(NOVA_BROWSER_PROFILE="$TMP/profile" \
    NOVA_BROWSER_DOWNLOAD_DIR="$TMP/downloads" \
    NOVA_BROWSER_NO_RESTORE=1 \
    NOVA_BROWSER_DOWNLOADS_OPEN=1 \
    NOVA_BROWSER_AUTOCLOSE=1 \
    NOVA_GUI_AX_DUMP=1 \
    "$BIN" 2>&1)
echo "$OUT3" | grep '^ax:' | tail -4
echo "$OUT3" | grep -q 'label=Show nova-fixture.bin in Finder'
echo "$OUT3" | grep -q 'label=Show nova-fixture-1.bin in Finder'
ROWS=$(grep -c . "$TMP/profile/downloads.tsv")
[ "$ROWS" = "2" ] || { echo "downloads: history rows $ROWS, expected 2" >&2; exit 1; }
FIRST=$(cut -f3 "$TMP/profile/downloads.tsv" | head -1)
[ "$FIRST" = "$TMP/downloads/nova-fixture.bin" ] || { echo "downloads: history path $FIRST" >&2; exit 1; }
echo "$OUT3" | grep -q 'label=Clear finished downloads'
echo "downloads: 2 completed rows survived restart"
echo "--- clear finished ---"
OUT4=$(NOVA_BROWSER_PROFILE="$TMP/profile" \
    NOVA_BROWSER_NO_RESTORE=1 \
    NOVA_BROWSER_CLEAR_DOWNLOADS=1 \
    NOVA_BROWSER_AUTOCLOSE=1 \
    NOVA_GUI_AX_DUMP=1 \
    "$BIN" 2>&1)
echo "$OUT4" | grep '^ax:' | tail -3
if [ -s "$TMP/profile/downloads.tsv" ]; then
    echo "downloads: clear left $(wc -c < "$TMP/profile/downloads.tsv") bytes" >&2
    exit 1
fi
echo "downloads: clear emptied the profile record"
echo "downloads: PASS"
