# Current Nova checkpoint

Updated: 2026-11-24
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `selfhost/pure-nova`
HEAD: this checkpoint is committed by the accompanying code/doc commits; prior
`4476a35`.
Working tree: unrelated `editor/` and playground changes remain untouched.

Active milestone: `COMP-01` / `CODEGEN-01` — frontend and LLVM lowering
Active task: `CODEGEN-01.08` — explicit unmatched-match panic
Status: PARTIAL (failure path panics; string/nested patterns and guards remain)

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 101, failed 0.
  - `List<Int>` literals allocate with libc `malloc`; empty lists supported.
  - List parameters and returns use by-value `%nova.list`; a literal survives
    return, identity and indexing (native 13).
  - Mutable local `.push(x)` calls `nova_list_push(list, &x, 8)`, which grows
    with `realloc` and copies the element; a grown list survives return and
    indexing (native 16).
  - `points[i]` and `points[i] = v` call `nova_list_at`, which performs the seed
    bounds check/panic and returns the element address; list/String `.len()` use
    `extractvalue` index 1. A mutated list survives return (native 45).
  - A 5,000-element push loop then indexes the final element; native 10.
  - `for n in start..end` is an exclusive integer range; `for x in list` walks a
    `List<Int>` by index. `break` and `continue` use a nested loop-label stack;
    `continue` in a `for` jumps to the increment block. Native results: range 8,
    list 30, nested 30.
  - `match` supports Int/Bool/enum scrutinees, integer and `true`/`false`
    literal patterns, `Variant`, `Variant(_)` and `Variant(binding)` patterns,
    `_`, expression arms and Unit block arms. Expression arms merge through a
    `phi`; an unmatched or empty match calls `nova_panic_str` and exits 1.
    Native results: Int match 60, Bool/Unit match 9, enum 36, panic exit 1.
  - User structs are emitted as `%nova.struct.<Name>` aggregates with nested
    value fields. Literals use `insertvalue`, field reads use `extractvalue`,
    mutable local field assignment rebuilds and stores the aggregate, and
    struct parameters/returns pass by value. Native struct result: 14.
  - Enums are emitted as `%nova.enum.<Name>` aggregates with an `i32` tag and
    one `i64` slot per maximum payload arity. Bare/call construction, payload
    binding in match arms, wildcard arms and by-value parameters/returns are
    verified. Native enum result: 36.
  - Native results: assignment 45; push 16; push loop 10; list index+length
    23; list ABI 13; Int 9/2/17; Unit caller 4; Unit `main` 0; string 5.
  - `ptr %arg_env`, `nova_user_<index>` and the `@main` wrapper are emitted.
- `sh examples/nova-tool/tests/parser.sh` → passed 50, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 24, failed 0.
  - Duplicate struct fields/initializers and duplicate enum variants report
    `E0307`.
  - Enum constructor arity and match payload arity report `E0303`; unknown
    variants report `E0305`; duplicate enum arms report `E0312`; missing
    variants report `E0313`.
  - Int/Bool/enum patterns are validated against the scrutinee, and payload
    bindings carry their declared types into the arm body.
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
- Native enums: tag-plus-payload aggregate declarations, bare and call
  construction, Int payload bindings, wildcard arms, exhaustive variant
  matching and by-value parameters/returns. Duplicate/unknown variants and
  arity errors are diagnosed in sema.
- Seed list runtime linkage: `nova_list_at` for indexing/assignment and
  `nova_list_push` for growth; `build-ir` compiles and links `runtime/runtime.c`.
  New push/index temporaries are hoisted to the function prologue so loops do
  not grow the stack per iteration.
- Unmatched and empty matches call `nova_panic_str("non-exhaustive match")`
  instead of emitting LLVM `unreachable`.
- Focused native and diagnostic fixtures.

## Incomplete work / known failures

- Match lacks struct/string and nested enum patterns, payload guards and
  first-class match alternatives. Sema rejects unsupported patterns, so they do
  not reach codegen silently.
- No short-circuit `&&`/`||` or first-class `Range` values.
- `for` supports only exclusive ranges and `List<Int>`.
- Assignment only writes an existing element of a mutable local list; there is
  no insertion, removal or slicing.
- List literals still allocate their backing array directly with libc
  `malloc`; storage is never freed and ownership/GC is unresolved.
- `nova_list_at` now supplies the seed bounds panic for indexing and assignment;
  overflow, division-by-zero and alignment checks remain.
- Structs and enums are value-only: no generics, methods, references, tuples
  or arrays. Struct/enum equality and printing remain.
- Enum payloads are `Int`-only in codegen; String/struct payloads and payloads
  nested in other value types are rejected with an error.
- Maps and sets remain.
- Strings lack concat, equality, interpolation and runtime calls.
- Overflow checks and the full runtime ABI remain.
- `nova build` still uses the Rust seed; the Nova tool is not yet the CLI.
- Sema lacks modules, generics, traits, methods and full diagnostics.
- No Nova LSP or differential Rust/Nova harness.

## Resume here

1. Add string and nested enum patterns.
2. Broaden enum payloads beyond Int and add list concat/equality through the
   runtime.
3. Run codegen/parser/sema suites; update `STATE.md`, `EVIDENCE.md`,
   `TASKS.md` and `PARITY.md`; commit.

## Validation

- Command: `(cd examples/nova-tool && ../../target/debug/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (101/0)
- Command: `sh examples/nova-tool/tests/sema-ast.sh`
- Result: PASS (24/0)

## Blockers and approvals

- No external blocker for remaining codegen work.
- List literals still bypass the runtime allocator and storage is never freed;
  do not claim full `nova_list_*`/ownership parity yet.
- Push, publish, deployment and branch deletion require owner authorization.
- LSP needs sema, stdio and the parity slices above.
