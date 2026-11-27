# Nova Engine evidence

Date: 2026-11-27 (session 16; earlier evidence 2026-11-26)
Platform: macOS ARM64, LLVM/Clang, self-hosted Nova compiler, `main` branch,
uncommitted working tree.

## Baseline before this session

```
$ sh examples/physics-lab/tests/physics.sh
...
FAIL scene gravity moves a dynamic entity
FAIL collision exchanges velocity

passed 20, failed 2
```

Cause: `scene_tick` and three setters mutated `var` list copies and returned the
input `Scene`; writes were discarded. Fixed by returning a rebuilt `Scene`.

`scene-gui` failed to build on `BROWSER-COMP-01` (imported qualified struct
parameters), and no Float fixture existed.

## World, tick and interpolation

```
$ cd examples/engine-lab && ../../bootstrap/nova build
built .../examples/engine-lab/target/nova/engine-lab

$ ./target/nova/engine-lab
engine-lab: frames=120 steps=120 live=2 capacity=2
engine-lab: position=6,8 first_alive=true recycled_generation=2
engine-lab: rotated=0,1 dropped_us=0
engine-lab: world_live=2 world_steps=600 world_hash=...
```

```
$ ../../bootstrap/nova test
...
26 passed, 0 failed
```

New coverage: spawn/recycle with generations; position/velocity/scale roundtrip;
stale handle resolves to zero and index -1; gravity + integration after one
step (`dt = 1092`); interpolation exact at alpha 0/65536 and 546 at the
midpoint; deferred spawn absent during the step and present after flush; dying
entity simulated for its last step; 600-step trace hash identical across two
runs.

```
$ sh examples/engine-lab/tests/engine.sh     # exit 0
$ sh examples/physics-lab/tests/physics.sh   # passed 22, failed 0
```

## Float codegen

```
$ NOVA_TOOL=emit-ir NOVA_TOOL_PATH=.../codegen-float.nova nova-tool
define double @nova_user_0(ptr %arg_env, double %p0, double %p1) {
  %t0 = fadd double %p0, %p1
  %t1 = fdiv double %t0, 2.0e+00
  ...
  %t3 = fmul double %t2, 2.0e+00
  %t6 = fcmp oeq double %t5, 2.0e+00
```

`clang` accepted the IR and the binary exited 7.

```
$ sh examples/nova-tool/tests/codegen.sh
passed 255, failed 0
```

This began as codegen evidence only. The CLI suite now builds the same fixture
through the full parser → sema → LLVM → Clang path:

```
$ sh examples/nova-tool/tests/cli.sh
passed 35, failed 0
```

The produced binary exits 7, so Float is verified through `nova build`/sema on
macOS arm64. Wasm float behaviour is still unmeasured, and engine simulation
remains Q16.16 (ADR-ENG-001).

## Qualified imported struct types / scene-gui

`cg_unqualified` normalises module-qualified type names at lookup, and
signatures store the declared name. A second issue surfaced: `gui.min(w, h)`
was claimed by the `List.min()` builtin; the builtin now requires zero
arguments and a list receiver.

```
$ cd examples/scene-gui && ../../examples/nova-tool/target/nova/nova-tool build
built .../examples/scene-gui/target/nova/scene-gui
```

The checked-in stage-0 `bootstrap/nova` still reports the old `theme` error; it
was not replaced because that requires the BOOT-02 self-build gate.

## Nova Engine Studio app

```
$ sh examples/scene-gui/install-macos.sh
...
Installed /Applications/Nova Engine Studio.app
Version 0.1.0 · dev.nova.engine.studio · ad-hoc signed
```

Verification:

```
$ codesign --verify --strict "/Applications/Nova Engine Studio.app"   # clean
$ plutil -p .../Info.plist
  CFBundleName => Nova Engine Studio
  CFBundleIdentifier => dev.nova.engine.studio
  CFBundleIconFile => AppIcon
  CFBundleExecutable => nova-engine-studio
$ sips -g pixelWidth .../Resources/AppIcon.icns                       # 1024
$ NOVA_SCENE_SHOT=dist/preview-installed.png .../nova-engine-studio
scene-gui: saved=true entities=144 gravity=0
$ file dist/preview-installed.png
PNG image data, 2400 x 1520, 8-bit/color RGBA, non-interlaced
```

`gui.save_png` does not create directories; the first screenshot attempt
returned `saved=false` until `dist/` was created. The installer renders the
icon through Chrome at 1024 px, then `sips`/`iconutil` build the `.icns`.

## Soak and storage scale (ENG-01.007)

