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
    ( cd "$tmp/project/app" && NOVA_TOOL_BIN="$tool" "$nova" box publish >/dev/null 2>&1 ) && no "unsupported publish exited ok" || ok
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

# ---- JSON reader -------------------------------------------------------
if command -v python3 >/dev/null 2>&1; then
    printf '%s' '{"a":[1,2.5,true,null],"b":"x\u00e9","c":{"d":"q\"z"},"e":"\u0001"}' > "$tmp/doc.json"
    out=$(NOVA_TOOL=box-json NOVA_TOOL_PATH="$tmp/doc.json" "$tool")
    expected=$(python3 -c 'import json,sys; print(json.dumps(json.load(open(sys.argv[1])),separators=(",",":"),ensure_ascii=False))' "$tmp/doc.json")
    [ "$out" = "$expected" ] && ok || no "json round trip: got $out"
    printf '%s' '{"a":1,}' > "$tmp/bad.json"
    NOVA_TOOL=box-json NOVA_TOOL_PATH="$tmp/bad.json" "$tool" | grep -q "error:" && ok || no "json rejects trailing comma"
    printf '%s' '[01]' > "$tmp/bad2.json"
    NOVA_TOOL=box-json NOVA_TOOL_PATH="$tmp/bad2.json" "$tool" | grep -q "leading zero" && ok || no "json rejects leading zero"
fi

