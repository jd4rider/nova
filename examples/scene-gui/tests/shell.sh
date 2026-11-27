#!/bin/sh
# ENG-04.002 / ENG-04.003 / ENG-04.004 / ENG-11.003 acceptance: capture the
# Unity-first shell from real World state, with the Metal viewport composited
# into the same bitmap. Asserts the shell regions, selected hierarchy row,
# traffic-light clearance, distinct Scene/Game tabs, a real lit Metal frame,
# editable inspector history, working toolbar/menu commands, viewport drag
# transforms, persisted panel layout, and a clean live loop. Leaves
# target/engine-*.png for review.
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
GAME_CAM=target/engine-game-camera.png
EDIT=target/engine-edit.png
UNDO=target/engine-undo.png
REDO=target/engine-redo.png
PLAY=target/editor-play.png
STEP=target/editor-step.png
MOVE=target/editor-move.png
MOVE_UNDO=target/editor-move-undo.png
ROTATE=target/editor-rotate.png
SCALE=target/editor-scale.png
ADD=target/editor-add.png
DUPLICATE=target/editor-duplicate.png
DELETE=target/editor-delete.png
SNAP=target/editor-snap.png
GRID_OFF=target/editor-grid-off.png
MENU_CUBE=target/editor-menu-cube.png
MENU_WINDOW=target/editor-menu-window.png
CAM_ORBIT=target/editor-camera-orbit.png
CAM_PAN=target/editor-camera-pan.png
CAM_ZOOM=target/editor-camera-zoom.png
CAM_GAME=target/editor-camera-game.png
LAYOUT_CHANGE=target/engine-layout-change.png
LAYOUT_RESTORED=target/engine-layout-restored.png
TRACE=target/shell-traffic.log
LAYOUT=target/layout.txt
LAYOUT_PERSIST=target/layout-persist.txt
CAM_LAYOUT=target/camera-layout.txt
rm -f "$TRACE" "$LAYOUT" "$LAYOUT_PERSIST" "$CAM_LAYOUT"
OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_GUI_TRACE="$TRACE" NOVA_SCENE_SHOT="$SHOT" ./target/nova/scene-gui)
echo "$OUT"
echo "$OUT" | grep -q "shell=captured saved=true entities=7 selected=3:1 tab=scene viewport=metal"
echo "$OUT" | grep -q "layout=252,304,196 h=1 i=1 d=1"
grep -q "traffic center_y=14 placed=14" "$TRACE" || { echo "shell: traffic lights not placed at y=14" >&2; exit 1; }
echo "shell: traffic lights placed at y=14"

GAME_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_CLICK=game NOVA_SCENE_SHOT="$GAME" ./target/nova/scene-gui)
echo "$GAME_OUT"
echo "$GAME_OUT" | grep -q "shell=captured saved=true entities=7 selected=3:1 tab=game viewport=metal"

# gui.save_png owns parent-directory creation; do not mkdir this path here.
NESTED=target/nested/capture/shell.png
rm -rf target/nested
NESTED_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_SHOT="$NESTED" ./target/nova/scene-gui)
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

# Inspector editing goes through the same row hit test and text handler as a
# real click and keystrokes. Each run starts from the demo world, applies one
# Position edit, then optionally undoes and redoes it.
EDIT_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_EDIT=position NOVA_SCENE_TEXT="2.5, 1.4, 1" NOVA_SCENE_SHOT="$EDIT" ./target/nova/scene-gui)
echo "$EDIT_OUT"
echo "$EDIT_OUT" | grep -q "history=1/1 position=2.50, 1.40, 1.00"
UNDO_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_EDIT=position NOVA_SCENE_TEXT="2.5, 1.4, 1" NOVA_SCENE_UNDO=1 NOVA_SCENE_SHOT="$UNDO" ./target/nova/scene-gui)
echo "$UNDO_OUT"
echo "$UNDO_OUT" | grep -q "history=0/1 position=0.00, 1.40, 1.00"
REDO_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_EDIT=position NOVA_SCENE_TEXT="2.5, 1.4, 1" NOVA_SCENE_UNDO=1 NOVA_SCENE_REDO=1 NOVA_SCENE_SHOT="$REDO" ./target/nova/scene-gui)
echo "$REDO_OUT"
echo "$REDO_OUT" | grep -q "history=1/1 position=2.50, 1.40, 1.00"

