#!/bin/sh
# Blob downloads: chatgpt.com's "Export data" and most modern "Download"
# buttons build a Blob, create an <a download>, and click it.
#
# WKWebView cannot turn a blob: URL into a WKDownload, so a page-side bridge
# reads the blob, hands the bytes to the host, and the host writes the file and
# lists it as a finished download. This drives that path with the page's own JS
# and checks the bytes on disk.
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="$ROOT/target/nova/nova-browser"
if [ ! -x "$BIN" ]; then
    echo "blob: missing $BIN (build the browser first)"
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

tmp=$(mktemp -d "/tmp/nova-blob.XXXXXX")
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT

NOVA_BROWSER_PROFILE="$tmp/profile" \
NOVA_BROWSER_DOWNLOAD_DIR="$tmp/downloads" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_NO_FAVICONS=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_BLOB_CHECK=1 \
    "$BIN" >"$tmp/run.log" 2>&1
status=$?
check "$([ "$status" -eq 0 ] && echo 1 || echo 0)" "app exits cleanly (status=$status)"

line=$(grep -h '^blob:' "$tmp/run.log" | tail -1)
check "$([ -n "$line" ] && echo 1 || echo 0)" "the blob check ran"
check "$(printf '%s' "$line" | grep -q 'count=1' && echo 1 || echo 0)" "one download was registered"
check "$(printf '%s' "$line" | grep -q 'state=2' && echo 1 || echo 0)" "the download is finished"
check "$(printf '%s' "$line" | grep -q 'name=nova-blob.txt' && echo 1 || echo 0)" "the suggested file name is kept"

file="$tmp/downloads/nova-blob.txt"
check "$([ -f "$file" ] && echo 1 || echo 0)" "the file exists in the download directory"
check "$([ "$(cat "$file" 2>/dev/null)" = "nova blob" ] && echo 1 || echo 0)" "the file has the blob's bytes"

echo "blob: pass=$pass fail=$fail"
[ "$fail" -eq 0 ] || exit 1
exit 0
