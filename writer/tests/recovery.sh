#!/bin/sh
# Crash recovery: one process writes unsaved work and exits without saving;
# a second process must find and load it.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}
DIR=/tmp/nova-writer-recovery-crash

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "writer recovery: compiler missing" >&2; exit 1; }

echo "--- writer recovery ---"
rm -rf "$DIR"
(cd "$HERE/recovery" && "$TOOL" build >/dev/null)
BIN="$HERE/recovery/target/nova/nova-writer-recovery-test"
[ -x "$BIN" ] || { echo "writer recovery: no binary" >&2; exit 1; }

export NOVA_WRITER_RECOVERY_DIR="$DIR"
"$BIN" write | tee /tmp/nova-writer-recovery-write.txt
ls "$DIR"/recovery-*.novadoc >/dev/null

# New process: the writer is gone, only the recovery file remains.
"$BIN" read | tee /tmp/nova-writer-recovery-read.txt
grep -q '^recovered=Crash Report$' /tmp/nova-writer-recovery-read.txt
grep -q '^text=Unsaved work survives a crash.$' /tmp/nova-writer-recovery-read.txt
echo "PASS recovery survives a process exit"