```
$ sh examples/engine-soak/tests/soak.sh
engine-soak: entities=10000 steps=10000 live=10000 capacity=10000 hash=6639442
real 9.61
user 6.00
sys 2.12

$ sh examples/engine-soak/tests/soak.sh
engine-soak: entities=10000 steps=10000 live=10000 capacity=10000 hash=6639442
real 8.80
user 5.89
sys 2.00
```

Both runs replay the same trace hash. The timing includes process start-up and
10k spawns with component writes; it is a record of this host, not a target.

Storage scale gate (part of the 27-test engine suite): spawn 1,000 entities,
destroy every even slot, assert 500 live and every stale first-generation
handle rejected, then reuse one slot with generation 2 without growing
capacity. Result: 27 passed, 0 failed.

```
$ sh examples/engine-lab/tests/engine.sh      # 27/0, exit 0
$ sh examples/physics-lab/tests/physics.sh    # 22/0
```

## Metal triangle (ENG-02.002)

```
$ sh examples/metal-lab/tests/metal.sh
metal-lab: ok=true frames=3 attempts=3 resize=1 redrawn=1 capture=1 resized_capture=1 error=
metal-lab: shutdown-complete

frame.png           PNG 1800 x 1280
frame.png.resized   PNG 1280 x  960

$ sh examples/metal-lab/tests/shutdown.sh
metal shutdown: 5/5 cycles clean
```

Each shutdown cycle opens a window, draws 3 frames, resizes, writes two PNGs,
and tears down; 5/5 exited 0 with the shutdown marker present. This is the
direct regression for the reported `nova_metal_shutdown` crash.

`runtime/metal.m` owns one session: device, queue, `MTKView`, runtime-compiled
packed-vertex shaders, geometry buffers, offscreen capture and resize. `libraries/nova-render/src/metal.nova` is the whole FFI (Int handle,
Ints, strings). The compiler driver detects `@nova_metal_` and links
Metal/MetalKit only when a program references it; non-render programs do not
pay for it. Captures were opened and checked: a centred cyan→blue→violet
triangle on the dark clear colour at both sizes.

Three defects were found and fixed rather than hidden:

1. The initial 24-byte C vertex struct was read with Metal's 16-byte `float4`
   alignment, producing an off-screen white sliver. Fixed with
   `packed_float2/packed_float4` matched to the buffer layout.
2. Driving `MTKView.currentDrawable` manually presented the same drawable twice
   and warned from CAMetalLayer. Drawing moved to an `MTKViewDelegate` and
   `[view draw]`.
3. Teardown segfaulted because a programmatic window was released by `close`
   and again by ARC. Fixed with an explicit non-released close and cleared
   delegate before releasing. The owner-supplied crash report for the old
   binary matches exactly: `EXC_BAD_ACCESS` in `objc_release`
   ← `AutoreleasePoolPage::releaseUntil` ← `objc_autoreleasePoolPop` ←
   `nova_metal_shutdown + 156`. The current binary is a different build; the
   regression below runs the full teardown repeatedly rather than once.

## Textured quad (ENG-02.003)

```
$ sh examples/metal-lab/tests/metal.sh
metal-lab: ok=true frames=3 attempts=3 resize=1 redrawn=1 texture=1 capture=1 resized_capture=1 error=
metal-lab: shutdown-complete
metal_exit=0

$ sh examples/metal-lab/tests/shutdown.sh
metal shutdown: 5/5 cycles clean
```

The 64×64 fixture `examples/metal-lab/assets/checker.png` has red top-left,
green top-right, blue bottom-left and yellow bottom-right. The test crops each
sample with `sips`, converts the 1×1 crop to uncompressed BMP and reads its BGR
bytes with `od`; no Python or image library is needed at test time. All four
quadrants at 1800×1280 and two at 1280×960 match within ±12.

The first run failed with `top-left quadrant expected 255,0,0 got 0,0,255`.
`CGBitmapContext` already places the image's first row at scanline 0, which is
Metal's texture row 0; an extra vertical flip made the quad upside down.
Removing the flip fixed all samples. This is why the check asserts pixel values
rather than only file size: the geometry, resize and file writes all passed
while the texture was inverted.

The ABI is unchanged: `nova_metal_texture(handle, path) -> Int` adds no struct
or pointer, only a path and a 0/1 result. A white 1×1 texture is bound until a
PNG loads, so the draw path never depends on load order. Texture upload is
top-down with no mipmaps or alpha blending yet.

## Depth and 3D cube (ENG-02.004)

