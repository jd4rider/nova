#!/bin/sh
# Protocol test for the Nova language server built into nova-tool.
set -eu
root=$(cd "$(dirname "$0")/../../.." && pwd)
tool="${1:-$root/examples/nova-tool/target/nova/nova-tool}"

[ -x "$tool" ] || { echo "build examples/nova-tool first"; exit 1; }

python3 - "$tool" <<'PY'
import json, subprocess, sys

tool = sys.argv[1]
uri = "file:///tmp/nova-lsp-test.nova"
bad = 'fn main() {\n    let x: Int = "no"\n}\n'
good = 'fn helper() {}\nfn main() {\n    let x: Int = 1\n}\n'

def frame(message):
    body = json.dumps(message).encode()
    return b"Content-Length: " + str(len(body)).encode() + b"\r\n\r\n" + body

messages = [
    {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}},
    {"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
        "textDocument": {"uri": uri, "languageId": "nova", "version": 1, "text": bad}}},
    {"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": uri, "version": 2},
        "contentChanges": [{"text": good}]}},
    {"jsonrpc": "2.0", "id": 2, "method": "textDocument/completion", "params": {
        "textDocument": {"uri": uri}, "position": {"line": 2, "character": 4}}},
    {"jsonrpc": "2.0", "method": "textDocument/didClose", "params": {
        "textDocument": {"uri": uri}}},
    {"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": None},
    {"jsonrpc": "2.0", "method": "exit"},
]

result = subprocess.run([tool, "lsp"], input=b"".join(frame(m) for m in messages),
                        capture_output=True, timeout=30)
text = result.stdout.decode("utf-8", "replace")

def messages_from(text):
    out = []
    while text:
        head, sep, rest = text.partition("\r\n\r\n")
        if not sep:
            break
        length = None
        for line in head.split("\r\n"):
            if line.lower().startswith("content-length:"):
                length = int(line.split(":", 1)[1].strip())
        if length is None or len(rest) < length:
            break
        out.append(json.loads(rest[:length]))
        text = rest[length:]
    return out

parsed = messages_from(text)
if result.returncode != 0:
    print("nova-tool lsp exited", result.returncode)
    print(text)
    sys.exit(1)
initialize = next((m for m in parsed if m.get("id") == 1), None)
if initialize is None or initialize.get("result", {}).get("capabilities", {}).get("textDocumentSync") != 1:
    print("missing initialize capabilities:", parsed)
    sys.exit(1)
if initialize["result"]["capabilities"].get("completionProvider", {}).get("triggerCharacters") != [".", "$"]:
    print("missing completion capability:", initialize)
    sys.exit(1)
published = [m for m in parsed if m.get("method") == "textDocument/publishDiagnostics"]
if len(published) < 3:
    print("expected diagnostics for open, change and close:", parsed)
    sys.exit(1)
first = published[0]["params"]["diagnostics"]
if not any(d.get("code") == "E0301" for d in first):
    print("expected E0301 on the invalid buffer:", first)
    sys.exit(1)
if published[1]["params"]["diagnostics"]:
    print("expected no diagnostics after the fix:", published[1]["params"]["diagnostics"])
    sys.exit(1)
if published[2]["params"]["diagnostics"]:
    print("expected diagnostics cleared on close:", published[2]["params"]["diagnostics"])
    sys.exit(1)
completion = next((m for m in parsed if m.get("id") == 2), None)
if completion is None:
    print("missing completion response:", parsed)
    sys.exit(1)
labels = {item.get("label") for item in completion.get("result", {}).get("items", [])}
for wanted in ("helper", "main", "fn", "println"):
    if wanted not in labels:
        print("completion missing", wanted, "in", sorted(labels))
        sys.exit(1)
print("nova-lsp diagnostics and completion ok")
PY