# Game is a real camera view, not Scene with a new label: moving the selected
# Game Camera through the inspector must change only the Game viewport. The
# camera entity itself is hidden from the Game render.
GAME_CAM_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_CLICK=game NOVA_SCENE_SELECT=camera NOVA_SCENE_EDIT=position NOVA_SCENE_TEXT="0, 2.2, 4" NOVA_SCENE_SHOT="$GAME_CAM" ./target/nova/scene-gui)
echo "$GAME_CAM_OUT"
echo "$GAME_CAM_OUT" | grep -q "entities=7 selected=6:1 tab=game viewport=metal"
echo "$GAME_CAM_OUT" | grep -q "history=1/1 position=0.00, 2.20, 4.00"

# Compare only the viewport crop so the inspector's Undo/Redo buttons or the
# Game Camera selection cannot mask whether the World actually changed. Scene
# and Game must differ, moving the camera must differ, undo must restore the
# base viewport, and redo must restore the edit exactly.
crop_view() {
    sips -c 1156 1768 --cropOffset 252 504 "$1" --out "$2.png" >/dev/null
    sips -s format bmp "$2.png" --out "$2.bmp" >/dev/null
}
crop_view "$BASE" target/base-view
crop_view "$GAME" target/game-view
crop_view "$GAME_CAM" target/game-cam-view
crop_view "$EDIT" target/edit-view
crop_view "$UNDO" target/undo-view
crop_view "$REDO" target/redo-view
if cmp -s target/base-view.bmp target/game-view.bmp; then echo "shell: Game did not use the camera view" >&2; exit 1; fi
if cmp -s target/game-view.bmp target/game-cam-view.bmp; then echo "shell: moving the Game Camera did not change the view" >&2; exit 1; fi
cmp -s target/base-view.bmp target/undo-view.bmp || { echo "shell: undo did not restore the base viewport" >&2; exit 1; }
if cmp -s target/base-view.bmp target/edit-view.bmp; then echo "shell: edit did not change the viewport" >&2; exit 1; fi
cmp -s target/edit-view.bmp target/redo-view.bmp || { echo "shell: redo did not restore the edited viewport" >&2; exit 1; }
echo "shell: Game camera and edit/undo/redo viewport checks passed"

# The toolbar and menu are clicked through handle_mouse_down. A painted button
# that lost its hit test, or a command that only changes a label, fails here.
PLAY_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=play NOVA_SCENE_SHOT="$PLAY" ./target/nova/scene-gui)
echo "$PLAY_OUT"
echo "$PLAY_OUT" | grep -q "playing=1"
STEP_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=step NOVA_SCENE_SHOT="$STEP" ./target/nova/scene-gui)
echo "$STEP_OUT"
echo "$STEP_OUT" | grep -q "step=1"

MOVE_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=move NOVA_SCENE_DRAG="80,-20" NOVA_SCENE_SHOT="$MOVE" ./target/nova/scene-gui)
echo "$MOVE_OUT"
echo "$MOVE_OUT" | grep -q "history=1/1 position=4.00, 1.40, 0.00"
echo "$MOVE_OUT" | grep -q "tool=MOVE"
MOVE_UNDO_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=move NOVA_SCENE_DRAG="80,-20" NOVA_SCENE_UNDO_AFTER=1 NOVA_SCENE_SHOT="$MOVE_UNDO" ./target/nova/scene-gui)
echo "$MOVE_UNDO_OUT"
echo "$MOVE_UNDO_OUT" | grep -q "history=0/1 position=0.00, 1.40, 1.00"

