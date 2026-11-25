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

printf 'struct P {\n    x: Int\n    x: Int\n}\n\nfn main() {\n}\n' > "$tmp/dup-field.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/dup-field.nova" "$tool")
echo "$out" | grep -q "E0307" && ok || no "duplicate struct field"

printf 'struct P {\n    x: Int\n}\n\nfn main() {\n    let p = P { x: 1, x: 2 }\n}\n' > "$tmp/dup-init.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/dup-init.nova" "$tool")
echo "$out" | grep -q "E0307" && ok || no "duplicate field initializer"

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

cat > "$tmp/enum-ok.nova" <<'NOVA'
enum Color {
    Red
    Green(Int)
    Blue
}

fn value(c: Color) -> Int {
    return match c {
        Red => 0
        Green(n) => n
        Blue => 1
    }
}

fn main() {
    let c = Green(2)
    let n = value(c)
}
NOVA
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/enum-ok.nova" "$tool")
echo "$out" | grep -q "^OK " && ok || no "exhaustive enum match"

printf 'enum Color {\n    Red\n    Red\n}\n\nfn main() {\n}\n' > "$tmp/enum-dup-variant.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/enum-dup-variant.nova" "$tool")
echo "$out" | grep -q "E0307" && ok || no "duplicate enum variant"

printf 'enum Color {\n    Green(Int)\n}\n\nfn main() {\n    let c = Green\n}\n' > "$tmp/enum-arity.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/enum-arity.nova" "$tool")
echo "$out" | grep -q "E0303" && ok || no "enum constructor arity"

printf 'enum Color {\n    Red\n    Green(Int)\n}\n\nfn main() {\n    let c = Green(1)\n    let n = match c {\n        Red => 0\n    }\n}\n' > "$tmp/enum-exhaustive.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/enum-exhaustive.nova" "$tool")
echo "$out" | grep -q "E0313" && ok || no "non-exhaustive enum match"

printf 'enum Color {\n    Red\n}\n\nfn main() {\n    let c = Red\n    let n = match c {\n        Blue => 0\n        _ => 1\n    }\n}\n' > "$tmp/enum-unknown.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/enum-unknown.nova" "$tool")
echo "$out" | grep -q "E0305" && ok || no "unknown enum variant"

printf 'enum Color {\n    Red\n    Green(Int)\n}\n\nfn main() {\n    let c = Green(1)\n    let n = match c {\n        Red => 0\n        Green(s) => s + "bad"\n    }\n}\n' > "$tmp/enum-binding.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/enum-binding.nova" "$tool")
echo "$out" | grep -q "E0309" && ok || no "enum payload binding type"

printf 'enum Color {\n    Red\n    Green(Int)\n}\n\nfn main() {\n    let c = Green(1)\n    let n = match c {\n        Green(x) => x\n        Green(y) => y\n        Red => 0\n    }\n}\n' > "$tmp/enum-duplicate-arm.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/enum-duplicate-arm.nova" "$tool")
echo "$out" | grep -q "E0312" && ok || no "duplicate enum arm"

cat > "$tmp/string-match-ok.nova" <<'NOVA'
fn classify(text: String) -> Int {
    return match text {
        "zero" => 0
        "one" => 1
        _ => 2
    }
}

fn main() {
    let n = classify("zero")
}
NOVA
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/string-match-ok.nova" "$tool")
echo "$out" | grep -q "^OK " && ok || no "string match"

printf 'fn classify(text: String) -> Int {\n    return match text {\n        "zero" => 0\n    }\n}\n' > "$tmp/string-match-exhaustive.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/string-match-exhaustive.nova" "$tool")
echo "$out" | grep -q "E0313" && ok || no "non-exhaustive String match"

printf 'fn classify(text: String) -> Int {\n    return match text {\n        "zero" => 0\n        "zero" => 1\n        _ => 2\n    }\n}\n' > "$tmp/string-match-dup.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/string-match-dup.nova" "$tool")
echo "$out" | grep -q "E0312" && ok || no "duplicate String match arm"

printf 'fn classify(text: String) -> Int {\n    return match text {\n        zero => 0\n        _ => 1\n    }\n}\n' > "$tmp/string-match-bad.nova"
out=$(NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$tmp/string-match-bad.nova" "$tool")
echo "$out" | grep -q "E0311" && ok || no "non-literal String pattern"

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
