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
clang_ir() {
    clang -x ir "$1" -x none "$root/runtime/runtime.c" -o "$2" -lm > "$3" 2>&1
}
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
if clang_ir "$tmp/out.ll" "$tmp/out" "$tmp/clang.log"; then
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
if clang_ir "$tmp/var.ll" "$tmp/var" "$tmp/var-clang.log"; then
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
if clang_ir "$tmp/flow.ll" "$tmp/flow" "$tmp/flow-clang.log"; then
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

out=$(NOVA_TOOL=build-ir NOVA_TOOL_OUT="$tmp/built-list" NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list.nova" "$tool")
echo "$out" | grep -q "^built " && ok || no "Nova tool links the runtime"
if [ -x "$tmp/built-list" ]; then
    "$tmp/built-list"
    code=$?
    [ "$code" -eq 23 ] && ok || no "Nova-built list result (got $code, expected 23)"
else
    no "Nova-built list binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-unit.nova" "$tool")
echo "$out" | grep -q "define void @nova_user_0(ptr %arg_env)" && ok || no "Unit signature"
echo "$out" | grep -q "call void @nova_user_0(ptr null)" && ok || no "Unit call"
printf '%s\n' "$out" > "$tmp/unit.ll"
if clang_ir "$tmp/unit.ll" "$tmp/unit" "$tmp/unit-clang.log"; then
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
if clang_ir "$tmp/string.ll" "$tmp/string" "$tmp/string-clang.log"; then
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
echo "$out" | grep -q "call ptr @nova_list_at" && ok || no "list bounds runtime call"
printf '%s\n' "$out" > "$tmp/list.ll"
if clang_ir "$tmp/list.ll" "$tmp/list" "$tmp/list-clang.log"; then
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
if clang_ir "$tmp/list-abi.ll" "$tmp/list-abi" "$tmp/list-abi-clang.log"; then
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

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list-loop.nova" "$tool")
echo "$out" | grep -q "call void @nova_list_push" && ok || no "looped list push call"
echo "$out" | grep -q "call ptr @nova_list_at" && ok || no "looped list index call"
printf '%s\n' "$out" > "$tmp/list-loop.ll"
if clang_ir "$tmp/list-loop.ll" "$tmp/list-loop" "$tmp/list-loop-clang.log"; then
    ok
else
    no "clang rejects looped list IR"
    cat "$tmp/list-loop-clang.log"
fi
if [ -x "$tmp/list-loop" ]; then
    "$tmp/list-loop"
    code=$?
    [ "$code" -eq 10 ] && ok || no "looped list result (got $code, expected 10)"
else
    no "looped list binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list-push.nova" "$tool")
echo "$out" | grep -q "declare void @nova_list_push(ptr, ptr, i64)" && ok || no "list push declaration"
echo "$out" | grep -q "call void @nova_list_push" && ok || no "list push call"
echo "$out" | grep -q "alloca i64" && ok || no "pushed element slot"
echo "$out" | grep -q "store i64 12, ptr" && ok || no "pushed element store"
printf '%s\n' "$out" > "$tmp/list-push.ll"
if clang_ir "$tmp/list-push.ll" "$tmp/list-push" "$tmp/list-push-clang.log"; then
    ok
else
    no "clang rejects list push IR"
    cat "$tmp/list-push-clang.log"
fi
if [ -x "$tmp/list-push" ]; then
    "$tmp/list-push"
    code=$?
    [ "$code" -eq 16 ] && ok || no "list push result (got $code, expected 16)"
else
    no "list push binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list-set.nova" "$tool")
echo "$out" | grep -q "store i64 40, ptr" && ok || no "list element assignment store"
echo "$out" | grep -q "call ptr @nova_list_at" && ok || no "list assignment bounds runtime call"
printf '%s\n' "$out" > "$tmp/list-set.ll"
if clang_ir "$tmp/list-set.ll" "$tmp/list-set" "$tmp/list-set-clang.log"; then
    ok
else
    no "clang rejects list assignment IR"
    cat "$tmp/list-set-clang.log"
