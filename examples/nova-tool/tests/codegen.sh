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
echo "$out" | grep -q "define i64 @nova_user_0(ptr %arg_env, i64 %p0, i64 %p1)" && ok || no "add signature"
echo "$out" | grep -q "add i64" && ok || no "integer addition"
echo "$out" | grep -q "call i64 @nova_user_0(ptr null, i64 1, i64 2)" && ok || no "direct call"
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

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-flow.nova" "$tool")
echo "$out" | grep -q "icmp slt i64" && ok || no "comparison lowering"
echo "$out" | grep -q "br i1" && ok || no "conditional branch"
printf '%s\n' "$out" > "$tmp/flow.ll"
if clang -x ir "$tmp/flow.ll" -o "$tmp/flow" > "$tmp/flow-clang.log" 2>&1; then
    ok
else
    no "clang rejects control-flow IR"
    cat "$tmp/flow-clang.log"
fi
if [ -x "$tmp/flow" ]; then
    "$tmp/flow"
    code=$?
    [ "$code" -eq 17 ] && ok || no "control-flow result (got $code, expected 17)"
else
    no "control-flow binary missing"
fi

out=$(NOVA_TOOL=build-ir NOVA_TOOL_OUT="$tmp/built" NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-sample.nova" "$tool")
echo "$out" | grep -q "^built " && ok || no "Nova tool invokes clang"
if [ -x "$tmp/built" ]; then
    "$tmp/built"
    code=$?
    [ "$code" -eq 9 ] && ok || no "Nova-built binary result (got $code, expected 9)"
else
    no "Nova-built binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-unit.nova" "$tool")
echo "$out" | grep -q "define void @nova_user_0(ptr %arg_env)" && ok || no "Unit signature"
echo "$out" | grep -q "call void @nova_user_0(ptr null)" && ok || no "Unit call"
printf '%s\n' "$out" > "$tmp/unit.ll"
if clang -x ir "$tmp/unit.ll" -o "$tmp/unit" > "$tmp/unit-clang.log" 2>&1; then
    ok
else
    no "clang rejects Unit IR"
    cat "$tmp/unit-clang.log"
fi
if [ -x "$tmp/unit" ]; then
    "$tmp/unit"
    code=$?
    [ "$code" -eq 4 ] && ok || no "Unit result (got $code, expected 4)"
else
    no "Unit binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-string.nova" "$tool")
echo "$out" | grep -q "%nova.string = type { ptr, i64 }" && ok || no "string type declaration"
echo "$out" | grep -q "define %nova.string @nova_user_0(ptr %arg_env)" && ok || no "String return signature"
echo "$out" | grep -q "define i64 @nova_user_1(ptr %arg_env, %nova.string %p0)" && ok || no "String parameter signature"
echo "$out" | grep -q 'c"hello"' && ok || no "string literal global"
echo "$out" | grep -q "extractvalue %nova.string" && ok || no "String.len lowering"
printf '%s\n' "$out" > "$tmp/string.ll"
if clang -x ir "$tmp/string.ll" -o "$tmp/string" > "$tmp/string-clang.log" 2>&1; then
    ok
else
    no "clang rejects string IR"
    cat "$tmp/string-clang.log"
fi
if [ -x "$tmp/string" ]; then
    "$tmp/string"
    code=$?
    [ "$code" -eq 5 ] && ok || no "string length result (got $code, expected 5)"
else
    no "string binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list.nova" "$tool")
echo "$out" | grep -q "%nova.list = type { ptr, i64, i64 }" && ok || no "list type declaration"
echo "$out" | grep -q "declare ptr @malloc(i64)" && ok || no "list allocator declaration"
echo "$out" | grep -q "call ptr @malloc(i64" && ok || no "list element storage"
echo "$out" | grep -q "insertvalue %nova.list" && ok || no "list aggregate construction"
echo "$out" | grep -q "insertvalue %nova.list zeroinitializer, i64 0, 1" && ok || no "empty list construction"
echo "$out" | grep -q "extractvalue %nova.list" && ok || no "List.len lowering"
echo "$out" | grep -q "getelementptr i64" && ok || no "list index lowering"
echo "$out" | grep -q "call void @llvm.trap()" && ok || no "list bounds trap"
printf '%s\n' "$out" > "$tmp/list.ll"
if clang -x ir "$tmp/list.ll" -o "$tmp/list" > "$tmp/list-clang.log" 2>&1; then
    ok
else
    no "clang rejects list IR"
    cat "$tmp/list-clang.log"
fi
if [ -x "$tmp/list" ]; then
    "$tmp/list"
    code=$?
    [ "$code" -eq 23 ] && ok || no "list result (got $code, expected 23)"
else
    no "list binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list-abi.nova" "$tool")
echo "$out" | grep -q "define i64 @nova_user_0(ptr %arg_env, %nova.list %p0)" && ok || no "list parameter signature"
echo "$out" | grep -q "define %nova.list @nova_user_1(ptr %arg_env)" && ok || no "list return signature"
echo "$out" | grep -q "define %nova.list @nova_user_2(ptr %arg_env, %nova.list %p0)" && ok || no "list identity signature"
echo "$out" | grep -q "call %nova.list @nova_user_2" && ok || no "list argument/return call"
printf '%s\n' "$out" > "$tmp/list-abi.ll"
if clang -x ir "$tmp/list-abi.ll" -o "$tmp/list-abi" > "$tmp/list-abi-clang.log" 2>&1; then
    ok
else
    no "clang rejects list ABI IR"
    cat "$tmp/list-abi-clang.log"
fi
if [ -x "$tmp/list-abi" ]; then
    "$tmp/list-abi"
    code=$?
    [ "$code" -eq 13 ] && ok || no "list ABI result (got $code, expected 13)"
else
    no "list ABI binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-unit-main.nova" "$tool")
echo "$out" | grep -q "define void @nova_user_0(ptr %arg_env)" && ok || no "Unit main signature"
echo "$out" | grep -q "call void @nova_user_0(ptr null)" && ok || no "Unit main call"
printf '%s\n' "$out" > "$tmp/unit-main.ll"
if clang -x ir "$tmp/unit-main.ll" -o "$tmp/unit-main" > "$tmp/unit-main-clang.log" 2>&1; then
    ok
else
    no "clang rejects Unit main IR"
    cat "$tmp/unit-main-clang.log"
fi
if [ -x "$tmp/unit-main" ]; then
    "$tmp/unit-main"
    code=$?
    [ "$code" -eq 0 ] && ok || no "Unit main result (got $code, expected 0)"
else
    no "Unit main binary missing"
fi

printf 'fn main() -> Int {\n    return "x"\n}\n' > "$tmp/bad.nova"
out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$tmp/bad.nova" "$tool")
echo "$out" | grep -q "^error:" && ok || no "unsupported expression is reported"

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
