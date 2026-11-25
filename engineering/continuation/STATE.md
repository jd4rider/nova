# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: this checkpoint is committed by the accompanying code/doc commits; prior
`625b53a`.
Working tree: unrelated `editor/` and playground changes remain untouched.

Active milestone: `COMP-01` / `CODEGEN-01` — frontend and LLVM lowering
Active task: `COMP-01.02` / `CODEGEN-01.06` — value structs and field checks
Status: PARTIAL (structs lower natively; enums and enum patterns pending)

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 82, failed 0.
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
  - User structs are emitted as `%nova.struct.<Name>` aggregates with nested
    value fields. Literals use `insertvalue`, field reads use `extractvalue`,
    mutable local field assignment rebuilds and stores the aggregate, and
    struct parameters/returns pass by value. Native struct result: 14.
  - Native results: assignment 45; push 16; list index+length 23; list ABI 13;
    Int 9/2/17; Unit caller 4; Unit `main` 0; string 5.
  - `ptr %arg_env`, `nova_user_<index>` and the `@main` wrapper are emitted.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 17, failed 0.
  - Duplicate struct fields and duplicate field initializers report `E0307`.
  - Int/Bool patterns are validated against the scrutinee.
  - Unsupported and empty patterns report `E0311`; duplicate or unreachable arms
    report `E0312`; empty/non-exhaustive matches report `E0313`.
  - Int matches require `_`; Bool matches require both literals or `_`.
  - Mixed expression-arm types report `E0301`; Unit block-arm bodies are checked
    with `sa_block` instead of being skipped.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- Mach-O confirmed on this Apple Silicon host.

## Work completed since previous checkpoint

- Int/Bool match codegen with literal and wildcard patterns.
- Unit block match arms, nested match expressions and phi-based result merging.
- Match semantic validation, block-arm body checking, arm type consistency and
  Int/Bool exhaustiveness.
- Native value structs: aggregate type declarations, construction, field reads,
  field assignment, nested value fields, by-value parameters/returns and
  recursive-layout rejection. Duplicate field declarations/initializers are
  diagnosed in sema.
- Focused native and diagnostic fixtures.

## Incomplete work / known failures

- Match lacks enum/struct/string/binding patterns and an explicit no-match
  panic. Sema rejects unsupported patterns, so they no longer reach codegen
  silently.
- No short-circuit `&&`/`||` or first-class `Range` values.
- `for` supports only exclusive ranges and `List<Int>`.
- Assignment only writes an existing element of a mutable local list; there is
  no insertion, removal or slicing.
- Push uses libc `realloc`, not `nova_list_push` or the seed allocator. List
  storage is never freed; ownership/GC is unresolved.
- Bounds failure is `llvm.trap`, not the seed's Nova panic.
- Structs are value-only: no generic structs, methods, references, enums,
  tuples, arrays or Option/Result. Struct equality and printing remain.
- Maps and sets remain.
- Strings lack concat, equality, interpolation and runtime calls.
- Overflow checks and the full runtime ABI remain.
- `nova build` still uses the Rust seed; the Nova tool is not yet the CLI.
- Sema lacks modules, generics, traits, methods and full diagnostics.
- No Nova LSP or differential Rust/Nova harness.

## Resume here

1. Implement enums and payload-carrying variants, then extend match to enum
   variants and payloads.
2. Design seed runtime linkage for `nova_list_push`/`nova_list_at`; do not stub
   it.
3. Run codegen/parser/sema suites; update `STATE.md`, `EVIDENCE.md`,
   `TASKS.md` and `PARITY.md`; commit.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (82/0)
- Command: `sh examples/nova-tool/tests/sema-ast.sh`
- Result: PASS (17/0)

## Blockers and approvals

- No external blocker for enums or seed runtime linkage.
- Seed runtime linkage remains an explicit design decision; do not claim
  `nova_list_*` parity while push uses libc.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP needs sema, stdio and the parity slices above.
