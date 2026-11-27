# Nova Engine decisions

## ADR-ENG-001 — Q16.16 fixed point for simulation math
- Status: accepted; scope clarified 2026-11-26.
- Context: deterministic replay on native and wasm, and at the time no passing
  Float codegen fixture.
- Decision: simulation math stays integer Q16.16 (`fx_scale() == 65536`).
  Float is now verified at the codegen level (`codegen-float` fixture) and may
  be used at renderer, tooling and asset boundaries once Float through
  `nova build`/sema is verified. Do not move the fixed step or collision
  pipeline to Float.
- Alternatives: Float simulation (still rejected: determinism and wasm float
  behaviour are unmeasured); fixed point only forever (rejected: needless
  precision loss at the GPU boundary).
- Consequence: authors think in `fx()` units; golden tests use documented
  tolerances.

## ADR-ENG-002 — Generational entity handles, no raw indices
- Status: accepted.
- Decision: `EntityId { index, generation }`; destroy bumps the generation and
  queues the slot; spawn reuses it with the new generation. Stale handles
  resolve to `false` and index `-1`.
- Alternatives: plain integer indices (rejected: aliases recycled entities);
  pointer identity (rejected: no stable pointers across reload).
- Consequence: component columns are indexed by `id.index` only after
  `pool_is_alive`/`pool_index` succeeds. Scripts and editor selections store
  `EntityId`.

## ADR-ENG-003 — Extend the Phase 0 scene instead of replacing it
- Status: accepted.
- Decision: reuse integer millipixel physics and the scene as the 2D gameplay
  substrate. `world.nova` adds concrete component columns beside it; the scene
  migrates only when the replacement has equal tests.
- Alternatives: rewrite ECS first (rejected: loses working collision tests);
  external physics engine now (rejected: no measured need, determinism audit).
- Consequence: two representations coexist; new game code should target
  `world.nova`, not scene internals.

## ADR-ENG-004 — Runtime and editor are separate Nova modules
- Status: accepted.
- Decision: `libraries/nova-engine` never imports editor code. Editor panels
  own selection/docking/undo. A headless harness must run without a window.
- Consequence: Studio is a consumer of the runtime; runtime systems cannot
  reach editor state.

## ADR-ENG-005 — Time is integer microseconds and fixed-step
- Status: accepted.
- Decision: `Clock` stores microseconds, a fixed `step_us` and a Q16.16 alpha;
  long stalls are clamped with the excess in `dropped_us`, never replayed.
- Consequence: 60 Hz is 16,666 µs (40 µs/s short of true 1/60). Documented.

## ADR-ENG-006 — Module-qualified type names are normalised at codegen
- Status: accepted.
- Context: `gui.Theme` in a signature is rejected because the struct table is
  keyed `Theme`; this blocked `scene-gui` and every typed editor panel.
- Decision: strip everything before the last `.` when looking up a struct or
  enum type, and store parameter, field, return and enum-payload types
  unqualified. The namespace remains flat, matching the existing import model.
- Alternatives: qualify every struct/enum table entry (rejected: changes the
  ABI and all existing fixtures for no gain); require primitive accessors
  (rejected: loses the typed editor contract).
- Consequence: two modules cannot both declare `Theme`; duplicate unqualified
  names remain an error. `cg_unqualified` is the single normalisation point.

## ADR-ENG-007 — Studio is packaged from scene-gui as phase 0
- Status: accepted.
- Context: the owner tests each slice as a real app, like Nova Browser.
- Decision: `examples/scene-gui/install-macos.sh` packages the current scene +
  inspector surface as `Nova Engine Studio.app` (`dev.nova.engine.studio`),
  versioned by `VERSION`, with a rendered icon and ad-hoc signature. It is
  explicitly phase 0, not the ENG-04 editor.
- Alternatives: wait for the full editor (rejected: no testable surface);
  build a mock editor shell (rejected by the master directive).
- Consequence: every UI slice should be shippable through the same installer;
  the app bundle is a consumer, never a source of mock state.

