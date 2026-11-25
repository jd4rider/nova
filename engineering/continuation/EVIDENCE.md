# Evidence

All results below were run on `selfhost/pure-nova` on 2026-11-24.

## Build

```sh
cargo build -p nova-cli
(cd examples/nova-tool && ../../target/debug/nova build)
```

Result: PASS.

## Parser

```sh
sh examples/nova-tool/tests/parser.sh
```

Result: PASS — passed 50, failed 0; 29-file AST corpus.

Commits: `93d61dc`, `983fd6a`.

## Body sema

```sh
sh examples/nova-tool/tests/sema-ast.sh
```

Result: PASS — passed 9, failed 0. Commit `47302b6`.

## Declaration sema

```sh
sh examples/nova-tool/tests/sema.sh
```

Result: PASS — passed 4, failed 0.

## Codegen and native execution

```sh
sh examples/nova-tool/tests/codegen.sh
```

Result: PASS — passed 48, failed 0.

- Straight-line Int module: no `alloca` for immutable params/`let`.
- Mutable module compiles and exits 2.
- Loop/branch module contains `icmp slt i64` and `br i1`; binary exits 17.
- Immutable arithmetic module exits 9.
- ABI assertions: `define i64 @nova_user_0(ptr %arg_env, ...)`, direct call
  `@nova_user_0(ptr null, ...)`, and `@main` calls the entry with `ptr null`.
- Unit function emits `define void @nova_user_0(ptr %arg_env)` and `call void`;
  implicit return; caller exits 4. Unit `main` calls the entry and exits 0.
- String function emits `%nova.string`, a literal global, by-value String
  parameter/return signatures, and `extractvalue` for `.len()`; native result 5.
- List fixture emits `%nova.list { ptr, i64, i64 }`, a malloc-backed element
  array, aggregate `insertvalue` construction (including empty list),
  `extractvalue` for `.len()`, bounds-checked `getelementptr` indexing and
  `llvm.trap`; native result 23.
- List ABI fixture emits by-value `%nova.list` parameter/return signatures and
  calls: a literal returned by one function passes through an identity function
  and is indexed by another; native result 13.
- `NOVA_TOOL=build-ir` invokes clang, checks status and produces a native binary;
  the test executes that Nova-built binary.
- Host clang accepted every module; Mach-O confirmed on this host.

Limit: ABI parity covers Int, Unit, the string-literal/len subset and the
`List<Int>` literal/parameter/return subset. List push, mutation, slices and the
`nova_list_*` runtime are not implemented. Literals use libc `malloc` and are
never freed; the bounds trap is not a Nova panic. Maps, sets, structs, enums,
tuples, arrays and Option/Result remain. `emit-ir` is still not the full CLI.
This is not a compiler-speed measurement.

## Parity audit

`PARITY.md` maps every audited compiler and LSP capability to a stable task ID.
The Rust LSP header (`bootstrap/nova-lsp/src/lib.rs`) confirms full-document
sync, diagnostics, completion, hover, definition, document symbols and
formatting; incremental sync, rename, references, semantic tokens, code actions
and cancellation are absent even in Rust.

## Commit history

- `List<Int>` ABI checkpoint — 48/0.
- `List<Int>` literals/indexing checkpoint — 41/0.
- Unit/String ABI checkpoint — 32/0.
- `332555b` — Nova tool invokes clang and builds a native binary.
- `8092f97` — Int ABI alignment.
- `98c1ded` — native control flow and parity matrix.
- `0fa226f` — SSA immutable bindings and performance gate.
- `e9d9e4a` — native codegen path in Nova.
- `47302b6` — AST-based semantic checker in Nova.
- `983fd6a` — full-corpus AST parser regression gate.
