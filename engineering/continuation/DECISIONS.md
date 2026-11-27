# Decisions

## DEC-001 — Branch isolation for self-hosting
- Status: accepted.
- Decision: all self-hosting work stays on `selfhost/pure-nova` until the Nova
  compiler runs end to end.
- Alternatives: land slices on `main`; rejected because the Rust seed must stay
  usable and the branch includes incomplete compiler stages.
- Consequence: do not merge or claim completion early.

## DEC-002 — Arena AST with index children
- Status: accepted.
- Decision: `libraries/nova-parser/src/ast.nova` stores nodes in a flat list and
  child indices in a shared link list.
- Alternatives: recursive structs/enums; rejected because the Nova compiler's
  recursive value layout and ownership are not proven for this shape.
- Consequence: traversal uses `a_child(ast, node, index)`; spans currently carry
  line/column, not byte offsets.

## DEC-003 — Conservative sema
- Status: accepted for the current slice.
- Decision: unknown host/imported constructs return type `?` instead of guessed
  types; only provable mismatches are errors.
- Alternatives: reject everything unresolved; rejected because imported host
  APIs would drown real diagnostics.
- Consequence: sema is not yet a complete acceptance gate.

## DEC-004 — Keep the Rust seed until stage-2/3 passes

> Superseded 2026-11-26: the Nova compiler self-builds with identical IR and
> every suite passes from the checked-in bootstrap, so the Rust workspace was
> removed. The Rust-only language features it provided are now tracked as
> parity gaps, not as a retained dependency.
- Status: accepted.
- Decision: do not delete `bootstrap/` while it is the only compiler that can
  build `nova`.
- Alternatives: delete first and bootstrap from a partial Nova compiler;
  rejected as an unrecoverable development path.

## DEC-005 — Temporary self-contained codegen ABI
- Status: accepted for `CODEGEN-01.01`.
- Decision: the first Nova IR slice emits self-contained `i64` functions and a
  C `i32 main` wrapper without the Rust compiler's hidden environment pointer.
- Alternatives: match the full runtime ABI immediately; deferred to
  `CODEGEN-01.04` because it would block a smaller native-execution proof.
- Consequence: the slice proves lowering and clang execution, not ABI parity.

## DEC-006 — No claim without evidence
- Status: accepted.
- Decision: status `VERIFIED` requires a command, result and commit/evidence
  reference. A green build alone is not semantic correctness.

## DEC-007 — Slots are allocated once, in the entry block
- Status: accepted (2026-11-27).
- Decision: `alloca` for locals, loop iterators, `match`/`?` temporaries and
  aggregate scratch space goes through `cg_entry_slot` and lands in the
  function's entry block. Only the `store` that initialises a binding stays at
  the declaration point.
- Alternatives: `llvm.stacksave`/`stackrestore` around loop bodies; rejected as
  more IR and more room for an unbalanced pair. Leaving allocas in place and
  documenting the leak; rejected because it kills long-running apps.
- Consequence: an `alloca` in a loop body is now a codegen bug by definition.
  It was the cause of the Nova Terminal SIGSEGV: 16 bytes per main-loop
  iteration exhausted the 8 MB stack in ~37 minutes (see
  `terminal/EVIDENCE.md`). `codegen.sh` enforces it —
  `codegen-loop-slots.nova` declares slots inside loops and the suite fails if
  any `alloca` follows a function's first block label.
