# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: `List<Int>` parameter/return ABI checkpoint; prior `6cab5dd`.
Working tree: this checkpoint is committed by the accompanying code/doc
commits; unrelated `editor/` and playground changes remain untouched.

Active milestone: `CODEGEN-01` — Nova LLVM IR lowering
Active task: `CODEGEN-01.06` — collections/structs and runtime types
Status: PARTIAL (Int, Unit, String, and List<Int> literals/params/returns)

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 48, failed 0.
  - `List<Int>` literals allocate with libc `malloc` and build
    `%nova.list { ptr, i64, i64 }`; empty lists are supported.
  - List parameters and returns use by-value `%nova.list`; calls pass the
    aggregate. A literal survives return, identity and indexing (native 13).
  - `points[i]` checks low/high bounds, traps, then loads i64; `List.len()` and
    `String.len()` use `extractvalue` index 1.
  - Native results: list index+length 23; Int 9/2/17; Unit caller 4; Unit `main`
    0; string length 5.
  - `ptr %arg_env`, `nova_user_<index>` and the `@main` wrapper are emitted.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 9, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- Mach-O confirmed on this Apple Silicon host.

## Work completed since previous checkpoint

- `List<Int>` parameters and return types in function signatures and calls.
- Heap-backed list literals, including null-allocation trapping.
- Native fixture `codegen-list-abi.nova` for return-through-identity indexing.

## Incomplete work / known failures

- List storage uses libc `malloc` and is never freed. This is safe for the tested
  lifetime but is not the seed allocation/ownership model.
- No list push, mutation, slices or `nova_list_*` runtime calls.
- Bounds failure is `llvm.trap`, not the seed's Nova panic.
- Maps, sets, structs, enums, tuples, arrays and Option/Result remain.
- Strings lack concat, equality, interpolation and runtime calls.
- Codegen has no `for`/`match`, overflow checks or full runtime ABI.
- `nova build` still uses the Rust seed; the Nova tool is not yet the CLI.
- Sema lacks modules, generics, traits, methods and full diagnostics.
- No Nova LSP or differential Rust/Nova harness.

## Resume here

1. Design runtime linkage for `nova_list_push`/mutation; do not stub it.
2. Extend `CODEGEN-01.06` to structs or another collection, with native fixtures.
3. Add `for`/`match` and `CODEGEN-01.07` checks.
4. Run codegen/parser/sema suites; update `STATE.md`, `EVIDENCE.md`,
   `TASKS.md` and `PARITY.md`; commit.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (48/0)

## Blockers and approvals

- No external blocker for structs or list runtime design.
- Runtime linkage must decide how the generated module links `runtime.c`
  before push/mutation can claim parity.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP needs sema, stdio and the parity slices above.
