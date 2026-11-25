# Executable tasks

Stable IDs are permanent. A parent is not VERIFIED until its required children
and integration gate pass. The full matrix is `PARITY.md`.

## COMP-01 — Nova compiler frontend

- COMP-01.01 — AST parser corpus: **VERIFIED**. 50/0, 29-file corpus.
- COMP-01.02 — Body semantic slice: **PARTIAL**. 28/0; declarations, scopes,
  expression/return types, calls, fields (including duplicate declarations and
  initializers), enums (duplicate/unknown variants and payload arity),
  control-flow conditions, match pattern validation, payload bindings,
  block-arm checking and Int/Bool/enum exhaustiveness.
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
- CODEGEN-01.04 — Runtime ABI alignment: **PARTIAL**. Int/Unit/String/List
  signatures and calls use `ptr %arg_env`, `nova_user_<index>`, `void` returns,
  `%nova.string` and `%nova.list` by value, and the `@main` wrapper; list
  `nova_list_at`/`nova_list_push` calls and the linked C runtime are verified.
  Evidence: 122/0.
- CODEGEN-01.06 — Strings, lists, maps, sets, structs, enums, tuples, arrays,
  Option/Result: **PARTIAL**. String literals, parameters/returns and `.len()`  are verified; `List<Int>` literals (including empty), `.len()`, by-value
  parameters/returns and runtime-linked indexing/assignment/push are verified.
  `nova_list_at` supplies bounds enforcement and `nova_list_push` supplies
  growth; both survive a return. Value
  structs are verified with nested fields, literals, reads, field assignment,
  by-value parameters/returns and recursive-layout rejection. Enums are
  verified with tags, Int payloads, construction, payload bindings, wildcard
  arms, exhaustive matching and by-value parameters/returns. Lists index, assign
  and push through `nova_list_at`/`nova_list_push`; literals use malloc-backed
  storage, and `==`/`!=` compare lengths/elements. Concat, slices, maps, sets,
  non-Int enum payloads, tuples,
  arrays and Option/Result remain. String concatenation, equality and literal
  match patterns are verified through the runtime.
- CODEGEN-01.07 — Overflow, division-by-zero, bounds and alignment checks:
  **PARTIAL**. List indexing/assignment bounds checks route through
  `nova_list_at`/`nova_panic_bounds`; overflow, division-by-zero and alignment
  checks remain.
- CODEGEN-01.08 — `for`, `match`, `break`/`continue`, short-circuit booleans:
  **PARTIAL**. Range/list/nested `for`, `break`/`continue`, Int/Bool `match`
  expressions, enum variants with Int payload bindings, and Unit block arms are
  verified natively. Sema rejects string/nested patterns and duplicate or
  unreachable arms, checks arm/binding types and enforces Int/Bool/enum
  exhaustiveness. An unmatched or empty match calls `nova_panic_str`; String
  match uses literal patterns. Only short-circuit boolean evaluation and nested
  patterns remain.
- CODEGEN-01.09 — Clang invocation and linking from the Nova tool: **VERIFIED**.
  `build-ir` writes IR, invokes clang with quoted paths, checks exit status and
  produces a native binary and compiles/links `runtime/runtime.c` for list
  runtime calls. Evidence: `tests/codegen.sh` → 122/0.
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
