# Evidence

All results below were run on `selfhost/pure-nova` on 2026-11-24.

## Build

```sh
cargo build -p nova-cli
(cd examples/nova-tool && ../../target/debug/nova build)
```

Result: PASS. The Nova tool was rebuilt by the Rust stage-0 compiler.

## Parser

```sh
sh examples/nova-tool/tests/parser.sh
```

Result: PASS — passed 50, failed 0. Includes the 29-file AST corpus gate.

Commits: `93d61dc`, gate `983fd6a`.

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

Result: PASS — passed 8, failed 0.

Evidence from the suite:

- `define i64 @nova_add(i64 %p0, i64 %p1)` emitted.
- Direct call `call i64 @nova_add(i64 1, i64 2)` emitted.
- Host clang accepted the module.
- Native binary exited with 9 (the expected arithmetic result).
- On this Apple Silicon host, `file` reported Mach-O.

Limits: the test invokes clang; `nova-tool emit-ir` only emits IR. The slice
does not use the seed runtime ABI and has no control flow yet.

## Nova Boxes

Previously recorded: `sh examples/nova-tool/tests/boxes.sh` → passed 64, failed
0. Not rerun in the latest session.

## Commit history

- `47302b6` — AST-based semantic checker in Nova.
- `983fd6a` — full-corpus AST parser regression gate.
- `93d61dc` — full-corpus AST parsing.
- `1735f70` — first Nova AST/parser slice.
- Codegen checkpoint commit: see branch HEAD after this checkpoint.