ROTATE_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=rotate NOVA_SCENE_DRAG="45,0" NOVA_SCENE_SHOT="$ROTATE" ./target/nova/scene-gui)
echo "$ROTATE_OUT"
echo "$ROTATE_OUT" | grep -q "history=1/1 position=0.00, 1.40, 1.00 rotation=0.71, 0.00, 0.71, 0.00"
SCALE_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=scale NOVA_SCENE_DRAG="60,0" NOVA_SCENE_SHOT="$SCALE" ./target/nova/scene-gui)
echo "$SCALE_OUT"
echo "$SCALE_OUT" | grep -q "scale=1.20, 2.60, 1.20"
echo "$SCALE_OUT" | grep -q "tool=SCALE"

ADD_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=add NOVA_SCENE_SHOT="$ADD" ./target/nova/scene-gui)
echo "$ADD_OUT"
echo "$ADD_OUT" | grep -q "entities=8 selected=7:1 tab=scene"
echo "$ADD_OUT" | grep -q "position=0.00, 0.50, 0.00"
echo "$ADD_OUT" | grep -q "tool=MOVE"
DUPLICATE_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=duplicate NOVA_SCENE_SHOT="$DUPLICATE" ./target/nova/scene-gui)
echo "$DUPLICATE_OUT"
echo "$DUPLICATE_OUT" | grep -q "entities=8 selected=7:1 tab=scene"
echo "$DUPLICATE_OUT" | grep -q "position=0.60, 1.40, 1.00"
DELETE_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=delete NOVA_SCENE_SHOT="$DELETE" ./target/nova/scene-gui)
echo "$DELETE_OUT"
echo "$DELETE_OUT" | grep -q "entities=6 selected=0:1 tab=scene"
echo "$DELETE_OUT" | grep -q "tool=SELECT"

SNAP_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=snap NOVA_SCENE_SHOT="$SNAP" ./target/nova/scene-gui)
echo "$SNAP_OUT"
echo "$SNAP_OUT" | grep -q "snap=0"
GRID_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_ACTION=grid NOVA_SCENE_SHOT="$GRID_OFF" ./target/nova/scene-gui)
echo "$GRID_OUT"
echo "$GRID_OUT" | grep -q "grid=0"

MENU_CUBE_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_MENU="3:0" NOVA_SCENE_SHOT="$MENU_CUBE" ./target/nova/scene-gui)
echo "$MENU_CUBE_OUT"
echo "$MENU_CUBE_OUT" | grep -q "entities=8 selected=7:1 tab=scene"
echo "$MENU_CUBE_OUT" | grep -q "tool=MOVE"
MENU_WINDOW_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_MENU="5:1" NOVA_SCENE_SHOT="$MENU_WINDOW" ./target/nova/scene-gui)
echo "$MENU_WINDOW_OUT"
echo "$MENU_WINDOW_OUT" | grep -q "layout=252,304,196 h=1 i=0 d=1"

crop_view "$GRID_OFF" target/grid-off-view
if cmp -s target/base-view.bmp target/grid-off-view.bmp; then echo "shell: Grid toggle did not change the viewport" >&2; exit 1; fi
echo "shell: toolbar, menu, drag and grid checks passed"

# Scene camera controls use the right-button path. Orbit and pan synthesize a
# press, drag and release through the same handlers as a live window; zoom uses
# the scroll handler. Game must ignore all three because it renders through its
# own World camera entity.
ORBIT_OUT=$(NOVA_SCENE_LAYOUT="$CAM_LAYOUT" NOVA_SCENE_CAMERA="orbit:80,-20" NOVA_SCENE_SHOT="$CAM_ORBIT" ./target/nova/scene-gui)
echo "$ORBIT_OUT"
echo "$ORBIT_OUT" | grep -q "camera=8.90, 2.99, 5.19"
PAN_OUT=$(NOVA_SCENE_LAYOUT="$CAM_LAYOUT" NOVA_SCENE_CAMERA="pan:40,10" NOVA_SCENE_SHOT="$CAM_PAN" ./target/nova/scene-gui)
echo "$PAN_OUT"
echo "$PAN_OUT" | grep -q "target=0.18, 0.45, -0.11"
ZOOM_OUT=$(NOVA_SCENE_LAYOUT="$CAM_LAYOUT" NOVA_SCENE_CAMERA="zoom:120" NOVA_SCENE_SHOT="$CAM_ZOOM" ./target/nova/scene-gui)
echo "$ZOOM_OUT"
echo "$ZOOM_OUT" | grep -q "camera_distance=9.96"

