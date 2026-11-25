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

cat > "$tmp/match-ok.nova" <<'NOVA'
fn classify(n: Int) -> Int {
    return match n {
        1 => 10
        _ => 20
    }
}

fn main() {
    let b = match true {
        true => 1
        false => 2
    }
}
NOVA
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/match-ok.nova" "$tool")
echo "$out" | grep -q "^OK " && ok || no "exhaustive match"

printf 'fn main() {\n    let x = match 1 {\n        1 => 1\n        1 => 2\n        _ => 3\n    }\n}\n' > "$tmp/match-dup.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/match-dup.nova" "$tool")
echo "$out" | grep -q "E0312" && ok || no "duplicate match arm"

printf 'fn main() {\n    let x = match 1 {\n        n => 2\n        _ => 3\n    }\n}\n' > "$tmp/match-bind.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/match-bind.nova" "$tool")
echo "$out" | grep -q "E0311" && ok || no "unsupported match pattern"

printf 'fn main() {\n    let x = match true {\n        true => 1\n    }\n}\n' > "$tmp/match-exhaustive.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/match-exhaustive.nova" "$tool")
echo "$out" | grep -q "E0313" && ok || no "non-exhaustive Bool match"

printf 'fn main() {\n    let x = match 1 {\n        1 => 1\n        _ => "bad"\n    }\n}\n' > "$tmp/match-types.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/match-types.nova" "$tool")
echo "$out" | grep -q "match arms must return the same type" && ok || no "match arm type"

printf 'fn main() {\n    let n = 1\n    match n {\n        _ => {\n            let x: Int = "bad"\n        }\n    }\n}\n' > "$tmp/match-block.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/match-block.nova" "$tool")
echo "$out" | grep -q "E0301" && ok || no "match block arm body"

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
