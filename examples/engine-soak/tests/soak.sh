#!/bin/sh
# ENG-01.007 acceptance: 10k entities over 10k fixed steps, with the wall-clock
# cost measured on this host. This records a number; it does not set a target.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
NOVA="${NOVA_BIN:-$ROOT/bootstrap/nova}"

cd "$HERE/.."
mkdir -p target
"$NOVA" build > target/soak-build.log 2>&1

/usr/bin/time -p ./target/nova/engine-soak > target/soak.out 2> target/soak-time.log
OUT="$(cat target/soak.out)"
echo "$OUT"
cat target/soak-time.log
echo "$OUT" | grep -q "entities=10000 steps=10000 live=10000 capacity=10000"
