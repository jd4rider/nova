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
symbols = (
    'fn helper(value: Int) -> Int {\n'
    '    return value + 1\n'
    '}\n'
    '\n'
    'struct Point {\n'
    '    x: Int,\n'
    '    y: Int,\n'
    '}\n'
    '\n'
    'enum Mode {\n'
    '    On,\n'
    '    Off,\n'
    '}\n'
    '\n'
    'fn main() {\n'
    '    let total: Int = helper(2)\n'
    '}\n'
)
messy = 'fn main() {\n        let x: Int = 1\n}\n'

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
    {"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": uri, "version": 3},
        "contentChanges": [{"text": symbols}]}},
    {"jsonrpc": "2.0", "id": 4, "method": "textDocument/hover", "params": {
        "textDocument": {"uri": uri}, "position": {"line": 15, "character": 23}}},
    {"jsonrpc": "2.0", "id": 5, "method": "textDocument/definition", "params": {
        "textDocument": {"uri": uri}, "position": {"line": 15, "character": 23}}},
    {"jsonrpc": "2.0", "id": 6, "method": "textDocument/documentSymbol", "params": {
        "textDocument": {"uri": uri}}},
    {"jsonrpc": "2.0", "id": 7, "method": "textDocument/hover", "params": {
        "textDocument": {"uri": uri}, "position": {"line": 5, "character": 5}}},
    {"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": uri, "version": 4},
        "contentChanges": [{"text": messy}]}},
    {"jsonrpc": "2.0", "id": 8, "method": "textDocument/formatting", "params": {
        "textDocument": {"uri": uri}, "options": {"tabSize": 4, "insertSpaces": True}}},
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
capabilities = initialize["result"]["capabilities"]
if capabilities.get("hoverProvider") is not True:
    print("missing hover capability:", capabilities)
    sys.exit(1)
if capabilities.get("definitionProvider") is not True:
    print("missing definition capability:", capabilities)
    sys.exit(1)
if capabilities.get("documentSymbolProvider") is not True:
    print("missing document symbol capability:", capabilities)
    sys.exit(1)
if capabilities.get("documentFormattingProvider") is not True:
    print("missing formatting capability:", capabilities)
    sys.exit(1)
published = [m for m in parsed if m.get("method") == "textDocument/publishDiagnostics"]
if len(published) < 4:
    print("expected diagnostics for open, change and close:", parsed)
    sys.exit(1)
first = published[0]["params"]["diagnostics"]
if not any(d.get("code") == "E0308" for d in first):
    print("expected E0308 on the invalid buffer:", first)
    sys.exit(1)
if published[1]["params"]["diagnostics"]:
    print("expected no diagnostics after the fix:", published[1]["params"]["diagnostics"])
    sys.exit(1)
if published[-1]["params"]["diagnostics"]:
    print("expected diagnostics cleared on close:", published[-1]["params"]["diagnostics"])
    sys.exit(1)
hover = next((m for m in parsed if m.get("id") == 4), None)
if hover is None:
    print("missing hover response:", parsed)
    sys.exit(1)
value = hover.get("result", {}).get("contents", {}).get("value", "")
if "fn helper(value: Int) -> Int" not in value:
    print("hover did not describe helper:", hover)
    sys.exit(1)
if hover["result"].get("range", {}).get("start", {}).get("line") != 15:
    print("hover range did not cover the usage:", hover)
    sys.exit(1)
definition = next((m for m in parsed if m.get("id") == 5), None)
if definition is None or "result" not in definition:
    print("missing definition response:", parsed)
    sys.exit(1)
location = definition["result"]
if location.get("uri") != uri or location.get("range", {}).get("start", {}).get("line") != 0:
    print("definition did not point at helper:", location)
    sys.exit(1)
if location["range"]["start"].get("character") != 3:
    print("definition did not point at the helper name:", location)
    sys.exit(1)
symbol_response = next((m for m in parsed if m.get("id") == 6), None)
if symbol_response is None:
    print("missing document symbols:", parsed)
    sys.exit(1)
symbols_by_name = {s.get("name"): s for s in symbol_response.get("result", [])}
for wanted, kind in (("helper", 12), ("Point", 23), ("Mode", 10), ("main", 12)):
    if wanted not in symbols_by_name:
        print("document symbols missing", wanted, "in", sorted(symbols_by_name))
        sys.exit(1)
    if symbols_by_name[wanted].get("kind") != kind:
        print("document symbol kind for", wanted, "is", symbols_by_name[wanted].get("kind"))
        sys.exit(1)
point_fields = {c.get("name") for c in symbols_by_name["Point"].get("children", [])}
if point_fields != {"x", "y"}:
    print("Point children are", point_fields)
    sys.exit(1)
field_hover = next((m for m in parsed if m.get("id") == 7), None)
if field_hover is None or "field `x: Int`" not in field_hover.get("result", {}).get("contents", {}).get("value", ""):
    print("hover did not describe the struct field:", field_hover)
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
formatting = next((m for m in parsed if m.get("id") == 8), None)
if formatting is None:
    print("missing formatting response:", parsed)
    sys.exit(1)
edits = formatting.get("result") or []
if len(edits) != 1 or edits[0].get("newText") != "fn main() {\n    let x: Int = 1\n}\n":
    print("formatting did not normalize indentation:", formatting)
    sys.exit(1)
print("nova-lsp diagnostics, completion, hover, definition, symbols and formatting ok")
PY
