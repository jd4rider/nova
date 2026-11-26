#!/bin/sh
# DIFF-01.01: golden-output regression for the Nova-built driver.
#
# These fixtures were compared live against the Rust seed until the seed was
# removed. `diff-golden.txt` records the seed's normalized stdout for each one,
# so the comparison survives without a Rust toolchain; a divergence fails the
# suite. Each fixture's entry point is renamed to `diff_main` and wrapped in
# `fn main() { println(diff_main()) }`, so a wrong value shows up even though
# both programs exit 0.
set -eu
dir=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$dir/../../.." && pwd)
tool=${NOVA_TOOL_BIN:-"$dir/../target/nova/nova-tool"}
runtime="$root/runtime/runtime.c"
golden="$dir/diff-golden.txt"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
passed=0
failed=0

[ -x "$tool" ] || { echo "diff: build examples/nova-tool first" >&2; exit 1; }

while IFS='|' read -r name expected; do
    [ -n "$name" ] || continue
    src="$dir/$name.nova"
    if [ ! -f "$src" ]; then
        failed=$((failed + 1))
        echo "FAIL $name: fixture missing"
        continue
    fi

    mkdir -p "$tmp/$name/src"
    printf '[package]\nname = "diff"\nversion = "0.1.0"\n' > "$tmp/$name/Box.toml"
    {
        echo 'import std.io { println }'
        sed 's/^fn main(/fn diff_main(/' "$src"
        echo 'fn main() {'
        echo '    println(diff_main())'
        echo '}'
    } > "$tmp/$name/src/main.nova"

    if ! (cd "$tmp/$name" && NOVA_TOOL_RUNTIME="$runtime" "$tool" build > "$tmp/$name/build.log" 2>&1); then
        failed=$((failed + 1))
        echo "FAIL $name: build failed"
        tail -n 3 "$tmp/$name/build.log" | sed 's/^/  /'
        continue
    fi
    if [ ! -x "$tmp/$name/target/nova/diff" ]; then
        failed=$((failed + 1))
        echo "FAIL $name: produced no executable"
        continue
    fi

    "$tmp/$name/target/nova/diff" > "$tmp/$name/actual.txt" 2>&1 || true
    printf '%b\n' "$expected" > "$tmp/$name/expected.txt"
    if cmp -s "$tmp/$name/expected.txt" "$tmp/$name/actual.txt"; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
        echo "FAIL $name: output changed"
        diff -u "$tmp/$name/expected.txt" "$tmp/$name/actual.txt" || true
    fi
done < "$golden"

echo "golden: $passed matched, $failed differed"
[ "$failed" -eq 0 ]
