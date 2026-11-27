#!/bin/sh
# ENG-04.002 / ENG-04.003 / ENG-11.003 acceptance: capture the Unity-first
# shell from real World state, with the Metal viewport composited into the same
# bitmap. Asserts the shell regions, selected hierarchy row, traffic-light
# clearance, distinct Scene/Game tabs, a real lit Metal frame, editable
# inspector history, and a clean live loop. Leaves target/engine-*.png for
# review.
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
"$NOVA" build >/tmp/scene-gui-build.log 2>&1
mkdir -p target
SHOT=target/engine-shell.png
BASE="$SHOT"
GAME=target/engine-game.png
EDIT=target/engine-edit.png
UNDO=target/engine-undo.png
REDO=target/engine-redo.png
TRACE=target/shell-traffic.log
rm -f "$TRACE"
OUT=$(NOVA_GUI_TRACE="$TRACE" NOVA_SCENE_SHOT="$SHOT" ./target/nova/scene-gui)
echo "$OUT"
echo "$OUT" | grep -q "shell=captured saved=true entities=6 selected=3:1 tab=scene viewport=metal"
grep -q "traffic center_y=14 placed=14" "$TRACE" || { echo "shell: traffic lights not placed at y=14" >&2; exit 1; }
echo "shell: traffic lights placed at y=14"

GAME_OUT=$(NOVA_SCENE_CLICK=game NOVA_SCENE_SHOT="$GAME" ./target/nova/scene-gui)
echo "$GAME_OUT"
echo "$GAME_OUT" | grep -q "shell=captured saved=true entities=6 selected=3:1 tab=game viewport=metal"

# gui.save_png owns parent-directory creation; do not mkdir this path here.
NESTED=target/nested/capture/shell.png
rm -rf target/nested
NESTED_OUT=$(NOVA_SCENE_SHOT="$NESTED" ./target/nova/scene-gui)
echo "$NESTED_OUT"
echo "$NESTED_OUT" | grep -q "shell=captured saved=true"
echo "$NESTED_OUT" | grep -q "viewport=metal"
[ -f "$NESTED" ] || { echo "shell: save_png did not create $NESTED" >&2; exit 1; }
echo "shell: save_png created nested capture without mkdir"
rm -rf target/nested

WIDTH=$(sips -g pixelWidth "$SHOT" | awk '/pixelWidth/ {print $2}')
HEIGHT=$(sips -g pixelHeight "$SHOT" | awk '/pixelHeight/ {print $2}')
[ "$WIDTH" = "2880" ] || { echo "shell: expected width 2880, got $WIDTH" >&2; exit 1; }
[ "$HEIGHT" = "1800" ] || { echo "shell: expected height 1800, got $HEIGHT" >&2; exit 1; }
echo "shell: capture ${WIDTH}x${HEIGHT}"

pixel() {
    x=$1
    y=$2
    one="target/shell-pixel-$$.png"
    sips -c 1 1 --cropOffset "$y" "$x" "$SHOT" --out "$one" >/dev/null
    sips -s format bmp "$one" --out "$one.bmp" >/dev/null
    offset=$(od -An -tu4 -j10 -N4 "$one.bmp" | tr -d ' ')
    od -An -tu1 -j"$offset" -N3 "$one.bmp" | awk '{print $3, $2, $1}'
    rm -f "$one" "$one.bmp"
}

# expect_pixel X Y R G B LABEL; ±12 per channel, like the Metal probes.
expect_pixel() {
    x=$1
    y=$2
    want_r=$3
    want_g=$4
    want_b=$5
    label=$6
    got=$(pixel "$x" "$y")
    echo "$got" | awk -v r="$want_r" -v g="$want_g" -v b="$want_b" -v label="$label" '
        {
            dr = $1 - r; if (dr < 0) dr = -dr;
            dg = $2 - g; if (dg < 0) dg = -dg;
            db = $3 - b; if (db < 0) db = -db;
            if (dr <= 12 && dg <= 12 && db <= 12) exit 0;
            printf "shell: %s expected %d,%d,%d got %d,%d,%d\n", label, r, g, b, $1, $2, $3;
            exit 1;
        }'
}

