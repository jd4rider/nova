#!/bin/sh
# IME pipeline test.
#
# A real Japanese/Chinese/Korean input source cannot be driven from a script,
# so this drives the two NSTextInputClient entry points AppKit itself uses:
# setMarkedText:selectedRange:replacementRange: (the composition shown under the
# caret) and insertText:replacementRange: (the commit). The host does that when
# NOVA_BROWSER_IME_PROBE is set; Nova must see the marked text (preedit) and then
# apply the committed string to the focused address field.
#
# Requires a macOS GUI session, like the other live window checks.
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="$ROOT/target/nova/nova-browser"
if [ ! -x "$BIN" ]; then
    echo "ime: missing $BIN (build the browser first)"
    exit 1
fi

PROFILE=$(mktemp -d /tmp/nova-browser-ime.XXXXXX)
LOG="$PROFILE/run.log"

NOVA_BROWSER_PROFILE="$PROFILE" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_FOCUS_ADDRESS=1 \
NOVA_BROWSER_IME_PROBE=1 \
NOVA_BROWSER_TRACE=1 \
    "$BIN" >"$LOG" 2>&1 &
pid=$!

# The probe marks text at 0.4s and commits at 1.2s after the first pump.
sleep 4
kill -TERM "$pid" 2>/dev/null
wait "$pid" 2>/dev/null

pass=0
fail=0
check() {
    if grep -q "$1" "$LOG"; then
        echo "PASS $2"
        pass=$((pass + 1))
    else
        echo "FAIL $2"
        fail=$((fail + 1))
    fi
}

check 'ime: marked=にほ has=1' 'host marked text is live'
check 'browser: preedit=にほ' 'Nova receives the composition'
check 'ime: committed=日本 has=0' 'host clears marked text on commit'
check 'browser: address text=.*日本' 'committed text reaches the address field'

echo "ime: $pass PASS, $fail FAIL"
echo "ime: log $LOG"
[ "$fail" -eq 0 ]
