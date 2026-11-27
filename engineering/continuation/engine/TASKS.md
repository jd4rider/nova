# Nova Engine tasks

Statuses: done / in-progress / ready / blocked / planned. Every task records a
reproducible acceptance check before it is marked done. Stable identifiers are  `ENG-<milestone>.<slice>`; sub-slices append `.a`, `.b`.

## ENG-00 — Audit and architecture

- **ENG-00.001 — Ecosystem audit.** DONE.
  Evidence: `EVIDENCE.md`; `../../continuation/STATE.md`;
  `docs/engineering/ecosystem-audit.md`.
- **ENG-00.002 — Engine continuation state.** DONE. This directory, registered
  in root `AGENTS.md`.
- **ENG-00.003 — Architecture decision records.** PARTIAL. ADR-ENG-001..013
  recorded; renderer, scripting VM, physics backend and asset format ADRs are
  still open. Acceptance: each subsystem has an ADR before its first line of
  code.

## ENG-01 — Runtime core and math

- **ENG-01.00 — Scene tick write-loss regression.** DONE.
  `scene_tick` and the three setters now return a rebuilt `Scene`.
  Acceptance: `sh examples/physics-lab/tests/physics.sh` → 22 passed, 0 failed
  (baseline 20/2).
- **ENG-01.001 — Deterministic scalar and vector math.** DONE.
  Q16.16 math, sqrt, trig, vectors, quaternions, matrices. Acceptance: engine
  suite (26 passed, 0 failed), tolerance-checked rotation/projection/ortho.
- **ENG-01.002 — Generational entity handles.** DONE. Spawn/reuse/destroy,
  stale-handle rejection, idempotent destroy.
- **ENG-01.003 — Fixed timestep.** DONE. Whole-step consumption, Q16.16 alpha,
  stall clamp with `dropped_us`.
- **ENG-01.004 — Component storage.** DONE for semantics; scale gate folded into
  ENG-01.007. `world.nova`: concrete position/rotation/scale/velocity/tag
  columns, O(1) slot access through `EntityId`, idempotent despawn. Acceptance:
  spawn/recycle, component roundtrip, stale-handle zero, deferred flush.
- **ENG-01.005 — World tick.** DONE. Snapshot, gravity, integration, ordered
  deferred spawn/destroy flush at step end. Acceptance: structural test
  (dying entity simulated this step, queued spawn absent until next step) and a
  600-step golden trace hash identical across two runs.
- **ENG-01.006 — Render interpolation.** DONE. `world_lerp_position` clamps
  alpha and returns exact prev/current endpoints; midpoint asserted at 546.
- **ENG-01.007 — Headless soak and budget.** DONE. 10,000 entities over 10,000
  fixed steps in `examples/engine-soak`: 8.80–9.61 s real (5.89–6.00 user,
  2.00–2.12 sys), 10,000 live at the end, trace hash `6639442` on both runs.
  The 1,000-entity spawn/remove-half gate passes in the engine suite (27/0).
  This is a host record, not a target.
- **ENG-01.008 — Installable Studio bundle.** DONE. `examples/scene-gui` builds
  with the fresh compiler, `install-macos.sh` renders a 1024 px icon, signs and
  installs `/Applications/Nova Engine Studio.app` (0.1.0,
  `dev.nova.engine.studio`). Acceptance: `codesign --verify` clean, icon
  present, installed binary renders a 2400×1520 PNG. This was the Phase 0
  scene/inspector surface, superseded at 0.2.0 by ENG-04.002.

## ENG-02 — Native graphics foundation

- **ENG-02.001 — Device layer ADR.** DONE (ADR-ENG-008): Metal behind a thin
  Objective-C host at `runtime/metal.m`, opaque Int handles and scalar ABI,
  Nova owns the frame loop, no callbacks, explicit idempotent destroy. One
  session is implemented; the per-window handle table is deferred.
- **ENG-02.002 — Triangle.** DONE. `runtime/metal.m` + `nova-render/metal.nova`:
  real Metal window, runtime-compiled shaders, 3 submitted frames, explicit
  resize, offscreen PNG capture at 1800×1280 and 1280×960, clean idempotent
  shutdown. `sh examples/metal-lab/tests/metal.sh` exits 0, and
  `tests/shutdown.sh` runs 5/5 full teardown cycles clean (regression for the
  reported `objc_release` crash). Driver detects `@nova_metal_` and links
  Metal/MetalKit only when referenced.
- **ENG-02.003 — Textured quad.** DONE. A 64×64 four-quadrant PNG is decoded
  with ImageIO (`CGImageSource`) in `runtime/metal.m`, uploaded as a BGRA8
  texture, and sampled through a UV attribute with a linear clamp-to-edge
  sampler. A white 1×1 fallback keeps the draw path valid before load. The ABI
  stays Int handles, Ints and strings. Acceptance:
  `sh examples/metal-lab/tests/metal.sh` exits 0 with `texture=1`, captures at
  1800×1280 and 1280×960, and samples all four quadrants within ±12 at both
  sizes; `tests/shutdown.sh` stays 5/5 clean.
