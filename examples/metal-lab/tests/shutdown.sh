#!/bin/sh
# Teardown regression: an NSWindow closed under ARC was released by `close` and
# again by the strong reference, crashing in the autorelease pool. Run several
# complete open/draw/capture/close cycles and require every one to exit cleanly.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
NOVA="${NOVA_BIN:-$ROOT/examples/nova-tool/target/nova/nova-tool}"

[ -x "$NOVA" ] || {
    echo "shutdown: missing compiler: $NOVA" >&2
    exit 1
}

cd "$HERE/.."
mkdir -p target
"$NOVA" build > target/shutdown-build.log 2>&1

CYCLES=5
i=1
while [ "$i" -le "$CYCLES" ]; do
    SHOT="target/cycle-${i}.png"
    LOG="target/cycle-${i}.log"
    rm -f "$SHOT" "$LOG"
    NOVA_METAL_SHOT="$SHOT" ./target/nova/metal-lab > "$LOG" 2>&1
    grep -q "ok=true" "$LOG"
    grep -q "shutdown-complete" "$LOG"
    [ -s "$SHOT" ] || { echo "shutdown: cycle $i produced no capture" >&2; exit 1; }
    i=$((i + 1))
done

echo "metal shutdown: $CYCLES/$CYCLES cycles clean"
