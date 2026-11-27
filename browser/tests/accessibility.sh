#!/bin/sh
# Accessibility coverage for the Nova-owned chrome.
#
# The chrome is one rasterized NSView, so AppKit cannot infer controls from a
# widget tree: the app pushes a flat element list while drawing and the host
# builds NSAccessibilityElement children from it. NOVA_GUI_AX_DUMP=1 walks
# exactly the children AppKit gets (role, label, value, selection, screen
# frame) and prints them once, which is what these checks read.
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="$ROOT/target/nova/nova-browser"
if [ ! -x "$BIN" ]; then
    echo "accessibility: missing $BIN (build the browser first)"
    exit 1
fi

PROFILE=$(mktemp -d /tmp/nova-browser-ax.XXXXXX)
LOG="$PROFILE/run.log"

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
check() {
    check_in "$1" "$LOG" "$2"
}

NOVA_BROWSER_PROFILE="$PROFILE" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_GUI_AX_DUMP=1 \
    "$BIN" >"$LOG" 2>&1

check 'ax: children=11 built=11' 'every pushed element becomes an accessibility child'
check 'role=AXTab label=' 'the tab is exposed as a tab'
check 'role=AXTab label=New Tab value= selected=1' 'the active tab reports selection'
check 'role=AXTextField label=Address' 'the address field is a text field'
check 'role=AXButton label=Back' 'back is a pressable button'
check 'role=AXButton label=Reload' 'reload is a pressable button'
check 'role=AXButton label=Tab layout' 'the tab-layout control is exposed'
check 'role=AXStaticText label=Rendering engine: WebKit' 'the engine badge is readable'
check 'browser: state=ready' 'the session still renders with accessibility on'

# Frames are converted to screen space; an empty or zero-height frame would
# make the element invisible to VoiceOver even though it is in the tree.
frames=$(grep -cE '^ax: role=.*frame=[0-9-]+,[0-9-]+,[1-9][0-9]*,[1-9][0-9]*$' "$LOG")
if [ "$frames" -eq 11 ]; then
    echo "PASS all element frames are non-empty and in the drawn geometry"
    pass=$((pass + 1))
else
    echo "FAIL expected 11 sized frames, found $frames"
    fail=$((fail + 1))
fi

# The download shelf owns its rectangle and its rows are Nova-drawn too, so it
# must publish its own names. No transfer is needed: the empty shelf still has a
# close control and a message.
echo "--- download shelf ---"
SHELF=$(mktemp -d /tmp/nova-browser-ax-shelf.XXXXXX)
SHELF_LOG="$SHELF/run.log"
NOVA_BROWSER_PROFILE="$SHELF" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_DOWNLOADS_OPEN=1 \
NOVA_GUI_AX_DUMP=1 \
    "$BIN" >"$SHELF_LOG" 2>&1
check_in 'ax: children=13 built=13' "$SHELF_LOG" 'the open shelf adds its controls to the tree'
check_in 'role=AXButton label=Close downloads' "$SHELF_LOG" 'the shelf close control is named'
check_in 'role=AXStaticText label=No downloads yet' "$SHELF_LOG" 'the empty shelf message is readable'

# Autocomplete rows are links: the visible text is the label and the URL is the
# value, so a screen reader reaches a suggestion without reading the field.
echo "--- suggestions ---"
SUG=$(mktemp -d /tmp/nova-browser-ax-sug.XXXXXX)
printf 'bookmark\thttps://nova.example/docs\tNova docs\n' >"$SUG/bookmarks.tsv"
SUG_LOG="$SUG/run.log"
NOVA_BROWSER_PROFILE="$SUG" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_FOCUS_ADDRESS=nov \
NOVA_GUI_AX_DUMP=1 \
    "$BIN" >"$SUG_LOG" 2>&1
check_in 'role=AXLink label=Nova docs value=https://nova.example/docs' "$SUG_LOG" 'suggestion rows are links with their URL'
check_in 'ax: children=14 built=14' "$SUG_LOG" 'a bookmark and the engine row add exactly one element each'
check_in 'role=AXLink label=Search Google for "nov" value=https://www.google.com/search?q=nov' "$SUG_LOG" 'plain text offers a Google search row'

# The dump is opt-in: a normal launch must not print accessibility lines.
PLAIN=$(mktemp -d /tmp/nova-browser-ax-plain.XXXXXX)
NOVA_BROWSER_PROFILE="$PLAIN" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
    "$BIN" >"$PLAIN/run.log" 2>&1
if grep -q '^ax:' "$PLAIN/run.log"; then
    echo "FAIL dump printed without NOVA_GUI_AX_DUMP"
    fail=$((fail + 1))
else
    echo "PASS dump stays opt-in"
    pass=$((pass + 1))
fi

echo "accessibility: $pass PASS, $fail FAIL"
echo "accessibility: log $LOG"
[ "$fail" -eq 0 ]
