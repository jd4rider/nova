# Nova Terminal decisions

## DEC-TERM-001 — Keep the pty in the Nova host, the terminal in Nova

- Status: accepted.
- Decision: `runtime/gui.m` owns fork/exec, the pty master, the reader thread,
  resize and reaping; `libraries/nova-term` owns parsing, screen state,
  scrollback and rendering.
- Why: the terminal core stays headless and testable; platform process code
  stays behind one narrow host boundary.
- Consequence: a future Windows ConPTY adapter replaces the host pool, not the
  parser or screen.

## DEC-TERM-002 — Aggregate list writes, not global alias semantics

- Status: accepted.
- Decision: index assignment through a struct field mutates the shared list
  handle; `var x = list` keeps its existing copy semantics and `push` still
  requires a mutable local.
- Why: the terminal must update a screen buffer handed to it as a value without
  silently writing to a discarded clone. Changing `var` copying globally would
  break the pinned copy fixture and unrelated code.
- Consequence: growth in a shared ring is expressed with a fixed capacity and
  an index/count pair, not `push`.

## DEC-TERM-003 — Bounded state in the core

- Status: accepted.
- Decision: 2000 scrollback lines, 512 prompt marks, 16 CSI parameter slots,
  96-cell draw runs.
- Why: hostile or very long output must not grow memory without bound.
- Consequence: old marks are shifted out; history beyond 2000 lines is dropped
  by design and is not claimed as recoverable.

## DEC-TERM-004 — Build with the current Nova tool

- Status: accepted.
- Decision: the installer defaults to
  `examples/nova-tool/target/nova/nova-tool`, with `NOVA_BIN` override.
- Why: the checked-in bootstrap seed predates imported-struct field codegen and
  cannot build the app; the current tool is the project's compiler being
  developed on `main`.
- Consequence: a clean checkout must build the tool first. The installer says
  so when the binary is missing.

## DEC-TERM-006 — Trailing-name comparison for imported types

- Status: accepted.
- Decision: sema treats `vt.Term` and `Term` as the same type name, and an
  index write through an aggregate field does not require a mutable local.
- Why: the flattened module namespace already rejects duplicate unqualified
  declarations; the two spellings refer to the same declaration. Codegen
  already normalised the same way.
- Consequence: a future real module system with per-module type identity must
  replace this with resolved type IDs, not string comparison.

## DEC-TERM-005 — Honest local packaging

- Status: accepted.
- Decision: render a real icon, bundle and ad-hoc sign locally; do not claim
  notarization or a release artifact.
- Why: the owner needs a clickable app for daily testing now; signing
  credentials are not available in this session.
