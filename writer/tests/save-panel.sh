#!/bin/sh
# Native Save As: drive the real NSSavePanel through Writer's Save As path.
# The host probe auto-confirms or cancels after the panel opens, so this checks
# the bridge, the chosen path and the save, not a bypassed file dialog.
set -eu

HERE=$(cd "$(dirname "$0")/.." && pwd)
ROOT=$(cd "$HERE/.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}
OUT=/tmp/nova-writer-save-panel
PROBE="$OUT/panel-save"
TARGET="$OUT/panel-save.novadoc"
TRACE="$OUT/trace.log"

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "writer save-panel: compiler missing" >&2; exit 1; }

echo "--- writer save panel ---"
(cd "$HERE" && "$TOOL" build >/dev/null)
BIN="$HERE/target/nova/nova-writer"
[ -x "$BIN" ] || { echo "writer save-panel: no binary" >&2; exit 1; }

rm -rf "$OUT"
mkdir -p "$OUT/recovery"
export NOVA_WRITER_NO_RECOVERY=1
export NOVA_WRITER_AUTOCLOSE=1
export NOVA_WRITER_RECOVERY_DIR="$OUT/recovery"
export NOVA_GUI_TRACE="$TRACE"

run_writer() {
    label=$1
    "$BIN" > "$OUT/$label.log" 2>&1 &
    pid=$!
    n=0
    while kill -0 "$pid" 2>/dev/null && [ "$n" -lt 120 ]; do
        sleep 0.1
        n=$((n + 1))
    done
    if kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
        echo "FAIL $label timed out" >&2
        return 1
    fi
    wait "$pid"
}

NOVA_WRITER_SAVE_PANEL_PROBE="$TARGET" NOVA_GUI_SAVE_PANEL_PROBE="$PROBE" \
    run_writer save
[ -s "$TARGET" ] || { echo "FAIL save panel did not write $TARGET" >&2; exit 1; }
grep -q '^NOVADOC	4$' "$TARGET" || { echo "FAIL saved file has no v4 header" >&2; exit 1; }
grep -q 'Nova Writer' "$TARGET" || { echo "FAIL saved file lost the document" >&2; exit 1; }
grep -q 'save panel' "$TRACE" || { echo "FAIL save panel did not run" >&2; exit 1; }
echo "PASS save panel wrote $TARGET"

rm -f "$TARGET"
NOVA_WRITER_SAVE_PANEL_PROBE=cancel NOVA_GUI_SAVE_PANEL_PROBE=cancel \
    run_writer cancel
[ ! -e "$TARGET" ] || { echo "FAIL cancelled save wrote $TARGET" >&2; exit 1; }
grep -q 'cancelled=1' "$TRACE" || { echo "FAIL save panel cancel was not observed" >&2; exit 1; }
echo "PASS save panel cancel left no file"
