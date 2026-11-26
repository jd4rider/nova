---
title: Compiler pipeline
description: From UTF-8 source to a native executable, with every stage implemented in Nova.
---

The compiler is self-hosted. `bootstrap/nova` is a Nova-built compiler that
compiles the toolchain in `libraries/` and `examples/nova-tool` into a new
compiler. There is no Rust source and no Cargo build step.

## Stages

| Stage | Implementation | Notes |
|---|---|---|
| Lexer | **Nova** (`libraries/nova-lexer`) | Tokens, comments, strings, interpolation and spans |
| Parser | **Nova** (`libraries/nova-parser`) | Recursive descent plus Pratt expressions, arena-backed AST |
| Declaration checks | **Nova** (`libraries/nova-check/src/check.nova`) | Duplicate items, parameters and unknown declared types |
| Body semantics | **Nova** (`libraries/nova-check/src/sema.nova`) | Local types, patterns, match arm checks and exhaustiveness for the supported subset |
| LLVM IR and linking | **Nova** (`libraries/nova-codegen`) | Textual LLVM IR plus host Clang |
| CLI | **Nova** (`examples/nova-tool`) | `build`, `run`, `check`, `fmt`, `box`, `tree`, `new`, `init`, `clean`, `doctor`, `lsp` |
| Language server | **Nova** (`libraries/nova-lsp`) | Diagnostics, completion, hover, definition, symbols, formatting |
| Formatter | **Nova** (`libraries/nova-fmt`) | `nova fmt` and the LSP |
| Diagnostics | **Nova** (`libraries/nova-diag`) | Spans, codes and caret rendering |
| Package manager | **Nova** (`libraries/nova-boxes`) | Manifests, SemVer, resolution, `.box` archives and registry fetch |
| Runtime | C and Objective-C (`runtime/`) | Linked into native programs and the GUI host |

```text
bootstrap/nova        checked-in Nova-built compiler
libraries/
  nova-lexer/         tokens, comments, strings, literals
  nova-parser/        AST, recursive descent, Pratt expressions
  nova-check/         declarations and body semantics
  nova-codegen/       textual LLVM IR, Clang invocation, linking
  nova-lsp/           stdio language server
  nova-fmt/           formatter
  nova-diag/          diagnostic rendering
  nova-boxes/         Nova Boxes
examples/nova-tool/   unified command line
runtime/              C runtime, TUI and Objective-C GUI host
```

## Pipeline

1. **Lex** UTF-8 source into tokens with byte spans; newlines are significant.
2. **Parse** declarations, statements and Pratt expressions into an arena-backed AST, with recovery at statement and declaration boundaries.
3. **Check** names, local types, signatures, structs, mutability, Option/Result matches and `?` for the supported subset.
4. **Generate** textual LLVM IR, including bounds checks and runtime calls.
5. **Compile/link** through host Clang with the runtime C file and `-O0`, `-O2` or `-O3`.
6. **Execute** or report diagnostics.

The backend deliberately emits definitions only, not duplicate `declare`/`define` pairs. The LLVM verifier and optimizer run inside Clang.

## Backend and targets

There is no C transpiler and no hand-written assembly. Nova emits textual LLVM
IR and the host Clang optimizes, verifies and links it, so no libLLVM
development package is required. Native output is a Mach-O arm64 PIE executable
on the tested host; `NOVA_TARGET=wasm32-unknown-unknown` selects a separate
wasm branch that assembles with Clang and links with `wasm-ld`. Rebuilding the
browser playground still needs a Nova wasm backend; the checked-in artifacts are
frozen until then. Only macOS arm64 has been verified end to end.

ADR `0003-llvm-backend.md` records the tradeoff.

## Testing

The Nova suites cover native executables for hello world, structs, Option/match,
lists and overflow traps (`sh examples/nova-tool/tests/codegen.sh`, 248
assertions; `diff.sh`, 18 frozen results), plus diagnostics (`sema-ast.sh`,
39 assertions), the CLI and language server
(`sh examples/nova-tool/tests/toolchain.sh`, 126 assertions) and packages
(`sh examples/nova-tool/tests/boxes.sh`, 64 assertions).

The parser has malformed-input regression tests. IR validation relies on LLVM,
not a custom verifier. Fuzzing, differential interpreter/native testing and
sanitizer runs are planned.

## Honest gaps

The compiler is self-hosted but not feature-complete. Generics, traits and
closures are not implemented by the Nova compiler, sema coverage is a subset,
and there is no Nova wasm backend, debug info, DAP, incremental compilation or
reproducible-build guarantee. `nova test` and the REPL are not ported yet. See
[Status](../../status/) and the repository
[roadmap](https://github.com/jd4rider/nova/blob/main/ROADMAP.md).