```
$ sh examples/metal-lab/tests/cube.sh
metal-lab: ok=true mode=cube geometry=1 camera=1 camera_resized=1 frames=3 attempts=3 resize=1 redrawn=1 texture=0 capture=1 resized_capture=1 error=
metal-lab: shutdown-complete
cube.png            PNG 1800 x 1280
cube.png.resized    PNG 1280 x  960

$ sh examples/metal-lab/tests/metal.sh
metal-lab: ok=true mode= geometry=0 camera=1 camera_resized=1 frames=3 attempts=3 resize=1 redrawn=1 texture=1 capture=1 resized_capture=1 error=

$ sh examples/metal-lab/tests/shutdown.sh
metal shutdown: 5/5 cycles clean
```

The host adds a Depth32Float attachment and a less-than/write depth state. The
cube has 36 vertices and six distinct face colours; the hidden -Z face is
drawn last specifically so a broken depth test would paint over the front and
fail the centre probe. The capture tests sample the visible +Z (blue), +Y
(green) and +X (orange) faces, plus the dark background, within ±12 at both
1800×1280 and 1280×960. Hidden -X and -Z colours are absent.

The camera is entirely `math.nova`: `mat4_perspective` with 50° converted to
Q16.16 radians, `mat4_look_at` from (1.2, 1.0, 3.0), and `mat4_mul`. The 16
row-major cells cross the FFI as Ints, the host transposes them into Metal's
column-major `float4x4`, and the shader remaps z from [-1,1] to [0,1] before
the perspective divide. `camera_resized=1` records the second matrix build at
the 640×480 aspect ratio.

The first cube render was hugely zoomed and showed only two faces because the
lab passed `fx(50)` as if fov were already radians; `mat4_perspective` expects
Q16.16 radians. The engine unit test only checked z, so it had not caught the
unit mistake. Converting 50° in the lab fixed the framing and exposed the
intended three-face view. This is the second renderer defect found by looking
at the capture rather than trusting exit codes.

## Unity-first shell (ENG-04.002)

```
$ sh examples/scene-gui/tests/shell.sh
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=scene drawer=console
shell: traffic lights placed at y=14
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=game drawer=console
shell: capture 2880x1800
shell: 12 panel checks passed

$ sh examples/scene-gui/install-macos.sh
Installed /Applications/Nova Engine Studio.app
Version 0.2.0 · dev.nova.engine.studio · ad-hoc signed

$ codesign --verify --strict "/Applications/Nova Engine Studio.app"   # clean
$ NOVA_SCENE_CLICK=game NOVA_SCENE_SHOT=dist/preview-installed-game.png \
    "/Applications/Nova Engine Studio.app/Contents/MacOS/nova-engine-studio"
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=game drawer=console
PNG image data, 2880 x 1800, 8-bit/color RGBA, non-interlaced
```

The 1440×900-point shell is drawn from one six-entity `World` value. Hierarchy
rows are live slots with their handle generation; the selected Player is
`3:1`. The inspector reads transform, quaternion rotation, scale, velocity,
tag, handle, world step, gravity and live/pending counts. The Scene tab
projects the same entities through `mat4_perspective` and `mat4_look_at` into a
perspective grid and 12-edge wireframes. Play/pause/step call `world_tick`
with `fx_ratio(1, 60)`; Scene/Game tabs, project/console tabs and hierarchy or
scene selection all update this same App value.

Panel probes are physical pixels at 2x: menu bar, hierarchy surface, selected
Player row (`theme.selection`), scene background, inspector surface and bottom
drawer. The capture was opened and reviewed after the test; it shows the
hierarchy, grid, six entities, Player selection box, inspector cards and
console.

Visual review caught truncated fixed-point fields (`1.69`/`0.79` for
`1.7`/`0.8`). `fixed2` now rounds the fractional part and carries into the
whole part. No acceptance assertion covered that cosmetic value; the fix was
re-captured and the same six checks passed.

At 0.2.0 the shell's scene view was a Nova GUI Q16.16 projection, not Metal:
one PNG included all editor panels while the Metal textured quad and
perspective cube remained separate native acceptance paths. That preview
boundary was replaced by ENG-11.003 below; the 0.2.0 captures are retained as
history.

### Chrome and Game-tab fix

The title strip uses `gui.place_traffic_lights(14)` and reserves 84 points on
the left, so AppKit's close/minimize/zoom buttons no longer sit under the
"Nova Engine" title or the File/Edit menu. The trace check requires
`traffic center_y=14 placed=14`; the traffic lights are host controls and are
not part of the framebuffer capture.

Scene and Game now have separate state. The active tab gets `theme.bg` plus the
accent rule, the inactive tab stays on `theme.panel`, and Game reports
`GAME VIEW` / `Game camera`. Game suppresses the editor grid, white selection
colour and entity labels while still drawing the six world entities. The
capture test passes a click through the same `handle_mouse_down` hit test, so a
Game tab that paints but cannot be entered fails; AppKit's own event delivery is
still checked by hand. Probes cover both active-tab states, a Scene grid-line
pixel (`52,52,52`) that becomes plain background (`30,30,30`) in Game, and the
traffic-light trace.

