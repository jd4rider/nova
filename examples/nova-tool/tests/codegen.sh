#!/bin/sh
# Nova codegen slice: emit LLVM IR, compile it with clang, execute the binary.
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

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-sample.nova" "$tool")
echo "$out" | grep -q "define i64 @nova_add(i64 %p0, i64 %p1)" && ok || no "add signature"
echo "$out" | grep -q "add i64" && ok || no "integer addition"
echo "$out" | grep -q "call i64 @nova_add(i64 1, i64 2)" && ok || no "direct call"
echo "$out" | grep -q "define i32 @main()" && ok || no "entry wrapper"
if echo "$out" | grep -q "alloca"; then
    no "immutable params/lets spill to stack"
else
    ok
fi

printf '%s\n' "$out" > "$tmp/out.ll"
if clang -x ir "$tmp/out.ll" -o "$tmp/out" > "$tmp/clang.log" 2>&1; then
    ok
else
    no "clang rejects emitted IR"
    cat "$tmp/clang.log"
fi

if [ -x "$tmp/out" ]; then
    "$tmp/out"
    code=$?
    [ "$code" -eq 9 ] && ok || no "native result (got $code, expected 9)"
    os=$(uname -s)
    if [ "$os" = "Darwin" ]; then
        file "$tmp/out" | grep -q "Mach-O" && ok || no "Mach-O output"
    elif [ "$os" = "Linux" ]; then
        file "$tmp/out" | grep -q "ELF" && ok || no "ELF output"
    else
        no "native format assertion unsupported on $os"
    fi
else
    no "native binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-var.nova" "$tool")
printf '%s\n' "$out" > "$tmp/var.ll"
if clang -x ir "$tmp/var.ll" -o "$tmp/var" > "$tmp/var-clang.log" 2>&1; then
    ok
else
    no "clang rejects mutable IR"
    cat "$tmp/var-clang.log"
fi
if [ -x "$tmp/var" ]; then
    "$tmp/var"
    code=$?
    [ "$code" -eq 2 ] && ok || no "mutable result (got $code, expected 2)"
else
    no "mutable binary missing"
fi

printf 'fn main() -> Int {\n    return "x"\n}\n' > "$tmp/bad.nova"
out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$tmp/bad.nova" "$tool")
echo "$out" | grep -q "^error:" && ok || no "unsupported expression is reported"

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
