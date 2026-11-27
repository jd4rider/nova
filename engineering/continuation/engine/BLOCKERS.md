# Nova Engine blockers

## Resolved

- **BROWSER-COMP-01 — imported qualified struct signatures.** Fixed 2026-11-26
  in codegen: `cg_unqualified` maps `gui.Theme` to `Theme` at every type lookup,
  and signatures store the declared name. `examples/scene-gui` builds with a
  fresh compiler; codegen suite 255/0. The browser side still uses primitive
  accessors and can migrate independently.
- **Codegen Float.** Fixed 2026-11-26: `Float` maps to `double`; literals,
  parameters, `let`/`var`, assignment, `fadd/fsub/fmul/fdiv/frem` and
  `fcmp` are lowered. `codegen-float.nova` compiles and exits 7; suite 255/0.
- **`min`/`max` builtin collision.** `gui.min(a, b)` was claimed by the
  `List.min()` builtin. The builtin now applies only with zero arguments and a
  list receiver, so typed user helpers resolve normally.

## Open

- **Stage-0 bootstrap is stale.** `bootstrap/nova` predates the codegen fixes
  and still rejects `gui.Theme`; it cannot build `scene-gui`. The install
  script and scene-gui checks use the freshly built compiler. Replacing
  `bootstrap/nova` requires the BOOT-02 gate (self-build IR equality plus every
  suite), so it was not done as part of a feature change.
- **Float is unverified above codegen.** No evidence that `nova build`/sema and
  the test runner accept Float arithmetic, and no wasm measurement. Simulation
  stays fixed point until that exists.
- **No generic structs or methods.** Component storage cannot be a generic
  sparse set; it is concrete per component. This shapes `world.nova` and will
  shape scripting component storage.
- **No threads, jobs, async or channels.** The job system cannot start; do not
  claim parallelism.
- **No DWARF/debug info.** Script/engine debugging (ENG-15) is blocked.
- **No wasm backend.** Portability claims beyond the frozen playground are
  unsupported.
- **Memory is process-lifetime.** No Drop/GC; long editor sessions and asset
  reload need explicit budgets and leak measurements.
- **Metal host is single-session.** ENG-02.002 implements one window/device.
  A per-viewport handle table is needed before the editor can show multiple
  scene/game views; keep the Int-handle ABI while adding it.
- **Metal host requires the stage-1 compiler.** The checked-in bootstrap does
  not know `@nova_metal_`; build `examples/nova-tool` first (ENG-X.003).
- **No depth, MSAA, GPU timing, texture loading or asset pipeline yet.** The
  triangle proves the device path only.
- **No asset database, importer, UUID store or scene serialization.** ENG-03 is
  greenfield.
- **`gui.save_png` does not create directories.** Screenshot tests must
  `mkdir -p` the target first; otherwise the call returns `saved=false`.

## Process

- Publishing, signing identities, release artifacts and visibility changes
  require owner authorization. The Studio bundle is ad-hoc signed for local
  testing only.
- The working tree contains unrelated uncommitted browser/GUI changes; engine
  work must not revert or stash them.
