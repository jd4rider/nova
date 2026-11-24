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
assert_rows_min 6
click 364 120
assert_fold 1
assert_rows_max 4
click 364 120
assert_fold 0
assert_rows_min 6
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
wait 4000
assert_term_has Built
SHOT_PATH
SCRIPT
sed -i '' "s|SHOT_PATH|shot $WORK/term.png|" "$WORK/term.txt"
run_case terminal "$WORK/term.txt"

echo
echo "passed $PASS, failed $FAIL"
[ "$FAIL" -eq 0 ]