crop_view "$CAM_ORBIT" target/camera-orbit-view
crop_view "$CAM_PAN" target/camera-pan-view
crop_view "$CAM_ZOOM" target/camera-zoom-view
if cmp -s target/base-view.bmp target/camera-orbit-view.bmp; then echo "shell: orbit did not change the Scene view" >&2; exit 1; fi
if cmp -s target/base-view.bmp target/camera-pan-view.bmp; then echo "shell: pan did not change the Scene view" >&2; exit 1; fi
if cmp -s target/base-view.bmp target/camera-zoom-view.bmp; then echo "shell: zoom did not change the Scene view" >&2; exit 1; fi

GAME_CAMERA_OUT=$(NOVA_SCENE_LAYOUT="$CAM_LAYOUT" NOVA_SCENE_CLICK=game NOVA_SCENE_CAMERA="orbit:80,-20" NOVA_SCENE_SHOT="$CAM_GAME" ./target/nova/scene-gui)
echo "$GAME_CAMERA_OUT"
echo "$GAME_CAMERA_OUT" | grep -q "camera=6.00, 4.00, 8.00"
crop_view "$CAM_GAME" target/camera-game-view
cmp -s target/game-view.bmp target/camera-game-view.bmp || { echo "shell: Scene camera input changed the Game view" >&2; exit 1; }
echo "shell: Scene orbit/pan/zoom changed framing; Game ignored camera input"

# Layout changes use the real splitter hit test and visibility chips, then a
# second process loads the same file. The full capture must match, proving the
# panels, sizes and World data all survived the restart.
LAYOUT_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT_PERSIST" NOVA_SCENE_LAYOUT_DRAG="left:320,right:260,drawer:150" NOVA_SCENE_LAYOUT_HIDE="inspector,drawer" NOVA_SCENE_SHOT="$LAYOUT_CHANGE" ./target/nova/scene-gui)
echo "$LAYOUT_OUT"
echo "$LAYOUT_OUT" | grep -q "entities=7 selected=3:1"
echo "$LAYOUT_OUT" | grep -q "layout=320,260,150 h=1 i=0 d=0"
RESTORED_OUT=$(NOVA_SCENE_LAYOUT="$LAYOUT_PERSIST" NOVA_SCENE_SHOT="$LAYOUT_RESTORED" ./target/nova/scene-gui)
echo "$RESTORED_OUT"
echo "$RESTORED_OUT" | grep -q "entities=7 selected=3:1"
echo "$RESTORED_OUT" | grep -q "layout=320,260,150 h=1 i=0 d=0"
sips -s format bmp "$LAYOUT_CHANGE" --out target/layout-change.bmp >/dev/null
sips -s format bmp "$LAYOUT_RESTORED" --out target/layout-restored.bmp >/dev/null
cmp -s target/layout-change.bmp target/layout-restored.bmp || { echo "shell: restart did not restore the layout and live data" >&2; exit 1; }
echo "shell: layout restart restored panels and live data"

# The live path reuses the same frame targets for several frames; it must not
# crash or lose the viewport after the capture path returns.
LIVE=$(NOVA_SCENE_LAYOUT="$LAYOUT" NOVA_SCENE_FRAMES=5 ./target/nova/scene-gui)
echo "$LIVE"
echo "$LIVE" | grep -q "live=stopped frames=5 viewport=metal"

echo "shell: 13 panel checks, toolbar/menu/drag, Scene camera, Game camera and layout checks passed"
