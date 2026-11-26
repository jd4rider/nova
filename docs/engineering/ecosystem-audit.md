# Ecosystem audit — compiler and LSP critical path

Scope: the self-hosting path only (`selfhost/pure-nova` branch). Status
vocabulary: verified working, partial, missing, broken, blocked, not applicable.

The Rust seed has been removed. `bootstrap/nova` is a Nova-built compiler;
there is no Cargo workspace and no `.rs` file in the repository.

## Evidence

| Component | Path | Language | Build / test | Status | Evidence |
|---|---|---|---|---|---|
| Bootstrap compiler | `bootstrap/nova` | Nova binary | `bootstrap/nova build` | verified working | Builds `examples/nova-tool` and self-builds to identical IR |
| Lexer | `libraries/nova-lexer` | Nova | `tests/parser.sh` | verified for the corpus | Tokenizes the toolchain and library sources |
| AST / parser | `libraries/nova-parser` | Nova | `tests/parser.sh` | verified for the corpus | 51/51; full expanded compiler source parses in 0.25 s |
| Declaration checks | `libraries/nova-check/src/check.nova` | Nova | `tests/sema.sh` | partial | Duplicate items/parameters, unknown declared types |
| Body sema | `libraries/nova-check/src/sema.nova` | Nova | `tests/sema-ast.sh` | partial | 39/39 focused diagnostics; no generics or full inference |
| LLVM codegen | `libraries/nova-codegen/src/llvm.nova` | Nova | `tests/codegen.sh`, `tests/diff.sh` | partial | 248/248 codegen assertions; 18/18 frozen native results |
| CLI | `examples/nova-tool` | Nova | `tests/cli.sh`, `tests/toolchain.sh` | verified | `build`, `run`, `check`, `fmt`, `box`, `tree`, `new`, `init`, `clean`, `doctor`, `lsp`; 22 + 126 assertions |
| LSP | `libraries/nova-lsp` | Nova | `tests/lsp.sh` | verified for the implemented slices | Diagnostics, completion, hover, definition, symbols, formatting |
| Nova Boxes | `libraries/nova-boxes` | Nova | `tests/boxes.sh` | verified | 64/64; registry fetch/lock/add/remove/pack/unpack |
| Playground wasm | checked-in `public/playground/*.wasm` | prebuilt | `npm run test:playground` | frozen artifact | A Nova wasm backend has not replaced this build path |

## Self-hosting status

`bootstrap/nova` compiles the Nova toolchain, and the resulting compiler
reproduces byte-identical IR. A fresh checkout builds with the checked-in
bootstrap and no Rust toolchain. The C runtime remains because LLVM/clang and a
host linker are external compilation tools, not a Rust dependency.

## Feature parity

The audited compiler/LSP matrix and stable slice IDs are in
`engineering/continuation/PARITY.md` and `engineering/continuation/TASKS.md`.
The largest remaining gaps are generics/traits/closures, full sema coverage and
a Nova wasm backend.

## Next critical path

1. Close the language gaps recorded in `PARITY.md` (`COMP-01.04`), with tests.
2. Add a Nova wasm backend so the playground no longer relies on frozen wasm.
3. Port `nova test` and the REPL surfaces that the former Rust CLI exposed.
4. Produce cross-platform bootstrap artifacts (the checked-in binary is macOS
   arm64) before advertising Linux/Windows CLI downloads.

## Verification commands

```sh
(cd examples/nova-tool && ../../bootstrap/nova build)
sh examples/nova-tool/tests/parser.sh
sh examples/nova-tool/tests/sema.sh
sh examples/nova-tool/tests/sema-ast.sh
sh examples/nova-tool/tests/codegen.sh
sh examples/nova-tool/tests/diff.sh
sh examples/nova-tool/tests/boxes.sh
sh examples/nova-tool/tests/lsp.sh
```
