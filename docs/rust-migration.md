# Replacing the Rust compiler with Nova

The compiler is currently Rust: **17,907 lines across 9 crates** in `bootstrap/`.
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
| `nova-fmt` | 186 | Source formatter | **deleted**; Nova formatter |
| `nova-runtime` | 98 | Thin Rust wrappers over the C runtime | **deleted**; C and Objective-C sources moved to `runtime/` |

The 98 lines in `nova-runtime` were wrappers around `runtime.c` and `gui.m`;
they are gone, and the sources now live in `runtime/` outside the Rust workspace.

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

## Layout change

`crates/` is gone. The remaining Rust compiler lives in `bootstrap/` and is
built by Cargo only as the stage-0 compiler; Nova packages live in `libraries/`
and are described by Nova Boxes. `nova-stdlib` (a 1-line placeholder) was
deleted, and the C/Objective-C runtime moved to `runtime/`.

## Phase 1 status (in progress)

The Nova implementations exist and are exercised by `examples/nova-tool`:

| Piece | Nova file | State |
|---|---|---|
| Formatter | `libraries/nova-fmt/src/fmt.nova` | formats every corpus file, idempotent; `nova fmt`, the LSP and the wasm playground all run this implementation |
| Parser | `libraries/nova-parser/src/parser.nova`, `expr.nova`, `ast.nova` | `ast.nova` is the phase-2 AST arena: imports, structs, enums, impls, externs, generics, functions, control flow, `match`, calls, literals, indexing and operators. It parses all 29 library/tool sources without errors |
| Semantic checks | `libraries/nova-check/src/sema.nova` | Body-level AST slice: declaration tables, scoped locals, expression/return types, call arity/types, struct fields, conditions, loop checks, match pattern validation, block-arm bodies, duplicate struct/enum declarations, String literal patterns and Int/Bool/String/enum exhaustiveness. Twenty-eight focused assertions; still partial and not wired into `nova build` |
| Codegen slice | `libraries/nova-codegen/src/llvm.nova` | Emits seed-shaped textual LLVM IR for Int, Unit, String and `List<Int>` functions, arithmetic, `if`/`else`, `while`, range/list `for` with `break`/`continue`, Int/Bool `match` with literals and `_`, comparisons, direct calls, string literals, `String.len`, malloc-backed list literals/indexing/index assignment/`List.len`, by-value list parameters/returns, runtime-linked indexing/assignment/push through `nova_list_at`/`nova_list_push`, String concat/equality and literal match patterns, value structs with nested fields, construction, reads, field assignment and by-value parameters/returns, and enums with tags, Int payloads, construction, payload bindings and exhaustive match. One hundred thirty-seven focused assertions; immutable params/`let` stay in SSA, clang validates the IR and native binaries exit 9/2/17 (plus Unit, string, list, loop, match, struct and enum fixtures); an unmatched match panics with exit 1. `build-ir` compiles and links `runtime/runtime.c`. No maps, tuple/array values, slices, nested enum match patterns, non-Int enum payloads or full runtime ABI yet; sema rejects unsupported patterns and checks Int/Bool/enum exhaustiveness |
| Semantic checks | `libraries/nova-check/src/check.nova` | declaration-level: duplicate items, duplicate parameters, unknown type names |
| Lexer | `libraries/nova-lexer/src/lexer.nova` | tokenizes the full 5,139-line studio file and the large-input corpus |
| Diagnostics | `libraries/nova-diag/src/diag.nova` | renders severity, location and caret |
| Packages (**Nova Boxes**) | `libraries/nova-boxes/src/*.nova` | `Box.toml`, `Box.lock`, SemVer, path/registry resolution, dependency graphs |
| Archives and registry | `libraries/nova-boxes/src/archive.nova`, `registry.nova`, `json.nova` | deterministic `.box` pack/verify/unpack, sparse index reads, `search`/`info`/`fetch`, offline cache |

Language gaps found while porting and fixed in the toolchain (sema + LLVM codegen
+ C runtime), because they belong in Nova rather than in workarounds:

- `String.split(separator) -> List<String>`
- `List<String>.join(separator) -> String`
- `String.trim_start()` and `String.trim_end()`

`nova_string_join` takes the list by pointer, matching `nova_list_push`; passing a
24-byte list by value produced a garbage length and crashed.

The lexer crash on large files was not a lexer bug: codegen emitted each `alloca`
at its use site, so a declaration inside a loop grew the stack on every
iteration until it overflowed. Allocas are now collected per function and spliced
into the entry block, which fixes large inputs for every Nova program.

The Rust `nova-fmt` crate is deleted. `nova fmt` and the LSP run the Nova
formatter through `nova-tool`, and the browser playground runs it as a wasm
module built from `examples/nova-fmt-wasm`. That needed two wasm runtime
additions, `nova_input_ptr`/`nova_input_set` to hand source in and the existing
output buffer to read the result back, plus value-ABI shims for `chars`, `split`,
`join` and the trim family.

Phase-2 suites: `tests/expr.sh` (expression trees), `tests/parser.sh`
(item structure plus the AST splice), `tests/sema.sh` (declaration checks),
`tests/sema-ast.sh` (body-level AST checks), `tests/codegen.sh` (IR, clang
and native execution), and `engineering/continuation/PARITY.md` (feature
parity matrix), and
`apps/website/scripts/test-playground-format.mjs` (the browser formatter wasm,
output and idempotence).

Phase-2 corpus gate: `ast.nova` parses all 29 files under `libraries/*/src`
plus `examples/nova-tool/src` with no parse errors. That is coverage of the
syntax those files use, not proof of full grammar parity: closures and forms
absent from the corpus are not covered yet, and the AST is not wired into
`nova build`. The phase-3 slice in `sema.nova` now checks bodies over this AST;
full sema parity remains the gate.

Test state: `sh examples/nova-tool/tests/toolchain.sh` → **passed 117, failed 0**
(including formatting the studio source), `sh examples/nova-tool/tests/parser.sh`
→ **passed 50, failed 0** (including the 29-file AST corpus), and
`sh examples/nova-tool/tests/boxes.sh`
→ **passed 64, failed 0**, and `sh examples/nova-tool/tests/codegen.sh`
→ **passed 137, failed 0** (Nova tool invokes clang and links the C runtime; native execution on this host). No Rust crate has been deleted yet: the bootstrap
compiler (lexer, parser, sema, codegen, CLI, LSP) is still Rust, and phases 2–4
of this document are what removes it. `nova fmt` runs the Nova formatter, and
`nova box` runs Nova Boxes; neither makes the compiler self-hosted.
