---
title: Compiler pipeline
description: From UTF-8 source to a native executable, with the actual crates and checks.
---

Nova is a Rust workspace. Compiler libraries are independent of Electron and can be used from the CLI or another editor.

```text
bootstrap/
  nova-diag/          spans, diagnostics, UTF-16 mapping
  nova-lexer/         tokens, comments, strings, literals
  nova-parser/        AST, recursive descent, Pratt expressions
  nova-sema/          resolution, local inference, typed IR
  nova-codegen-llvm/  textual LLVM IR, Clang invocation, linking
  nova-runtime/       C ABI and runtime helpers
  nova-pkg/           manifest and lockfile
  nova-lsp/           stdio language server
  nova-wasm/          browser front end for the playground
  nova-cli/           unified command line
```

## Pipeline

1. **Lex** UTF-8 source into tokens with byte spans; newlines are significant.
2. **Parse** declarations, statements and Pratt expressions into an AST with recovery at statement/declaration boundaries.
3. **Check** names, local types, signatures, structs, mutability, Option/Result matches and `?`.
4. **Lower** to typed IR with local slots and resolved callees.
5. **Generate** textual LLVM IR, including checked arithmetic helpers, bounds checks and runtime calls.
6. **Compile/link** through host Clang with the runtime C file and `-O0`, `-O2` or `-O3`.
7. **Execute** or report diagnostics.

The backend deliberately emits definitions only, not duplicate `declare`/`define` pairs, and uses LLVM aggregate extraction correctly. The LLVM verifier and optimizer run inside Clang.

## Why textual IR

`inkwell`/`llvm-sys` couple builds to a specific libLLVM installation. The host Clang already ships LLVM and the platform linker, so Nova emits textual IR and invokes it. This keeps setup small and remains a real optimizing native backend. ADR `0003-llvm-backend.md` records the tradeoff.

## Testing

`cargo test --workspace` currently runs **72 tests**, including native executables for hello world, structs, Option/match, lists and overflow traps, plus the browser front-end ABI tests. A race where concurrent codegen tests shared a nanosecond-named temporary directory was found and fixed with a process-local atomic counter; the suite was then run five consecutive times before this site was built.

The parser has malformed-input regression tests. IR validation relies on LLVM, not a custom verifier. Fuzzing, differential interpreter/native testing and sanitizer runs are planned.

## Honest gaps

Typed HIR/MIR are represented by the current checker IR, not a separate verified MIR with basic blocks. There is no incremental compilation, DWARF debug info, DAP, monomorphization (no generics), LTO validation matrix or reproducible-build guarantee. See [Status](../../status/).
