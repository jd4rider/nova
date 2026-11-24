#!/bin/sh
# Regression tests for `nova fmt` and LSP formatting, both of which run the
# formatter written in Nova through nova-tool.
set -u
root=$(cd "$(dirname "$0")/../../.." && pwd)
nova="$root/target/debug/nova"
tool="$root/examples/nova-tool/target/nova/nova-tool"
lsp="$root/target/debug/nova-lsp"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
pass=0
fail=0
ok() { pass=$((pass + 1)); }
no() { fail=$((fail + 1)); echo "FAIL $1"; }

[ -x "$nova" ] || { echo "build the nova binary first"; exit 1; }
[ -x "$tool" ] || { echo "build examples/nova-tool first"; exit 1; }
[ -x "$lsp" ] || { echo "build nova-lsp first"; exit 1; }
export NOVA_TOOL_BIN="$tool"

printf 'fn main() {\n    let x = 1\n}\n' > "$tmp/clean.nova"
"$nova" fmt --check "$tmp/clean.nova" >/dev/null 2>&1 && ok || no "check on a formatted file"

printf 'fn main() {\nlet x = 1\n}\n' > "$tmp/ugly.nova"
"$nova" fmt "$tmp/ugly.nova" >/dev/null 2>&1 && ok || no "format ran"
cmp -s "$tmp/ugly.nova" "$tmp/clean.nova" && ok || no "format output"

printf 'fn main() {\nlet y = 2\n}\n' > "$tmp/check.nova"
"$nova" fmt --check "$tmp/check.nova" >/dev/null 2>&1 && no "check accepted unformatted" || ok

NOVA_TOOL=fmt NOVA_TOOL_PATH="$tmp/clean.nova" NOVA_TOOL_OUT="$tmp/tool.out" "$tool" >/dev/null 2>&1
cmp -s "$tmp/tool.out" "$tmp/clean.nova" && ok || no "nova fmt and nova-tool disagree"

python3 - "$lsp" <<'PY' && ok || no "LSP formatting"
import json, subprocess, sys
lsp = sys.argv[1]
uri = "file:///tmp/nova-format-test.nova"
source = "fn main() {\nlet x = 1\n}\n"

def frame(message):
    body = json.dumps(message).encode()
    return b"Content-Length: " + str(len(body)).encode() + b"\r\n\r\n" + body

messages = [
    {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}},
    {"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": uri, "languageId": "nova", "version": 1, "text": source}}},
    {"jsonrpc": "2.0", "id": 2, "method": "textDocument/formatting", "params": {
        "textDocument": {"uri": uri}, "options": {"tabSize": 4, "insertSpaces": True}}},
    {"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": None},
    {"jsonrpc": "2.0", "method": "exit"},
]
result = subprocess.run([lsp], input=b"".join(frame(m) for m in messages),
                        capture_output=True, timeout=30)
text = result.stdout.decode("utf-8", "replace")
if '"newText"' not in text or "    let x = 1" not in text:
    print("unexpected LSP output:", text)
    sys.exit(1)
PY

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
