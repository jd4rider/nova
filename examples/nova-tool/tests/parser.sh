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
echo "$out" | grep -q "(import std/io.nova as io)" && ok || no "ast import"
echo "$out" | grep -q "(struct Point (field x:Int) (field y:Int))" && ok || no "ast struct"
echo "$out" | grep -q "(extern abs (param value:Int) -> Int)" && ok || no "ast extern"
echo "$out" | grep -q "(while (< i limit)" && ok || no "ast while"
echo "$out" | grep -q "(if (== i 1)" && ok || no "ast if/else"
echo "$out" | grep -q "(for n (.. 0 3)" && ok || no "ast for"
echo "$out" | grep -q "(enum Color (variant Red) (variant Green Int) (variant Blue))" && ok || no "ast enum"
echo "$out" | grep -q "(lit Point (init x 1) (init y 2))" && ok || no "ast struct literal"
echo "$out" | grep -q "(list 1 2)" && ok || no "ast list literal"
echo "$out" | grep -q "(tuple (index xs 0) (index xs 1))" && ok || no "ast tuple"
echo "$out" | grep -q "(fn identity (param value:T) -> T" && ok || no "ast generic function"
echo "$out" | grep -q "(match cmd (arm Red true) (arm Green(_) false) (arm Blue true))" && ok || no "ast match"
echo "$out" | grep -q "(+= i 1)" && ok || no "ast compound assignment"

printf 'type Alias = Int\n' > "$tmp/unsupported.nova"
out=$(NOVA_TOOL=ast-dump NOVA_TOOL_PATH="$tmp/unsupported.nova" "$tool")
echo "$out" | grep -q "unsupported item" && ok || no "ast rejects unsupported item"
printf 'fn main( {\n' > "$tmp/broken.nova"
out=$(NOVA_TOOL=ast-dump NOVA_TOOL_PATH="$tmp/broken.nova" "$tool")
echo "$out" | grep -q "error:" && ok || no "ast reports a parse error"

# The AST must read every library and tool source without a parse error.
for file in "$root"/libraries/*/src/*.nova "$root"/examples/nova-tool/src/*.nova; do
    out=$(NOVA_TOOL=ast-dump NOVA_TOOL_PATH="$file" "$tool")
    if echo "$out" | grep -q '^error:'; then
        no "ast $(basename "$file")"
    else
        ok
    fi
done

for file in "$root"/libraries/*/src/*.nova; do
    if ! NOVA_TOOL=parse NOVA_TOOL_PATH="$file" "$tool" >/dev/null 2>&1; then
        no "parse $(basename "$file")"
    fi
done
echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
