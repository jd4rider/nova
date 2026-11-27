# Nova Engine checkpoint

Updated: 2026-11-26 (Studio shell UI fix)
Branch: `main`
Master directive: owner-supplied 2026-11-26; tracked by `ROADMAP.md`.
Working tree: uncommitted. Unrelated browser/GUI changes were present before
this session and were not touched.

Active milestone: **ENG-11 — 3D vertical slice**
Active task: **ENG-11.003 — Studio 3D viewport**
Status: **ENG-01 DONE** (measured soak); **ENG-02 DONE** (device host, textured
quad, depth-tested cube, resize, capture, pixel checks and teardown verified);
**ENG-04.002 DONE** (Unity-first shell over real `World` state); **ENG-11.001
DONE** (lit textured cube); **ENG-11.002 DONE** (orbit/pan/zoom camera and
physics-driven cube in the Metal lab).

## Verified this session

| Check | Command | Result |
|---|---|---|
| Engine unit suite | `cd examples/engine-lab && ../../bootstrap/nova test` | 27 passed, 0 failed |
| Physics/scene suite | `sh examples/physics-lab/tests/physics.sh` | 22 passed, 0 failed |
| Soak | `sh examples/engine-soak/tests/soak.sh` | 10k × 10k, hash `6639442`, 8.80–9.61 s |
| Codegen suite | `sh examples/nova-tool/tests/codegen.sh` | 255 passed, 0 failed |
| Metal lab | `sh examples/metal-lab/tests/metal.sh` | 3 frames, textured quad, two PNGs, 6 pixel checks, exit 0 |
| Metal cube | `sh examples/metal-lab/tests/cube.sh` | depth-tested cube, 10 face/background checks, both sizes, exit 0 |
| Metal lit cube | `sh examples/metal-lab/tests/lit.sh` | lit textured cube, 19 face/texture/light checks, both sizes, exit 0 |
| Metal camera/physics | `sh examples/metal-lab/tests/physics.sh` | 2 camera poses, 2 simulation states, 11 checks, exit 0 |
| Metal teardown | `sh examples/metal-lab/tests/shutdown.sh` | 5/5 open/draw/capture/close cycles clean |
| Editor shell | `sh examples/scene-gui/tests/shell.sh` | 6 entities, selected `3:1`, Scene/Game active-tab capture, traffic-light trace, 12 panel checks, exit 0 |
| Studio install | `sh examples/scene-gui/install-macos.sh` | `/Applications/Nova Engine Studio.app` 0.2.0, signature verified |

Physics output:

```
$ sh examples/metal-lab/tests/physics.sh
metal-lab: physics step=93 body=204,148
metal-lab: physics step=153 body=111,170
metal-lab: ok=true mode=physics geometry=2 camera=1 camera_resized=1 camera_preset=orbit light=1 ...
metal-lab: physics step=93 body=204,148
metal-lab: physics step=153 body=111,170
metal-lab: ok=true mode=physics geometry=2 camera=1 camera_resized=1 camera_preset=front light=1 ...
physics: 11 camera/motion checks passed
```

The cube is a lit checker body driven by the integer swept-AABB physics: gravity
120, bounce 80, clamped to a 300 px field at 100 px per world unit. It is
captured at step 93 and step 153, so `body=204,148` becomes `body=111,170`.
The orbit and front presets are rendered at the same step and disagree at
stable pixels, proving the camera pose; the moved cube's four quadrants are
checked in the resized capture. `lit.sh`, `metal.sh`, `cube.sh` and
`shutdown.sh` remain green after the input/camera changes.

The checked-in `bootstrap/nova` is stage 0 and does not contain the codegen,
Metal or shell changes; use the freshly built
`examples/nova-tool/target/nova/nova-tool` for Metal, `scene-gui` and the
Studio installer. Replacing `bootstrap/nova` remains the BOOT-02 gate
(ENG-X.003).

## What exists

- `libraries/nova-engine/src/{math,core,world}.nova` — deterministic runtime
  core, component storage, fixed tick, interpolation, trace hash.
- `libraries/nova-engine/src/{scene,render}.nova` — Phase 0 2D scene and Nova
  GUI drawing.
- `libraries/nova-game/src/physics.nova` — integer swept-AABB substrate, now
  also driving the Metal lab's 3D cube.
- `libraries/nova-render/src/metal.nova` — the entire Metal FFI: Int handles,
  Ints and strings, polled input slots, geometry/MVP/light scalars, no
  callbacks.
- `runtime/metal.m` — Objective-C Metal host: window, device/queue,
  runtime-compiled shaders, lit textured cube, 48-byte vertex with normals,
  Depth32Float, Q16.16 camera/light, AppKit mouse/scroll accumulation, PNG
  decode, capture, resize, explicit idempotent shutdown.
- `examples/metal-lab` — quad, colour cube, lit cube and physics-camera
  acceptances with captures and sampled pixels.
- `examples/scene-gui` — Studio 0.2.0 Unity-first shell with a traffic-light-safe
  title strip and real Scene/Game view states; capture test and signed installer.
- `examples/engine-lab` — 27-test engine suite; `examples/engine-soak` — 10k
  measured run.

## Exact next action

1. **ENG-11.003:** embed the Metal 3D view (or an offscreen 3D render) in the
   Studio Scene tab, keeping the hierarchy/inspector/console live. Acceptance:
   one Studio PNG showing the 3D viewport with real world state.
2. Return to **ENG-04.003** (editable inspector fields and undo) and
   **ENG-04.004** (dockable panels) before declaring ENG-04 complete.
3. Keep `runtime/metal.m` to Int handles/scalars; no callback or event struct
   should cross the ABI.

## Limits

- The shell window is fixed at 1440×900 points. There is no docking, resizable
  panel split, menu command handling or multi-window editor yet.
- The shell's scene view is still a Q16.16 CPU projection drawn with Nova GUI,
  not the Metal view. The orbit camera and physics cube are in the Metal lab;
  ENG-11.003 is the embedding step.
- The Metal camera supports left-drag orbit, shift/right-drag pan and scroll
  zoom in its live path. The acceptance test exercises two camera presets, not
  synthesized AppKit mouse events.
- The physics field is a 2D X/Y plane at fixed Z; there is no 3D collision,
  rotation, contact manifold or character controller.
- One directional light and one texture slot are implemented; no shadows,
  multiple lights, smooth normals, normal mapping or material system yet.
- The Metal host owns exactly one session; no MSAA, GPU timing, mipmaps, alpha
  blending or asset pipeline yet.
- The C vertex layout is packed to 48 bytes to match
  `packed_float3/packed_float2/packed_float4/packed_float3`; changing the shader
  struct alignment without changing the buffer silently corrupts the frame.
- Float is verified at codegen only; engine simulation stays Q16.16.
- `gui.save_png` still needs its target directory to exist.
