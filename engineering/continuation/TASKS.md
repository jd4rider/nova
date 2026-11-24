# Executable tasks

Stable IDs are permanent. A parent is not VERIFIED until all required children
and the integration gate pass.

## COMP-01 — Nova frontend

### COMP-01.01 — AST parser corpus
- Status: VERIFIED
- Evidence: `examples/nova-tool/tests/parser.sh` → 50/0, 29-file corpus.
- Files: `libraries/nova-parser/src/ast.nova`.

### COMP-01.02 — Body-level semantic checker slice
- Status: PARTIAL
- Evidence: `examples/nova-tool/tests/sema-ast.sh` → 9/0; declaration suite 4/0.
- Gap: generics, methods, imported modules, builtin tables and full parity.

### COMP-01.03 — Module and imported-symbol resolution
- Status: READY
- Prerequisites: COMP-01.01, COMP-01.02.
- Files: `libraries/nova-check/src/sema.nova`, `libraries/nova-parser/src/ast.nova`.
- Steps: load relative imports without cycles; collect public declarations;
  qualify or flatten names consistently with the Rust compiler; add positive and
  negative module tests.
- Acceptance: a two-file program resolves a function across the import and an
  unknown imported name is diagnosed.
- Recovery: keep the single-file checker path unchanged; gate module loading
  behind the new function.

### COMP-01.04 — Generics and inherent methods
- Status: TODO
- Prerequisites: COMP-01.02.

## CODEGEN-01 — Nova LLVM IR lowering

### CODEGEN-01.01 — Straight-line Int functions to textual LLVM IR
- Status: VERIFIED
- Evidence: `tests/codegen.sh` → 8/0; IR assertions pass.
- Files: `libraries/nova-codegen/src/llvm.nova`.
- Scope: Int parameters/return, `let`/`var`, assignment, literals, `+ - * /`,
  direct calls and `return`.
- Rollback: additive `emit-ir` mode; Rust build untouched.

### CODEGEN-01.02 — Native execution through clang
- Status: VERIFIED
- Evidence: test compiles the emitted module with host clang and executes a
  native binary; result 9; Mach-O on this host.
- Limit: the test invokes clang; the Nova tool does not yet.

### CODEGEN-01.03 — Control flow and comparisons
- Status: READY
- Prerequisites: CODEGEN-01.02.
- Files: `libraries/nova-codegen/src/llvm.nova`, `tests/codegen.sh`.
- Steps: lower `if`/`else`, `while`, comparison operators, boolean values and
  branch labels; keep unsupported forms diagnosed.
- Acceptance: a native program with a loop and branch exits with the computed
  value; IR contains valid labels and conditional branches.
- Recovery: additive new cases; the straight-line path stays green.

### CODEGEN-01.04 — Runtime ABI alignment
- Status: TODO
- Prerequisites: CODEGEN-01.03.
- Goal: match the seed compiler's Int/Unit calling convention and entry wrapper;
  remove the temporary slice-only ABI.

## CLI-01 — Self-hosted driver

### CLI-01.01 — `nova build` via Nova frontend/codegen
- Status: TODO
- Prerequisites: CODEGEN-01.04.
- Goal: invoke the Nova compiler path from the CLI with an explicit fallback to
  the Rust seed while the port is incomplete.

## LSP-01 — Nova language server

### LSP-01.01 — Stdio streaming host primitive
- Status: TODO
- Prerequisites: COMP-01.02.
- Goal: line/byte-oriented stdin/stdout support from Nova without buffering the
  whole session; protocol transcript tests.
- Note: internal dependency, not an owner blocker.

### LSP-01.02 — Diagnostics and document sync
- Status: TODO
- Prerequisites: LSP-01.01, COMP-01.03.

## BOOT-02 — Self-compilation

### BOOT-02.01 — Generation 1 from seed
- Status: TODO
- Prerequisites: CLI-01.01.

### BOOT-02.02 — Generation 2 from generation 1
- Status: TODO
- Prerequisites: BOOT-02.01.

### BOOT-02.03 — Generation 3 and comparison
- Status: TODO
- Prerequisites: BOOT-02.02.
