# Ecosystem audit — compiler and LSP critical path

Scope: the self-hosting path only (`selfhost/pure-nova` branch). Other parts of
the ecosystem have not been re-audited in this pass. Status vocabulary:
verified working, partial, missing, broken, blocked, not applicable.

## Evidence

| Component | Path | Language | Build / test | Status | Evidence |
|---|---|---|---|---|---|
| Seed compiler | `bootstrap/` (9 crates) | Rust | `cargo build -p nova-cli` | verified working | Still the only compiler used by `nova build` |
| Lexer | `libraries/nova-lexer` | Nova | `NOVA_TOOL=lex`; corpus suite | partial | Tokenizes the corpus; Rust lexer remains stage-0 |
| AST / parser | `libraries/nova-parser/src/ast.nova` | Nova | `tests/parser.sh` | partial | Parses 29/29 library and tool sources; corpus coverage is not full grammar parity |
| Declaration checks | `libraries/nova-check/src/check.nova` | Nova | `tests/sema.sh` | partial | Duplicate items/parameters, unknown declared types |
| Body sema slice | `libraries/nova-check/src/sema.nova` | Nova | `tests/sema-ast.sh` | partial | 28/28 focused diagnostics, including duplicate struct/enum declarations, enum constructor/pattern arity, unknown variants, payload binding types, String literal patterns, match patterns, duplicate/unreachable arms, block-arm bodies, arm types and Int/Bool/String/enum exhaustiveness; not full type inference or generics |
| LLVM codegen | `bootstrap/nova-codegen-llvm` | Rust | `cargo build` | verified working | Full seed backend; no Nova replacement in `nova build` |
| Codegen slice | `libraries/nova-codegen/src/llvm.nova` | Nova | `tests/codegen.sh` | partial | Int/Unit/String/`List<Int>` IR with `if`/`else`, `while`, range/list/nested `for`, `break`/`continue`, Int/Bool/enum `match` with literals, variants, payload bindings and `_`, comparisons, SSA bindings, seed-shaped `ptr %arg_env`, `%nova.string`, malloc-backed lists, by-value `%nova.list` params/returns, runtime-linked `nova_list_at`/`nova_list_push` indexing/assignment/push, String concat/equality and literal match patterns, value structs with nested fields, construction, reads, field assignment and by-value params/returns, and enums with tags, Int payloads, construction, payload bindings and exhaustive match; unmatched matches call `nova_panic_str` and exit 1; `build-ir` invokes clang and links `runtime/runtime.c`; native results include 9/2/17, Unit 4/0, string 5, list 23, list ABI 13, push 16, push loop 10, assignment 45, range-for 8, list/nested-for 30, Int match 60, Unit match 9, struct 14, enum 36, String match 13, print exit 7 with exact stdout, assert exit 5/failure exit 1, list equality 12, list concat 11, String methods 5, Bool enum payloads 7, String enum payloads 10, list slices 10, String transforms 16, String bytes 95, String upper 72, list ends 49, list contains 16, list reverse 31, list sort 13, list sum 10, list min/max 29, list index_of 11 and struct enum 9 |
| CLI | `bootstrap/nova-cli` | Rust | `cargo build -p nova-cli` | verified working | Nova tool covers fmt/lex/parse/boxes, not `build/run/test` |
| LSP | `bootstrap/nova-lsp` | Rust | Rust build | missing in Nova | Blocked on full sema and a Nova stdio streaming primitive |
| Nova tool | `examples/nova-tool` | Nova | built by `nova build` | partial | `ast-dump`, `sema`, `sema-ast`, formatter and Boxes modes |
| Nova Boxes | `libraries/nova-boxes` | Nova | `tests/boxes.sh` | verified working | 64/0 on the branch; registry still requires owner deployment authorization |

## Self-hosting status

The parser, sema and codegen slices are real, but they are not connected to
`nova build`. The Nova tool emits IR and `build-ir` invokes clang, but there is
no stage-1 Nova compiler that can compile its own source, and no Rust crate has
been deleted. Do not describe the toolchain as self-hosted, pure Nova,
or end-to-end yet.

## Feature parity

The audited compiler/LSP matrix and stable slice IDs are in
`engineering/continuation/PARITY.md` and `engineering/continuation/TASKS.md`.

## Next critical path

1. Complete remaining checks (`CODEGEN-01.07`), then wire the driver
   (`CLI-01.01`).
2. Finish sema coverage over the AST: generics, methods/impl resolution,
   imported modules, builtin method tables and diagnostics parity.
3. Make the Nova compiler driver build a nontrivial example end to end.
4. Port the remaining LSP feature slices over the same semantic API; the Nova
   stdio transport, diagnostics and completion slice now exist in `nova-tool lsp`.
5. Only then delete `bootstrap/`, after stage-2/stage-3 conformance and a
   documented seed-recovery route.

## Known blockers

- The Nova LSP now has framed stdin/stdout, diagnostics and completion; hover,
  definition, formatting and Studio wiring remain.
- Sema needs module/import resolution; `ast.nova` records imports but does not
  load them.
- Codegen needs a backend decision and ABI acceptance tests before replacing
  the Rust emitter.

## Verification commands

```sh
(cd examples/nova-tool && ../../target/debug/nova build)
sh examples/nova-tool/tests/parser.sh
sh examples/nova-tool/tests/sema.sh
sh examples/nova-tool/tests/sema-ast.sh
sh examples/nova-tool/tests/boxes.sh
```