- **ENG-02.004 — Depth and 3D cube.** DONE. A Depth32Float attachment and a
  less-than/write depth state now render a 36-vertex cube with per-face
  colours. The camera is built in `math.nova` (`mat4_perspective`, `mat4_look_at`,
  `mat4_mul`) and sent as 16 Q16.16 Ints, transposed by the host into Metal's
  column-major `float4x4`; the shader remaps OpenGL-style NDC z [-1,1] to
  Metal's [0,1]. Acceptance: `sh examples/metal-lab/tests/cube.sh` exits 0 with
  `geometry=1`, `camera=1`, `camera_resized=1`, captures at 1800×1280 and
  1280×960, visible +Z/+Y/+X face pixels within ±12, and no hidden -Z/-X face
  leak; `metal.sh` and `shutdown.sh` stay green.

## Cross-cutting

- **ENG-X.001 — Imported qualified struct signatures.** DONE.
  `cg_unqualified` normalises `gui.Theme` to `Theme` at type lookup, and
  parameter/field/return/enum-payload types are stored unqualified.
  Acceptance: `scene-gui` builds; codegen suite 255/0. The browser can now
  migrate off primitive accessors, but has not done so yet.
- **ENG-X.002 — Float codegen and full-build evidence.** DONE.
  `examples/nova-tool/tests/codegen-float.nova`: Float parameters, literals,
  var, assignment, `fadd`, `fmul`, `fdiv`, `fcmp`. The full codegen suite is
  255/0, and `cli.sh` now builds the same source through `nova build`, links it
  and runs it with exit 7 (35/0). Wasm float behaviour remains unmeasured.
- **ENG-X.003 — Refresh stage-0 bootstrap.** BLOCKED on BOOT-02. The checked-in
  `bootstrap/nova` predates the codegen fixes. Acceptance: self-build IR
  equality plus the full suite, then replace the binary.
- **ENG-X.004 — `gui.save_png` parent directories.** DONE. The nova-gui wrapper
  now creates each missing parent with `nova_fs_mkdir` before writing, so a
  headless capture can target a new nested folder without a shell `mkdir`.
  Acceptance: `tests/shell.sh` saves `target/nested/capture/shell.png`, asserts
  `saved=true` and the file exists; the Scene/Game checks remain green.

## ENG-04 — Editor shell

- **ENG-04.001 — UX contract.** DONE (ADR-ENG-009). Unity-first information
  architecture; Godot/GDevelop/Defold may inform local patterns only; every
  panel must read real runtime state; no mock editor.
- **ENG-04.002 — Shell layout.** DONE. Hierarchy left, Scene/Game centre with
  play/pause/step, inspector right, project/console drawer bottom, dark theme.
  The 1440×900 shell is driven by one six-entity `world.nova` value: hierarchy,
  selection, perspective grid/wireframe view, inspector fields, play/step and
  console all read it. Acceptance: `sh examples/scene-gui/tests/shell.sh`
  captures 2880×1800 at 2x with `selected=3:1`, and 15 panel/selection pixel
  checks pass. The Game tab is reached through its real mouse-down hit test and
  gets its own active state, camera label and overlay-free view; the title strip
  places the traffic lights at y=14 and starts the brand at x=84. Installed as
  Studio 0.2.0; 0.3.0 adds the Metal viewport (ENG-11.003).
- **ENG-04.003 — Inspector edits and undo.** DONE. Position, rotation and
  scale rows accept composed text, parse decimal values to Q16.16, and commit
  through `world_set_position/rotation/scale`. A 16-entry editor-only command
  stack stores before/after values; Undo/Redo buttons and ⌘Z/⇧⌘Z replay it.
  Invalid input keeps the field active, and a new edit truncates redo.
  Acceptance: `sh examples/scene-gui/tests/shell.sh` captures base, edit, undo
  and redo, checks `history=1/1 → 0/1 → 1/1`, and compares Scene viewport
  crops so undo restores the base image and redo restores the edit exactly.
  Installed as Studio 0.4.0.
- **ENG-04.004 — Persisted panel layout.** DONE. Hierarchy, Inspector and the
  bottom drawer resize through splitters and hide/show through menu-bar chips.
  A `Panels` value lives in the editor `App`; `layout.txt` stores sizes and
  visibility under `~/Library/Application Support/Nova Engine Studio/` (or
  `NOVA_SCENE_LAYOUT` for tests). Hiding a panel never clears the World or the
  selection. Acceptance: `sh examples/scene-gui/tests/shell.sh` drags the
  splitters to `320,260,150`, hides Inspector and drawer through the real chip
  hit test, launches a second process and gets a byte-identical full capture
  with `layout=320,260,150 h=1 i=0 d=0`, seven entities and Player `3:1`.
  Installed as Studio 0.5.0.
