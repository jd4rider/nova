#!/bin/sh
# Gemstorm acceptance.
#
#   1. the rules suite
#   2. two hint-driven runs of the same seed land on the same hash and end
#   3. every screen captures and reports the state it drew
#   4. pixel probes prove the board, HUD, menu, over card and icon drew
#
# Run: sh examples/gemstorm/tests/run.sh
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
if [ -n "${NOVA_BIN:-}" ]; then
    NOVA="$NOVA_BIN"
elif [ -x "$ROOT/examples/nova-tool/target/nova/nova-tool" ]; then
    NOVA="$ROOT/examples/nova-tool/target/nova/nova-tool"
else
    NOVA="$ROOT/bootstrap/nova"
fi

cd "$HERE/.."
echo "--- build ---"
"$NOVA" build >/dev/null
BIN=./target/nova/gemstorm
[ -x "$BIN" ] || { echo "run: build did not produce $BIN" >&2; exit 1; }

echo "--- rules suite ---"
"$NOVA" test 2>&1 | tail -3 | tee target/rules.log
grep -q "0 failed" target/rules.log || { echo "run: rules suite failed" >&2; exit 1; }

echo "--- determinism ---"
A=$(NOVA_GEMSTORM_SIM=25 NOVA_GEMSTORM_SEED=42 "$BIN")
B=$(NOVA_GEMSTORM_SIM=25 NOVA_GEMSTORM_SEED=42 "$BIN")
echo "$A"
echo "$A" | grep -q "hash=" || { echo "run: sim printed no hash" >&2; exit 1; }
echo "$A" | grep -q "spent=25" || { echo "run: sim did not spend its budget" >&2; exit 1; }
echo "$A" | grep -q "phase=2" || { echo "run: sim did not end the run" >&2; exit 1; }
HASH_A=$(echo "$A" | sed -n 's/.*hash=\([0-9-]*\).*/\1/p')
HASH_B=$(echo "$B" | sed -n 's/.*hash=\([0-9-]*\).*/\1/p')
[ "$HASH_A" = "$HASH_B" ] || { echo "run: sim hashes differ ($HASH_A vs $HASH_B)" >&2; exit 1; }
echo "run: 25-move hash $HASH_A stable"