The first shell test had to `mkdir -p target` because `gui.save_png` returned
`saved=false` for a missing parent. The nova-gui wrapper now creates each
missing directory through `nova_fs_mkdir` before writing. `tests/shell.sh`
proves it by saving `target/nested/capture/shell.png` with no pre-created
`nested` or `capture` folder, then checking the file exists.

## Lit textured cube (ENG-11.001)

```
$ sh examples/metal-lab/tests/lit.sh
metal-lab: ok=true mode=lit geometry=2 camera=1 camera_resized=1 light=1 frames=3 attempts=3 resize=1 redrawn=1 texture=1 capture=1 resized_capture=1 error=
metal-lab: shutdown-complete
lit.png             PNG 1800 x 1280
lit.png.resized     PNG 1280 x  960
lit: 19 texture/lighting checks passed
```

The vertex is now `packed_float3 position`, `packed_float2 uv`,
`packed_float4 colour`, `packed_float3 normal`: 48 bytes with no hidden
padding. A third built-in geometry is the existing cube positions and UVs with
a white tint and one normal per face. It uses the checker PNG, so each face
shows all four quadrants. Lighting is one directional light toward
`(0.45, 0.85, 0.55)`, set through `nova_metal_light(handle, enabled, x, y, z)`
with Q16.16 direction scalars. Lighting defaults off, which is why `metal.sh`
and `cube.sh` keep their previous pixels and both still exit 0.

The same red checker quadrant reads:

| Face | Normal | RGB |
|---|---|---|
| top | +Y | 213, 0, 0 |
| front | +Z | 165, 0, 0 |
| right | +X | 149, 0, 0 |

That difference is the evidence for normals and the directional light. The
front face also samples green `0,165,0`, blue `0,0,165` and yellow `165,165,0`
at 1800×1280; the resized 1280×960 capture repeats the quadrants and all three
brightness levels. A disabled light or a constant normal would collapse the
three red values; a missing UV map would remove the quadrants. The capture was
opened and reviewed: a checker-textured cube with a bright top, mid front and
darker right face.

## Camera and cube physics (ENG-11.002)

```
$ sh examples/metal-lab/tests/physics.sh
metal-lab: physics step=93 body=204,148
metal-lab: physics step=153 body=111,170
metal-lab: ok=true mode=physics geometry=2 camera=1 camera_resized=1 camera_preset=orbit light=1 frames=3 attempts=3 resize=1 redrawn=1 texture=1 capture=1 resized_capture=1 steps=153 body=111,170 error=
metal-lab: shutdown-complete
metal-lab: physics step=93 body=204,148
metal-lab: physics step=153 body=111,170
metal-lab: ok=true mode=physics geometry=2 camera=1 camera_resized=1 camera_preset=front light=1 frames=3 attempts=3 resize=1 redrawn=1 texture=1 capture=1 resized_capture=1 steps=153 body=111,170 error=
metal-lab: shutdown-complete
physics: 11 camera/motion checks passed
```

`runtime/metal.m` now accumulates left-drag orbit, shift/right-drag pan and
scroll deltas in five integer slots. Nova polls the slots with
`nova_metal_input(handle, slot)`, applies them to the Q16.16 camera and clears
them with `nova_metal_input_reset`; no callback or event struct crosses the FFI.
The camera uses spherical eye placement around a target, camera-plane pan scaled
by distance, and a clamped zoom distance. Interactive use is
`NOVA_METAL_MODE=physics` with no capture path: the app stays live and reads
the same input slots each frame.

The physics cube uses `libraries/nova-game/src/physics.nova` directly: a 130 px
body with gravity 120, bounce 80, and `ph_clamp` against a 300 px field. Position
is mapped to Q16.16 world units at 100 px per unit, then used as a model
translation on the lit checker cube. The first capture is after 93 ticks; the
resized capture is after 60 more, so `body=204,148` becomes `body=111,170`.

At the same tick, the orbit capture and the front-camera capture disagree at
stable points: orbit is green/blue/yellow where front is red/background/green.
That proves the two poses are rendered, not just printed. The test does not
synthesize AppKit mouse events; it exercises the same camera state and capture
path through two presets, while the event plumbing remains the interactive
path. The resized capture shows the moved cube's four checker quadrants.

## Studio Metal viewport (ENG-11.003)

