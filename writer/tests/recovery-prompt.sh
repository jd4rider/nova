#!/bin/sh
# Recovery prompt: seed real snapshots, then drive Writer's native NSAlert.
# Covers Recover, Keep for Now, Discard and the explicit Discard command.
set -eu

HERE=$(cd "$(dirname "$0")/.." && pwd)
ROOT=$(cd "$HERE/.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}
OUT=/tmp/nova-writer-recovery-prompt
BIN="$HERE/target/nova/nova-writer"
REC_BIN="$HERE/tests/recovery/target/nova/nova-writer-recovery-test"

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "writer recovery-prompt: compiler missing" >&2; exit 1; }

echo "--- writer recovery prompt ---"
(cd "$HERE" && "$TOOL" build >/dev/null)
(cd "$HERE/tests/recovery" && "$TOOL" build >/dev/null)
[ -x "$BIN" ] || { echo "writer recovery-prompt: no writer binary" >&2; exit 1; }
[ -x "$REC_BIN" ] || { echo "writer recovery-prompt: no recovery binary" >&2; exit 1; }

export NOVA_WRITER_RECOVERY_DIR="$OUT/recovery"
export NOVA_WRITER_AUTOCLOSE=1
export NOVA_WRITER_TRACE=1
export NOVA_GUI_TRACE="$OUT/gui.log"

seed() {
    rm -rf "$OUT/recovery"
    rm -f "$OUT/recovered.png"
    mkdir -p "$OUT/recovery"
    "$REC_BIN" write >/dev/null
    ls "$OUT/recovery"/recovery-*.novadoc >/dev/null
}

run_writer() {
    label=$1
    shift
    env "$@" "$BIN" > "$OUT/$label.log" 2>&1 &
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

# Recover: the snapshot becomes a dirty tab and stays on disk until save.
seed
run_writer recover NOVA_GUI_ALERT_PROBE=primary NOVA_WRITER_SNAPSHOT="$OUT/recovered.png"
grep -q 'writer: recovery choice=recover' "$OUT/recover.log" || {
    echo "FAIL recover choice was not observed" >&2; exit 1;
}
grep -q 'writer: recovered=1' "$OUT/recover.log" || {
    echo "FAIL recover did not restore one document" >&2; exit 1;
}
[ -s "$OUT/recovered.png" ] || { echo "FAIL recovered app did not render" >&2; exit 1; }
ls "$OUT/recovery"/recovery-*.novadoc >/dev/null
echo "PASS recovery prompt restored a snapshot and kept the file"

# Keep for Now: no restore, but the snapshot remains available.
seed
run_writer keep NOVA_GUI_ALERT_PROBE=secondary
grep -q 'writer: recovery choice=keep' "$OUT/keep.log" || {
    echo "FAIL keep choice was not observed" >&2; exit 1;
}
ls "$OUT/recovery"/recovery-*.novadoc >/dev/null
echo "PASS keep for now left the snapshot on disk"

# Discard: the third button removes the snapshot and restores nothing.
seed
run_writer discard NOVA_GUI_ALERT_PROBE=tertiary
grep -q 'writer: recovery choice=discard' "$OUT/discard.log" || {
    echo "FAIL discard choice was not observed" >&2; exit 1;
}
grep -q 'writer: discarded=1' "$OUT/discard.log" || {
    echo "FAIL discard did not remove one snapshot" >&2; exit 1;
}
if ls "$OUT/recovery"/recovery-*.novadoc >/dev/null 2>&1; then
    echo "FAIL discard left a snapshot behind" >&2; exit 1;
fi
echo "PASS discard removed the snapshot"

# Dismiss, then use the explicit Discard command and its confirmation.
seed
run_writer command NOVA_GUI_ALERT_PROBE=dismiss,primary NOVA_WRITER_COMMAND_PROBE=discard_recovery
grep -q 'writer: recovery choice=keep' "$OUT/command.log" || {
    echo "FAIL dismissed prompt did not keep the snapshot" >&2; exit 1;
}
grep -q 'writer: discard files=1' "$OUT/command.log" || {
    echo "FAIL discard command did not see the pending snapshot" >&2; exit 1;
}
grep -q 'writer: discarded=1' "$OUT/command.log" || {
    echo "FAIL discard command did not remove the snapshot" >&2; exit 1;
}
if ls "$OUT/recovery"/recovery-*.novadoc >/dev/null 2>&1; then
    echo "FAIL discard command left a snapshot behind" >&2; exit 1;
fi
echo "PASS discard command removed a pending snapshot after confirmation"
