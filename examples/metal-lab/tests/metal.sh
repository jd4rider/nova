#!/bin/sh
# ENG-02.002/003 acceptance: build with the compiler made from the current
# sources, open a Metal window, load a PNG into a textured quad, submit three
# frames, capture PNGs and exit cleanly.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
NOVA="${NOVA_BIN:-$ROOT/examples/nova-tool/target/nova/nova-tool}"

[ -x "$NOVA" ] || {
    echo "metal: missing compiler: $NOVA" >&2
    echo "metal: build it first: (cd examples/nova-tool && ../../bootstrap/nova build)" >&2
    exit 1
}

cd "$HERE/.."
mkdir -p target
"$NOVA" build > target/metal-build.log 2>&1

SHOT="$HERE/../target/frame.png"
TEXTURE="$HERE/../assets/checker.png"
[ -s "$TEXTURE" ] || { echo "metal: missing texture fixture $TEXTURE" >&2; exit 1; }
rm -f "$SHOT"
OUT=$(NOVA_METAL_SHOT="$SHOT" NOVA_METAL_TEXTURE="$TEXTURE" ./target/nova/metal-lab)
echo "$OUT"
echo "$OUT" | grep -q "ok=true"
echo "$OUT" | grep -q "frames=3"
echo "$OUT" | grep -q "texture=1"

[ -s "$SHOT" ] || { echo "metal: capture missing" >&2; exit 1; }
file "$SHOT" | grep -q "PNG image data"

# The resize step must change the captured drawable size, not just the window.
RESIZED="$SHOT.resized.png"
[ -s "$RESIZED" ] || { echo "metal: resized capture missing" >&2; exit 1; }
file "$RESIZED" | grep -q "PNG image data"
WIDE=$(sips -g pixelWidth "$SHOT" | awk '/pixelWidth/{print $2}')
NARROW=$(sips -g pixelWidth "$RESIZED" | awk '/pixelWidth/{print $2}')
[ "$WIDE" = "1800" ] || { echo "metal: unexpected first width $WIDE" >&2; exit 1; }
[ "$NARROW" = "1280" ] || { echo "metal: resize did not take effect ($NARROW)" >&2; exit 1; }

# A wrong UV mapping, a vertical flip or an unbound texture changes at least
# one quadrant. The probe is shared with the cube acceptance check.
. "$HERE/pixels.sh"

# 1800x1280 capture: the quad spans x 342..1458 and y 243..1037.
expect_pixel "$SHOT" 720 480 255 0 0 "top-left quadrant" || exit 1
expect_pixel "$SHOT" 1080 480 0 255 0 "top-right quadrant" || exit 1
expect_pixel "$SHOT" 720 800 0 0 255 "bottom-left quadrant" || exit 1
expect_pixel "$SHOT" 1080 800 255 255 0 "bottom-right quadrant" || exit 1

# 1280x960 capture: the quad spans x 243..1037 and y 182..778. Reading two
# quadrants here also proves the texture survives the drawable resize.
expect_pixel "$RESIZED" 520 360 255 0 0 "resized top-left quadrant" || exit 1
expect_pixel "$RESIZED" 760 600 255 255 0 "resized bottom-right quadrant" || exit 1
