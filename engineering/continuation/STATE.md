# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied (2026-11-24); requirements tracked in
`engineering/continuation/ROADMAP.md` and `TASKS.md`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: `47302b6` plus the uncommitted codegen checkpoint described below.
Working tree: the codegen checkpoint is uncommitted; pre-existing unrelated
`editor/` and playground changes are still untouched.

Active milestone: `CODEGEN-01` — Nova LLVM IR lowering and native execution
Active task: `CODEGEN-01.03` — control flow and comparisons
Status: READY

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 8, failed 0. The Nova tool
  emits IR; clang compiles it; the native binary exits 9.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 9, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- The native artifact is Mach-O on this Apple Silicon host.

## Work completed since previous checkpoint

- `libraries/nova-codegen/src/llvm.nova`: Int functions, parameters, `let`/`var`,
  assignment, integer literals, `+ - * /`, direct calls and `return`.
- `NOVA_TOOL=emit-ir` mode in `examples/nova-tool`.
- `tests/codegen.sh` and `tests/codegen-sample.nova`.
- Durable continuation files under `engineering/continuation/`.

## Incomplete work / known failures

- Codegen has no `if`/`while`, comparisons, strings, bools, structs or module
  imports. Unsupported syntax returns an error.
- The test invokes clang; the Nova tool does not invoke a linker yet.
- The IR ABI is self-contained and does not match the seed compiler's hidden
  environment pointer or full runtime ABI.
- Sema does not resolve modules, generics or methods.
- `nova build` still uses the Rust seed; no Rust crate has been deleted.
- The LSP is still Rust.

## Resume here

1. Extend `libraries/nova-codegen/src/llvm.nova` with `if`/`else`, `while`,
   comparisons and branch labels; keep unsupported forms diagnosed.
2. Add IR and native execution cases to `examples/nova-tool/tests/codegen.sh`.
3. Run the codegen, parser and sema suites; update this file, `EVIDENCE.md` and
   `TASKS.md`; commit the checkpoint on the branch.
4. After control flow, align the ABI with the seed (`CODEGEN-01.04`) and add
   the CLI driver (`CLI-01.01`).

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (8/0)

## Blockers and approvals

- No external blocker for codegen.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP work awaits full sema and a Nova stdio primitive.