fi
if [ -x "$tmp/list-set" ]; then
    "$tmp/list-set"
    code=$?
    [ "$code" -eq 45 ] && ok || no "list assignment result (got $code, expected 45)"
else
    no "list assignment binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-match.nova" "$tool")
echo "$out" | grep -q "phi i64" && ok || no "match expression phi"
printf '%s\n' "$out" > "$tmp/match.ll"
if clang_ir "$tmp/match.ll" "$tmp/match" "$tmp/match-clang.log"; then
    ok
else
    no "clang rejects match IR"
    cat "$tmp/match-clang.log"
fi
if [ -x "$tmp/match" ]; then
    "$tmp/match"
    code=$?
    [ "$code" -eq 60 ] && ok || no "match result (got $code, expected 60)"
else
    no "match binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-match-unit.nova" "$tool")
echo "$out" | grep -q "icmp eq i1" && ok || no "Bool match pattern"
printf '%s\n' "$out" > "$tmp/match-unit.ll"
if clang_ir "$tmp/match-unit.ll" "$tmp/match-unit" "$tmp/match-unit-clang.log"; then
    ok
else
    no "clang rejects Unit match IR"
    cat "$tmp/match-unit-clang.log"
fi
if [ -x "$tmp/match-unit" ]; then
    "$tmp/match-unit"
    code=$?
    [ "$code" -eq 9 ] && ok || no "Unit match result (got $code, expected 9)"
else
    no "Unit match binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-match-fail.nova" "$tool")
echo "$out" | grep -q "call void @nova_panic_str(%nova.string" && ok || no "unmatched match panic call"
printf '%s\n' "$out" > "$tmp/match-fail.ll"
if clang_ir "$tmp/match-fail.ll" "$tmp/match-fail" "$tmp/match-fail-clang.log"; then
    ok
else
    no "clang rejects unmatched match IR"
    cat "$tmp/match-fail-clang.log"
fi
if [ -x "$tmp/match-fail" ]; then
    "$tmp/match-fail" > "$tmp/match-fail.out" 2> "$tmp/match-fail.err"
    code=$?
    [ "$code" -eq 1 ] && ok || no "unmatched match exit (got $code, expected 1)"
    grep -q "non-exhaustive match" "$tmp/match-fail.err" && ok || no "unmatched match panic message"
else
    no "unmatched match binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-for.nova" "$tool")
echo "$out" | grep -q "icmp slt i64" && ok || no "range loop condition"
printf '%s\n' "$out" > "$tmp/for.ll"
if clang_ir "$tmp/for.ll" "$tmp/for" "$tmp/for-clang.log"; then
    ok
else
    no "clang rejects range-for IR"
    cat "$tmp/for-clang.log"
fi
if [ -x "$tmp/for" ]; then
    "$tmp/for"
    code=$?
    [ "$code" -eq 8 ] && ok || no "range-for result (got $code, expected 8)"
else
    no "range-for binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-for-nested.nova" "$tool")
echo "$out" | grep -q "icmp slt i64" && ok || no "nested loop condition"
printf '%s\n' "$out" > "$tmp/for-nested.ll"
if clang_ir "$tmp/for-nested.ll" "$tmp/for-nested" "$tmp/for-nested-clang.log"; then
    ok
else
    no "clang rejects nested-for IR"
    cat "$tmp/for-nested-clang.log"
fi
if [ -x "$tmp/for-nested" ]; then
    "$tmp/for-nested"
    code=$?
    [ "$code" -eq 30 ] && ok || no "nested-for result (got $code, expected 30)"
else
    no "nested-for binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-for-list.nova" "$tool")
echo "$out" | grep -q "call ptr @nova_list_at" && ok || no "list loop element access"
printf '%s\n' "$out" > "$tmp/for-list.ll"
if clang_ir "$tmp/for-list.ll" "$tmp/for-list" "$tmp/for-list-clang.log"; then
    ok
else
    no "clang rejects list-for IR"
    cat "$tmp/for-list-clang.log"
