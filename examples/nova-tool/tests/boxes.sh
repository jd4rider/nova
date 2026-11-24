#!/bin/sh
# Nova Boxes: SemVer, manifest resolution, lock determinism, cycles.
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

cat > "$tmp/req.txt" <<'CASES'
1.2.3|^1.0.0 true
2.0.0|^1.0.0 false
1.2.9|~1.2.0 true
1.3.0|~1.2.0 false
1.5.0|1.* true
1.2.3-beta.1|^1.0.0 false
1.2.3-beta.1|>=1.2.3-beta.0 true
0.2.5|^0.2.0 true
0.3.0|^0.2.0 false
CASES
out=$(NOVA_TOOL=box-semver NOVA_TOOL_PATH="$tmp/req.txt" "$tool")
echo "$out" | grep -q "1.2.3 \^1.0.0 true" && ok || no "caret"
echo "$out" | grep -q "2.0.0 \^1.0.0 false" && ok || no "caret major"
echo "$out" | grep -q "1.3.0 ~1.2.0 false" && ok || no "tilde"
echo "$out" | grep -q "1.5.0 1.\* true" && ok || no "wildcard"
echo "$out" | grep -q "1.2.3-beta.1 \^1.0.0 false" && ok || no "prerelease excluded"
echo "$out" | grep -q "0.3.0 \^0.2.0 false" && ok || no "caret zero major"

out=$(NOVA_TOOL=box-resolve NOVA_TOOL_PATH="$root/examples/nova-tool/tests/boxes/app/Box.toml" "$tool")
echo "$out" | grep -q "app v1.0.0" && ok || no "root shown"
echo "$out" | grep -q "util v1.0.0" && ok || no "util resolved"
echo "$out" | grep -q "math v0.2.3" && ok || no "math resolved"
echo "$out" | grep -q "error:" && no "unexpected error" || ok

out=$(NOVA_TOOL=box-resolve NOVA_TOOL_PATH="$root/examples/nova-tool/tests/boxes/c1/Box.toml" "$tool")
echo "$out" | grep -q "cycle:" && ok || no "cycle detected"

out=$(NOVA_TOOL=box-resolve NOVA_TOOL_PATH="$root/examples/nova-tool/tests/boxes/bad/Box.toml" "$tool")
echo "$out" | grep -q "does not satisfy" && ok || no "requirement mismatch"

NOVA_TOOL=box-lock NOVA_TOOL_PATH="$root/examples/nova-tool/tests/boxes/app/Box.toml" "$tool" > "$tmp/one.lock"
NOVA_TOOL=box-lock NOVA_TOOL_PATH="$root/examples/nova-tool/tests/boxes/app/Box.toml" "$tool" > "$tmp/two.lock"
cmp -s "$tmp/one.lock" "$tmp/two.lock" && ok || no "lock deterministic"
grep -q 'box "math" "0.2.3"' "$tmp/one.lock" && ok || no "lock entry"

NOVA_TOOL=box-metadata NOVA_TOOL_PATH="$root/examples/nova-tool/tests/boxes/app/Box.toml" "$tool" > "$tmp/meta.json"
grep -q '"name":"app"' "$tmp/meta.json" && grep -q '"name":"math"' "$tmp/meta.json" && ok || no "metadata json"

nova="$root/target/debug/nova"
if [ -x "$nova" ]; then
    cp -R "$root/examples/nova-tool/tests/boxes" "$tmp/project"
    ( cd "$tmp/project/app" && NOVA_TOOL_BIN="$tool" "$nova" box tree ) > "$tmp/cli-tree.txt" 2>&1
    grep -q "math v0.2.3" "$tmp/cli-tree.txt" && ok || no "nova box tree"
    ( cd "$tmp/project/app" && NOVA_TOOL_BIN="$tool" "$nova" box lock ) > "$tmp/cli-lock.txt" 2>&1
    grep -q 'box "math"' "$tmp/project/app/Box.lock" && ok || no "nova box lock writes Box.lock"
    ( cd "$tmp/project/app" && NOVA_TOOL_BIN="$tool" "$nova" box check ) >/dev/null 2>&1 && ok || no "nova box check"
    ( cd "$tmp/project/app" && NOVA_TOOL_BIN="$tool" "$nova" box search >/dev/null 2>&1 ) && no "unsupported search exited ok" || ok
else
    no "nova binary missing"
fi

if [ -x "$nova" ]; then
    rm -rf "$tmp/archive"
    cp -R "$root/examples/nova-tool/tests/boxes" "$tmp/archive"
    ( cd "$tmp/archive/app" && NOVA_TOOL_BIN="$tool" "$nova" box pack "$tmp/archive/app.box" ) >/dev/null 2>&1 && ok || no "nova box pack"
    gzip -t "$tmp/archive/app.box" 2>/dev/null && ok || no "archive is valid gzip"
    tar -tzf "$tmp/archive/app.box" 2>/dev/null | grep -q "Box.toml" && ok || no "archive lists Box.toml"
    mkdir -p "$tmp/archive/out"
    tar -xzf "$tmp/archive/app.box" -C "$tmp/archive/out" 2>/dev/null && cmp -s "$tmp/archive/out/Box.toml" "$tmp/archive/app/Box.toml" && ok || no "archive extracts unchanged manifest"
    sha=$(shasum -a 256 "$tmp/archive/app.box" | cut -d' ' -f1)
    ( cd "$tmp/archive/app" && NOVA_TOOL_BIN="$tool" "$nova" box verify "$tmp/archive/app.box" ) 2>/dev/null | grep -q "sha256=$sha" && ok || no "nova box verify sha256"
    ( cd "$tmp/archive/app" && NOVA_TOOL_BIN="$tool" "$nova" box pack "$tmp/archive/again.box" ) >/dev/null 2>&1
    cmp -s "$tmp/archive/app.box" "$tmp/archive/again.box" && ok || no "archive is deterministic"
    ( cd "$tmp/archive/app" && NOVA_TOOL_BIN="$tool" "$nova" box pack && NOVA_TOOL_BIN="$tool" "$nova" box unpack "$tmp/archive/app/Box.box" "$tmp/archive/unpacked" ) >/dev/null 2>&1 && ok || no "nova box unpack"
    cmp -s "$tmp/archive/unpacked/Box.toml" "$tmp/archive/app/Box.toml" && ok || no "unpack restores manifest"
    if command -v python3 >/dev/null 2>&1; then
        python3 - "$tmp" <<'PY' >/dev/null 2>&1
import gzip, tarfile, io, sys
root = sys.argv[1]
buf = io.BytesIO()
with tarfile.open(fileobj=buf, mode="w") as tar:
    info = tarfile.TarInfo("../escaped.txt")
    payload = b"nope"
    info.size = len(payload)
    tar.addfile(info, io.BytesIO(payload))
with open(root + "/evil.box", "wb") as f:
    f.write(gzip.compress(buf.getvalue(), compresslevel=0))
PY
        ( cd "$tmp/archive/app" && NOVA_TOOL_BIN="$tool" "$nova" box unpack "$tmp/evil.box" "$tmp/archive/evil-out" ) > "$tmp/evil.log" 2>&1
        status=$?
        if [ $status -ne 0 ] && grep -q "unsafe archive path" "$tmp/evil.log" && [ ! -f "$tmp/escaped.txt" ] && [ ! -f "$tmp/archive/escaped.txt" ]; then ok; else no "unpack rejects traversal"; fi
    fi
fi

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
