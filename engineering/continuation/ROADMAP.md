# Self-hosting roadmap

Status vocabulary: `TODO`, `READY`, `IN_PROGRESS`, `PARTIAL`, `BLOCKED`,
`VERIFIED`, `DEFERRED`. Only `VERIFIED` means acceptance evidence passed.
The detailed matrix is `PARITY.md`.

| ID | Milestone | Status | Depends on |
|---|---|---|---|
| BOOT-01 | Preserve and verify the Rust stage-0 seed | VERIFIED | — |
| COMP-01 | Nova compiler frontend: lexer, parser, sema, modules | PARTIAL | BOOT-01 |
| CODEGEN-01 | Nova LLVM IR lowering and native execution | PARTIAL | COMP-01 parser |
| CLI-01 | Nova build/run driver | IN_PROGRESS | CODEGEN-01.04 |
| DIFF-01 | Rust/Nova differential validation | TODO | CLI-01, COMP-01 |
| LSP-01 | Nova language server, including current Rust surface | PARTIAL | COMP-01, CODEGEN-01, stdio |
| PERF-01 | Measured compiler/LSP performance at parity | TODO | CLI-01, LSP-01 |
| BOOT-02 | Stage-1/2/3 self-compilation and Nova-built LSP | PARTIAL | CLI-01 |
| REL-01 | Clean-machine distribution without a Rust SDK | TODO | BOOT-02 |

## Current verified slice

- AST parser 29/29 corpus files.
- Body sema 28/0, including duplicate struct/enum checks, enum payload
bindings, String literal patterns and Int/Bool/String/enum exhaustiveness.
- Codegen 221/0 across Int/control flow, runtime-linked lists, String
operations/match, print/println, assert, value structs and enums, generic
lists, tuples, externs and logical operators; native binaries exit
9, 2, 5, 7, 10, 11, 12, 13, 14, 17, 36, and the focused method/list/tuple
fixtures listed in `EVIDENCE.md`.
- Nova CLI project build/run/check: `nova.toml`, `[bin] path`, relative
  imports, `target/nova/<name>` and native status propagation (11/0).
- Nova LSP diagnostics slice: framed stdio transport, initialize, full-document
  sync and `publishDiagnostics`; `tests/lsp.sh` passes against the seed-built
  tool and Nova-built generations 1, 2 and 3. Completion, hover, definition,
  formatting and Studio wiring remain.
- Self-build passes through three generations: the Rust-built tool builds
  `/tmp/nova-self`, that tool builds `/tmp/nova-self-2`, and the second builds
  `/tmp/nova-self-3`. Each generation compiles the native sample, which exits 9.
  The hashes differ; this is behavioral bootstrap evidence, not a byte-identical
  fixed-point claim. The Rust seed and `nova build` remain in use.
- No claim of full compiler/LSP parity or speed.

## Critical path

CODEGEN-01.04 ABI → CODEGEN-01.09 runtime-linked clang invocation →
CLI-01.01 driver →
COMP-01.03 modules → LSP-01.01/02 transport/diagnostics (done) →
LSP feature slices → DIFF-01 → BOOT-02 → PERF-01.

Do not delete `bootstrap/` before BOOT-02 passes.