fi
if [ -x "$tmp/for-list" ]; then
    "$tmp/for-list"
    code=$?
    [ "$code" -eq 30 ] && ok || no "list-for result (got $code, expected 30)"
else
    no "list-for binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-struct.nova" "$tool")
echo "$out" | grep -q "%nova.struct.Point = type { i64, i64 }" && ok || no "struct type declaration"
echo "$out" | grep -q "%nova.struct.Pair = type { %nova.struct.Point, %nova.struct.Point }" && ok || no "nested struct type"
echo "$out" | grep -q "insertvalue %nova.struct.Point" && ok || no "struct literal construction"
echo "$out" | grep -q "extractvalue %nova.struct.Point" && ok || no "struct field read"
echo "$out" | grep -q "store %nova.struct.Point" && ok || no "struct field assignment"
echo "$out" | grep -q "define %nova.struct.Point @nova_user_0(ptr %arg_env, %nova.struct.Point %p0, i64 %p1)" && ok || no "struct parameter and return signature"
echo "$out" | grep -q "define i64 @nova_user_1(ptr %arg_env, %nova.struct.Pair %p0)" && ok || no "nested struct parameter signature"
printf '%s\n' "$out" > "$tmp/struct.ll"
if clang_ir "$tmp/struct.ll" "$tmp/struct" "$tmp/struct-clang.log"; then
    ok
else
    no "clang rejects struct IR"
    cat "$tmp/struct-clang.log"
fi
if [ -x "$tmp/struct" ]; then
    "$tmp/struct"
    code=$?
    [ "$code" -eq 14 ] && ok || no "struct result (got $code, expected 14)"
else
    no "struct binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-enum.nova" "$tool")
echo "$out" | grep -q "%nova.enum.Option = type { i32, i64 }" && ok || no "enum type declaration"
echo "$out" | grep -q "%nova.enum.Shape = type { i32, i64, i64 }" && ok || no "multi-payload enum type"
echo "$out" | grep -q "insertvalue %nova.enum.Option" && ok || no "enum variant construction"
echo "$out" | grep -q "icmp eq i32" && ok || no "enum tag comparison"
echo "$out" | grep -q "extractvalue %nova.enum.Option" && ok || no "enum payload binding"
echo "$out" | grep -q "define i64 @nova_user_0(ptr %arg_env, %nova.enum.Option %p0, i64 %p1)" && ok || no "enum parameter signature"
echo "$out" | grep -q "define %nova.enum.Option @nova_user_1(ptr %arg_env, i64 %p0)" && ok || no "enum return signature"
printf '%s\n' "$out" > "$tmp/enum.ll"
if clang_ir "$tmp/enum.ll" "$tmp/enum" "$tmp/enum-clang.log"; then
    ok
else
    no "clang rejects enum IR"
    cat "$tmp/enum-clang.log"
fi
if [ -x "$tmp/enum" ]; then
    "$tmp/enum"
    code=$?
    [ "$code" -eq 36 ] && ok || no "enum result (got $code, expected 36)"
else
    no "enum binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list-reverse.nova" "$tool")
echo "$out" | grep -q "call ptr @nova_list_at" && ok || no "list reverse uses nova_list_at"
echo "$out" | grep -q "call void @nova_list_push" && ok || no "list reverse uses nova_list_push"
printf '%s\n' "$out" > "$tmp/list-reverse.ll"
if clang_ir "$tmp/list-reverse.ll" "$tmp/list-reverse" "$tmp/list-reverse-clang.log"; then
    ok
else
    no "clang rejects list reverse IR"
    cat "$tmp/list-reverse-clang.log"
fi
if [ -x "$tmp/list-reverse" ]; then
    "$tmp/list-reverse"
    code=$?
    [ "$code" -eq 31 ] && ok || no "list reverse result (got $code, expected 31)"
else
    no "list reverse binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list-contains.nova" "$tool")
echo "$out" | grep -q "call ptr @nova_list_at" && ok || no "list contains uses nova_list_at"
printf '%s\n' "$out" > "$tmp/list-contains.ll"
if clang_ir "$tmp/list-contains.ll" "$tmp/list-contains" "$tmp/list-contains-clang.log"; then
    ok
