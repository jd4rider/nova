#!/bin/sh
# CLI-01.01 slice: the Nova driver builds single- and multi-file programs.
# COMP-01.03: relative/extensionless imports, transitive modules, path
# dependencies, std validation and missing/cyclic module diagnostics.
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

# ---- Test runner and REPL ------------------------------------------------
mkdir -p "$tmp/project/tests"
printf '@test\nfn passes() {\n    assert(true)\n}\n' > "$tmp/project/tests/sample_test.nova"
if (cd "$tmp/project" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" test >"$tmp/test.out" 2>"$tmp/test.err"); then
    ok
else
    no "nova test failed for a passing test"
    cat "$tmp/test.out" "$tmp/test.err"
fi
grep -q "1 passed, 0 failed" "$tmp/test.out" && ok || no "test summary ($(cat "$tmp/test.out"))"
printf '@test\nfn fails() {\n    assert(false)\n}\n' > "$tmp/project/tests/failing_test.nova"
status=0
(cd "$tmp/project" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" test >"$tmp/testfail.out" 2>&1) || status=$?
[ "$status" -ne 0 ] && grep -q "1 failed" "$tmp/testfail.out" && ok || no "failing test reported ($(cat "$tmp/testfail.out"))"
repl_out=$(printf '1 + 2\n:quit\n' | (cd "$tmp/project" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" repl 2>&1))
echo "$repl_out" | grep -q "3" && ok || no "REPL expression output ($repl_out)"

# ---- Module resolution: extensionless names and transitive imports ------
mkdir -p "$tmp/modules/src"
printf '[package]\nname = "modules"\nversion = "0.1.0"\n' > "$tmp/modules/Box.toml"
printf 'import "util"\nfn main() -> Int { return util.twice(4) }\n' > "$tmp/modules/src/main.nova"
printf 'import "inner.nova" as inner\nfn twice(n: Int) -> Int { return inner.base(n) + n }\n' > "$tmp/modules/src/util.nova"
printf 'fn base(n: Int) -> Int { return n }\n' > "$tmp/modules/src/inner.nova"
if (cd "$tmp/modules" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" build >"$tmp/modules.out" 2>&1); then
    ok
else
    no "transitive module build"
    cat "$tmp/modules.out"
fi
if [ -x "$tmp/modules/target/nova/modules" ]; then
    ok
    code=0
    "$tmp/modules/target/nova/modules" || code=$?
    [ "$code" -eq 8 ] && ok || no "module program exit (got $code, expected 8)"
else
    no "module build produced no executable"
fi
out=$( cd "$tmp/modules" && NOVA_TOOL=modules NOVA_TOOL_PATH="$tmp/modules/src/main.nova" "$tool" )
echo "$out" | grep -q "inner.nova name=inner decls=1" && ok || no "module graph lists declarations ($out)"
echo "$out" | grep -q "util.nova name=util decls=1" && ok || no "module graph lists the middle module ($out)"

# ---- A [dependencies] path dependency is importable by name ----------------
mkdir -p "$tmp/pathdep/lib/src" "$tmp/pathdep/app/src"
printf '[package]\nname = "lib"\nversion = "1.0.0"\n\n[bin]\npath = "src/lib.nova"\n' > "$tmp/pathdep/lib/Box.toml"
printf 'fn answer() -> Int { return 42 }\n' > "$tmp/pathdep/lib/src/lib.nova"
printf '[package]\nname = "pathdep"\nversion = "1.0.0"\n\n[dependencies]\nlib = { path = "../lib" }\n' > "$tmp/pathdep/app/Box.toml"
printf 'import "lib"\nfn main() -> Int { return answer() }\n' > "$tmp/pathdep/app/src/main.nova"
if (cd "$tmp/pathdep/app" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" build >"$tmp/pathdep.out" 2>&1); then
    ok
else
    no "path dependency build"
    cat "$tmp/pathdep.out"
fi
if [ -x "$tmp/pathdep/app/target/nova/pathdep" ]; then
    ok
    code=0
    "$tmp/pathdep/app/target/nova/pathdep" || code=$?
    [ "$code" -eq 42 ] && ok || no "path dependency exit (got $code, expected 42)"
else
    no "path dependency build produced no executable"
fi

# ---- Import diagnostics ---------------------------------------------------
mkdir -p "$tmp/badimport/src"
printf '[package]\nname = "badimport"\nversion = "0.1.0"\n' > "$tmp/badimport/Box.toml"
printf 'import std.bogus { x }\nfn main() { }\n' > "$tmp/badimport/src/main.nova"
status=0
out=$( cd "$tmp/badimport" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" build 2>&1 ) || status=$?
[ "$status" -ne 0 ] && echo "$out" | grep -q "unknown standard module" && ok || no "unknown std import diagnostic ($out)"
printf 'import "nope"\nfn main() { }\n' > "$tmp/badimport/src/main.nova"
status=0
out=$( cd "$tmp/badimport" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" build 2>&1 ) || status=$?
[ "$status" -ne 0 ] && echo "$out" | grep -q "cannot resolve module" && ok || no "missing module diagnostic ($out)"
printf 'import "b.nova"\nfn main() { }\n' > "$tmp/badimport/src/main.nova"
printf 'import "main.nova"\nfn helper() { }\n' > "$tmp/badimport/src/b.nova"
status=0
out=$( cd "$tmp/badimport" && NOVA_TOOL_RUNTIME="$root/runtime/runtime.c" "$tool" build 2>&1 ) || status=$?
[ "$status" -ne 0 ] && echo "$out" | grep -q "cyclic module import" && ok || no "cyclic import diagnostic ($out)"

echo "passed $passed, failed $failed"
[ "$failed" -eq 0 ]
