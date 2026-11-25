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

Result: PASS — passed 28, failed 0.

- Duplicate struct fields/initializers and duplicate enum variants report
  `E0307`.
- Enum constructor and match payload arity report `E0303`; unknown variants
  report `E0305`; duplicate enum arms report `E0312`; incomplete enum matches
  report `E0313`.
- Exhaustive Int, Bool, String and enum matches are accepted; payload bindings
  carry their declared types into the arm body. String matches reject
  non-literal patterns, duplicate arms and missing `_`.
- Duplicate arms report `E0312`; unsupported binding patterns report `E0311`;
  an incomplete Bool match reports `E0313`.
- Mixed arm types report `E0301`, and a Unit block-arm body is checked rather
  than skipped.

## Declaration sema

```sh
sh examples/nova-tool/tests/sema.sh
```

Result: PASS — passed 4, failed 0.

## Codegen and native execution

```sh
sh examples/nova-tool/tests/codegen.sh
```

Result: PASS — passed 141, failed 0.

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
  `extractvalue` for `.len()`, and `call ptr @nova_list_at` for bounds-checked
  indexing; native result 23.
- List ABI fixture emits by-value `%nova.list` parameter/return signatures and
  calls: a literal returned by one function passes through an identity function
  and is indexed by another; native result 13.
- List push fixture emits `call void @nova_list_push` with a prologue-hoisted
element slot; the runtime grows with `realloc`, copies the element and updates
  the length. A grown list is returned and indexed by the caller. Native 16.
- List push-loop fixture pushes 5,000 elements in a `for` loop and indexes the
  final element through `nova_list_at`; native result 10.
- List assignment fixture emits a bounds check and element store through a
  mutable local after a growth push; the mutated list is returned and summed by
  the caller. Native result 45.
- Range-for fixture exercises `continue` before an accumulator and `break`
  before the final value; native result 8.
- List-for fixture iterates a `List<Int>` by index and sums it; native 30.
- Nested-for fixture breaks only the inner loop and still accumulates once per
  outer iteration; native 30.
- Int-match fixture lowers three literal/`_` arms to a phi-merged expression;
  native 60.
- Bool-match fixture uses Unit block arms and a nested Int-match expression;
  native 9.
- Unmatched-match fixture emits `call void @nova_panic_str` with the message
  `non-exhaustive match`; the binary exits 1 and prints the panic.
- String fixture emits `nova_string_concat`/`nova_string_eq`, matches a
  concatenated String against literal patterns and returns 13.
- Print fixture emits `nova_print_str`, `nova_print_i64` and `nova_print_bool`;
  stdout is exactly `hello\n12true\n` and the process exits 7.
- List equality fixture compares equal, unequal and empty `List<Int>` values via
  a length check and an element loop using `nova_list_at`; native 12.
- List concat fixture builds a new list from two sources through
  `nova_list_at`/`nova_list_push`; native 11.
- String method fixture calls `nova_string_trim`, `starts_with`, `ends_with`
  and `contains`; the trimmed text length is 5.
- Bool enum fixture stores and matches `Bool` payloads through zext/trunc in
  the i64 slots; native 7. The fixture keeps nested match expressions in its
  arms, exercising the per-arm completion blocks that feed the outer merge PHI.
- String enum fixture packs `Text(String)` into pointer/length slots and
  `Label(String, String)` into four slots, then rebuilds both bindings; native 10.
- Assert fixtures emit `nova_assert_fail`; the passing program exits 5 and the
  failing program exits 1 with `assertion failed` on stderr.
- Struct fixture emits `%nova.struct.Point` and `%nova.struct.Pair` aggregate
  declarations, `insertvalue` construction, `extractvalue` field reads, a
  rebuilt-and-stored aggregate for field assignment, and by-value struct
  parameters/returns; native 14. Recursive by-value layouts are rejected.
- Enum fixture emits `%nova.enum.Option { i32, i64 }` and
  `%nova.enum.Shape { i32, i64, i64 }`, tag stores/comparisons, payload
  insertion/extraction, payload bindings, wildcard arms and by-value enum
  parameters/returns; native 36.
- `NOVA_TOOL=build-ir` invokes clang, checks status and produces a native binary;
  the test executes that Nova-built binary. A separate list program built through
  `build-ir` links `runtime/runtime.c` and exits 23.
- Host clang accepted every module; Mach-O confirmed on this host.

Limit: ABI parity covers Int, Unit, the string-literal/len subset and the
`List<Int>` literal/parameter/return/push/assignment subset. `for` supports
exclusive integer ranges and `List<Int>` only; range values outside `for`
remain. `match` supports Int/Bool/enum scrutinees, integer/Bool literals,
`Variant`, `Variant(_)` and `Variant(binding)` patterns, String literal patterns,
`_`, expression arms and Unit block arms; sema rejects nested patterns and duplicate or
unreachable arms, checks arm/binding types and enforces Int/Bool/enum
exhaustiveness. Short-circuit booleans remain. An unmatched or empty match calls
`nova_panic_str` and exits 1 instead of reaching `unreachable`. Assignment only
writes an
existing element of a mutable local list; slices, insertion and removal are not
implemented. Indexing, assignment and push use `nova_list_at`/`nova_list_push`;
bounds failure is the runtime's Nova panic. List literals still allocate with
libc `malloc` and storage is never freed. Structs are value-only: no generics,
methods, equality, printing or
references. Enum payloads support Int and Bool in codegen. String/struct
payloads, maps, sets, tuples, arrays and Option/Result remain.
`emit-ir` is still not the full CLI. This is not a compiler-speed measurement.

## Parity audit

`PARITY.md` maps every audited compiler and LSP capability to a stable task ID.
The Rust LSP header (`bootstrap/nova-lsp/src/lib.rs`) confirms full-document
sync, diagnostics, completion, hover, definition, document symbols and
formatting; incremental sync, rename, references, semantic tokens, code actions
and cancellation are absent even in Rust.

## Commit history

- String enum payloads checkpoint — 141/0 codegen, 28/0 sema.
- Nested match PHI fix checkpoint — 137/0 codegen, 28/0 sema.
- Bool enum payloads checkpoint — 137/0 codegen, 28/0 sema.
- String methods checkpoint — 132/0 codegen, 28/0 sema.
- List concat checkpoint — 126/0 codegen, 28/0 sema.
- List equality checkpoint — 122/0 codegen, 28/0 sema.
- Assert builtin checkpoint — 119/0 codegen, 28/0 sema.
- Output builtins checkpoint — 113/0 codegen, 28/0 sema.
- String operations/pattern checkpoint — 107/0 codegen, 28/0 sema.
- Match failure panic checkpoint — 101/0 codegen, 24/0 sema.
- List runtime checkpoint — 97/0 codegen, 24/0 sema.
- Enum checkpoint — 91/0 codegen, 24/0 sema.
- Struct checkpoint — 82/0 codegen, 17/0 sema.
- `match` sema checkpoint — 73/0 codegen, 15/0 sema.
- `match` codegen checkpoint — 73/0.
- `for`/loop-control checkpoint — 67/0.
- `List<Int>` assignment checkpoint — 58/0.
- `List<Int>` push checkpoint — 54/0.
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
