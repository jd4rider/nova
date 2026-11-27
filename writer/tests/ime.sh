#!/bin/sh
# IME marked-text acceptance.
#
# A real Japanese/Chinese/Korean input source cannot be driven from a script.
# The host probe calls the two NSTextInputClient entry points AppKit itself
# uses: setMarkedText:selectedRange:replacementRange: for the in-progress
# composition and insertText:replacementRange: for the commit. The app must
# paint the marked text, commit the string exactly once, and clear the
# composition when Escape arrives.
#
# Requires a macOS GUI session, like the other live window checks.
set -eu

HERE=$(cd "$(dirname "$0")/.." && pwd)
ROOT=$(cd "$HERE/.." && pwd)
TOOL=${NOVA_TOOL_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}

if [ ! -x "$TOOL" ]; then
    (cd "$ROOT/examples/nova-tool" && ../../bootstrap/nova build >/dev/null)
fi
[ -x "$TOOL" ] || { echo "writer ime: compiler missing" >&2; exit 1; }

echo "--- writer ime ---"
(cd "$HERE" && "$TOOL" build >/dev/null)
BIN="$HERE/target/nova/nova-writer"
[ -x "$BIN" ] || { echo "writer ime: no binary" >&2; exit 1; }

PROFILE=$(mktemp -d /tmp/nova-writer-ime.XXXXXX)
REC_DIR="$PROFILE/recovery"
OUT="$PROFILE/preedit.png"
LOG="$PROFILE/commit.log"
CANCEL_LOG="$PROFILE/cancel.log"

pass=0
fail=0
check() {
    if grep -q "$1" "$2"; then
        echo "PASS $3"
        pass=$((pass + 1))
    else
        echo "FAIL $3"
        fail=$((fail + 1))
    fi
}

count() {
    n=$(grep -c "$1" "$2" || true)
    if [ "$n" -eq "$3" ]; then
        echo "PASS $4"
        pass=$((pass + 1))
    else
        echo "FAIL $4 (got $n, want $3)"
        fail=$((fail + 1))
    fi
}

# The probe clock starts at the first pump, not at process launch. Wait for
# the commit instead of assuming the window opened within a fixed sleep.
wait_for() {
    pattern=$1
    file=$2
    tries=$3
    n=0
    while [ "$n" -lt "$tries" ]; do
        if grep -q "$pattern" "$file" 2>/dev/null; then
            return 0
        fi
        sleep 0.5
        n=$((n + 1))
    done
    return 1
}

NOVA_WRITER_NO_RECOVERY=1 NOVA_WRITER_RECOVERY_DIR="$REC_DIR" \
NOVA_GUI_IME_PROBE=1 NOVA_WRITER_TRACE=1 \
NOVA_WRITER_SNAPSHOT="$OUT" NOVA_WRITER_SNAPSHOT_PREEDIT=1 \
    "$BIN" >"$LOG" 2>&1 &
pid=$!
if ! wait_for 'ime: committed=日本' "$LOG" 40; then :; fi
sleep 1
kill -TERM "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true

check 'ime: marked=にほ has=1' "$LOG" 'host marked text is live'
check 'writer: preedit=にほ' "$LOG" 'Nova paints the composition'
check 'ime: committed=日本 has=0' "$LOG" 'host clears marked text on commit'
check 'writer: committed=日本' "$LOG" 'committed text reaches the editor'
check 'writer: paragraph=.*日本' "$LOG" 'commit is inserted into the document'
count 'writer: committed=日本' "$LOG" 1 'commit is delivered exactly once'

if [ -s "$OUT" ]; then
    bytes=$(wc -c <"$OUT" | tr -d ' ')
    if [ "$bytes" -gt 100000 ]; then
        echo "PASS composition snapshot rendered ($bytes bytes)"
        pass=$((pass + 1))
    else
        echo "FAIL composition snapshot too small ($bytes bytes)"
        fail=$((fail + 1))
    fi
else
    echo "FAIL composition snapshot missing"
    fail=$((fail + 1))
fi

NOVA_WRITER_NO_RECOVERY=1 NOVA_WRITER_RECOVERY_DIR="$REC_DIR" \
NOVA_GUI_IME_PROBE=cancel NOVA_WRITER_TRACE=1 \
    "$BIN" >"$CANCEL_LOG" 2>&1 &
pid=$!
if ! wait_for 'ime: committed=日本' "$CANCEL_LOG" 50; then :; fi
sleep 1
kill -TERM "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true

check 'ime: cancelled has=0' "$CANCEL_LOG" 'Escape clears the host composition'
check 'writer: escape' "$CANCEL_LOG" 'Escape reaches the editor'
check 'writer: composition cleared' "$CANCEL_LOG" 'cancelled preedit disappears'
check 'writer: committed=日本' "$CANCEL_LOG" 'a later composition still commits'
check 'writer: paragraph=.*日本' "$CANCEL_LOG" 'cancelled text is not inserted'
count 'ime: marked=にほ has=1' "$CANCEL_LOG" 2 'composition can start again after cancel'
count 'writer: committed=日本' "$CANCEL_LOG" 1 'cancel does not duplicate the commit'

echo "ime: $pass PASS, $fail FAIL"
echo "ime: logs $LOG $CANCEL_LOG"
echo "ime: snapshot $OUT"
[ "$fail" -eq 0 ]
