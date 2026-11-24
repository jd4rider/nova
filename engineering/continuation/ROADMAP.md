# Self-hosting roadmap

Status vocabulary: `TODO`, `READY`, `IN_PROGRESS`, `PARTIAL`, `BLOCKED`,
`VERIFIED`, `DEFERRED`. Only `VERIFIED` means acceptance evidence passed.

| ID | Milestone | Status | Depends on |
|---|---|---|---|
| BOOT-01 | Preserve and verify the Rust stage-0 seed | VERIFIED | — |
| COMP-01 | Nova frontend: parser, sema and module resolution | PARTIAL | BOOT-01 |
| CODEGEN-01 | Nova LLVM IR lowering and native execution | PARTIAL | COMP-01 parser |
| CLI-01 | Nova build/run driver over the Nova compiler | TODO | CODEGEN-01 |
| LSP-01 | Nova language server with the existing feature surface | TODO | COMP-01, CODEGEN-01, stdio host |
| BOOT-02 | Stage-1/2/3 self-compilation and conformance | TODO | CLI-01 |
| REL-01 | Clean-machine distribution without a Rust SDK | TODO | BOOT-02 |

## Acceptance gates

- **COMP-01**: parser corpus stays green; sema covers the language constructs
  required by the corpus and reports unsupported constructs honestly.
- **CODEGEN-01**: Nova lowers a nontrivial program through control flow, types
  and calls, and the host clang produces a native binary with verified results.
  The current straight-line Int slice is `VERIFIED`; the milestone is `PARTIAL`
  until control flow, ABI parity and driver integration land.
- **CLI-01**: `nova` itself is built in Nova and `nova build` uses the Nova
  frontend/codegen path, with a documented fallback to the seed.
- **LSP-01**: protocol transcript tests cover the previously working Rust LSP
  features; no feature regression is hidden.
- **BOOT-02**: a known-good seed builds generation 1, generation 1 builds
  generation 2, generation 2 builds generation 3, and each generation passes
  the conformance suite.

## Critical path

CODEGEN-01.03 → CODEGEN-01.04 → CLI-01.01 → COMP-01.03 → BOOT-02 → LSP-01.

Do not delete `bootstrap/` before BOOT-02 passes.
