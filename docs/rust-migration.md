# Rust removal

Status: **complete**. The repository contains no Rust sources, no Cargo
workspace and no Rust build step.

## Bootstrap

`bootstrap/nova` is a native compiler built by Nova from `examples/nova-tool`.
It compiles the same sources into a new compiler:

```sh
cd examples/nova-tool
../../bootstrap/nova build
```

The new compiler self-builds with byte-identical emitted IR (see
`engineering/continuation/EVIDENCE.md`). The C runtime (`runtime/runtime.c`) and
`clang` remain because LLVM and the host linker are external tools, not Rust.

## What replaced each crate

| Former Rust crate | Nova replacement |
|---|---|
| `nova-lexer` | `libraries/nova-lexer/src/lexer.nova` |
| `nova-parser` | `libraries/nova-parser/src/{parser,expr,ast}.nova` |
| `nova-sema` | `libraries/nova-check/src/{check,sema}.nova` |
| `nova-codegen-llvm` | `libraries/nova-codegen/src/llvm.nova` |
| `nova-cli` | `examples/nova-tool/src/main.nova` (`build`, `run`, `check`, `fmt`, `box`, `tree`, `new`, `init`, `clean`, `doctor`, `lsp`) |
| `nova-lsp` | `libraries/nova-lsp/src/lsp.nova` |
| `nova-pkg` | `libraries/nova-boxes/src/*.nova` |
| `nova-diag` | `libraries/nova-diag/src/diag.nova` |
| `nova-wasm` | prebuilt playground artifacts; a Nova wasm backend is still a gap |

## Frozen reference outputs

Before removal, the Rust compiler's outputs became goldens:

- `examples/nova-tool/tests/diff-golden.txt` — 18 native program results.
- `examples/nova-tool/tests/diff-errors.sh` — 27 normalized diagnostic sets.

Nova is now measured against those recordings, not against a live Rust binary.

## Remaining language gaps

Removal does not complete language parity. The Nova compiler still lacks
generics/traits/closures and full sema parity (`COMP-01.04`), and the browser
playground uses checked-in wasm artifacts until a Nova wasm backend exists
(phase 5). These are tracked in `engineering/continuation/PARITY.md` and
`TASKS.md`; no Rust dependency is required to close them.