else
    no "clang rejects list contains IR"
    cat "$tmp/list-contains-clang.log"
fi
if [ -x "$tmp/list-contains" ]; then
    "$tmp/list-contains"
    code=$?
    [ "$code" -eq 16 ] && ok || no "list contains result (got $code, expected 16)"
else
    no "list contains binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list-ends.nova" "$tool")
echo "$out" | grep -q "call ptr @nova_list_at" && ok || no "list first/last use nova_list_at"
printf '%s\n' "$out" > "$tmp/list-ends.ll"
if clang_ir "$tmp/list-ends.ll" "$tmp/list-ends" "$tmp/list-ends-clang.log"; then
    ok
else
    no "clang rejects list first/last IR"
    cat "$tmp/list-ends-clang.log"
fi
if [ -x "$tmp/list-ends" ]; then
    "$tmp/list-ends"
    code=$?
    [ "$code" -eq 49 ] && ok || no "list first/last result (got $code, expected 49)"
else
    no "list first/last binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-string-upper.nova" "$tool")
echo "$out" | grep -q "call %nova.string @nova_string_upper" && ok || no "String upper call"
echo "$out" | grep -q "call void @nova_string_bytes" && ok || no "String upper bytes call"
printf '%s\n' "$out" > "$tmp/string-upper.ll"
if clang_ir "$tmp/string-upper.ll" "$tmp/string-upper" "$tmp/string-upper-clang.log"; then
    ok
else
    no "clang rejects String upper IR"
    cat "$tmp/string-upper-clang.log"
fi
if [ -x "$tmp/string-upper" ]; then
    "$tmp/string-upper"
    code=$?
    [ "$code" -eq 72 ] && ok || no "String upper result (got $code, expected 72)"
else
    no "String upper binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-string-bytes.nova" "$tool")
echo "$out" | grep -q "call void @nova_string_bytes" && ok || no "String bytes call"
echo "$out" | grep -q "icmp eq i64" && ok || no "String is_empty check"
printf '%s\n' "$out" > "$tmp/string-bytes.ll"
if clang_ir "$tmp/string-bytes.ll" "$tmp/string-bytes" "$tmp/string-bytes-clang.log"; then
    ok
else
    no "clang rejects String bytes IR"
    cat "$tmp/string-bytes-clang.log"
fi
if [ -x "$tmp/string-bytes" ]; then
    "$tmp/string-bytes"
    code=$?
    [ "$code" -eq 95 ] && ok || no "String bytes result (got $code, expected 95)"
else
    no "String bytes binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-string-transform.nova" "$tool")
echo "$out" | grep -q "call %nova.string @nova_string_replace" && ok || no "String replace call"
echo "$out" | grep -q "call %nova.string @nova_string_lower" && ok || no "String lower call"
printf '%s\n' "$out" > "$tmp/string-transform.ll"
if clang_ir "$tmp/string-transform.ll" "$tmp/string-transform" "$tmp/string-transform-clang.log"; then
    ok
else
    no "clang rejects String transform IR"
    cat "$tmp/string-transform-clang.log"
fi
if [ -x "$tmp/string-transform" ]; then
    "$tmp/string-transform"
    code=$?
    [ "$code" -eq 16 ] && ok || no "String transform result (got $code, expected 16)"
else
    no "String transform binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list-slice.nova" "$tool")
echo "$out" | grep -q "call ptr @nova_list_at" && ok || no "slice reads through nova_list_at"
echo "$out" | grep -q "call void @nova_list_push" && ok || no "slice pushes through nova_list_push"
printf '%s\n' "$out" > "$tmp/list-slice.ll"
if clang_ir "$tmp/list-slice.ll" "$tmp/list-slice" "$tmp/list-slice-clang.log"; then
    ok
else
    no "clang rejects list slice IR"
    cat "$tmp/list-slice-clang.log"
