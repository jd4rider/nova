#!/bin/sh
# Body-level semantic checks over the Nova AST.
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

cat > "$tmp/ok.nova" <<'NOVA'
struct Point {
    x: Int,
    y: Int,
}

fn add(a: Int, b: Int) -> Int {
    return a + b
}

fn main() {
    let p = Point { x: 1, y: 2 }
    let total = add(p.x, p.y)
    println(total)
}
NOVA
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/ok.nova" "$tool")
echo "$out" | grep -q "^OK " && ok || no "clean program"

printf 'fn main() {\n    let x: Int = "bad"\n}\n' > "$tmp/type.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/type.nova" "$tool")
echo "$out" | grep -q "E0301" && ok || no "type mismatch"

printf 'fn add(a: Int, b: Int) -> Int {\n    return a + b\n}\n\nfn main() {\n    let y = add(1)\n}\n' > "$tmp/arity.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/arity.nova" "$tool")
echo "$out" | grep -q "E0303" && ok || no "call arity"

printf 'fn main() {\n    let x = missing(1)\n}\n' > "$tmp/unknown.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/unknown.nova" "$tool")
echo "$out" | grep -q "E0302" && ok || no "unknown function"

printf 'struct P {\n    x: Int\n}\n\nfn main() {\n    let p = P { z: 1 }\n}\n' > "$tmp/fields.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/fields.nova" "$tool")
echo "$out" | grep -q "E0305" && ok || no "unknown field"
echo "$out" | grep -q "E0306" && ok || no "missing field"

printf 'fn f() -> Int {\n    return "x"\n}\n' > "$tmp/ret.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/ret.nova" "$tool")
echo "$out" | grep -q "E0308" && ok || no "return type"

printf 'fn main() {\n    break\n}\n' > "$tmp/break.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/break.nova" "$tool")
echo "$out" | grep -q "E0310" && ok || no "break outside loop"

printf 'fn same() {\n}\n\nfn same() {\n}\n' > "$tmp/dup.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/dup.nova" "$tool")
echo "$out" | grep -q "E0307" && ok || no "duplicate declaration"

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
