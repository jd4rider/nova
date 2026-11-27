#!/bin/sh
# ENG-02.004 acceptance: build with the compiler made from the current sources,
# render the depth-tested cube through a perspective camera built in math.nova,
# capture both drawable sizes, and verify perspective faces plus occlusion.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
NOVA="${NOVA_BIN:-$ROOT/examples/nova-tool/target/nova/nova-tool}"

[ -x "$NOVA" ] || {
    echo "cube: missing compiler: $NOVA" >&2
    echo "cube: build it first: (cd examples/nova-tool && ../../bootstrap/nova build)" >&2
    exit 1
}

cd "$HERE/.."
mkdir -p target
"$NOVA" build > target/cube-build.log 2>&1

SHOT="$HERE/../target/cube.png"
rm -f "$SHOT" "$SHOT.resized.png"
OUT=$(NOVA_METAL_MODE=cube NOVA_METAL_SHOT="$SHOT" ./target/nova/metal-lab)
echo "$OUT"
echo "$OUT" | grep -q "ok=true"
echo "$OUT" | grep -q "mode=cube"
echo "$OUT" | grep -q "geometry=1"
echo "$OUT" | grep -q "camera=1"
echo "$OUT" | grep -q "camera_resized=1"
echo "$OUT" | grep -q "texture=0"

[ -s "$SHOT" ] || { echo "cube: capture missing" >&2; exit 1; }
file "$SHOT" | grep -q "PNG image data"

RESIZED="$SHOT.resized.png"
[ -s "$RESIZED" ] || { echo "cube: resized capture missing" >&2; exit 1; }
file "$RESIZED" | grep -q "PNG image data"
WIDE=$(sips -g pixelWidth "$SHOT" | awk '/pixelWidth/{print $2}')
NARROW=$(sips -g pixelWidth "$RESIZED" | awk '/pixelWidth/{print $2}')
[ "$WIDE" = "1800" ] || { echo "cube: unexpected first width $WIDE" >&2; exit 1; }
[ "$NARROW" = "1280" ] || { echo "cube: resize did not take effect ($NARROW)" >&2; exit 1; }

. "$HERE/pixels.sh"

# The camera is at (1.2, 1.0, 3.0): +Z blue faces the viewer, +X orange is on
# the right and +Y green on top. The hidden -Z face is drawn last and is
# magenta, so a broken depth test paints it over the front and the centre probe
# stops being blue. The hidden -X olive face must not leak through either.
expect_pixel "$SHOT" 800 650 61 107 242 "cube front (+Z) centre" || exit 1
expect_pixel "$SHOT" 520 650 61 107 242 "cube front covers hidden -X" || exit 1
expect_pixel "$SHOT" 850 360 64 209 97 "cube top (+Y) face" || exit 1
expect_pixel "$SHOT" 1180 550 245 92 56 "cube right (+X) face" || exit 1
expect_pixel "$SHOT" 100 100 13 15 20 "background outside cube" || exit 1

# The resized capture rebuilt the camera at the new aspect ratio, so the same
# faces stay in place rather than stretching.
expect_pixel "$RESIZED" 560 510 61 107 242 "resized cube front" || exit 1
expect_pixel "$RESIZED" 370 510 61 107 242 "resized front covers -X" || exit 1
expect_pixel "$RESIZED" 600 280 64 209 97 "resized cube top" || exit 1
expect_pixel "$RESIZED" 850 460 245 92 56 "resized cube right" || exit 1
expect_pixel "$RESIZED" 100 100 13 15 20 "resized background" || exit 1