```
$ sh examples/scene-gui/tests/shell.sh
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=scene viewport=metal drawer=console
shell: traffic lights placed at y=14
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=game viewport=metal drawer=console
shell: save_png created nested capture without mkdir
shell: capture 2880x1800
scene-gui: live=stopped frames=5 viewport=metal
shell: 15 panel checks passed
```

The viewport is an offscreen Metal render of the same six-entity World,
composited into nova-gui's sRGB bitmap. A child `MTKView` would show in the
window but not in `gui.save_png`; this path keeps one framebuffer and one
capture. `runtime/metal.m` gained an offscreen session, a bounded 64-entry MVP
draw list, a built-in four-quadrant checker texture and a shared BGRA readback.
`runtime/gui.m` gained `nova_gui_scale` and `nova_gui_metal_frame`; the latter
weak-imports the internal Metal accessor, so GUI-only programs still link
without Metal. Studio builds one Q16.16 MVP per live entity, uses the host's
checker and directional light, then draws the grid and selected-player box as
Nova overlays. Game hides those overlays but keeps the Metal scene.

Stable probes at 2x: Metal clear `13,15,20`, red checker `138,0,0`, green
checker `0,138,0`, Scene grid overlay `39,40,44`, and the same grid pixel in
Game is Metal clear `13,15,20`. The test also checks the six hierarchy rows,
selected Player row, inspector surface, drawer and both active-tab states.
The 5-frame live run reuses the frame targets after the capture path and exits
cleanly. The Metal lab regressions were rerun after the host change:
`metal.sh`, `cube.sh`, `lit.sh`, `physics.sh` and `shutdown.sh` all exit 0;
`lit` still passes 19 probes, `physics` 11, and shutdown 5/5 cycles.

The installed bundle was rebuilt as 0.3.0:

```
$ sh examples/scene-gui/install-macos.sh
Installed /Applications/Nova Engine Studio.app
Version 0.3.0 · dev.nova.engine.studio · ad-hoc signed

$ codesign --verify --strict "/Applications/Nova Engine Studio.app"   # clean
$ plutil -extract CFBundleShortVersionString raw .../Info.plist
0.3.0
$ NOVA_SCENE_SHOT=dist/preview-installed-0.3.png \
    "/Applications/Nova Engine Studio.app/Contents/MacOS/nova-engine-studio"
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=scene viewport=metal drawer=console
$ NOVA_SCENE_FRAMES=5 .../nova-engine-studio
scene-gui: live=stopped frames=5 viewport=metal
```

The Scene capture shows the lit checker ground, all six entities, the Player
selection box and label, hierarchy, inspector and console. The Game capture
shows the same Metal scene without the grid, selection box or label. Both are
2880×1800. The editor camera is fixed; orbit/pan/zoom remain in the Metal lab
and are not wired to Studio yet. There is no GPU timing, MSAA, shadow,
multi-light or multi-viewport support.

## Inspector edits and undo (ENG-04.003)

```
$ sh examples/scene-gui/tests/shell.sh
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=scene viewport=metal drawer=console history=0/0 position=0.00, 1.40, 1.00
shell: traffic lights placed at y=14
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=game viewport=metal drawer=console history=0/0 position=0.00, 1.40, 1.00
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=scene viewport=metal drawer=console history=0/0 position=0.00, 1.40, 1.00
shell: save_png created nested capture without mkdir
shell: capture 2880x1800
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=scene viewport=metal drawer=console history=1/1 position=2.50, 1.40, 1.00
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=scene viewport=metal drawer=console history=0/1 position=0.00, 1.40, 1.00
scene-gui: shell=captured saved=true entities=6 selected=3:1 tab=scene viewport=metal drawer=console history=1/1 position=2.50, 1.40, 1.00
shell: edit/undo/redo viewport checks passed
scene-gui: live=stopped frames=5 viewport=metal
shell: 15 panel checks passed
```

The test enters Position through the same row hit test a mouse uses, sends
`2.5, 1.4, 1` through `handle_text`, and presses Enter through `handle_key`.
The edit commits through `world_set_position`; Undo and Redo then click the
actual inspector buttons. The console and inspector show the same World:
`history=1/1 position=2.50, 1.40, 1.00`, then `0/1 position=0.00, 1.40,
1.00`, then `1/1 position=2.50, 1.40, 1.00`.

The acceptance compares the Scene viewport crop, not the whole window, so the
Undo/Redo chrome cannot hide a world change. `engine-undo.png` is byte-equal to
the base viewport, `engine-edit.png` differs, and `engine-redo.png` is
byte-equal to the edited viewport. The four captures are 2880×1800; the edit
and redo captures also show the Player moved in the Metal scene and the
inspector value updated.

