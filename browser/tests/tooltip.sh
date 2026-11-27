#!/bin/sh
# Tooltips stay above the page, including the ones at the right of the toolbar.
#
# The chrome is the window's content view and each WKWebView is its subview, so
# anything below the chrome's height belongs to the page. A tip drawn there is
# covered by the page — which is what hid the tips over the right-hand controls.
# The app registers the tip's box with `gui.overlay`; the host copies that part
# of the frame into a view above the page and the trace prints
# `overlay rects=N top=1`.
#
# NOVA_BROWSER_TOOLTIP_CHECK=<target> forces a tip at the right edge, where the
# controls sit, and the app prints the final box it drew.
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="$ROOT/target/nova/nova-browser"
if [ ! -x "$BIN" ]; then
    echo "tooltip: missing $BIN (build the browser first)"
    exit 1
fi

pass=0
fail=0
check() {
    if [ "$1" = "1" ]; then
        echo "PASS $2"
        pass=$((pass + 1))
    else
        echo "FAIL $2"
        fail=$((fail + 1))
    fi
}

profile=$(mktemp -d "/tmp/nova-tooltip.XXXXXX")
trace="$profile/gui.log"
NOVA_BROWSER_PROFILE="$profile" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_TOOLTIP_CHECK=9 \
NOVA_GUI_TRACE="$trace" \
    "$BIN" >"$profile/run.log" 2>&1
status=$?
check "$([ "$status" -eq 0 ] && echo 1 || echo 0)" "app exits cleanly (status=$status)"

line=$(grep -h '^tooltip:' "$profile/run.log" | tail -1)
check "$([ -n "$line" ] && echo 1 || echo 0)" "a tip is drawn at the right of the toolbar"
check "$(printf '%s' "$line" | grep -q 'text=Downloads' && echo 1 || echo 0)" "the tip names the hovered control"
check "$(printf '%s' "$line" | grep -q 'overlay=1' && echo 1 || echo 0)" "the tip is registered as an overlay"
check "$(printf '%s' "$line" | grep -q 'crosses=1' && echo 1 || echo 0)" "the tip crosses the page edge"

x=$(printf '%s\n' "$line" | sed -n 's/.* x=\(-*[0-9]*\).*/\1/p')
w=$(printf '%s\n' "$line" | sed -n 's/.* w=\(-*[0-9]*\).*/\1/p')
win=$(printf '%s\n' "$line" | sed -n 's/.* window=\(-*[0-9]*\).*/\1/p')
inside=0
if [ -n "$x" ] && [ -n "$w" ] && [ -n "$win" ] && [ "$x" -ge 0 ] && [ $((x + w)) -le "$win" ]; then
    inside=1
fi
check "$inside" "the tip stays inside the window"

check "$(grep -q 'overlay rects=[1-9]' "$trace" && echo 1 || echo 0)" "the host raises the overlay"
check "$(grep -q 'top=1' "$trace" && echo 1 || echo 0)" "the overlay view is above the page views"

rm -rf "$profile"
echo "tooltip: pass=$pass fail=$fail"
[ "$fail" -eq 0 ] || exit 1
exit 0
