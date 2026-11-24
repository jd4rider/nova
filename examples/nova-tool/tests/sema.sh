#!/bin/sh
# Declaration-level semantic checks.
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

cat > "$tmp/dup.nova" <<'NOVA'
fn greet(a: Int, a: Int) -> Int {
    return a
}

fn greet() {
}
NOVA
out=$(NOVA_TOOL=sema NOVA_TOOL_PATH="$tmp/dup.nova" "$tool")
echo "$out" | grep -q "E0200 duplicate \`greet\`" && ok || no "duplicate function"
echo "$out" | grep -q "E0201 duplicate parameter \`a\`" && ok || no "duplicate parameter"

cat > "$tmp/types.nova" <<'NOVA'
struct User {
    name: String
}

fn pick(user: User) -> Missing {
    return user
}
NOVA
out=$(NOVA_TOOL=sema NOVA_TOOL_PATH="$tmp/types.nova" "$tool")
echo "$out" | grep -q "E0202 unknown type \`Missing\`" && ok || no "unknown return type"

printf 'struct User {\n    name: String\n}\n\nfn pick(user: User) -> User {\n    return user\n}\n' > "$tmp/ok.nova"
NOVA_TOOL=sema NOVA_TOOL_PATH="$tmp/ok.nova" "$tool" | grep -q "^OK " && ok || no "clean file"

for file in "$root"/libraries/*/src/*.nova; do
    NOVA_TOOL=sema NOVA_TOOL_PATH="$file" "$tool" >/dev/null 2>&1 || no "sema $(basename "$file")"
done

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
