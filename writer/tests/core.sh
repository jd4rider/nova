#!/bin/sh
# Headless Writer core acceptance: text, document, undo, format and layout.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "writer core: compiler missing" >&2; exit 1; }

echo "--- writer core ---"
(cd "$HERE/core" && "$TOOL" build >/dev/null)
REC_DIR=/tmp/nova-writer-core-recovery
rm -rf "$REC_DIR"
NOVA_WRITER_RECOVERY_DIR="$REC_DIR" "$HERE/core/target/nova/nova-writer-core-test" | tee /tmp/nova-writer-core.txt
if grep -q '^FAIL ' /tmp/nova-writer-core.txt; then
    exit 1
fi
grep -q '^failures=0$' /tmp/nova-writer-core.txt
