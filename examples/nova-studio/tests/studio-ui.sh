#!/bin/sh
# Drives Nova Studio with synthetic events and asserts on the model it ends up
# with. Run after `nova build` in examples/nova-studio.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
APP="$HERE/.."
ROOT=$(cd "$APP/../.." && pwd)
BIN="$APP/target/nova/nova-studio"
NOVA_BIN="$ROOT/target/debug/nova"
FIXTURE="$HERE/fixtures/sample.nova"
if [ ! -x "$BIN" ]; then
    echo "missing $BIN — run: (cd examples/nova-studio && ../../target/debug/nova build)" >&2
    exit 2
fi
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
PASS=0
FAIL=0
run_case() {
    name=$1
    script=$2
    out=$(NOVA_BIN="$NOVA_BIN" NOVA_STUDIO_ROOT="$APP" NOVA_STUDIO_OPEN="$FIXTURE" \
          NOVA_STUDIO_SCRIPT="$script" "$BIN" 2>&1)
    p=$(printf '%s\n' "$out" | grep -c '^PASS')
    f=$(printf '%s\n' "$out" | grep -c '^FAIL')
    printf '%s\n' "$out" | grep '^PASS' | sed "s/^PASS /  ok   [$name] /"
    printf '%s\n' "$out" | grep '^FAIL' | sed "s/^FAIL /  FAIL [$name] /"
    PASS=$((PASS + p))
    FAIL=$((FAIL + f))
}

cat > "$WORK/select.txt" <<'SCRIPT'
# drag across a line, then copy it
click 400 139
drag 800 139
assert_selection_has needle
keymod 99 1
assert_clipboard_has needle
SCRIPT
run_case select "$WORK/select.txt"

cat > "$WORK/find.txt" <<'SCRIPT'
keymod 102 1
text hello
assert_find 2
SCRIPT
run_case find "$WORK/find.txt"

cat > "$WORK/fold.txt" <<'SCRIPT'
assert_rows_min 10
click 364 120
assert_fold 1
assert_rows_max 8
click 364 120
assert_fold 0
assert_rows_min 10
SCRIPT
run_case fold "$WORK/fold.txt"

cat > "$WORK/cursor.txt" <<'SCRIPT'
click 440 158
keymod 100 1
assert_cursors 1
key 105
text X
assert_line_has 3 X
assert_line_has 4 X
assert_cursors 1
SCRIPT
run_case multicursor "$WORK/cursor.txt"

cat > "$WORK/term.txt" <<'SCRIPT'
build_now
wait 14000
assert_term_has Built
SHOT_PATH
SCRIPT
sed -i '' "s|SHOT_PATH|shot $WORK/term.png|" "$WORK/term.txt"
run_case terminal "$WORK/term.txt"

cat > "$WORK/goto.txt" <<'SCRIPT'
click 442 272
key 1009
assert_cursor 1 3
assert_file_has sample
assert_line_has 1 fn
SCRIPT
run_case gotodefinition "$WORK/goto.txt"

cat > "$WORK/split.txt" <<'SCRIPT'
splittoggle
assert_split 1
click 900 200
assert_split 1
splittoggle
assert_split 0
SCRIPT
run_case split "$WORK/split.txt"

cat > "$WORK/terminput.txt" <<'SCRIPT'
termcmd cat
wait 400
termtype echo-ok
termenter
wait 700
assert_term_has echo-ok
SCRIPT
run_case termininput "$WORK/terminput.txt"

cat > "$WORK/grid.txt" <<'SCRIPT'
termclear
termfeed <E>[31mred<E>[0m and <E>[92mgreen<E>[0m
assert_term_cell 0 0 r
assert_term_colour 0 0 1
assert_term_cell 0 8 g
assert_term_colour 0 8 10
termfeed <E>[2;1Hsecond line
assert_term_cell 1 0 s
termresize 40 30
assert_term_cell 7 0 s
termfeed <E>[2J<E>[Hclean
assert_term_cell 0 0 c
SCRIPT
run_case terminalgrid "$WORK/grid.txt"

cat > "$WORK/pane.txt" <<'SCRIPT'
openfile tests/fixtures/other.nova
assert_pane_tab 0 1
openfile tests/fixtures/sample.nova
click 442 272
refs
assert_refs_min 2
key 27
openfile tests/fixtures/other.nova
assert_pane_tab 0 1
splittoggle
assert_split 1
assert_pane_tab 1 1
splittoggle
assert_split 0
SCRIPT
run_case panesandrefs "$WORK/pane.txt"

cat > "$WORK/termselect.txt" <<'SCRIPT'
termfeed <E>[2J<E>[Hhello selection
termselect 0 0 0 14
assert_term_sel_has hello
keymod 99 1
assert_clipboard_has hello
termfont
SCRIPT
run_case termselection "$WORK/termselect.txt"

echo
echo "passed $PASS, failed $FAIL"
[ "$FAIL" -eq 0 ]
