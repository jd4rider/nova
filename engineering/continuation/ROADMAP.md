# Self-hosting roadmap

Status vocabulary: `TODO`, `READY`, `IN_PROGRESS`, `PARTIAL`, `BLOCKED`,
`VERIFIED`, `DEFERRED`. Only `VERIFIED` means acceptance evidence passed.
The detailed matrix is `PARITY.md`.

| ID | Milestone | Status | Depends on |
|---|---|---|---|
| BOOT-01 | Preserve and verify the Rust stage-0 seed | VERIFIED | — |
| COMP-01 | Nova compiler frontend: lexer, parser, sema, modules | PARTIAL | BOOT-01 |
| CODEGEN-01 | Nova LLVM IR lowering and native execution | PARTIAL | COMP-01 parser |
| CLI-01 | Nova build/run driver | TODO | CODEGEN-01.04 |
| DIFF-01 | Rust/Nova differential validation | TODO | CLI-01, COMP-01 |
| LSP-01 | Nova language server, including current Rust surface | TODO | COMP-01, CODEGEN-01, stdio |
| PERF-01 | Measured compiler/LSP performance at parity | TODO | CLI-01, LSP-01 |
| BOOT-02 | Stage-1/2/3 self-compilation and Nova-built LSP | TODO | CLI-01 |
| REL-01 | Clean-machine distribution without a Rust SDK | TODO | BOOT-02 |

## Current verified slice

- AST parser 29/29 corpus files.
- Body sema 28/0, including duplicate struct/enum checks, enum payload
bindings, String literal patterns and Int/Bool/String/enum exhaustiveness.
- Codegen 145/0 across Int/control flow, runtime-linked lists, String
operations/match, print/println, assert, value structs and enums; native binaries exit
9, 2, 5, 7, 10, 11, 12, 13, 14, 17 and 36, String-method 5, Bool-enum 7, String-enum 10 and list-slice 10, and an unmatched match panics with exit 1.
- No claim of compiler/LSP parity or speed.

## Critical path

CODEGEN-01.04 ABI → CODEGEN-01.09 runtime-linked clang invocation →
CLI-01.01 driver →
COMP-01.03 modules → LSP-01.01/02 transport/diagnostics → LSP feature slices →
DIFF-01 → BOOT-02 → PERF-01.

Do not delete `bootstrap/` before BOOT-02 passes.
