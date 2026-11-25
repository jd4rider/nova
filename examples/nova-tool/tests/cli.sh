#!/bin/sh
# CLI-01.01 slice: the Nova driver builds a single-file program through argv.
set -eu
root=$(cd "$(dirname "$0")/../../.." && pwd)
tool="$root/examples/nova-tool/target/nova/nova-tool"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
passed=0
failed=0

ok() { passed=$((passed + 1)); }
no() { failed=$((failed + 1)); echo "FAIL $1"; }

if [ ! -x "$tool" ]; then
    echo "cli: build examples/nova-tool first" >&2
    exit 1
fi

if "$tool" build "$root/examples/nova-tool/tests/cli-hello.nova" "$tmp/cli-hello" >"$tmp/build.out" 2>"$tmp/build.err"; then
    ok
else
    no "nova-tool build failed"
    cat "$tmp/build.out" "$tmp/build.err"
fi

grep -q "built " "$tmp/build.out" && ok || no "build output"
if [ -x "$tmp/cli-hello" ]; then
    ok
    if "$tmp/cli-hello"; then
        code=0
    else
        code=$?
    fi
    [ "$code" -eq 7 ] && ok || no "native exit (got $code, expected 7)"
else
    no "build produced no executable"
fi

cp -R "$root/examples/nova-tool/tests/cli-project" "$tmp/project"
if (cd "$tmp/project" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" build >"$tmp/project.out" 2>"$tmp/project.err"); then
    ok
else
    no "project build failed"
    cat "$tmp/project.out" "$tmp/project.err"
fi
if [ -x "$tmp/project/target/nova/cli-project" ]; then
    ok
    if "$tmp/project/target/nova/cli-project"; then
        code=0
    else
        code=$?
    fi
    [ "$code" -eq 10 ] && ok || no "project exit (got $code, expected 10)"
else
    no "project build produced no executable"
fi

if (cd "$tmp/project" && NOVA_TOOL_OUT="$tmp/override" NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" build >"$tmp/override.out" 2>"$tmp/override.err"); then
    ok
else
    no "project output override failed"
    cat "$tmp/override.out" "$tmp/override.err"
fi
[ -x "$tmp/override" ] && ok || no "output override produced no executable"

if (cd "$tmp/project" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" check >"$tmp/check.out" 2>"$tmp/check.err"); then
    ok
else
    no "project check failed"
    cat "$tmp/check.out" "$tmp/check.err"
fi

if (cd "$tmp/project" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" run >"$tmp/run.out" 2>"$tmp/run.err"); then
    code=0
else
    code=$?
fi
[ "$code" -eq 10 ] && ok || no "project run exit (got $code, expected 10)"

echo "passed $passed, failed $failed"
[ "$failed" -eq 0 ]