## ADR-ENG-008 — Metal through a thin Objective-C host, not a Nova binding
- Status: accepted (ENG-02.001).
- Context: there is no GPU layer. `extern "C"` supports scalar, String and Unit
  values only; structs, pointers, callbacks and Objective-C dispatch are not
  available. `runtime/gui.m` already owns the Cocoa window and event loop, and
  the engine has no threads.
- Decision: add `runtime/metal.m` as a thin host and call it from Nova through
  `extern "C"` functions that pass opaque `Int` handles and scalars only.
  - Host owns device, command queue, `MTKView`/layer, pipelines, buffers and
    textures in a bounded handle table. Create returns a handle or `-1`;
    `metal_last_error()` returns the diagnostic as a String.
  - Nova owns the frame loop, scene traversal and draw ordering. A draw call
    passes handle, primitive count, viewport scalars and buffer offset.
  - Shaders are embedded source strings compiled at runtime; compile errors
    come back through `metal_last_error()`.
  - No callbacks: resize and close are polled once per frame. Destroy is
    explicit and idempotent. The first slice presents synchronously so a
    screenshot is stable without fences.
  - The C ABI is the portability boundary; a future Nova-native Metal binding
    must keep the same behavior and tests.
- Alternatives: Nova-native Metal binding now (rejected: needs pointer FFI,
  `dyn` dispatch and Objective-C message sends that the compiler does not
  have); stretching `gui.m` (rejected: conflates UI drawing with GPU resource
  lifetime); waiting for richer FFI (rejected: blocks the whole renderer).
- Consequences: integer handles are checked on every entry point; handle leaks
  and double-destroys need tests; one window/device per process initially; no
  multithreaded command recording; the engine remains Nova and no external
  engine/renderer is wrapped.
- Acceptance for ENG-02.002: a real Metal window clears and draws one triangle,
  survives resize, reports shader compile errors, destroys resources cleanly on
  close, and produces a captured frame.
- Implementation note (2026-11-26): the host owns one session and returns handle
  `1`; the bounded per-window handle table is deferred until multiple viewports
  exist. The external contract is unchanged (opaque Int handle). ENG-11.001 adds
  normals to the 48-byte vertex and one directional light through
  `nova_metal_light(handle, enabled, x, y, z)`; the direction is still four
  scalars, not a struct or pointer. ENG-11.002 adds five polled input slots
  (`nova_metal_input(handle, slot)` plus reset) for orbit/pan/zoom; events stay
  in AppKit and only accumulated deltas cross the ABI.

## ADR-ENG-009 — Editor UX is Unity-first, informed by open-source editors
- Status: accepted (owner preference, 2026-11-26). Applies to ENG-04 onward.
- Context: the owner tests every slice as a real app and asked for the editor to
  feel closer to Unity than to Godot, GDevelop or Defold. Those projects are
  open source and can inform specific patterns, but the engine must not become
  a Unity skin over mock state.
- Decision: the editor shell follows Unity's information architecture:
  - left column: hierarchy/tree and, when relevant, project/assets;
  - centre: scene view with a compact toolbar (play, pause, step, local/world,
    grid/snap) and a game view tab;
  - right column: inspector with component sections and add-component entry;
  - bottom: project/console drawer, collapsible;
  - top: menu bar plus a scene-identity strip, not a second toolbar;
  - dark neutral surfaces, one accent colour, selected rows and gizmo handles
    visibly distinct.
  Godot/Defold/GDevelop contribute only local patterns (for example GDevelop's
  event-sheet clarity or Defold's compact outliner), never the overall layout.
- Guardrail: every panel reads real runtime state (`world.nova`, assets,
  selection, undo). No placeholder components, fake assets or a Unity logo.
  Every editor slice must be capturable as a PNG through Nova GUI for review.
- Consequences: ENG-04 panels are designed against this layout before styling;
  if a feature cannot be wired to real state it is not shown yet.
- Status note: Studio 0.2.0 implements this layout as a real shell over one
  `World` value (ENG-04.002). Inspector editing, undo/redo and dockable panels
  remain open before ENG-04 is complete.