echo "--- captures ---"
mkdir -p target/shots
rm -f target/shots/*.png
capture() {
    scenario=$1
    out=$(NOVA_GEMSTORM_SCENARIO="$scenario" NOVA_GEMSTORM_SHOT="target/shots/$scenario.png" "$BIN")
    echo "$out"
    echo "$out" | grep -q "saved=true" || { echo "run: $scenario did not save" >&2; exit 1; }
}

OUT=$(capture play)
echo "$OUT" | grep -q "screen=play" || { echo "run: play screen wrong" >&2; exit 1; }
echo "$OUT" | grep -q "moves=25" || { echo "run: play did not start with a full run" >&2; exit 1; }

OUT=$(capture menu)
echo "$OUT" | grep -q "screen=menu" || { echo "run: menu screen wrong" >&2; exit 1; }

OUT=$(capture hint)
echo "$OUT" | grep -q "screen=play" || { echo "run: hint screen wrong" >&2; exit 1; }

OUT=$(capture specials)
echo "$OUT" | grep -q "screen=play" || { echo "run: specials screen wrong" >&2; exit 1; }

OUT=$(capture power)
echo "$OUT" | grep -q "power=2" || { echo "run: storm charges were not armed" >&2; exit 1; }

# A move in flight: the capture must freeze a real slide frame, not the
# already-settled board.
OUT=$(capture slide)
echo "$OUT" | grep -q "anim=8" || { echo "run: slide capture is not mid-animation" >&2; exit 1; }

OUT=$(capture over)
echo "$OUT" | grep -q "phase=2" || { echo "run: over screen did not end the run" >&2; exit 1; }
echo "$OUT" | grep -q "moves=0" || { echo "run: over screen still had moves" >&2; exit 1; }

echo "--- icon ---"
OUT=$(NOVA_GEMSTORM_ICON=target/shots/icon.png "$BIN")
echo "$OUT"
echo "$OUT" | grep -q "saved=true" || { echo "run: icon did not save" >&2; exit 1; }

echo "--- pixels ---"
# The framebuffer is 1x on a standard display and 2x on Retina. Probe in
# layout points and multiply by whatever scale this machine captured at.
WIDTH=$(sips -g pixelWidth target/shots/play.png | awk '/pixelWidth/ {print $2}')
HEIGHT=$(sips -g pixelHeight target/shots/play.png | awk '/pixelHeight/ {print $2}')
SCALE=$((WIDTH / 1000))
[ "$SCALE" -ge 1 ] && [ "$SCALE" -le 2 ] || { echo "run: unexpected capture width $WIDTH" >&2; exit 1; }
[ "$HEIGHT" = "$((760 * SCALE))" ] || { echo "run: expected $((760 * SCALE)) px tall, got $HEIGHT" >&2; exit 1; }
echo "run: capture ${WIDTH}x${HEIGHT} at ${SCALE}x"
px() {
    echo $(($1 * SCALE))
}


pixel() {
    file=$1
    x=$2
    y=$3
    one="target/shots/pixel-$$.png"
    sips -c 1 1 --cropOffset "$y" "$x" "$file" --out "$one" >/dev/null
    sips -s format bmp "$one" --out "$one.bmp" >/dev/null
    offset=$(od -An -tu4 -j10 -N4 "$one.bmp" | tr -d ' ')
    od -An -tu1 -j"$offset" -N3 "$one.bmp" | awk '{print $3, $2, $1}'
    rm -f "$one" "$one.bmp"
}

# expect_pixel FILE X Y R G B LABEL; +-14 per channel.
expect_pixel() {
    file=$1
    x=$2
    y=$3
    want_r=$4
    want_g=$5
    want_b=$6
    label=$7
    got=$(pixel "$file" "$x" "$y")
    echo "$got" | awk -v r="$want_r" -v g="$want_g" -v b="$want_b" -v label="$label" '
        {
            dr = $1 - r; if (dr < 0) dr = -dr;
            dg = $2 - g; if (dg < 0) dg = -dg;
            db = $3 - b; if (db < 0) db = -db;
            if (dr <= 14 && dg <= 14 && db <= 14) exit 0;
            printf "run: %s expected %d,%d,%d got %d,%d,%d\n", label, r, g, b, $1, $2, $3;
            exit 1;
        }'
}

# Coordinates are physical (2x). Board origin is 296,116 with 60 px cells.
# The scenario pins a sapphire at (0,0); probe the filled part of its square,
# not the square's inner hole.
# Board origin is 296,116 with 60 pt cells. The scenario pins a sapphire at
# (0,0); probe the filled part of its square, not the square's inner hole.
expect_pixel target/shots/play.png "$(px 310)" "$(px 150)" 88 146 248 "sapphire-gem"
# The score card surface.
expect_pixel target/shots/play.png "$(px 60)" "$(px 185)" 19 23 38 "score-card"
# The menu PLAY pill, left of the label.
expect_pixel target/shots/menu.png "$(px 414)" "$(px 265)" 120 220 235 "menu-play"
# The over card surface between the copy and the button.
expect_pixel target/shots/over.png "$(px 500)" "$(px 400)" 28 33 52 "over-card"
# The crate at (1,2) in the specials scenario.
expect_pixel target/shots/specials.png "$(px 375)" "$(px 250)" 150 112 72 "crate"
# The prism core at (4,0).
expect_pixel target/shots/specials.png "$(px 566)" "$(px 146)" 120 220 235 "prism-core"

IW=$(sips -g pixelWidth target/shots/icon.png | awk '/pixelWidth/ {print $2}')
IH=$(sips -g pixelHeight target/shots/icon.png | awk '/pixelHeight/ {print $2}')
ISCALE=$((IW / 512))
[ "$ISCALE" -ge 1 ] && [ "$ISCALE" -le 2 ] || { echo "run: icon width $IW, expected 512 or 1024" >&2; exit 1; }
[ "$IH" = "$((512 * ISCALE))" ] || { echo "run: icon height $IH, expected $((512 * ISCALE))" >&2; exit 1; }
expect_pixel target/shots/icon.png "$((256 * ISCALE))" "$((256 * ISCALE))" 120 220 235 "icon-prism-core"

echo "run: 7 pixel probes passed"
echo "run: gemstorm acceptance ok"
