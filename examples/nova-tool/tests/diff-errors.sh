#!/bin/sh
# DIFF-01.02: golden invalid-program diagnostics for the Nova frontend.
#
# Every fixture is rejected by the Nova frontend via `NOVA_TOOL=sema-ast`.
# Output is normalized to `CODE|message|line:column`, one diagnostic per line,
# and compared with the Rust reference's recorded golden passed to
# `check_case` (the third argument kept for the old table is unused). Changing
# Nova's diagnostics fails this suite until the golden is updated deliberately.
set -eu
dir=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$dir/../../.." && pwd)
tool=${NOVA_TOOL_BIN:-"$dir/../target/nova/nova-tool"}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
passed=0
failed=0

[ -x "$tool" ] || { echo "diff-errors: build examples/nova-tool first" >&2; exit 1; }

# `CODE message at line L, column C`.
norm_nova() {
    sed -n 's/^\(E[0-9][0-9]*\) \(.*\) at line \([0-9][0-9]*\), column \([0-9][0-9]*\)$/\1|\2|\3:\4/p'
}

expect() {
    if [ -n "$2" ]; then
        printf '%b\n' "$2" > "$1"
    else
        : > "$1"
    fi
}

# check_case NAME GOLDEN [UNUSED], fixture on stdin.
check_case() {
    name=$1
    src="$tmp/$name.nova"
    cat > "$src"
    expect "$tmp/$name.expected" "$2"
    if [ ! -s "$tmp/$name.expected" ]; then
        failed=$((failed + 1))
        echo "FAIL $name: empty diagnostic golden"
        return
    fi

    nova_status=0
    NOVA_TOOL=sema-ast NOVA_TOOL_PATH="$src" "$tool" > "$tmp/$name.nova.raw" 2>&1 || nova_status=$?
    norm_nova < "$tmp/$name.nova.raw" > "$tmp/$name.nova.actual"

    if ! cmp -s "$tmp/$name.expected" "$tmp/$name.nova.actual"; then
        failed=$((failed + 1))
        echo "FAIL $name: Nova diagnostics changed"
        diff -u "$tmp/$name.expected" "$tmp/$name.nova.actual" || true
        return
    fi
    [ "$nova_status" -eq 0 ] || {
        failed=$((failed + 1))
        echo "FAIL $name: nova-tool sema-ast exited $nova_status"
        return
    }
    passed=$((passed + 1))
}

check_case type_binding \
    'E0308|type mismatch: expected `Int`, found `String`|2:18' \
    'E0308|type mismatch: expected `Int`, found `String`|2:18' <<'NOVA'
fn main() {
    let x: Int = "bad"
}
NOVA

check_case return_type \
    'E0308|type mismatch: expected `Int`, found `String`|2:12' \
    'E0308|type mismatch: expected `Int`, found `String`|2:12' <<'NOVA'
fn f() -> Int {
    return "x"
}

fn main() {
}
NOVA

check_case unknown_name \
    'E0203|unknown name `y`|2:13' \
    'E0203|unknown name `y`|2:13' <<'NOVA'
fn main() {
    let x = y
}
NOVA

check_case unknown_function \
    'E0203|unknown function `missing`|2:13' \
    'E0203|unknown function `missing`|2:13' <<'NOVA'
fn main() {
    let x = missing(1)
}
NOVA

check_case call_arity \
    'E0208|`add` expects 2 arguments, found 1|6:13' \
    'E0208|`add` expects 2 arguments, found 1|6:13' <<'NOVA'
fn add(a: Int, b: Int) -> Int {
    return a + b
}

fn main() {
    let y = add(1)
}
NOVA

check_case unknown_field \
    'E0207|struct `P` has no field `z`|6:17\nE0207|missing field `x` in `P` literal|6:13' \
    'E0207|struct `P` has no field `z`|6:17\nE0207|missing field `x` in `P` literal|6:13' <<'NOVA'
struct P {
    x: Int
}

fn main() {
    let p = P { z: 1 }
}
NOVA

check_case missing_field \
    'E0207|missing field `y` in `P` literal|7:13' \
    'E0207|missing field `y` in `P` literal|7:13' <<'NOVA'
struct P {
    x: Int
    y: Int
}

fn main() {
    let p = P { x: 1 }
}
NOVA

check_case break_outside_loop \
    'E0302|`break` outside of a loop|2:5' \
    'E0302|`break` outside of a loop|2:5' <<'NOVA'
fn main() {
    break
}
NOVA

check_case if_condition \
    'E0308|type mismatch: expected `Bool`, found `Int`|2:8' \
    'E0308|type mismatch: expected `Bool`, found `Int`|2:8' <<'NOVA'
fn main() {
    if 1 { }
}
NOVA

check_case non_exhaustive_match \
    'E0310|non-exhaustive `match` on `Int`|2:13' \
    'E0310|non-exhaustive `match` on `Int`|2:13' <<'NOVA'