fi
if [ -x "$tmp/list-slice" ]; then
    "$tmp/list-slice"
    code=$?
    [ "$code" -eq 10 ] && ok || no "list slice result (got $code, expected 10)"
else
    no "list slice binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-enum-string.nova" "$tool")
echo "$out" | grep -q "%nova.enum.Message = type { i32, i64, i64, i64, i64 }" && ok || no "String enum slot layout"
echo "$out" | grep -q "extractvalue %nova.string" && ok || no "String enum payload packing"
printf '%s\n' "$out" > "$tmp/enum-string.ll"
if clang_ir "$tmp/enum-string.ll" "$tmp/enum-string" "$tmp/enum-string-clang.log"; then
    ok
else
    no "clang rejects String enum IR"
    cat "$tmp/enum-string-clang.log"
fi
if [ -x "$tmp/enum-string" ]; then
    "$tmp/enum-string"
    code=$?
    [ "$code" -eq 10 ] && ok || no "String enum result (got $code, expected 10)"
else
    no "String enum binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-enum-bool.nova" "$tool")
echo "$out" | grep -q "%nova.enum.Flag = type { i32, i64, i64 }" && ok || no "Bool enum type"
echo "$out" | grep -q "zext i1" && ok || no "Bool enum payload construction"
echo "$out" | grep -q "trunc i64" && ok || no "Bool enum payload binding"
printf '%s\n' "$out" > "$tmp/enum-bool.ll"
if clang_ir "$tmp/enum-bool.ll" "$tmp/enum-bool" "$tmp/enum-bool-clang.log"; then
    ok
else
    no "clang rejects Bool enum IR"
    cat "$tmp/enum-bool-clang.log"
fi
if [ -x "$tmp/enum-bool" ]; then
    "$tmp/enum-bool"
    code=$?
    [ "$code" -eq 7 ] && ok || no "Bool enum result (got $code, expected 7)"
else
    no "Bool enum binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-string-methods.nova" "$tool")
echo "$out" | grep -q "call %nova.string @nova_string_trim" && ok || no "String trim call"
echo "$out" | grep -q "call i32 @nova_string_starts_with" && ok || no "String starts_with call"
echo "$out" | grep -q "call i32 @nova_string_ends_with" && ok || no "String ends_with call"
echo "$out" | grep -q "call i32 @nova_string_contains" && ok || no "String contains call"
printf '%s\n' "$out" > "$tmp/string-methods.ll"
if clang_ir "$tmp/string-methods.ll" "$tmp/string-methods" "$tmp/string-methods-clang.log"; then
    ok
else
    no "clang rejects String method IR"
    cat "$tmp/string-methods-clang.log"
fi
if [ -x "$tmp/string-methods" ]; then
    "$tmp/string-methods"
    code=$?
    [ "$code" -eq 5 ] && ok || no "String methods result (got $code, expected 5)"
else
    no "String methods binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list-concat.nova" "$tool")
echo "$out" | grep -q "call ptr @nova_list_at" && ok || no "list concat read call"
echo "$out" | grep -q "call void @nova_list_push" && ok || no "list concat push call"
printf '%s\n' "$out" > "$tmp/list-concat.ll"
if clang_ir "$tmp/list-concat.ll" "$tmp/list-concat" "$tmp/list-concat-clang.log"; then
    ok
else
    no "clang rejects list concat IR"
    cat "$tmp/list-concat-clang.log"
fi
if [ -x "$tmp/list-concat" ]; then
    "$tmp/list-concat"
    code=$?
    [ "$code" -eq 11 ] && ok || no "list concat result (got $code, expected 11)"
else
    no "list concat binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-list-eq.nova" "$tool")
echo "$out" | grep -q "phi i1" && ok || no "list equality phi"
printf '%s\n' "$out" > "$tmp/list-eq.ll"
if clang_ir "$tmp/list-eq.ll" "$tmp/list-eq" "$tmp/list-eq-clang.log"; then
    ok
else
    no "clang rejects list equality IR"
    cat "$tmp/list-eq-clang.log"