Implementation: Position, Rotation and Scale rows open an inline field on
click. Decimal strings are parsed to Q16.16; invalid input keeps the field
active. Each commit records `EditCommand { entity, field, before, after }` and
applies through the existing World setters. A 16-entry editor-only stack
supports buttons and ⌘Z/⇧⌘Z; a new edit truncates redo. The runtime has no undo
state. Installed as Studio 0.4.0:

```
$ sh examples/scene-gui/install-macos.sh
Installed /Applications/Nova Engine Studio.app
Version 0.4.0 · dev.nova.engine.studio · ad-hoc signed

$ codesign --verify --strict "/Applications/Nova Engine Studio.app"   # clean
$ plutil -extract CFBundleShortVersionString raw .../Info.plist
0.4.0
```

The installed captures are byte-identical to the local test captures. The
first build of this slice failed with `in handle_mouse_down: cannot assign to
immutable binding app`; the click-commit path now uses a local `var app = app`.
No runtime or GUI-host change was needed.

## Persisted panels and Game Camera (ENG-04.004 / ENG-11.004)

```
$ sh examples/scene-gui/tests/shell.sh
scene-gui: shell=captured saved=true entities=7 selected=3:1 tab=scene viewport=metal drawer=console history=0/0 position=0.00, 1.40, 1.00 layout=252,304,196 h=1 i=1 d=1
shell: traffic lights placed at y=14
scene-gui: shell=captured saved=true entities=7 selected=3:1 tab=game viewport=metal drawer=console history=0/0 position=0.00, 1.40, 1.00 layout=252,304,196 h=1 i=1 d=1
shell: save_png created nested capture without mkdir
shell: capture 2880x1800
scene-gui: shell=captured saved=true entities=7 selected=3:1 tab=scene viewport=metal drawer=console history=1/1 position=2.50, 1.40, 1.00 layout=252,304,196 h=1 i=1 d=1
scene-gui: shell=captured saved=true entities=7 selected=3:1 tab=scene viewport=metal drawer=console history=0/1 position=0.00, 1.40, 1.00 layout=252,304,196 h=1 i=1 d=1
scene-gui: shell=captured saved=true entities=7 selected=3:1 tab=scene viewport=metal drawer=console history=1/1 position=2.50, 1.40, 1.00 layout=252,304,196 h=1 i=1 d=1
scene-gui: shell=captured saved=true entities=7 selected=6:1 tab=game viewport=metal drawer=console history=1/1 position=0.00, 2.20, 4.00 layout=252,304,196 h=1 i=1 d=1
shell: Game camera and edit/undo/redo viewport checks passed
scene-gui: shell=captured saved=true entities=7 selected=3:1 tab=scene viewport=metal drawer=console history=0/0 position=0.00, 1.40, 1.00 layout=320,260,150 h=1 i=0 d=0
scene-gui: shell=captured saved=true entities=7 selected=3:1 tab=scene viewport=metal drawer=console history=0/0 position=0.00, 1.40, 1.00 layout=320,260,150 h=1 i=0 d=0
shell: layout restart restored panels and live data
scene-gui: live=stopped frames=5 viewport=metal
shell: 13 panel checks, Game camera and layout checks passed
```

Panel state is a `Panels` value in the editor `App`: left/right/drawer sizes,
three visibility flags, drag state and a layout path. Splitter presses start a
drag through the same `handle_mouse_down` path a mouse uses; `event_drag` updates
clamped sizes and mouse-up writes the file. The chips use their real hit test and
toggle hierarchy, inspector or drawer without touching the World. The layout
file is plain text under `~/Library/Application Support/Nova Engine Studio/`, or
`NOVA_SCENE_LAYOUT` for the test. A second process loads
`320,260,150 h=1 i=0 d=0`, and its full BMP is byte-equal to the changed-layout
capture; both report seven entities and selected Player `3:1`.

Game now spawns a tag-6 Game Camera at `(0, 2.60, 7.50)` with a -12° pitch.
`make_game_view` derives eye, forward and up from `world_position` and
`world_rotation`; the Metal and CPU paths skip tag 6 in Game, while Scene still
draws the camera marker. The default Scene and Game viewport crops differ, and
moving the selected camera to `(0, 2.20, 4.00)` changes the Game crop again.
The camera is hidden from its own view, so the Game frame contains the same six
visible scene entities plus the new camera entity only in the hierarchy.

Installed as 0.5.0:

```
$ sh examples/scene-gui/install-macos.sh
Installed /Applications/Nova Engine Studio.app
Version 0.5.0 · dev.nova.engine.studio · ad-hoc signed

$ codesign --verify --strict "/Applications/Nova Engine Studio.app"   # clean
$ plutil -extract CFBundleShortVersionString raw .../Info.plist
0.5.0
```

