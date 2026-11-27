# Nova Engine evidence

Date: 2026-11-26
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

This proves codegen lowering only. Float through `nova build`/sema and wasm is
not verified, and engine simulation remains Q16.16 (ADR-ENG-001).

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

The shell's scene view is deliberately a Nova GUI Q16.16 projection, not the
Metal window: one PNG can then include all editor panels. The Metal textured
quad and depth-tested cube remain separate native acceptance paths. This is a
preview boundary, not a second renderer claim.

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

## Not measured

- No frame-time breakdown, allocation count or throughput profile inside the
  soak; only whole-process wall clock.
- No GPU timing, MSAA, mipmaps, alpha blending, shadows, multiple lights or
  asset pipeline yet. One Depth32Float attachment and one texture slot.
- The camera is orbit/pan/zoom around a target, not a full editor viewport;
  Studio still uses its CPU wireframe preview. Mouse events are not synthesized
  by the acceptance test.
- Float remains codegen-only evidence; no wasm float measurement.
