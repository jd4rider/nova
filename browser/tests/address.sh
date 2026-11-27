#!/bin/sh
# Headless tests for Nova Browser address normalization and editing.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "address: compiler missing" >&2; exit 1; }

echo "--- address model ---"
(cd "$HERE/address" && "$TOOL" build >/dev/null)
"$HERE/address/target/nova/nova-browser-address-test" | tee /tmp/nova-browser-address.txt
if grep -q '^FAIL ' /tmp/nova-browser-address.txt; then
    exit 1
fi
grep -q '^failures=0$' /tmp/nova-browser-address.txt
