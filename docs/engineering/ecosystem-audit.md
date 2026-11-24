# Ecosystem audit — compiler and LSP critical path

Scope: the self-hosting path only (`selfhost/pure-nova` branch). Other parts of
the ecosystem have not been re-audited in this pass. Status vocabulary:
verified working, partial, missing, broken, blocked, not applicable.

## Evidence

| Component | Path | Language | Build / test | Status | Evidence |
|---|---|---|---|---|---|
| Seed compiler | `bootstrap/` (9 crates) | Rust | `cargo build -p nova-cli` | verified working | Still the only compiler used by `nova build` |
| Lexer | `libraries/nova-lexer` | Nova | `NOVA_TOOL=lex`; corpus suite | partial | Tokenizes the corpus; Rust lexer remains stage-0 |
| AST / parser | `libraries/nova-parser/src/ast.nova` | Nova | `tests/parser.sh` | partial | Parses 28/28 library and tool sources; corpus coverage is not full grammar parity |
| Declaration checks | `libraries/nova-check/src/check.nova` | Nova | `tests/sema.sh` | partial | Duplicate items/parameters, unknown declared types |
| Body sema slice | `libraries/nova-check/src/sema.nova` | Nova | `tests/sema-ast.sh` | partial | 9/9 focused diagnostics; not full type inference or generics |
| LLVM codegen | `bootstrap/nova-codegen-llvm` | Rust | `cargo build` | verified working | No Nova replacement yet |
| CLI | `bootstrap/nova-cli` | Rust | `cargo build -p nova-cli` | verified working | Nova tool covers fmt/lex/parse/boxes, not `build/run/test` |
| LSP | `bootstrap/nova-lsp` | Rust | Rust build | missing in Nova | Blocked on full sema and a Nova stdio streaming primitive |
| Nova tool | `examples/nova-tool` | Nova | built by `nova build` | partial | `ast-dump`, `sema`, `sema-ast`, formatter and Boxes modes |
| Nova Boxes | `libraries/nova-boxes` | Nova | `tests/boxes.sh` | verified working | 64/0 on the branch; registry still requires owner deployment authorization |

## Self-hosting status

The parser and sema slices are real, but they are not connected to `nova build`.
There is no stage-1 Nova compiler that can compile its own source, and no Rust
crate has been deleted. Do not describe the toolchain as self-hosted, pure Nova,
or end-to-end yet.

## Next critical path

1. Finish sema coverage over the AST: generics, methods/impl resolution,
   imported modules, builtin method tables and diagnostics parity.
2. Add a Nova codegen path (textual LLVM IR through the existing host `clang`
   first, then an FFI shim if needed) behind an explicit tool mode.
3. Make the Nova compiler driver build a nontrivial example end to end.
4. Port the CLI build/run flow, then the LSP over the same semantic API.
5. Only then delete `bootstrap/`, after stage-2/stage-3 conformance and a
   documented seed-recovery route.

## Known blockers

- LSP needs streaming stdin/stdout; the current Nova host exposes command and
  file calls but not a line-oriented stdio loop.
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
