#!/bin/sh
# Tab context-menu coverage.
#
# The menu is an AppKit NSMenu shown on right click, so it cannot be clicked
# from a script. Two halves are testable:
#   NOVA_BROWSER_TAB_MENU_ITEMS=1  builds the menu and reports its items
#                                  (title, enabled) without showing it;
#   NOVA_BROWSER_TAB_ACTION=1      dispatches the two multi-tab commands the
#                                  menu adds (Duplicate, Close Other Tabs)
#                                  through the same helpers the menu calls.
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="$ROOT/target/nova/nova-browser"
if [ ! -x "$BIN" ]; then
    echo "tabs: missing $BIN (build the browser first)"
    exit 1
fi

PROFILE=$(mktemp -d /tmp/nova-browser-tabs.XXXXXX)
LOG="$PROFILE/run.log"

NOVA_BROWSER_PROFILE="$PROFILE" \
NOVA_BROWSER_NO_RESTORE=1 \
NOVA_BROWSER_NO_HISTORY=1 \
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_TAB_ACTION=1 \
NOVA_BROWSER_TAB_MENU_ITEMS=1 \
    "$BIN" >"$LOG" 2>&1

pass=0
fail=0
check() {
    if grep -qF "$1" "$LOG"; then
        echo "PASS $2"
        pass=$((pass + 1))
    else
        echo "FAIL $2"
        fail=$((fail + 1))
    fi
}

check 'menu: items=New Tab|Duplicate Tab|Close Tab|Close Other Tabs' 'menu has the four tab commands'
check 'enabled=1,1,1,0' 'close-others is disabled with one tab'
check 'browser: action duplicate tabs=2' 'duplicate adds one tab'
check 'browser: action close-others tabs=1' 'close-others leaves one tab'
check 'browser: state=ready' 'session still renders after the commands'

echo "tabs: $pass PASS, $fail FAIL"
echo "tabs: log $LOG"
[ "$fail" -eq 0 ]
