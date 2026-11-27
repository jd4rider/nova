#!/bin/sh
# WebKit embedding evidence for Nova Browser.
#
# 1. Build and run the headless lifecycle project (real WKWebView navigation).
# 2. Build the Nova GUI shell and run it once with a local trusted fixture and
#    a WebKit snapshot. The shell test requires a macOS GUI session; if the
#    window cannot open, report that honestly rather than passing it.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "webkit: compiler missing" >&2; exit 1; }

echo "--- lifecycle ---"
(cd "$HERE/lifecycle" && "$TOOL" build >/dev/null)
"$HERE/lifecycle/target/nova/webkit-lifecycle" | tee /tmp/nova-browser-webkit-lifecycle.txt
if grep -q '^FAIL ' /tmp/nova-browser-webkit-lifecycle.txt; then
    exit 1
fi
grep -q '^failures=0$' /tmp/nova-browser-webkit-lifecycle.txt

if [ "${NOVA_BROWSER_LIFECYCLE_ONLY:-0}" = "1" ]; then
    echo "shell smoke: skipped (NOVA_BROWSER_LIFECYCLE_ONLY=1)"
    exit 0
fi

echo "--- shell build ---"
(cd "$ROOT/browser" && "$TOOL" build >/dev/null)
[ -x "$ROOT/browser/target/nova/nova-browser" ]

echo "--- shell smoke ---"
rm -f /tmp/nova-browser-shell.png
if (cd "$ROOT" && NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_SNAPSHOT=/tmp/nova-browser-shell.png \
    "$ROOT/browser/target/nova/nova-browser" > /tmp/nova-browser-shell.txt 2>&1); then
    cat /tmp/nova-browser-shell.txt
    grep -q 'state=ready' /tmp/nova-browser-shell.txt
    if [ -s /tmp/nova-browser-shell.png ]; then
        echo "shell snapshot: /tmp/nova-browser-shell.png"
    else
        echo "shell snapshot unavailable (window/compositor limitation)"
    fi
else
    code=$?
    cat /tmp/nova-browser-shell.txt 2>/dev/null || true
    echo "shell smoke could not run in this session (exit $code); lifecycle passed" >&2
    exit 2
fi