# Physical coordinates at 2x. Menu chrome, hierarchy surface, selected Player
# row, Metal clear colour, grid overlay, textured ground, inspector and drawer.
expect_pixel 1000 20 51 51 51 "menu-bar"
expect_pixel 100 1100 37 37 38 "hierarchy-panel"
expect_pixel 300 420 4 57 94 "selected-row"
expect_pixel 700 350 13 15 20 "metal-scene-background"
expect_pixel 590 290 39 40 44 "scene-grid-line"
expect_pixel 1000 900 138 0 0 "metal-red-texture"
expect_pixel 1300 1000 0 138 0 "metal-green-texture"
expect_pixel 2600 1100 37 37 38 "inspector-panel"
expect_pixel 1000 1700 51 51 51 "drawer"

# Active tab is theme.bg with an accent underline; the other is panel. Probe
# blank padding at the left of Scene and right of Game so text cannot skew it.
expect_pixel 532 150 30 30 30 "scene-tab-active"
expect_pixel 820 150 37 37 38 "game-tab-inactive"
SHOT="$GAME"
expect_pixel 532 150 37 37 38 "scene-tab-inactive-game"
expect_pixel 820 150 30 30 30 "game-tab-active"

# Game is a real runtime view, not Scene with a new label: the pixel that is a
# grid line in Scene is Metal background in Game, while the same Metal frame
# and its textured geometry remain.
expect_pixel 590 290 13 15 20 "game-no-editor-grid"
expect_pixel 1000 900 138 0 0 "game-metal-red-texture"

# Inspector editing goes through the same row hit test and text handler as a
# real click and keystrokes. Each run starts from the demo world, applies one
# Position edit, then optionally undoes and redoes it.
EDIT_OUT=$(NOVA_SCENE_EDIT=position NOVA_SCENE_TEXT="2.5, 1.4, 1" NOVA_SCENE_SHOT="$EDIT" ./target/nova/scene-gui)
echo "$EDIT_OUT"
echo "$EDIT_OUT" | grep -q "history=1/1 position=2.50, 1.40, 1.00"
UNDO_OUT=$(NOVA_SCENE_EDIT=position NOVA_SCENE_TEXT="2.5, 1.4, 1" NOVA_SCENE_UNDO=1 NOVA_SCENE_SHOT="$UNDO" ./target/nova/scene-gui)
echo "$UNDO_OUT"
echo "$UNDO_OUT" | grep -q "history=0/1 position=0.00, 1.40, 1.00"
REDO_OUT=$(NOVA_SCENE_EDIT=position NOVA_SCENE_TEXT="2.5, 1.4, 1" NOVA_SCENE_UNDO=1 NOVA_SCENE_REDO=1 NOVA_SCENE_SHOT="$REDO" ./target/nova/scene-gui)
echo "$REDO_OUT"
echo "$REDO_OUT" | grep -q "history=1/1 position=2.50, 1.40, 1.00"

# Compare only the Scene viewport crop so the inspector's Undo/Redo buttons
# cannot mask whether the World actually changed. Undo must restore the base
# viewport, the edit must differ, and redo must restore the edit exactly.
crop_view() {
    sips -c 1156 1768 --cropOffset 252 504 "$1" --out "$2.png" >/dev/null
    sips -s format bmp "$2.png" --out "$2.bmp" >/dev/null
}
crop_view "$BASE" target/base-view
crop_view "$EDIT" target/edit-view
crop_view "$UNDO" target/undo-view
crop_view "$REDO" target/redo-view
cmp -s target/base-view.bmp target/undo-view.bmp || { echo "shell: undo did not restore the base viewport" >&2; exit 1; }
if cmp -s target/base-view.bmp target/edit-view.bmp; then echo "shell: edit did not change the viewport" >&2; exit 1; fi
cmp -s target/edit-view.bmp target/redo-view.bmp || { echo "shell: redo did not restore the edited viewport" >&2; exit 1; }
echo "shell: edit/undo/redo viewport checks passed"

# The live path reuses the same frame targets for several frames; it must not
# crash or lose the viewport after the capture path returns.
LIVE=$(NOVA_SCENE_FRAMES=5 ./target/nova/scene-gui)
echo "$LIVE"
echo "$LIVE" | grep -q "live=stopped frames=5 viewport=metal"

echo "shell: 15 panel checks passed"