The four installed captures are byte-identical to the local test captures:

```
8f5e4bc7dceb189446ab7c68faa1b6c34ce8effb  engine-scene.png
e08207c314e7d7a6a2208ac58f0b5dcb9bbdf76e  engine-game.png
eb5bfc2527450417ff8db1528de7ba66d22d11af  engine-edit.png
5c00f4a41166b7b5fb707c556ce0d632a2058c1f  engine-layout.png
```

## Working toolbar, menus and scene editing (ENG-04.006)

```
$ sh examples/scene-gui/tests/shell.sh
scene-gui: shell=captured ... playing=1 step=0 ... tool=SELECT snap=1 grid=1
scene-gui: shell=captured ... playing=0 step=1 ... tool=SELECT snap=1 grid=1
scene-gui: shell=captured ... history=1/1 position=4.00, 1.40, 0.00 ... tool=MOVE
scene-gui: shell=captured ... history=0/1 position=0.00, 1.40, 1.00 ... tool=MOVE
scene-gui: shell=captured ... history=1/1 position=0.00, 1.40, 1.00 rotation=0.71, 0.00, 0.71, 0.00 ... tool=ROTATE
scene-gui: shell=captured ... history=1/1 ... scale=1.20, 2.60, 1.20 ... tool=SCALE
scene-gui: shell=captured ... entities=8 selected=7:1 ... position=0.00, 0.50, 0.00 ... tool=MOVE
scene-gui: shell=captured ... entities=8 selected=7:1 ... position=0.60, 1.40, 1.00
scene-gui: shell=captured ... entities=6 selected=0:1 ... tool=SELECT
scene-gui: shell=captured ... snap=0 grid=1
scene-gui: shell=captured ... snap=1 grid=0
scene-gui: shell=captured ... entities=8 selected=7:1 ... tool=MOVE   # GameObject > Create Cube
scene-gui: shell=captured ... layout=252,304,196 h=1 i=0 d=1        # Window > Inspector
shell: toolbar, menu, drag and grid checks passed
shell: 13 panel checks, toolbar/menu/drag, Game camera and layout checks passed
```

`ToolState` lives in the editor `App`, separate from the World: mode, snap,
grid, open menu and the in-flight drag. The toolbar's Play/Pause/Step,
Select/Move/Rotate/Scale, Snap/Grid and Add/Dup/Del buttons all have real hit
tests in `handle_mouse_down`. Move changes X/Z from the screen delta, Rotate
composes a yaw quaternion around Y, and Scale applies a uniform factor; snapping
rounds to 0.5 m, 15° and 0.1 scale. Releasing records one `EditCommand`, so the
same Undo path restores the pre-drag transform (`history=1/1 → 0/1`).

The menu bar is no longer a painted string: File, Edit, Assets, GameObject,
Component, Window and Help open dropdowns and execute the same command
functions as the toolbar. Add spawns a tag-2 cube at `(0, 0.50, 0)`, selects it
as entity `7:1`, and switches to Move; Duplicate offsets the copy by 0.6 m;
Delete despawns the selection and selects the first live entity. Escape now
closes a menu, cancels a drag or returns to Select instead of quitting.

Installed as 0.6.0:

```
$ sh examples/scene-gui/install-macos.sh
Installed /Applications/Nova Engine Studio.app
Version 0.6.0 · dev.nova.engine.studio · ad-hoc signed

$ codesign --verify --strict "/Applications/Nova Engine Studio.app"   # clean
$ plutil -extract CFBundleShortVersionString raw .../Info.plist
0.6.0
```

All six installed captures are byte-identical to their local counterparts:

```
084b60a4b5240331b83face852b798b0eedf30d4abf3fc5ad98ea6fd10b1efd3  engine-scene.png
d65430f087939933590d4d213426c7fe8fc25aac038fd7f76ddb51598368aa11  engine-game.png
fee318511184b96542922f11d09b1d75fe7b40f1505276239e3cd5496409aaa6  engine-edit.png
c0a5f44586eac6d4c8292b58cdb896b29e92200e0e5efa23c20129fb0a7cbf9d  engine-layout.png
242c67358b240a5c4a792494271ef28e10f4a9d33d2ab032fa094bf8ee977ea4  engine-toolbar.png
565574b4b0b16706310da3fd46347bcc4294e872d4c454566045459d9ee50d14  engine-add.png
```

## Scene camera orbit/pan/zoom (ENG-04.005)

