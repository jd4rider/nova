# Replacing the Rust compiler with Nova

The compiler is currently Rust: **17,922 lines across 16 files in 12 crates**.
This page is the inventory and the order in which each crate can become Nova.
It is a bootstrapping project, not a port: the tool that compiles Nova has to be
written in Nova, compiled by the Rust tool, and then compile itself before the
Rust one can go.

## Inventory

| Crate | Lines | Role | Replacement |
|---|---:|---|---|
| `nova-sema` | 6,767 | Type checking, name resolution, method tables | Nova, phase 3 |
| `nova-codegen-llvm` | 4,536 | Emits LLVM IR | Nova, phase 4 (needs an FFI story for LLVM) |
| `nova-parser` | 2,847 | AST and recursive-descent parser | Nova, phase 2 |
| `nova-cli` | 993 | `nova build/run/fmt/test`, host bundling | Nova, phase 1 |
| `nova-lexer` | 894 | Tokens, literals, interpolation | Nova, phase 1 |
| `nova-lsp` | 803 | Language server over stdio | Nova, phase 5 |
| `nova-diag` | 334 | Diagnostic rendering, spans | Nova, phase 1 |
| `nova-pkg` | 234 | `nova.toml`, lockfile, module graph | Nova, phase 1 |
| `nova-wasm` | 229 | wasm32 backend | Nova, phase 5 (a backend, not a port) |
| `nova-fmt` | 186 | Source formatter | Nova, phase 1 |
| `nova-runtime` | 98 | Thin Rust wrappers over the C runtime | Fold into the C runtime; no Rust needed |
| `nova-stdlib` | 1 | Placeholder | Delete |

The 98 lines in `nova-runtime` are wrappers around `runtime.c` and `gui.m`;
they disappear first, since the C host already does the work.

## Phases

1. **Tooling first.** fmt, diag, pkg, lexer and the CLI's file walking have no
   dependency on codegen. Rewriting them in Nova is the cheapest proof that a
   self-hosted toolchain is pleasant, and it removes ~2,600 lines of Rust
   without touching the bootstrap.
2. **Parser in Nova.** The AST becomes a Nova data model. The Rust compiler
   keeps compiling until the Nova parser passes the same corpus (the existing
   examples are the corpus: `examples/*` plus `libraries/*`).
3. **Sema in Nova.** The largest crate and the real gate: type checking,
   generic instantiation and the builtin method tables. This is where the
   compiler either becomes maintainable in Nova or does not.
4. **Codegen in Nova.** Needs a C-ABI surface for LLVM (`LLVMBuildAdd`,
   `LLVMBuildCall2`, …). The `gui.m` host pattern is the model: a small C shim
   that Nova calls. Emitting the same IR as today is the acceptance test:
   byte-identical `.ll` for every example.
5. **Delete the Rust compiler.** Keep `cargo` only for the runtime and the
   shims while those stay C. Then `nova build nova` is the only compiler.

## What must exist before phase 2

- A Nova module system that can express a compiler's data types (it can).
- Strings, lists, maps and recursion without visible cost (they exist; the
  `List` layout fix unblocked struct-carrying lists).
- Error reporting with spans (today only `println`; a `diagnostic` type in
  `libraries/nova-diag` is the first deliverable).
- A test runner so each phase can be checked against the previous compiler's
  output.

## Honest assessment

Phase 1 is a few days of work. Phase 3 is months, because sema is 6,767 dense
lines and a compiler must be *exactly* as permissive as its predecessor. Phase 4
is gated on LLVM bindings, which is the one place where C is not optional.
Nothing here is blocked on language features — only on time and on keeping the
test corpus green at every step.
