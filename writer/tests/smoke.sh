#!/bin/sh
# Launch the real Writer window, render the demo document, save a PNG and exit.
set -eu

HERE=$(cd "$(dirname "$0")/.." && pwd)
ROOT=$(cd "$HERE/.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}
OUT=${1:-/tmp/nova-writer-smoke.png}

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "writer smoke: compiler missing" >&2; exit 1; }

echo "--- writer smoke ---"
(cd "$HERE" && "$TOOL" build >/dev/null)
BIN="$HERE/target/nova/nova-writer"
[ -x "$BIN" ] || { echo "writer smoke: no binary" >&2; exit 1; }

rm -f "$OUT"
REC_DIR=/tmp/nova-writer-smoke-recovery
rm -rf "$REC_DIR"
NOVA_WRITER_NO_RECOVERY=1 NOVA_WRITER_RECOVERY_DIR="$REC_DIR" \
NOVA_WRITER_AUTOCLOSE=1 NOVA_WRITER_SNAPSHOT="$OUT" "$BIN"
[ -s "$OUT" ] || { echo "writer smoke: no snapshot at $OUT" >&2; exit 1; }
echo "PASS writer window rendered $OUT"
