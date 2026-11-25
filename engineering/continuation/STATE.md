# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: this checkpoint is committed by the accompanying code/doc commits; prior
`ddebf4f`.
Working tree: unrelated `editor/` and playground changes remain untouched.

Active milestone: `CODEGEN-01` — Nova LLVM IR lowering
Active task: `CODEGEN-01.08` — loops, match and control flow
Status: PARTIAL (Int/Bool match; enum patterns and exhaustiveness pending)

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 73, failed 0.
  - `List<Int>` literals allocate with libc `malloc`; empty lists supported.
  - List parameters and returns use by-value `%nova.list`; a literal survives
    return, identity and indexing (native 13).
  - Mutable local `.push(x)` grows with libc `realloc`, updates data/cap/len and
    stores the element; a grown list survives return and indexing (native 16).
  - `points[i]` checks low/high bounds, traps, then loads i64; list/String
    `.len()` use `extractvalue` index 1.
  - `points[i] = v` checks the same bounds, traps, then stores i64 through the
    mutable local list; a mutated list survives return (native 45).
  - `for n in start..end` is an exclusive integer range; `for x in list` walks a
    `List<Int>` by index. `break` and `continue` use a nested loop-label stack;
    `continue` in a `for` jumps to the increment block. Native results: range 8,
    list 30, nested 30.
  - `match` supports Int/Bool scrutinees, integer and `true`/`false` literal
    patterns, `_`, expression arms and Unit block arms. Expression arms merge
    through a `phi`; unmatched matches reach `unreachable`. Native results:
    Int match 60, Bool/Unit match 9.
  - Native results: assignment 45; push 16; list index+length 23; list ABI 13;
    Int 9/2/17; Unit caller 4; Unit `main` 0; string 5.
  - `ptr %arg_env`, `nova_user_<index>` and the `@main` wrapper are emitted.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 9, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- Mach-O confirmed on this Apple Silicon host.

## Work completed since previous checkpoint

- Int/Bool match expressions with literal and wildcard patterns.
- Unit block match arms, nested match expressions and phi-based result merging.
- Native fixtures for Int and Bool/Unit matches.

## Incomplete work / known failures

- Match lacks enum/struct/string/binding patterns, exhaustiveness diagnostics
  and an explicit no-match panic; sema does not validate match patterns.
- No short-circuit `&&`/`||` or first-class `Range` values.
- `for` supports only exclusive ranges and `List<Int>`.
- Assignment only writes an existing element of a mutable local list; there is
  no insertion, removal or slicing.
- Push uses libc `realloc`, not `nova_list_push` or the seed allocator. List
  storage is never freed; ownership/GC is unresolved.
- Bounds failure is `llvm.trap`, not the seed's Nova panic.
- Maps, sets, structs, enums, tuples, arrays and Option/Result remain.
- Strings lack concat, equality, interpolation and runtime calls.
- Overflow checks and the full runtime ABI remain.
- `nova build` still uses the Rust seed; the Nova tool is not yet the CLI.
- Sema lacks modules, generics, traits, methods and full diagnostics.
- No Nova LSP or differential Rust/Nova harness.

## Resume here

1. Add sema diagnostics for match patterns and exhaustiveness, or implement
   structs/enums so enum match patterns become possible.
2. Design seed runtime linkage for `nova_list_push`/`nova_list_at`; do not stub
   it.
3. Run codegen/parser/sema suites; update `STATE.md`, `EVIDENCE.md`,
   `TASKS.md` and `PARITY.md`; commit.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (73/0)

## Blockers and approvals

- No external blocker for sema match validation or structs/enums.
- Seed runtime linkage remains an explicit design decision; do not claim
  `nova_list_*` parity while push uses libc.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP needs sema, stdio and the parity slices above.
