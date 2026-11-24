#!/bin/sh
# Expression parser cases: input expression -> expected prefix form.
set -u
root=$(cd "$(dirname "$0")/../../.." && pwd)
tool="$root/examples/nova-tool/target/nova/nova-tool"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
pass=0
fail=0
[ -x "$tool" ] || { echo "build examples/nova-tool first"; exit 1; }

check() {
    printf '%s\n' "$1" > "$tmp/input.nova"
    got=$(NOVA_TOOL=expr NOVA_TOOL_PATH="$tmp/input.nova" "$tool")
    if [ "$got" = "$2" ]; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        echo "FAIL '$1' -> '$got' (want '$2')"
    fi
}

check '1 + 2 * 3' '(+ 1 (* 2 3))'
check '(1 + 2) * 3' '(* (+ 1 2) 3)'
check 'a == b && c < d' '(&& (== a b) (< c d))'
check '-x + 1' '(+ (neg x) 1)'
check '!done || ok' '(|| (not done) ok)'
check 'foo(a, 1 + 2)' '(call foo a (+ 1 2))'
check 'user.name.len()' '(call (. (. user name) len))'
check 'items[0]' '(index items 0)'
check 'a.b[1].c' '(. (index (. a b) 1) c)'
check '1 + 2 3' '(+ 1 2) ?'

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
