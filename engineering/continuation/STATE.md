# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: `List<Int>` push checkpoint; prior `10d910e`.
Working tree: this checkpoint is committed by the accompanying code/doc
commits; unrelated `editor/` and playground changes remain untouched.

Active milestone: `CODEGEN-01` — Nova LLVM IR lowering
Active task: `CODEGEN-01.06` — collections/structs and runtime types
Status: PARTIAL (Int, Unit, String, List literals/params/returns/push)

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 54, failed 0.
  - `List<Int>` literals allocate with libc `malloc`; empty lists supported.
  - List parameters and returns use by-value `%nova.list`; a literal survives
    return, identity and indexing (native 13).
  - Mutable local `.push(x)` grows with libc `realloc`, updates data/cap/len and
    stores the element; a grown list survives return and indexing (native 16).
  - `points[i]` checks low/high bounds, traps, then loads i64; list/String
    `.len()` use `extractvalue` index 1.
  - Native results: list push 16; list index+length 23; list ABI 13; Int 9/2/17;
    Unit caller 4; Unit `main` 0; string 5.
  - `ptr %arg_env`, `nova_user_<index>` and the `@main` wrapper are emitted.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 9, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- Mach-O confirmed on this Apple Silicon host.

## Work completed since previous checkpoint

- `List<Int>.push` for mutable local lists, including capacity growth.
- `realloc` failure trapping; list data/cap/len updates.
- Native fixture `codegen-list-push.nova` that returns a grown list.

## Incomplete work / known failures

- Push uses libc `realloc`, not `nova_list_push` or the seed allocator. List
  storage is never freed; ownership/GC is unresolved.
- `List<Int>` index assignment, slices and seed runtime calls remain.
- Bounds failure is `llvm.trap`, not the seed's Nova panic.
- Maps, sets, structs, enums, tuples, arrays and Option/Result remain.
- Strings lack concat, equality, interpolation and runtime calls.
- Codegen has no `for`/`match`, overflow checks or full runtime ABI.
- `nova build` still uses the Rust seed; the Nova tool is not yet the CLI.
- Sema lacks modules, generics, traits, methods and full diagnostics.
- No Nova LSP or differential Rust/Nova harness.

## Resume here

1. Add mutable index assignment for `List<Int>` (bounds-checked store), or design
   seed runtime linkage for `nova_list_push`; do not stub it.
2. Then structs and `for`/`match`.
3. Add `CODEGEN-01.07` checks and complete runtime ABI work.
4. Run codegen/parser/sema suites; update `STATE.md`, `EVIDENCE.md`,
   `TASKS.md` and `PARITY.md`; commit.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (54/0)

## Blockers and approvals

- No external blocker for index assignment or structs.
- Seed runtime linkage remains an explicit design decision; do not claim
  `nova_list_*` parity while push uses libc.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP needs sema, stdio and the parity slices above.