# ---- Sparse index, search, info, fetch ---------------------------------
if [ -x "$nova" ]; then
    reg="$tmp/registry"
    mkdir -p "$reg/index/v1/packages" "$reg/files"
    cp -R "$root/examples/nova-tool/tests/boxes/app" "$tmp/pkgapp"
    ( cd "$tmp/pkgapp" && NOVA_TOOL_BIN="$tool" "$nova" box pack "$reg/files/app-1.0.0.box" ) >/dev/null 2>&1
    sha=$(shasum -a 256 "$reg/files/app-1.0.0.box" | cut -d' ' -f1)
    size=$(wc -c < "$reg/files/app-1.0.0.box" | tr -d ' ')
    h=$(printf '%s' '@nova/app' | shasum -a 256 | cut -d' ' -f1)
    aa=$(printf '%s' "$h" | cut -c1-2)
    bb=$(printf '%s' "$h" | cut -c3-4)
    mkdir -p "$reg/index/v1/packages/$aa/$bb/nova"
    cat > "$reg/index/v1/packages/$aa/$bb/nova/app.json" <<JSON
{"name":"@nova/app","versions":[{"version":"1.0.0","dependencies":[{"name":"@nova/math","req":"^0.2.0"}],"license":"MIT","nova":">=0.1.0","archive":"files/app-1.0.0.box","sha256":"$sha","size":$size,"published":"2026-01-01T00:00:00Z","yanked":false},{"version":"1.1.0","dependencies":[],"license":"MIT","nova":">=0.1.0","archive":"files/app-1.0.0.box","sha256":"$sha","size":$size,"published":"2026-01-02T00:00:00Z","yanked":true}]}
JSON
    cat > "$reg/config.json" <<'JSON'
{"protocol_version":1,"index":"index/","publishing":"github-pr","capabilities":["sparse-index","sha256","yank"]}
JSON
    cat > "$reg/index/v1/catalogue.json" <<'JSON'
{"packages":[{"name":"@nova/app","version":"1.0.0","description":"Fixture application"},{"name":"@nova/math","version":"0.2.3","description":"Vectors and matrices"}]}
JSON
    cp -R "$root/examples/nova-tool/tests/boxes/math" "$tmp/pkgmath"
    ( cd "$tmp/pkgmath" && NOVA_TOOL_BIN="$tool" "$nova" box pack "$reg/files/math-0.2.3.box" ) >/dev/null 2>&1
    msha=$(shasum -a 256 "$reg/files/math-0.2.3.box" | cut -d' ' -f1)
    mh=$(printf '%s' '@nova/math' | shasum -a 256 | cut -d' ' -f1)
    ma=$(printf '%s' "$mh" | cut -c1-2)
    mb=$(printf '%s' "$mh" | cut -c3-4)
    mkdir -p "$reg/index/v1/packages/$ma/$mb/nova"
    cat > "$reg/index/v1/packages/$ma/$mb/nova/math.json" <<JSON
{"name":"@nova/math","versions":[{"version":"0.2.3","dependencies":[],"license":"MIT","archive":"files/math-0.2.3.box","sha256":"$msha","yanked":false}]}
JSON

    out=$( cd "$tmp" && NOVA_BOXES_REGISTRY="$reg" NOVA_TOOL_BIN="$tool" "$nova" box search math )
    echo "$out" | grep -q "@nova/math 0.2.3 Vectors and matrices" && ok || no "search finds a box"
    out=$( cd "$tmp" && NOVA_BOXES_REGISTRY="$reg" NOVA_TOOL_BIN="$tool" "$nova" box search )
    echo "$out" | grep -q "@nova/app" && echo "$out" | grep -q "@nova/math" && ok || no "search lists the catalogue"
    out=$( cd "$tmp" && NOVA_BOXES_REGISTRY="$reg" NOVA_TOOL_BIN="$tool" "$nova" box search "nothing here" )
    echo "$out" | grep -q "no boxes match" && ok || no "search reports no match"

    out=$( cd "$tmp" && NOVA_BOXES_REGISTRY="$reg" NOVA_TOOL_BIN="$tool" "$nova" box info @nova/app )
    echo "$out" | grep -q "versions: 2" && echo "$out" | grep -q "latest: 1.0.0" && ok || no "info reads the sharded record"
    echo "$out" | grep -q "1.1.0 license=MIT yanked=true" && ok || no "info shows yanked versions"

    out=$( cd "$tmp" && NOVA_BOXES_REGISTRY="$reg" NOVA_BOXES_CACHE="$tmp/cache" NOVA_TOOL_BIN="$tool" "$nova" box fetch "@nova/app@^1.0.0" )
    echo "$out" | grep -q "fetched @nova/app 1.0.0" && ok || no "fetch skips yanked versions"
    [ -f "$tmp/cache/src/nova/app/1.0.0/Box.toml" ] && ok || no "fetch unpacks into the cache"

    badreg="$tmp/badreg"
    mkdir -p "$badreg/index/v1/packages/$aa/$bb/nova" "$badreg/files"
    cp "$reg/config.json" "$badreg/config.json"
    cp "$reg/files/app-1.0.0.box" "$badreg/files/app-1.0.0.box"
    cat > "$badreg/index/v1/packages/$aa/$bb/nova/app.json" <<'JSON'
{"name":"@nova/app","versions":[{"version":"1.0.0","dependencies":[],"license":"MIT","archive":"files/app-1.0.0.box","sha256":"0000000000000000000000000000000000000000000000000000000000000000","yanked":false}]}
JSON
    ( cd "$tmp" && NOVA_BOXES_REGISTRY="$badreg" NOVA_BOXES_CACHE="$tmp/badcache" NOVA_TOOL_BIN="$tool" "$nova" box fetch @nova/app ) > "$tmp/badfetch.log" 2>&1
    status=$?
    [ $status -ne 0 ] && grep -q "checksum mismatch" "$tmp/badfetch.log" && ok || no "fetch rejects a bad checksum"

    proto="$tmp/proto"
    mkdir -p "$proto/index"
    printf '%s' '{"protocol_version":2}' > "$proto/config.json"
    ( cd "$tmp" && NOVA_BOXES_REGISTRY="$proto" NOVA_TOOL_BIN="$tool" "$nova" box search ) > "$tmp/proto.log" 2>&1
    status=$?
    [ $status -ne 0 ] && grep -q "protocol 2" "$tmp/proto.log" && ok || no "unknown protocol stops the client"

    ( cd "$tmp" && NOVA_TOOL_BIN="$tool" "$nova" box search ) > "$tmp/noreg.log" 2>&1
    status=$?
    [ $status -ne 0 ] && grep -q "NOVA_BOXES_REGISTRY" "$tmp/noreg.log" && ok || no "missing registry reports the variable"

    # ---- Manifest editing and registry-backed update --------------------
    cp -R "$root/examples/nova-tool/tests/boxes" "$tmp/edit-project"
    ( cd "$tmp/edit-project/app" && NOVA_TOOL_BIN="$tool" "$nova" box add '@nova/app@^1.0.0' ) > "$tmp/add.log" 2>&1 && ok || no "nova box add"
    grep -q '"@nova/app" = "^1.0.0"' "$tmp/edit-project/app/Box.toml" && ok || no "add writes a quoted dependency"
    ( cd "$tmp/edit-project/app" && NOVA_TOOL_BIN="$tool" "$nova" box add '@nova/app@~1.0.0' ) >/dev/null 2>&1
    [ "$(grep -c '"@nova/app"' "$tmp/edit-project/app/Box.toml")" = "1" ] && grep -q '~1.0.0' "$tmp/edit-project/app/Box.toml" && ok || no "add replaces instead of duplicating"
    ( cd "$tmp/edit-project/app" && NOVA_TOOL_BIN="$tool" "$nova" box add '@nova/math@nonsense' ) > "$tmp/addbad.log" 2>&1
    status=$?
    [ $status -ne 0 ] && grep -q "not a SemVer requirement" "$tmp/addbad.log" && ok || no "add rejects a bad requirement"

    ( cd "$tmp/edit-project/app" && NOVA_BOXES_REGISTRY="$reg" NOVA_TOOL_BIN="$tool" "$nova" box update ) > "$tmp/update.log" 2>&1 && ok || no "nova box update"
    grep -q 'box "@nova/app" "1.0.0" registry' "$tmp/edit-project/app/Box.lock" && ok || no "update records a registry box"
    grep -q 'box "@nova/math" "0.2.3" registry' "$tmp/edit-project/app/Box.lock" && ok || no "update walks registry dependencies"
    grep -q 'sha256 "' "$tmp/edit-project/app/Box.lock" && ok || no "update records the checksum"
    grep -q 'box "util" "1.0.0" path' "$tmp/edit-project/app/Box.lock" && ok || no "update keeps path boxes"

    # The index now yanks 1.0.0 and publishes 1.0.1, which `~1.0.0` accepts.
    cat > "$reg/index/v1/packages/$aa/$bb/nova/app.json" <<JSON
{"name":"@nova/app","versions":[{"version":"1.0.0","dependencies":[{"name":"@nova/math","req":"^0.2.0"}],"license":"MIT","archive":"files/app-1.0.0.box","sha256":"$sha","yanked":true},{"version":"1.0.1","dependencies":[{"name":"@nova/math","req":"^0.2.0"}],"license":"MIT","archive":"files/app-1.0.0.box","sha256":"$sha","yanked":false}]}
JSON
    out=$( cd "$tmp/edit-project/app" && NOVA_BOXES_REGISTRY="$reg" NOVA_TOOL_BIN="$tool" "$nova" box update 2>&1 )
    echo "$out" | grep -q "warning: @nova/app 1.0.0 is yanked" && ok || no "update warns about a yanked lock"
    grep -q 'box "@nova/app" "1.0.1" registry' "$tmp/edit-project/app/Box.lock" && ok || no "update moves off a yanked version"

    ( cd "$tmp/edit-project/app" && NOVA_TOOL_BIN="$tool" "$nova" box remove '@nova/app' ) >/dev/null 2>&1 && ok || no "nova box remove"
    grep -q '"@nova/app"' "$tmp/edit-project/app/Box.toml" && no "remove leaves the dependency" || ok
    ( cd "$tmp/edit-project/app" && NOVA_TOOL_BIN="$tool" "$nova" box remove '@nova/app' ) > "$tmp/remove.log" 2>&1
    status=$?
    [ $status -ne 0 ] && grep -q "no dependency" "$tmp/remove.log" && ok || no "remove reports a missing dependency"

    # ---- HTTP read path, against a local static server ------------------
    if command -v python3 >/dev/null 2>&1 && command -v curl >/dev/null 2>&1; then
        port=$((21000 + $$ % 10000))
        python3 -m http.server "$port" --directory "$reg" >/dev/null 2>&1 &
        server=$!
        for _ in $(seq 1 50); do
            curl -fsS "http://127.0.0.1:$port/config.json" >/dev/null 2>&1 && break
            sleep 0.1
        done
        out=$( cd "$tmp" && NOVA_BOXES_REGISTRY="http://127.0.0.1:$port" NOVA_TOOL_BIN="$tool" "$nova" box info @nova/app )
        echo "$out" | grep -q "versions: 2" && ok || no "http index read"
        kill "$server" 2>/dev/null
        wait "$server" 2>/dev/null
    fi

    ( cd "$tmp" && NOVA_BOXES_OFFLINE=1 NOVA_BOXES_REGISTRY="http://127.0.0.1:9" NOVA_TOOL_BIN="$tool" "$nova" box info @nova/app ) > "$tmp/offline.log" 2>&1
    status=$?
    [ $status -ne 0 ] && grep -q "NOVA_BOXES_OFFLINE" "$tmp/offline.log" && ok || no "offline mode refuses a remote registry"

    # ---- The compiler accepts Box.toml and honours Box.lock ------------
    if command -v clang >/dev/null 2>&1; then
        mkdir -p "$tmp/boxproj/src"
        printf '[package]\nname = "boxed"\nversion = "1.0.0"\n' > "$tmp/boxproj/Box.toml"
        printf 'fn main() { }\n' > "$tmp/boxproj/src/main.nova"
        ( cd "$tmp/boxproj" && "$nova" build --locked ) > "$tmp/boxlock.log" 2>&1
        status=$?
        [ $status -ne 0 ] && grep -q "Box.lock" "$tmp/boxlock.log" && ok || no "--locked requires Box.lock"
        ( cd "$tmp/boxproj" && "$nova" build ) > "$tmp/boxbuild.log" 2>&1 && ok || no "build a Box.toml project"
        [ -f "$tmp/boxproj/Box.lock" ] && ok || no "build writes Box.lock"
        ( cd "$tmp/boxproj" && "$nova" build --locked ) >/dev/null 2>&1 && ok || no "--locked accepts a fresh Box.lock"
    fi
fi

echo "passed $pass, failed $fail"
[ "$fail" -eq 0 ]