- **ENG-04.006 — Working toolbar, menus and scene editing.** DONE. The centre
  toolbar now has working Play/Pause/Step, Select/Move/Rotate/Scale, Snap/Grid
  and Add/Dup/Del controls; File/Edit/Assets/GameObject/Component/Window/Help
  open real dropdowns. Move/Rotate/Scale drag in Scene with snapping and record
  one undo command on release; Add/Duplicate/Delete spawn or despawn real World
  entities. `Q`/`W`/`E`/`R`, `G`, `Shift+S`, `⌘D`, `⌘R`, `⌘Q` and `Escape`
  share the same command path, and Escape no longer quits the editor.
  Acceptance: `sh examples/scene-gui/tests/shell.sh` clicks the real toolbar
  and menu rectangles, drags Player to `4.00, 1.40, 0.00`, rotates 90°, scales
  to `1.20, 2.60, 1.20`, creates entity `7:1`, duplicates, deletes, toggles
  snap/grid, opens GameObject and Window menus, and checks drag undo. Installed
  as Studio 0.6.0.
- **ENG-04.005 — Editor camera controls.** READY. Wire the existing
  orbit/pan/zoom input slots to Studio's Scene camera through GUI events; keep
  the Game Camera entity independent. Acceptance: synthesized drags change
  Scene framing and leave Game unchanged.

## ENG-11 — 3D vertical slice

- **ENG-11.001 — Lit textured mesh.** DONE. The Metal vertex is now 48 bytes
  with a packed normal; a third geometry is a white-tinted checker cube with
  per-face normals. `nova_metal_light(handle, enabled, x, y, z)` sets one
  directional light from Q16.16 scalars, and lighting defaults off so the quad
  and colour cube are unchanged. Acceptance: `sh examples/metal-lab/tests/lit.sh`
  reports `mode=lit geometry=2 light=1 texture=1`, captures at 1800×1280 and
  1280×960, and passes 19 face/texture/lighting probes. The same red texture
  quadrant reads 213 on top, 165 on front and 149 on right.
- **ENG-11.002 — 3D camera and cube physics.** DONE in the Metal lab. The host
  exposes polled orbit/pan/zoom deltas as five Int slots, still with no
  callbacks. A lit checker cube is driven by the existing swept-AABB physics
  with gravity and bounce; `NOVA_METAL_CAMERA=front|orbit` selects the same
  camera state used by the input path. Acceptance:
  `sh examples/metal-lab/tests/physics.sh` captures the same step through two
  camera poses and a second state after 60 ticks; cube pixels move from
  `body=204,148` to `111,170`, and 11 camera/motion probes pass. The interactive
  path is `NOVA_METAL_MODE=physics` with no `NOVA_METAL_SHOT` (drag orbits,
  shift/right-drag pans, scroll zooms).
- **ENG-11.003 — Studio 3D viewport.** DONE. `runtime/metal.m` now has an
  offscreen session, a bounded 64-entry MVP draw list, a built-in checker
  texture and a shared BGRA frame. `runtime/gui.m` composites that frame into
  the shell bitmap through a weak internal symbol, so `gui.save_png` includes
  the viewport without a child window. Studio builds one Q16.16 MVP per live
  entity, keeps the grid and selected-player box as Nova GUI overlays, and Game
  suppresses those overlays while keeping the Metal scene. Acceptance:
  `sh examples/scene-gui/tests/shell.sh` reports `viewport=metal`, passes 15
  pixel checks including Metal clear/texture and grid/Game differences, and
  completes a 5-frame live loop. Both captures are 2880×1800; installed as
  Studio 0.3.0, signature verified.
- **ENG-11.004 — Studio Game Camera view.** DONE. The demo world adds a tag-6
  Game Camera at `(0, 2.60, 7.50)` with a -12° pitch. Game derives eye,
  forward and up from its World position/rotation, hides the camera entity and
  editor overlays, and leaves Scene on the fixed editor view. Moving the
  camera through the inspector changes Game without a runtime camera API.
  Acceptance: the shell test compares Scene and Game crops, then selects the
  camera, edits Position to `(0, 2.20, 4.00)` and requires the Game crop to
  change; `selected=6:1 history=1/1`.

## Next action

ENG-04.005: wire Studio's Scene orbit/pan/zoom through GUI events. Then add
gizmo handles, local/world space and multi-select/grouped undo on top of the
working drag tools. Scene save/load and free panel docking remain later editor
work.