fn main() {
    let x = match 1 {
        1 => 1
    }
}
NOVA

check_case duplicate_bool_arm \
    'E0310|non-exhaustive `match` on `Bool`|2:13' \
    'E0310|non-exhaustive `match` on `Bool`|2:13' <<'NOVA'
fn main() {
    let x = match true {
        true => 1
        true => 2
    }
}
NOVA

check_case immutable_assign \
    'E0301|cannot assign to immutable binding `x`|3:5' \
    'E0301|cannot assign to immutable binding `x`|3:5' <<'NOVA'
fn main() {
    let x = 1
    x = 2
}
NOVA

check_case immutable_push \
    'E0301|`push` requires a mutable list binding|3:5' \
    'E0301|`push` requires a mutable list binding|3:5' <<'NOVA'
fn main() {
    let xs = [1, 2]
    xs.push(3)
}
NOVA

check_case duplicate_function \
    'E0200|duplicate function `same`|4:4' \
    'E0200|duplicate function `same`|4:4' <<'NOVA'
fn same() {
}

fn same() {
}
NOVA

check_case duplicate_parameter \
    'E0200|duplicate parameter `a`|1:14\nE0200|duplicate binding `a` in the same scope|1:14' \
    'E0200|duplicate parameter `a`|1:14\nE0200|duplicate binding `a` in the same scope|1:14' <<'NOVA'
fn f(a: Int, a: Int) {
}

fn main() {
}
NOVA

check_case int_plus_string \
    'E0308|type mismatch: expected `Int`, found `String`|2:17' \
    'E0308|type mismatch: expected `Int`, found `String`|2:17' <<'NOVA'
fn main() {
    let x = 1 + "a"
}
NOVA

check_case index_type \
    'E0308|type mismatch: expected `Int`, found `String`|3:16' \
    'E0308|type mismatch: expected `Int`, found `String`|3:16' <<'NOVA'
fn main() {
    let xs = [1, 2]
    let n = xs["a"]
}
NOVA

check_case call_non_function \
    'E0205|only functions can be called, found `Int`|3:5' \
    'E0205|only functions can be called, found `Int`|3:5' <<'NOVA'
fn main() {
    let x = 1
    x()
}
NOVA

check_case parse_parameter \
    'E0103|expected an identifier for a parameter name, found `{`|1:10\nE0103|expected `)` to close the parameter list, found `{`|1:10' \
    'E0103|expected an identifier for a parameter name, found `{`|1:10\nE0103|expected `)` to close the parameter list, found `{`|1:10' <<'NOVA'
fn main( {
}
NOVA

check_case option_binding \
    'E0308|expected `Int`, found `Option<_>`|2:18' \
    'E0308|expected `Int`, found `Option<_>`|2:18' <<'NOVA'
fn main() {
    let x: Int = None
}
NOVA

check_case option_none_inference \
    'E0210|cannot infer the type parameter of `None`|2:13' \
    'E0210|cannot infer the type parameter of `None`|2:13' <<'NOVA'
fn main() {
    let x = None
}
NOVA

check_case option_some_arity \
    'E0208|`Some` expects 1 argument, found 2|2:13' \
    'E0208|`Some` expects 1 argument, found 2|2:13' <<'NOVA'
fn main() {
    let x = Some(1, 2)
}
NOVA

check_case option_match_exhaustive \
    'E0310|non-exhaustive `match` on `Option<Int>`|3:5' \
    'E0310|non-exhaustive `match` on `Option<Int>`|3:5' <<'NOVA'
fn main() {
    let x = Some(1)
    match x {
        Some(v) => println(v)
    }
}
NOVA

check_case option_none_payload \
    'E0311|`None` does not take a payload pattern|5:9' \
    'E0311|`None` does not take a payload pattern|5:9' <<'NOVA'
fn main() {
    let x = Some(1)
    match x {
        Some(v) => println(v)
        None(v) => println(0)
    }
}
NOVA

check_case result_match_exhaustive \
    'E0310|non-exhaustive `match` on `Result<Int, String>`|6:5' \
    'E0310|non-exhaustive `match` on `Result<Int, String>`|6:5' <<'NOVA'
fn get() -> Result<Int, String> {
    return Ok(1)
}
fn main() {
    let x = get()
    match x {
        Ok(v) => println(v)
    }
}
NOVA

check_case array_length \
    'E0308|array length mismatch: expected 3 element(s), found 2|2:23' \
    'E0308|array length mismatch: expected 3 element(s), found 2|2:23' <<'NOVA'
fn main() {
    let a: [Int; 3] = [1, 2]
}
NOVA

check_case array_element_type \
    'E0308|type mismatch: expected `Int`, found `String`|2:27' \
    'E0308|type mismatch: expected `Int`, found `String`|2:27' <<'NOVA'
fn main() {
    let a: [Int; 2] = [1, "x"]
}
NOVA

echo "diagnostics: $passed fixtures matched the recorded reference, $failed mismatched"
[ "$failed" -eq 0 ]
