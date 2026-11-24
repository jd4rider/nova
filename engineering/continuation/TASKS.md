# Executable tasks

Stable IDs are permanent. A parent is not VERIFIED until its required children
and integration gate pass. The full matrix is `PARITY.md`.

## COMP-01 — Nova compiler frontend

- COMP-01.01 — AST parser corpus: **VERIFIED**. 50/0, 29-file corpus.
- COMP-01.02 — Body semantic slice: **PARTIAL**. 9/0; declarations, scopes,
  expression/return types, calls, fields, control-flow conditions.
- COMP-01.03 — Module/import resolution: **READY**. Load relative imports,
  collect declarations, diagnose unknown imports, test a two-file program.
- COMP-01.04 — Generics, methods, traits, closures: **TODO**.
- COMP-01.05 — Lexer parity, comments/interpolation/recovery: **TODO**.
- COMP-01.06 — Diagnostic parity: codes, spans, incomplete-source recovery:
  **TODO**.
- COMP-01.07 — Full type system (collections, Option/Result, arrays/slices):
  **TODO**.
- COMP-01.08 — Incomplete-source AST/sema for the LSP: **TODO**.
- COMP-01.09 — FFI/module runtime semantics: **TODO**.

## CODEGEN-01 — Nova LLVM IR lowering

- CODEGEN-01.01 — Straight-line Int functions: **VERIFIED**. 15/0 evidence.
- CODEGEN-01.02 — Native execution through clang: **VERIFIED**. Native results
  9, 2 and 17; Mach-O on this host.
- CODEGEN-01.03 — Control flow and comparisons: **VERIFIED**. `if`/`else`,
  `while`, `icmp`, labels; loop/branch binary exits 17.
- CODEGEN-01.05 — Immutable-binding SSA lowering: **VERIFIED**. No `alloca` for
  params/`let`; mutable fixture exits 2.
- CODEGEN-01.04 — Runtime ABI alignment: **PARTIAL**. Int signatures/calls now
  use `ptr %arg_env`, `nova_user_<index>` symbols and the `@main` wrapper;
  Unit/void, strings/lists and runtime-call ABI remain. Evidence: 17/0.
- CODEGEN-01.06 — Strings, lists, maps, sets, structs, enums, tuples, arrays,
  Option/Result: **TODO**.
- CODEGEN-01.07 — Overflow, division-by-zero, bounds and alignment checks:
  **TODO**.
- CODEGEN-01.08 — `for`, `match`, `break`/`continue`, short-circuit booleans:
  **TODO**.
- CODEGEN-01.09 — Clang invocation and linking from the Nova tool: **VERIFIED**.
  `build-ir` writes IR, invokes clang with quoted paths, checks exit status and
  produces a native binary. Evidence: `tests/codegen.sh` → 17/0.
- CODEGEN-01.10 — Optimization pipeline parity: **TODO**.

## CLI-01 — Self-hosted driver

- CLI-01.01 — `nova build` via the Nova frontend/codegen with seed fallback:
  **TODO**, depends CODEGEN-01.04/01.09.

## LSP-01 — Nova language server

- LSP-01.01 — Stdio/JSON-RPC streaming primitive: **TODO** (internal dependency).
- LSP-01.02 — initialize, capabilities, document sync, diagnostics: **TODO**.
- LSP-01.03 — Completion: **TODO**.
- LSP-01.04 — Hover, definition, document symbols: **TODO**.
- LSP-01.05 — Whole-document formatting: **TODO**.
- LSP-01.06 — Incremental sync, stale documents, incomplete source, error
  recovery: **TODO** (beyond the current Rust server).
- LSP-01.07 — Rename, references, semantic tokens, code actions, cancellation:
  **TODO** (beyond the current Rust server).
- LSP-01.08 — Protocol transcript tests and Studio integration: **TODO**.

## DIFF-01 — Differential validation

- DIFF-01.01 — Same-program Rust/Nova compile/run differential harness: **TODO**.
- DIFF-01.02 — Golden invalid-program diagnostics comparison: **TODO**.

## PERF-01 — Performance at parity

- PERF-01.01 — Comparable benchmark harness: **TODO**, depends CLI-01.01 and
  LSP-01.02. Rules: `PERFORMANCE.md`. No faster-than-Rust claim before it passes.

## BOOT-02 — Self-compilation

- BOOT-02.01 — Generation 1 from the seed: **TODO**.
- BOOT-02.02 — Generation 2 from generation 1: **TODO**.
- BOOT-02.03 — Generation 3 and comparison: **TODO**.
- BOOT-02.04 — Build the Nova LSP with the self-hosted compiler: **TODO**.
- BOOT-02.05 — Studio uses the Nova-built LSP: **TODO**.
