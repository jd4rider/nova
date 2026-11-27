#!/bin/sh
# ENG-01 acceptance: headless core build, deterministic lab output, unit tests
# and the physics-lab suite whose scene-tick regression this slice fixes.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
NOVA="${NOVA_BIN:-$ROOT/bootstrap/nova}"

cd "$HERE/.."
"$NOVA" build >/tmp/engine-lab-build.log 2>&1
OUT="$(./target/nova/engine-lab)"
echo "$OUT"
echo "$OUT" | grep -q "frames=120 steps=120 live=2 capacity=2"
echo "$OUT" | grep -q "position=6,8"
echo "$OUT" | grep -q "rotated=0,1"
echo "$OUT" | grep -q "world_live=2 world_steps=600"

"$NOVA" test
