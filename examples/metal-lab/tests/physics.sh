#!/bin/sh
# ENG-11.002 acceptance: a lit checker cube is moved by the integer physics
# substrate, rendered through an orbit camera, and captured at two simulation
# states. A second run uses a front camera at the same state, so the images
# prove the camera pose is real rather than a fixed view.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
NOVA="${NOVA_BIN:-$ROOT/examples/nova-tool/target/nova/nova-tool}"

[ -x "$NOVA" ] || {
    echo "physics: missing compiler: $NOVA" >&2
    exit 1
}

cd "$HERE/.."
mkdir -p target
"$NOVA" build > target/physics-build.log 2>&1

TEXTURE="$HERE/../assets/checker.png"
[ -s "$TEXTURE" ] || { echo "physics: missing texture fixture $TEXTURE" >&2; exit 1; }

SHOT="$HERE/../target/physics.png"
FRONT="$HERE/../target/physics-front.png"
rm -f "$SHOT" "$SHOT.resized.png" "$FRONT" "$FRONT.resized.png"

OUT=$(NOVA_METAL_MODE=physics NOVA_METAL_CAMERA=orbit NOVA_METAL_TEXTURE="$TEXTURE" NOVA_METAL_SHOT="$SHOT" ./target/nova/metal-lab)
echo "$OUT"
echo "$OUT" | grep -q "ok=true"
echo "$OUT" | grep -q "mode=physics"
echo "$OUT" | grep -q "geometry=2"
echo "$OUT" | grep -q "camera=1"
echo "$OUT" | grep -q "camera_resized=1"
echo "$OUT" | grep -q "camera_preset=orbit"
echo "$OUT" | grep -q "light=1"
echo "$OUT" | grep -q "texture=1"
echo "$OUT" | grep -q "physics step=93 body=204,148"
echo "$OUT" | grep -q "physics step=153 body=111,170"

FRONT_OUT=$(NOVA_METAL_MODE=physics NOVA_METAL_CAMERA=front NOVA_METAL_TEXTURE="$TEXTURE" NOVA_METAL_SHOT="$FRONT" ./target/nova/metal-lab)
echo "$FRONT_OUT"
echo "$FRONT_OUT" | grep -q "ok=true"
echo "$FRONT_OUT" | grep -q "camera_preset=front"
echo "$FRONT_OUT" | grep -q "physics step=93 body=204,148"
echo "$FRONT_OUT" | grep -q "physics step=153 body=111,170"

[ -s "$SHOT" ] || { echo "physics: orbit capture missing" >&2; exit 1; }
[ -s "$SHOT.resized.png" ] || { echo "physics: resized capture missing" >&2; exit 1; }
[ -s "$FRONT" ] || { echo "physics: front capture missing" >&2; exit 1; }
file "$SHOT" | grep -q "PNG image data"
file "$SHOT.resized.png" | grep -q "PNG image data"
file "$FRONT" | grep -q "PNG image data"

WIDE=$(sips -g pixelWidth "$SHOT" | awk '/pixelWidth/{print $2}')
NARROW=$(sips -g pixelWidth "$SHOT.resized.png" | awk '/pixelWidth/{print $2}')
[ "$WIDE" = "1800" ] || { echo "physics: unexpected first width $WIDE" >&2; exit 1; }
[ "$NARROW" = "1280" ] || { echo "physics: resize did not take effect ($NARROW)" >&2; exit 1; }

. "$HERE/pixels.sh"

# Same step, different camera: orbit sees green/blue where front sees red or
# background. This is a camera-pose check, not a pair of identical frames.
expect_pixel "$SHOT" 1000 640 0 165 0 "orbit cube green" || exit 1
expect_pixel "$FRONT" 1000 640 165 0 0 "front cube red" || exit 1
expect_pixel "$SHOT" 800 700 0 0 165 "orbit cube blue" || exit 1
expect_pixel "$FRONT" 800 700 13 15 20 "front background" || exit 1
expect_pixel "$SHOT" 1100 500 213 213 0 "orbit yellow face" || exit 1
expect_pixel "$FRONT" 1100 500 0 165 0 "front green face" || exit 1

# The resized capture advances 60 physics steps; the cube has moved left and
# down, and the same four texture quadrants are still visible.
expect_pixel "$SHOT.resized.png" 500 500 165 0 0 "moved cube red" || exit 1
expect_pixel "$SHOT.resized.png" 600 500 0 165 0 "moved cube green" || exit 1
expect_pixel "$SHOT.resized.png" 500 600 0 0 165 "moved cube blue" || exit 1
expect_pixel "$SHOT.resized.png" 600 600 165 165 0 "moved cube yellow" || exit 1
expect_pixel "$SHOT.resized.png" 100 100 13 15 20 "resized background" || exit 1

echo "physics: 11 camera/motion checks passed"
