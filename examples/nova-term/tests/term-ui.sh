#!/bin/sh
# Scripted UI tests for Nova Terminal: ANSI parsing, pty typing, tabs.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
APP=$(cd "$HERE/.." && pwd)
ROOT=$(cd "$APP/../.." && pwd)
BIN="$APP/target/nova/nova-term"
if [ ! -x "$BIN" ]; then
    echo "missing $BIN — run: (cd examples/nova-term && ../../target/debug/nova build)" >&2
    exit 2
fi
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
PASS=0
FAIL=0
run_case() {
    name=$1
    script=$2
    out=$(NOVA_TERM_ROOT="$ROOT" NOVA_TERM_SCRIPT="$script" "$BIN" 2>&1)
    p=$(printf '%s\n' "$out" | grep -c '^PASS')
    f=$(printf '%s\n' "$out" | grep -c '^FAIL')
    printf '%s\n' "$out" | grep '^PASS' | sed "s/^PASS /  ok   [$name] /"
    printf '%s\n' "$out" | grep '^FAIL' | sed "s/^FAIL /  FAIL [$name] /"
    PASS=$((PASS + p))
    FAIL=$((FAIL + f))
}

cat > "$WORK/ansi.txt" <<'SCRIPT'
feed <E>[31mred<E>[0m <E>[92mgreen<E>[0m
assert_cell 0 0 r
assert_colour 0 0 1
assert_cell 0 4 g
assert_colour 0 4 10
feed <E>[2;1Hsecond
assert_cell 1 0 s
feed <E>[38;5;196mX
assert_colour 0 0 1
SCRIPT
run_case ansi "$WORK/ansi.txt"

cat > "$WORK/shell.txt" <<'SCRIPT'
wait 1200
type pwd
enter
wait 900
assert_has Users
SCRIPT
run_case shell "$WORK/shell.txt"

cat > "$WORK/tabs.txt" <<'SCRIPT'
assert_tabs 1
newtab
assert_tabs 2
cmd close
assert_tabs 1
SCRIPT
run_case tabs "$WORK/tabs.txt"

cat > "$WORK/title.txt" <<'SCRIPT'
feed <E>]0;my-shell<E>\
assert_title my-shell
SCRIPT
run_case title "$WORK/title.txt"

cat > "$WORK/alt.txt" <<'SCRIPT'
feed row-one
feed <E>[?1049hZZZ<E>[?1049l
assert_cell 0 0 r
assert_has row-one
SCRIPT
run_case altscreen "$WORK/alt.txt"

cat > "$WORK/font.txt" <<'SCRIPT'
fontname
assert_font_has Nerd
SCRIPT
run_case font "$WORK/font.txt"

cat > "$WORK/structs.txt" <<'SCRIPT'
assert_lit
assert_lit_push
assert_tab_cwd
assert_tab_title shell
SCRIPT
run_case structs "$WORK/structs.txt"

echo
echo "passed $PASS, failed $FAIL"
[ "$FAIL" -eq 0 ]
