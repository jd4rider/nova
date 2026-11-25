# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: Unit/String ABI checkpoint; prior `332555b`.
Working tree: this checkpoint is committed by the accompanying code/doc
commits; unrelated `editor/` and playground changes remain untouched.

Active milestone: `CODEGEN-01` — Nova LLVM IR lowering
Active task: `CODEGEN-01.06` — collections/structs and runtime types
Status: PARTIAL (Int, Unit and string-literal/len slices verified)

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 32, failed 0.
  - Unit functions use `void`, implicit `ret void` and `call void`; caller exits 4.
  - Unit `main` calls the entry and exits 0.
  - String functions use `%nova.string { ptr, i64 }`, literal globals and
    by-value parameters/returns; `String.len()` uses `extractvalue`; result 5.
  - Int binaries exit 9, 2 and 17; `build-ir` invokes clang.
  - `ptr %arg_env`, `nova_user_<index>` and the `@main` wrapper are emitted.
  - Immutable params/`let` stay in SSA.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 9, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- Mach-O confirmed on this Apple Silicon host.

## Work completed since previous checkpoint

- `CODEGEN-01.04` advanced: Unit/void returns, Unit `main`, implicit Unit return,
  Unit calls, String literals, String parameters/returns and `String.len()`.
- String globals are emitted per function with stable `@nova.str.<fn>.<n>` names.
- Shell-safe clang invocation from `build-ir` retained.

## Incomplete work / known failures

- ABI parity is not complete: lists, maps, structs, enums, tuples and runtime
  calls remain; no list/runtime-call ABI.
- Strings are partial: no concat, equality, interpolation, format/runtime calls
  or non-literal construction.
- Codegen has no collections, `for`/`match`, overflow or bounds checks.
- `nova build` still uses the Rust seed; the Nova tool is not yet the CLI.
- Sema lacks modules, generics, traits, methods and full diagnostics.
- No Nova LSP or stdio transport; no differential Rust/Nova harness.

## Resume here

1. Extend `CODEGEN-01.06` to `List<Int>` literals, indexing and `.len()` using
   the seed `%nova.list { ptr, i64, i64 }` shape.
2. Add native tests for list construction, indexing and length.
3. Run codegen/parser/sema suites; update `STATE.md`, `EVIDENCE.md`,
   `TASKS.md` and `PARITY.md`; commit.
4. Then structs/strings runtime operations, `CODEGEN-01.07` checks, and
   `CLI-01.01`.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (32/0)

## Blockers and approvals

- No external blocker for collection ABI work.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP needs sema, stdio and the parity slices above.
