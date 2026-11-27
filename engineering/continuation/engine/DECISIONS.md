# Nova Engine decisions

## ADR-ENG-001 — Q16.16 fixed point for simulation math
- Status: accepted; scope clarified 2026-11-26.
- Context: deterministic replay on native and wasm, and at the time no passing
  Float codegen fixture.
- Decision: simulation math stays integer Q16.16 (`fx_scale() == 65536`).
  Float is now verified through codegen and full `nova build`/sema on macOS
  arm64 (`codegen-float` and `cli.sh`) and may be used at renderer, tooling and
  asset boundaries. Do not move the fixed step or collision pipeline to Float.
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
  - centre: scene view with a compact toolbar (play, pause, step,
    select/move/rotate/scale, grid/snap, add/duplicate/delete) and a game view
    tab;
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
- Status note: Studio 0.6.0 implements this layout as a real shell over one
  `World` value (ENG-04.002/ENG-04.003/ENG-04.004/ENG-04.006/ENG-11.003/
  ENG-11.004). Editable transforms, undo/redo, persisted panel sizes/visibility,
  working toolbar/menus and viewport drag tools are verified; free docking,
  gizmos and editor camera controls remain open before ENG-04 is complete.

## ADR-ENG-010 — Studio composites an offscreen Metal frame
- Status: accepted (ENG-11.003).
- Context: `gui.save_png` captures nova-gui's CPU bitmap. A child `MTKView`
  would appear in the window but not in that PNG, and the acceptance path needs
  one capture containing both the viewport and the editor panels. Pointers and
  callbacks still cannot cross the Nova FFI.
- Decision: `runtime/metal.m` can create one offscreen session
  (`nova_metal_offscreen`) with no window. Nova fills a bounded 64-entry
  model-view-projection draw list through Int scalars
  (`nova_metal_scene_clear/add`) and calls `nova_metal_render_frame`. The host
  renders into a depth-tested BGRA texture, reads it back into a shared buffer
  and exposes it only through an internal C accessor. `runtime/gui.m` weak-
  imports that accessor and composites the frame into the same sRGB bitmap as
  the chrome. Studio builds one Q16.16 MVP per live entity, uses the built-in
  checker texture and directional light, then draws the grid and selected box
  as Nova overlays. Game skips the overlays.
- Alternatives: embed an `MTKView` child (rejected: absent from `save_png`, and
  it would sit above the CPU overlays); write a PNG per frame and reload it
  (rejected: needless I/O and decode cost); pass a pixel pointer through Nova
  (rejected: the FFI has no pointer type).
- Consequences: GUI-only programs still link without Metal because the
  accessor is weak; the host remains one session; the draw list is capped at 64
  and the frame is read back synchronously. There is no GPU timing, MSAA,
  shadow or multi-viewport support yet. Studio's Scene camera is fixed; Game
  uses the tag-6 Game Camera entity. Scene orbit/pan/zoom still needs wiring
  through GUI events before the viewport is called interactive.

## ADR-ENG-011 — Inspector edits are editor commands over World setters
- Status: accepted (ENG-04.003).
- Context: the runtime must not own selection or undo (ADR-ENG-004), and the
  editor already reads one `World` value. Directly writing component columns
  would duplicate the setters and bypass their handle checks.
- Decision: clicking a Position, Rotation or Scale row opens an inline field.
  Composed text is filtered to digits, sign, decimal point and comma, parsed
  into Q16.16, and committed only on Enter. The editor records an
  `EditCommand { entity, field, before, after }` and applies it through
  `world_set_position/rotation/scale`. Undo/redo replays the before/after value;
  a 16-entry stack is per-session, and a new edit truncates redo. Undo/Redo
  buttons and ⌘Z/⇧⌘Z share the same path. Invalid input keeps the field active.
- Alternatives: mutate World columns directly (rejected: bypasses the runtime
  contract); put undo in `world.nova` (rejected: mixes editor state with the
  runtime); full text-field widget framework (deferred: this slice needs one
  focused numeric field, not a retained widget tree).
- Consequences: history is not persisted and does not group multi-select edits;
  the inspector edits whole vectors, not individual axes; gizmos, prefabs and
  scene serialization remain future work.

## ADR-ENG-012 — Panel layout is editor-owned and persisted
- Status: accepted (ENG-04.004).
- Context: ADR-ENG-004 keeps selection, docking and undo in the editor, but the
  shell needs panel sizes and visibility to survive a restart. The runtime
  World must not gain layout state.
- Decision: Hierarchy, Inspector and the bottom drawer are three fixed regions
  with splitter hit tests and menu-bar visibility chips. Their sizes and flags
  live in a `Panels` value inside the editor `App`. On mouse-up or toggle the
  editor writes a small `layout.txt` under
  `~/Library/Application Support/Nova Engine Studio/`; `NOVA_SCENE_LAYOUT`
  overrides the path for tests. Missing or invalid values fall back to
  `252/304/196` and all panels visible. Hiding a panel never changes the World
  or selection.
- Alternatives: store layout in scene serialization (rejected: layout is not
  scene data); use `NSUserDefaults` in the GUI host (rejected: the editor owns
  its state and the host stays generic); implement free docking now (deferred:
  three persisted regions prove the state and restart path first).
- Consequences: restart restores sizes and visibility; panels cannot float,
  reorder or split into a second window yet; the layout file is plain text and
  has no migration beyond clamped defaults.

## ADR-ENG-013 — Game view uses a tagged World camera entity
- Status: accepted (ENG-11.004).
- Context: the runtime has no camera component type, and Game was still the
  fixed editor perspective with overlays hidden. The owner requires Game to
  look through the camera's perspective.
- Decision: the demo world spawns a tag-6 `Game Camera` entity at
  `(0, 2.60, 7.50)` with a -12° pitch. Game derives eye, forward and up from
  that entity's existing `world_position` and `world_rotation`, hides the
  camera entity from its own render, and leaves Scene on the fixed editor
  camera. The inspector can edit the camera like any other entity; no new
  runtime camera API or component type is introduced.
- Alternatives: add a runtime camera component now (deferred until the
  component registry exists); keep a separate editor-only camera struct
  (rejected: Game must read real World data); hard-code a second camera pose
  (rejected: it would not be inspectable or editable).
- Consequences: moving or rotating the Game Camera changes Game immediately;
  FOV, near/far and runtime input are not components yet; tag 6 is a convention
  until component types land.
