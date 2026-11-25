# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: `List<Int>` literal/index checkpoint; prior `9613739`.
Working tree: this checkpoint is committed by the accompanying code/doc
commits; unrelated `editor/` and playground changes remain untouched.

Active milestone: `CODEGEN-01` — Nova LLVM IR lowering
Active task: `CODEGEN-01.06` — collections/structs and runtime types
Status: PARTIAL (Int, Unit, String and `List<Int>` literal/index slices verified)

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 41, failed 0.
  - `List<Int>` literals allocate an element array and build
    `%nova.list { ptr, i64, i64 }` with `insertvalue`; empty lists are supported.
  - `points[i]` checks the low and high bounds, traps on failure, then loads i64.
  - `points.len()` and `String.len()` both use `extractvalue` index 1.
  - Native list result 23; Int 9/2/17; Unit caller 4; Unit `main` 0; string 5.
  - `ptr %arg_env`, `nova_user_<index>` and the `@main` wrapper are emitted.
  - Immutable params/`let` stay in SSA.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 9, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- Mach-O confirmed on this Apple Silicon host.

## Work completed since previous checkpoint

- `List<Int>` literals, empty list construction, indexing and `.len()`.
- `%nova.list = type { ptr, i64, i64 }` and `llvm.trap` bounds handling.
- Native fixture `codegen-list.nova`, including empty-list coverage.

## Incomplete work / known failures

- List ABI is not complete: parameters/returns, push, mutation, slices and the
  runtime `nova_list_*` calls remain. Literals allocate stack storage.
- The bounds failure is `llvm.trap`, not the seed's Nova panic/diagnostic.
- Maps, sets, structs, enums, tuples, arrays and Option/Result remain.
- Strings lack concat, equality, interpolation and runtime calls.
- Codegen has no `for`/`match`, overflow checks or full runtime ABI.
- `nova build` still uses the Rust seed; the Nova tool is not yet the CLI.
- Sema lacks modules, generics, traits, methods and full diagnostics.
- No Nova LSP or differential Rust/Nova harness.

## Resume here

1. Extend `CODEGEN-01.04`/`01.06` to `List<Int>` parameters and returns using
   the seed `%nova.list` by-value ABI; add a native fixture.
2. Add list push/mutation only after runtime linkage is designed; do not fake
   runtime behavior with a stub.
3. Then structs/`for`/`match` and `CODEGEN-01.07` checks.
4. Run codegen/parser/sema suites; update `STATE.md`, `EVIDENCE.md`,
   `TASKS.md` and `PARITY.md`; commit.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (41/0)

## Blockers and approvals

- No external blocker for list parameter/return ABI work.
- List push/runtime calls need a runtime-linking decision before implementation.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP needs sema, stdio and the parity slices above.
