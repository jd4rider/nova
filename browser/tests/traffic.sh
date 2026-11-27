#!/bin/sh
# Window buttons and the window drag handle follow the layout.
#
# The close/minimize/fullscreen lights are AppKit's own buttons, so they are not
# in the framebuffer snapshot. The host reports where it put them through
# NOVA_GUI_TRACE: the requested row centre, the centre it could actually use,
# and the close button's frame inside AppKit's titlebar view.
#
# Horizontal tabs: the tab strip is 42 px tall and tabs start at y=8 with height
# 30, so the centre is 23. Vertical tabs: toolbar_y is 12 and the controls are
# 30 tall starting at +2, so the requested centre is 29 — but the titlebar view
# is only 32 pt tall, so the row is pulled up to 25 and the buttons stay whole.
#
# NOVA_BROWSER_DRAG_CHECK reports which chrome points start a window drag:
# strip/toolbar/header/sidebar background do, tabs and toolbar controls do not.
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
    request="$2"
    placed="$3"
    header="$4"
    empty="$5"
    profile=$(mktemp -d "/tmp/nova-traffic-$layout.XXXXXX")
    trace="$profile/gui.log"
    NOVA_BROWSER_PROFILE="$profile" \
    NOVA_BROWSER_NO_RESTORE=1 \
    NOVA_BROWSER_NO_HISTORY=1 \
    NOVA_BROWSER_AUTOCLOSE=1 \
    NOVA_BROWSER_LAYOUT="$layout" \
    NOVA_BROWSER_DRAG_CHECK=1 \
    NOVA_GUI_TRACE="$trace" \
        "$BIN" >"$profile/run.log" 2>&1

    check_in "traffic center_y=$request placed=$placed" "$trace" "$layout tabs put the window buttons on row centre $placed"
    check_in 'browser: state=ready' "$profile/run.log" "$layout tabs still render"

    line=$(grep -h "traffic center_y=" "$trace" | tail -1)
    origin_x=$(printf '%s\n' "$line" | sed -n 's/.* close=\(-*[0-9]*\),.*/\1/p')
    origin_y=$(printf '%s\n' "$line" | sed -n 's/.* close=[-0-9]*,\(-*[0-9]*\).*/\1/p')
    host_h=$(printf '%s\n' "$line" | sed -n 's/.*hostH=\(-*[0-9]*\).*/\1/p')
    button_h=$(printf '%s\n' "$line" | sed -n 's/.*buttonH=\(-*[0-9]*\).*/\1/p')
    if [ "$origin_x" = "-1" ] || [ -z "$origin_x" ]; then
        echo "FAIL $layout tabs have no close button to place"
        fail=$((fail + 1))
    else
        echo "PASS $layout tabs expose a real close button"
        pass=$((pass + 1))
    fi
    # A button that hangs off the titlebar is the bug this guards: the row used
    # to be computed from the window height, which put all three off the strip.
    if [ -n "$host_h" ] && [ "$host_h" -gt 0 ] && [ -n "$button_h" ] && \
       [ "$origin_y" -ge 0 ] && [ $((origin_y + button_h)) -le "$host_h" ]; then
        echo "PASS $layout window buttons sit whole inside the titlebar"
        pass=$((pass + 1))
    else
        echo "FAIL $layout window buttons hang outside the titlebar (y=$origin_y h=$button_h host=$host_h)"
        fail=$((fail + 1))
    fi

    check_in "drag strip=1 tab=0 toolbar=1 button=0 address=0 page=0 header=$header empty=$empty" \
        "$profile/run.log" "$layout chrome drags the window from empty space only"
}

echo "--- horizontal tabs ---"
run_layout horizontal 23 23 1 0
echo "--- vertical tabs ---"
run_layout vertical 29 25 1 1
echo "--- collapsed rail ---"
run_layout rail 29 25 0 1

echo "traffic: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
