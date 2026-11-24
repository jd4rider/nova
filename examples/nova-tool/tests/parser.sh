#!/bin/sh
# The Nova parser covers item and statement structure; check it on a fixture and
# make sure it survives every library file.
set -u
root=$(cd "$(dirname "$0")/../../.." && pwd)
tool="$root/examples/nova-tool/target/nova/nova-tool"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
pass=0
fail=0
ok() { pass=$((pass + 1)); }
no() { fail=$((fail + 1)); echo "FAIL $1"; }
[ -x "$tool" ] || { echo "build examples/nova-tool first"; exit 1; }

out=$(NOVA_TOOL=parse NOVA_TOOL_PATH="$root/examples/nova-tool/tests/parse-sample.nova" "$tool")
echo "$out" | grep -q "struct User params=0" && ok || no "struct"
echo "$out" | grep -q "fn greet params=1 stmts=3 depth=2" && ok || no "greet"
echo "$out" | grep -q "fn main params=0 stmts=0 depth=2" && ok || no "main"

# Phase-2 AST: functions, parameters, lets, returns, calls and precedence.
out=$(NOVA_TOOL=ast-dump NOVA_TOOL_PATH="$root/examples/nova-tool/tests/ast-sample.nova" "$tool")
echo "$out" | grep -q "(fn add (param a:Int) (param b:Int) -> Int" && ok || no "ast function and params"
echo "$out" | grep -q "(let sum (+ a b))" && ok || no "ast let and binary"
echo "$out" | grep -q "(call println (call add 1 2))" && ok || no "ast nested calls"

printf 'struct Point { }\n' > "$tmp/unsupported.nova"
out=$(NOVA_TOOL=ast-dump NOVA_TOOL_PATH="$tmp/unsupported.nova" "$tool")
echo "$out" | grep -q "unsupported item" && ok || no "ast rejects unsupported item"
printf 'fn main( {\n' > "$tmp/broken.nova"
out=$(NOVA_TOOL=ast-dump NOVA_TOOL_PATH="$tmp/broken.nova" "$tool")
echo "$out" | grep -q "error:" && ok || no "ast reports a parse error"

for file in "$root"/libraries/*/src/*.nova; do
    if ! NOVA_TOOL=parse NOVA_TOOL_PATH="$file" "$tool" >/dev/null 2>&1; then
        no "parse $(basename "$file")"
    fi
done
echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
