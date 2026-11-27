#!/bin/sh
# ENG-11.001 acceptance: render a white-tinted checker cube with per-face
# normals and one directional light. The same texture region has different
# brightness on top/front/right, so the probes prove both texture and lighting.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
NOVA="${NOVA_BIN:-$ROOT/examples/nova-tool/target/nova/nova-tool}"

[ -x "$NOVA" ] || {
    echo "lit: missing compiler: $NOVA" >&2
    echo "lit: build it first: (cd examples/nova-tool && ../../bootstrap/nova build)" >&2
    exit 1
}

cd "$HERE/.."
mkdir -p target
"$NOVA" build > target/lit-build.log 2>&1

SHOT="$HERE/../target/lit.png"
TEXTURE="$HERE/../assets/checker.png"
[ -s "$TEXTURE" ] || { echo "lit: missing texture fixture $TEXTURE" >&2; exit 1; }
rm -f "$SHOT" "$SHOT.resized.png"
OUT=$(NOVA_METAL_MODE=lit NOVA_METAL_TEXTURE="$TEXTURE" NOVA_METAL_SHOT="$SHOT" ./target/nova/metal-lab)
echo "$OUT"
echo "$OUT" | grep -q "ok=true"
echo "$OUT" | grep -q "mode=lit"
echo "$OUT" | grep -q "geometry=2"
echo "$OUT" | grep -q "camera=1"
echo "$OUT" | grep -q "camera_resized=1"
echo "$OUT" | grep -q "light=1"
echo "$OUT" | grep -q "texture=1"

[ -s "$SHOT" ] || { echo "lit: capture missing" >&2; exit 1; }
file "$SHOT" | grep -q "PNG image data"

RESIZED="$SHOT.resized.png"
[ -s "$RESIZED" ] || { echo "lit: resized capture missing" >&2; exit 1; }
file "$RESIZED" | grep -q "PNG image data"
WIDE=$(sips -g pixelWidth "$SHOT" | awk '/pixelWidth/{print $2}')
NARROW=$(sips -g pixelWidth "$RESIZED" | awk '/pixelWidth/{print $2}')
[ "$WIDE" = "1800" ] || { echo "lit: unexpected first width $WIDE" >&2; exit 1; }
[ "$NARROW" = "1280" ] || { echo "lit: resize did not take effect ($NARROW)" >&2; exit 1; }

. "$HERE/pixels.sh"

# The front (+Z) face is lit at 165, the top (+Y) at 213 and the right (+X) at
# 149 for the same red texture quadrant. A constant normal or disabled light
# would erase those differences; a missing UV map would erase the quadrants.
expect_pixel "$SHOT" 600 550 165 0 0 "lit front red" || exit 1
expect_pixel "$SHOT" 900 550 0 165 0 "lit front green" || exit 1
expect_pixel "$SHOT" 600 850 0 0 165 "lit front blue" || exit 1
expect_pixel "$SHOT" 900 850 165 165 0 "lit front yellow" || exit 1
expect_pixel "$SHOT" 800 380 213 0 0 "lit top red brighter" || exit 1
expect_pixel "$SHOT" 900 380 0 213 0 "lit top green brighter" || exit 1
expect_pixel "$SHOT" 1100 380 213 213 0 "lit top yellow brighter" || exit 1
expect_pixel "$SHOT" 1200 450 149 0 0 "lit right red dimmer" || exit 1
expect_pixel "$SHOT" 1180 700 0 0 149 "lit right blue dimmer" || exit 1
expect_pixel "$SHOT" 100 100 13 15 20 "lit background" || exit 1

# The resize rebuilds the camera at the new aspect ratio; the same face and
# texture probes must still land.
expect_pixel "$RESIZED" 400 400 165 0 0 "resized lit front red" || exit 1
expect_pixel "$RESIZED" 600 400 0 165 0 "resized lit front green" || exit 1
expect_pixel "$RESIZED" 420 620 0 0 165 "resized lit front blue" || exit 1
expect_pixel "$RESIZED" 640 620 165 165 0 "resized lit front yellow" || exit 1
expect_pixel "$RESIZED" 560 280 213 0 0 "resized lit top red" || exit 1
expect_pixel "$RESIZED" 600 300 0 213 0 "resized lit top green" || exit 1
expect_pixel "$RESIZED" 850 460 149 0 0 "resized lit right red" || exit 1
expect_pixel "$RESIZED" 860 500 0 0 149 "resized lit right blue" || exit 1
expect_pixel "$RESIZED" 100 100 13 15 20 "resized lit background" || exit 1

echo "lit: 19 texture/lighting checks passed"
