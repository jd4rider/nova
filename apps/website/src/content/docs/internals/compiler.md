---
title: Compiler pipeline
description: From UTF-8 source to a native executable, with the actual implementation split between the Rust bootstrap and Nova tooling.
---

The compiler is still a **Rust bootstrap** in `bootstrap/`. That is deliberate
and phase 1 of [the migration](https://github.com/jd4rider/nova/blob/main/docs/rust-migration.md) is in
progress: Nova now implements the formatter, the package manager and a growing
set of compiler tools, but `nova build`, `nova run`, `nova check`, `nova test`
and `nova lsp` run the Rust pipeline.

## What is Rust and what is Nova

| Stage | Implementation | Notes |
|---|---|---|
| Lexer, parser | Rust (`nova-lexer`, `nova-parser`) | Nova versions exist in `libraries/nova-lexer` and `libraries/nova-parser` and are exercised by `nova-tool` |
| Name resolution, type checking | Rust (`nova-sema`) | Nova declaration checks exist in `libraries/nova-check`; this is not a self-hosted checker yet |
| LLVM IR, linking | Rust (`nova-codegen-llvm`) | Textual LLVM IR plus host Clang |
| CLI, LSP, browser front end | Rust (`nova-cli`, `nova-lsp`, `nova-wasm`) | The language server is not rewritten in Nova |
| Formatter | **Nova** (`libraries/nova-fmt`) | `nova fmt`, the LSP and the wasm playground all call this implementation through `nova-tool` |
| Diagnostics | **Nova** (`libraries/nova-diag`) | Rendered by `nova-tool`; the Rust compiler has its own renderer |
| Package manager | **Nova** (`libraries/nova-boxes`) | Manifests, SemVer, resolution, `.box` archives and the registry client |
| Runtime | C and Objective-C (`runtime/`) | Embedded in the compiler where used |

```text
bootstrap/
  nova-diag/          spans, diagnostics, UTF-16 mapping
  nova-lexer/         tokens, comments, strings, literals
  nova-parser/        AST, recursive descent, Pratt expressions
  nova-sema/          resolution, local inference, typed program
  nova-codegen-llvm/  textual LLVM IR, Clang invocation, linking
  nova-pkg/           manifest, lockfile and dependency import map
  nova-lsp/           stdio language server
  nova-wasm/          browser front end
  nova-cli/           unified command line

libraries/
  nova-fmt/           formatter in Nova
  nova-lexer/         lexer in Nova
  nova-parser/        parser in Nova
  nova-check/         declaration checks in Nova
  nova-diag/          diagnostic rendering in Nova
  nova-boxes/         Nova Boxes in Nova
```

## Pipeline

1. **Lex** UTF-8 source into tokens with byte spans; newlines are significant.
2. **Parse** declarations, statements and Pratt expressions into an AST with recovery at statement/declaration boundaries.
3. **Check** names, local types, signatures, structs, mutability, Option/Result matches and `?`.
4. **Lower** to the typed program with local slots and resolved callees.
5. **Generate** textual LLVM IR, including checked arithmetic helpers, bounds checks and runtime calls.
6. **Compile/link** through host Clang with the runtime C file and `-O0`, `-O2` or `-O3`.
7. **Execute** or report diagnostics.

The backend deliberately emits definitions only, not duplicate `declare`/`define` pairs. The LLVM verifier and optimizer run inside Clang.

## Backend and targets

There is no C transpiler and no hand-written assembly. Nova emits textual LLVM
IR and the host Clang optimizes, verifies and links it, so no libLLVM
development package is required. Native output is a Mach-O arm64 PIE executable
on the tested host; `NOVA_TARGET=wasm32-unknown-unknown` selects a separate
wasm branch that assembles with Clang and links with `rust-lld`/`wasm-ld`.
Only macOS arm64 has been verified end to end.

ADR `0003-llvm-backend.md` records the tradeoff.

## Testing

`cargo test --workspace` covers the Rust bootstrap, including native executables
for hello world, structs, Option/match, lists and overflow traps, plus the
browser front-end ABI tests. The Nova implementation has its own suite:
`sh examples/nova-tool/tests/toolchain.sh` (lexer, parser, checks, formatter,
diagnostics) and `sh examples/nova-tool/tests/boxes.sh` (manifests, SemVer,
resolution, archives, registry reads).

The parser has malformed-input regression tests. IR validation relies on LLVM,
not a custom verifier. Fuzzing, differential interpreter/native testing and
sanitizer runs are planned.

## Honest gaps

The compiler and language server are **not self-hosted**: sema and codegen in
Nova are later phases, and there is no separate verified MIR with basic blocks.
There is no incremental compilation, DWARF debug info, DAP, LTO validation
matrix or reproducible-build guarantee. The LSP cannot resolve registry boxes
by name yet. See [Status](../../status/) and
[the migration plan](https://github.com/jd4rider/nova/blob/main/docs/rust-migration.md).