```
$ sh examples/scene-gui/tests/shell.sh
...
scene-gui: shell=captured ... camera=8.90, 2.99, 5.19 target=0.00, 0.50, 0.00 camera_distance=10.59 ...
scene-gui: shell=captured ... camera=6.18, 3.95, 7.89 target=0.18, 0.45, -0.11 camera_distance=10.59 ...
scene-gui: shell=captured ... camera=5.64, 3.79, 7.52 target=0.00, 0.50, 0.00 camera_distance=9.96 ...
scene-gui: shell=captured ... camera=6.00, 4.00, 8.00 target=0.00, 0.50, 0.00 camera_distance=10.59 ...
shell: Scene orbit/pan/zoom changed framing; Game ignored camera input
shell: 13 panel checks, toolbar/menu/drag, Scene camera, Game camera and layout checks passed
```

`CameraState { eye, target, fov }` lives in the editor `ToolState`, separate
from the World. Right-drag orbits, Shift+right-drag pans, Option+left-drag
orbits for trackpads, and scroll zooms. The handlers are the same
`handle_mouse_down_button`, `handle_drag`, `handle_mouse_up` and `handle_scroll`
path a live window uses. Orbit rotates the eye offset around the target; pan
moves eye and target together in the camera plane; zoom clamps the distance to
2.5–40 m. The default eye `(6, 4, 8)` and target `(0, 0.50, 0)` are unchanged,
so the existing pixel probes and base crop still pass.

Orbit `80,-20` moves the eye to `8.90, 2.99, 5.19`; pan `40,10` moves the
target to `0.18, 0.45, -0.11`; scroll `120` shortens the distance to `9.96 m`.
Each Scene crop differs from the base. The same orbit in Game reports the
unchanged `camera=6.00, 4.00, 8.00` and its Game crop is byte-identical to the
base Game crop, proving Scene camera input cannot move the tag-6 Game Camera.

The first build of this slice failed with `in camera_test: unknown function
handle_scroll`; the test hook was calling a wrapper that had not been written.
Adding the wrapper and routing `event_scroll` through it fixed the build. No
runtime, Metal or GUI-host change was needed.

Installed as 0.7.0:

```
$ sh examples/scene-gui/install-macos.sh
Installed /Applications/Nova Engine Studio.app
Version 0.7.0 · dev.nova.engine.studio · ad-hoc signed

$ codesign --verify --strict "/Applications/Nova Engine Studio.app"   # clean
$ plutil -extract CFBundleShortVersionString raw .../Info.plist
0.7.0
```

All eight installed captures are byte-identical to the local acceptance
captures:

```
4a5f6ef957f3e2207ddc8c1e0978583c35f4fe8b6b94a01cd30be41cb14460a1  engine-scene.png
814d46708ae5c9c233c7ec363bb781a7830d88cc4290d602444cee4659434bd4  engine-game.png
7fd488ae1a7d4f21b90e6f879c1b92b59566efe774fd681ac2c442d74de34475  engine-edit.png
28177c32693e3cbffb0d824171a973914ab916e4aebb0f8ec6e003a1283ba21c  engine-layout.png
13cfc83b657a26b1ca732339decaf6c7a5b6996ab0104c8df54ae9177b54b564  engine-toolbar.png
68bf3c03fbb1c6b4ea672339b110c7deb15e38d7be667a23b260a50058ef22ed  engine-add.png
56f3a733bd1848e7263f9b7a5da40dd981a16d61c7365fe0d206f20d695d0f88  engine-camera.png
76448254ee37a7802d9c8752618da49394888e8b213fadaee2ef4089a8a55c7b  engine-camera-zoom.png
```

## Not measured

- No frame-time breakdown, allocation count or throughput profile inside the
  soak; only whole-process wall clock.
- No GPU timing, MSAA, mipmaps, alpha blending, shadows, multiple lights or
  asset pipeline yet. One Depth32Float attachment and one texture slot.
- Inspector history is per-session and single-field. No multi-select, grouped
  commands, persisted undo, gizmo, prefab or scene save/load yet.
- Transform tools are drag-only: no gizmo handles, local/world toggle,
  multi-select or grouped undo. Menus are minimal; no keyboard navigation or
  context menus.
- Scene camera controls are mouse-only; there is no focus-selected, preset,
  keyboard fly or persisted camera. Game still has no runtime input. The shell
  test drives the same handlers, not AppKit's window server.
- Panel layout persists sizes and visibility for three fixed regions; there is
  no free docking, floating/reordered panel or multi-window editor.
- The toolbar is drawn by nova-gui and hit-tested in Nova, not a native
  `NSToolbar`; there are no icons, tooltips or overflow menu yet.
- The Game Camera is a tag convention, not a component type. FOV, near/far and
  runtime input are fixed or absent.
- Float is codegen- and full-build-verified on macOS arm64; wasm float has no
  measurement yet.
