#!/bin/sh
# PERF-01.01: Nova-only build and check measurements.
#
# The Rust comparison ended with the seed. This still builds and checks the
# same workload from a clean copy, verifies both produce a working compiler,
# and records medians for regression tracking. Timings are measurements, not a
# gate: only build/check/correctness failures exit non-zero.
#
# Env: PERF_SAMPLES (default 5), PERF_WARMUP (default 1).
set -eu
dir=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$dir/../../.." && pwd)
nova=${NOVA_BIN:-"$root/bootstrap/nova"}
tool=${NOVA_TOOL_BIN:-"$dir/../target/nova/nova-tool"}
runtime="$root/runtime/runtime.c"
samples=${PERF_SAMPLES:-5}
warmups=${PERF_WARMUP:-1}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

[ -x "$nova" ] || { echo "perf: missing $nova" >&2; exit 1; }
[ -x "$tool" ] || { echo "perf: build examples/nova-tool first" >&2; exit 1; }
case "$samples" in ''|*[!0-9]*) echo "perf: PERF_SAMPLES must be a positive integer" >&2; exit 1;; esac
case "$warmups" in ''|*[!0-9]*) echo "perf: PERF_WARMUP must be a non-negative integer" >&2; exit 1;; esac
[ "$samples" -ge 1 ] || { echo "perf: PERF_SAMPLES must be at least 1" >&2; exit 1; }

mkdir -p "$tmp/work/examples"
cp -R "$root/libraries" "$tmp/work/libraries"
cp -R "$dir/.." "$tmp/work/examples/nova-tool"
rm -rf "$tmp/work/examples/nova-tool/target"

measure() { # measure CWD LOG COMMAND...
    cwd=$1
    log=$2
    shift 2
    python3 - "$cwd" "$log" "$runtime" "$@" <<'PY'
import os, subprocess, sys, time
cwd, log, runtime = sys.argv[1], sys.argv[2], sys.argv[3]
command = sys.argv[4:]
env = dict(os.environ)
env["NOVA_TOOL_RUNTIME"] = runtime
with open(log, "wb") as out:
    started = time.perf_counter()
    run = subprocess.run(command, cwd=cwd, stdout=out, stderr=subprocess.STDOUT, env=env)
    elapsed = (time.perf_counter() - started) * 1000.0
if run.returncode != 0:
    sys.exit(run.returncode)
print("%.1f" % elapsed)
PY
}

median() {
    printf '%s\n' $1 | sort -n | awk '
        { value[NR] = $1 }
        END {
            if (NR % 2 == 1) printf "%.1f", value[(NR + 1) / 2]
            else printf "%.1f", (value[NR / 2] + value[NR / 2 + 1]) / 2
        }'
}

cpu=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m)
clang_version=$(clang --version 2>/dev/null | sed -n '1p' || echo "clang not on PATH")
commit=$(git -C "$root" rev-parse --short HEAD 2>/dev/null || echo unknown)
files=$(find "$root/libraries" "$dir/.." -name '*.nova' | wc -l | tr -d ' ')
bytes=$(find "$root/libraries" "$dir/.." -name '*.nova' -exec wc -c {} + | tail -n 1 | awk '{print $1}')

echo "perf: environment: $(uname -srm); $cpu"
echo "perf: toolchain: $clang_version; commit $commit"
echo "perf: workload: build examples/nova-tool ($files .nova files, $bytes bytes),"
echo "perf:   clean target per run, $samples samples and $warmups warmup run(s)"

build_ms=$(
    i=0
    total=$((warmups + samples))
    while [ "$i" -lt "$total" ]; do
        i=$((i + 1))
        rm -rf "$tmp/work/examples/nova-tool/target"
        ms=$(measure "$tmp/work/examples/nova-tool" "$tmp/build-$i.log" "$nova" build) || {
            echo "perf: build failed" >&2
            cat "$tmp/build-$i.log" >&2
            exit 1
        }
        if [ "$i" -gt "$warmups" ]; then
            echo "$ms"
        else
            echo "perf: warmup $i: ${ms}ms" >&2
        fi
    done
)
echo "perf: build samples (ms): $(printf '%s' "$build_ms" | tr '\n' ' ') median $(median "$build_ms")"

check_ms=$(
    i=0
    total=$((warmups + samples))
    while [ "$i" -lt "$total" ]; do
        i=$((i + 1))
        rm -rf "$tmp/work/examples/nova-tool/target"
        ms=$(measure "$tmp/work/examples/nova-tool" "$tmp/check-$i.log" "$nova" check) || {
            echo "perf: check failed" >&2
            cat "$tmp/check-$i.log" >&2
            exit 1
        }
        if ! grep -q "no errors" "$tmp/check-$i.log"; then
            echo "perf: check did not report a clean project" >&2
            cat "$tmp/check-$i.log" >&2
            exit 1
        fi
        if [ "$i" -gt "$warmups" ]; then
            echo "$ms"
        else
            echo "perf: check warmup $i: ${ms}ms" >&2
        fi
    done
)
echo "perf: check samples (ms): $(printf '%s' "$check_ms" | tr '\n' ' ') median $(median "$check_ms")"

# The check workload removes target/ each run; rebuild the artifact before
# using it to compile the correctness program.
(cd "$tmp/work/examples/nova-tool" && NOVA_TOOL_RUNTIME="$runtime" "$nova" build > "$tmp/final-build.log" 2>&1) || {
    echo "perf: final build failed" >&2
    cat "$tmp/final-build.log" >&2
    exit 1
}

hello="$tmp/hello.nova"
printf 'import std.io { println }\n\nfn main() {\n    println("hello-perf")\n}\n' > "$hello"
built="$tmp/work/examples/nova-tool/target/nova/nova-tool"
[ -x "$built" ] || { echo "perf: build produced no executable" >&2; exit 1; }
if ! NOVA_TOOL_RUNTIME="$runtime" "$built" build "$hello" "$tmp/hello-out" > "$tmp/hello.log" 2>&1; then
    echo "perf: built compiler could not compile the hello program" >&2
    cat "$tmp/hello.log" >&2
    exit 1
fi
text=$("$tmp/hello-out")
[ "$text" = "hello-perf" ] || { echo "perf: hello printed '$text'" >&2; exit 1; }
echo "perf: correctness PASS (artifact compiled and ran hello-perf)"
echo "perf: measurement only; no faster-than-Rust claim (the Rust reference is gone)"
