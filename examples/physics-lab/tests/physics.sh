#!/bin/sh
# Runs the physics scenarios and requires every one to pass.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
cd "$HERE/.."
"${NOVA_BIN:-$ROOT/bootstrap/nova}" build >/dev/null
OUT="$(./target/nova/physics-lab)"
echo "$OUT"
echo "$OUT" | grep -q "failed 0"