fi
if [ -x "$tmp/list-eq" ]; then
    "$tmp/list-eq"
    code=$?
    [ "$code" -eq 12 ] && ok || no "list equality result (got $code, expected 12)"
else
    no "list equality binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-assert.nova" "$tool")
echo "$out" | grep -q "call void @nova_assert_fail" && ok || no "assert failure call"
printf '%s\n' "$out" > "$tmp/assert.ll"
if clang_ir "$tmp/assert.ll" "$tmp/assert" "$tmp/assert-clang.log"; then
    ok
else
    no "clang rejects assert IR"
    cat "$tmp/assert-clang.log"
fi
if [ -x "$tmp/assert" ]; then
    "$tmp/assert"
    code=$?
    [ "$code" -eq 5 ] && ok || no "assert result (got $code, expected 5)"
else
    no "assert binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-assert-fail.nova" "$tool")
printf '%s\n' "$out" > "$tmp/assert-fail.ll"
if clang_ir "$tmp/assert-fail.ll" "$tmp/assert-fail" "$tmp/assert-fail-clang.log"; then
    ok
else
    no "clang rejects failing assert IR"
    cat "$tmp/assert-fail-clang.log"
fi
if [ -x "$tmp/assert-fail" ]; then
    "$tmp/assert-fail" > "$tmp/assert-fail.out" 2> "$tmp/assert-fail.err"
    code=$?
    [ "$code" -eq 1 ] && ok || no "assert failure exit (got $code, expected 1)"
    grep -q "assertion failed" "$tmp/assert-fail.err" && ok || no "assert failure message"
else
    no "failing assert binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-print.nova" "$tool")
echo "$out" | grep -q "call void @nova_print_str" && ok || no "println String call"
echo "$out" | grep -q "call void @nova_print_i64" && ok || no "print Int call"
echo "$out" | grep -q "call void @nova_print_bool" && ok || no "println Bool call"
printf '%s\n' "$out" > "$tmp/print.ll"
if clang_ir "$tmp/print.ll" "$tmp/print" "$tmp/print-clang.log"; then
    ok
else
    no "clang rejects print IR"
    cat "$tmp/print-clang.log"
fi
if [ -x "$tmp/print" ]; then
    "$tmp/print" > "$tmp/print.out"
    code=$?
    printf 'hello\n12true\n' > "$tmp/print.expected"
    if cmp -s "$tmp/print.out" "$tmp/print.expected"; then ok; else no "print output"; fi
    [ "$code" -eq 7 ] && ok || no "print exit (got $code, expected 7)"
else
    no "print binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-string-match.nova" "$tool")
echo "$out" | grep -q "call %nova.string @nova_string_concat" && ok || no "string concatenation call"
echo "$out" | grep -q "call i32 @nova_string_eq" && ok || no "string equality call"
echo "$out" | grep -q "icmp ne i32" && ok || no "string equality comparison"
echo "$out" | grep -q "define i64 @nova_user_0(ptr %arg_env, %nova.string %p0)" && ok || no "String match parameter signature"
printf '%s\n' "$out" > "$tmp/string-match.ll"
if clang_ir "$tmp/string-match.ll" "$tmp/string-match" "$tmp/string-match-clang.log"; then
    ok
else
    no "clang rejects String match IR"
    cat "$tmp/string-match-clang.log"
fi
if [ -x "$tmp/string-match" ]; then
    "$tmp/string-match"
    code=$?
    [ "$code" -eq 13 ] && ok || no "String match result (got $code, expected 13)"
else
    no "String match binary missing"
fi

out=$(NOVA_TOOL=emit-ir NOVA_TOOL_PATH="$root/examples/nova-tool/tests/codegen-unit-main.nova" "$tool")
echo "$out" | grep -q "define void @nova_user_0(ptr %arg_env)" && ok || no "Unit main signature"
echo "$out" | grep -q "call void @nova_user_0(ptr null)" && ok || no "Unit main call"
printf '%s\n' "$out" > "$tmp/unit-main.ll"
if clang_ir "$tmp/unit-main.ll" "$tmp/unit-main" "$tmp/unit-main-clang.log"; then
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
