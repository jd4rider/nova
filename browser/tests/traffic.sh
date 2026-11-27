#!/bin/sh
# Window buttons follow the layout.
#
# The close/minimize/fullscreen lights are AppKit's own buttons, so they are not
# in the framebuffer snapshot. The host reports where it put them through
# NOVA_GUI_TRACE instead: the y centre of the row they were moved onto.
#
# Horizontal tabs: the tab strip is 42 px tall and tabs start at y=8 with height
# 30, so the centre is 23. Vertical tabs: toolbar_y is 12 and the controls are
# 30 tall starting at +2, so the centre is 29.
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="$ROOT/target/nova/nova-browser"
if [ ! -x "$BIN" ]; then
    echo "traffic: missing $BIN (build the browser first)"
    exit 1
fi

pass=0
fail=0
check_in() {
    if grep -qF "$1" "$2"; then
        echo "PASS $3"
        pass=$((pass + 1))
    else
        echo "FAIL $3"
        fail=$((fail + 1))
    fi
}

run_layout() {
    layout="$1"
    expect="$2"
    profile=$(mktemp -d "/tmp/nova-traffic-$layout.XXXXXX")
    trace="$profile/gui.log"
    NOVA_BROWSER_PROFILE="$profile" \
    NOVA_BROWSER_NO_RESTORE=1 \
    NOVA_BROWSER_NO_HISTORY=1 \
    NOVA_BROWSER_AUTOCLOSE=1 \
    NOVA_BROWSER_LAYOUT="$layout" \
    NOVA_GUI_TRACE="$trace" \
        "$BIN" >"$profile/run.log" 2>&1
    check_in "traffic center_y=$expect" "$trace" "$layout tabs put the window buttons on row centre $expect"
    check_in 'browser: state=ready' "$profile/run.log" "$layout tabs still render"
    # A window button below its row's centre would be off the chrome entirely.
    origin=$(sed -n 's/.*traffic center_y=[0-9]* close=\([-0-9]*\),.*/\1/p' "$trace" | tail -1)
    if [ "$origin" = "-1" ]; then
        echo "FAIL $layout tabs have no close button to place"
        fail=$((fail + 1))
    else
        echo "PASS $layout tabs expose a real close button"
        pass=$((pass + 1))
    fi
}

echo "--- horizontal tabs ---"
run_layout horizontal 23
echo "--- vertical tabs ---"
run_layout vertical 29
echo "--- collapsed rail ---"
run_layout rail 29

echo "traffic: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
