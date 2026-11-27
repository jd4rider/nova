# Nova Engine roadmap

Master directive: owner-supplied 2026-11-26. This file tracks it as
evidence-backed state.

Status vocabulary: **done** (tested), **implemented** (manual evidence),
**partial**, **ready**, **planned**, **blocked**. A milestone is a parent task,
never a session.

| ID | Deliverable | Status | Gate |
|---|---|---|---|
| ENG-00 | Audit, architecture, continuation state | partial | inventory + baselines recorded; ADRs for renderer/scripting/assets open |
| ENG-01 | Runtime core and math | done | math, handles, timestep, storage, tick, interpolation, 1k gate and 10k×10k soak measured |
| ENG-02 | Native graphics foundation | done | textured quad, PNG load, depth-tested perspective cube, resize, capture, pixel checks and teardown verified |
| ENG-03 | Scene and asset system | planned | save/reopen, stable UUIDs, incremental import |
| ENG-04 | Editor shell | partial | Unity-first layout, real hierarchy/selection and live inspector verified; edits, undo/redo and docking remain |
| ENG-05 | 2D vertical slice | partial | Breakout exists as a game; no standalone export pipeline yet |
| ENG-06 | Shared scripting ABI | planned | reflected properties, lifecycle, cross-mode values |
| ENG-07 | Compiled Nova scripting | planned | gameplay component runs in editor and export |
| ENG-08 | NovaScript core | planned | parser/VM with diagnostics and tests |
| ENG-09 | Visual scripting core | planned | graph compiles and runs headlessly |
| ENG-10 | Mixed scripting | planned | three-mode acceptance scene |
| ENG-11 | 3D vertical slice | partial | lit textured mesh, orbit camera, cube physics and Studio's offscreen Metal viewport verified; editor camera controls and richer 3D content remain |
| ENG-12 | Production editor workflows | planned | prefabs, browser, gizmos, play isolation |
| ENG-13 | Animation/audio/UI | planned | real transitions, mixer, responsive UI |
| ENG-14 | Advanced graphics | planned | PBR, shadows, particles, profiling |
| ENG-15 | Debugging and reload | planned | source/graph stepping, safe reload |
| ENG-16 | Navigation and networking | planned | tested navigation, optional multiplayer |
| ENG-17 | Packages and documentation | planned | Nova Boxes, templates, docs examples |
| ENG-18 | Export and platform expansion | planned | clean-machine builds per target |
| ENG-19 | Release hardening | planned | security review, regressions, benchmarks |

## Existing assets this builds on (verified 2026-11-26)

- `libraries/nova-engine/src/scene.nova` — Phase 0 scene: entity ids, parallel
  component columns, swept-AABB tick; simulation writes now persist.
- `libraries/nova-engine/src/world.nova` — generational component storage,
  deferred structural changes, fixed tick, interpolation, trace hash.
- `libraries/nova-engine/src/render.nova` — scene drawing on Nova GUI.
- `libraries/nova-game/src/physics.nova` — deterministic integer millipixel
  swept AABB.
- `libraries/nova-game/src/engine.nova` — playable Breakout simulation.
- `examples/engine-lab` — headless demo and the 27-test acceptance suite.
- `examples/engine-soak` — measured 10k entity × 10k step run.
- `libraries/nova-render` + `runtime/metal.m` — Metal session, textured quad,
  PNG texture load, depth-tested perspective cube, lit textured cube with
  per-face normals and one directional light, polled orbit/pan/zoom input,
  physics-driven cube, resize, offscreen PNG capture, a bounded editor draw
  list and a shared frame for GUI compositing; `examples/metal-lab` is its
  acceptance.
- `examples/scene-gui` — Nova Engine Studio 0.3.0: Unity-first shell over a real
  six-entity `world.nova` world, with hierarchy, Scene/Game tabs, live inspector,
  play/pause/step, project/console drawer and an offscreen Metal viewport
  composited into the same 2880×1800 capture; installs with an icon.
- `libraries/nova-gui` — native macOS window, events, immediate-mode drawing,
  PNG capture, Metal-frame compositing and accessibility bridge.
- Nova compiler — self-hosted, LLVM/Clang, macOS ARM64. Float is verified
  through codegen and `nova build`/sema, and module-qualified struct signatures
  have a fixture; stage-0 bootstrap still needs a gated refresh.

## Ordering rule

Vertical slices first. ENG-01 is closed with a measured soak and the 1k storage
gate. ENG-02 proves the GPU path with a textured quad and a depth-tested
perspective cube. ENG-04.002 proves the Unity-first shell against real World
state, ENG-11.001 proves lit texture and normals on Metal, ENG-11.002 proves a
polled 3D camera with a physics-driven cube, and ENG-11.003 puts that Metal
scene inside Studio's capture path. Next is ENG-04.003 (inspector edits and
undo), then ENG-04.004 (docking). Studio's editor camera remains fixed until
the GUI event path is wired to the existing camera slots. Editor panels must be
built against real runtime APIs; no mock state.
