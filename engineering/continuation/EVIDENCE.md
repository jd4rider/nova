# Evidence

All results below were run on `selfhost/pure-nova` on 2026-11-25. The Rust seed
was removed on 2026-11-26; commands below that mention `cargo` or
`bootstrap/nova` are historical and are not runnable now. The post-removal
verification is in `sessions/2026-11-26.md`.

## Build

```sh
(cd examples/nova-tool && ../../bootstrap/nova build)
```

Result: PASS.

## Nova CLI driver

```sh
sh examples/nova-tool/tests/cli.sh
```

Result: PASS — passed 22, failed 0 (the module checks below grew it).

## Nova LSP

```sh
sh examples/nova-tool/tests/lsp.sh
sh examples/nova-tool/tests/lsp.sh /tmp/nova-self
sh examples/nova-tool/tests/lsp.sh /tmp/nova-self-2
```

Result: PASS. `nova-tool lsp` reads Content-Length-framed JSON-RPC from stdin
and writes framed responses/notifications to stdout. It implements `initialize`
(full-document sync, completion, hover, definition, document symbol and
formatting capabilities), `shutdown`, `exit`, `didOpen`, `didChange`,
`didSave`, `didClose`, `textDocument/completion`, `textDocument/hover`,
`textDocument/definition`, `textDocument/documentSymbol` and
`textDocument/formatting`. Diagnostics come from the Nova parser/sema slice; an
invalid `Int` binding reports `E0301` at the source position, the fixed buffer
reports none, and closing the document clears diagnostics. Completion returns
keywords, builtins and declarations from the current buffer; the transcript
checks `helper`, `main`, `fn` and `println`. Hover over the `helper` call
returns `fn helper(value: Int) -> Int` over the usage range, and hovering a
struct field returns `field x: Int`. Definition returns the document URI and
the declaration range (line 0, character 3 for `helper`). Document symbols
return `helper`/`main` (Function), `Point` (Struct with `x`/`y` children) and
`Mode` (Enum with `On`/`Off` children). Formatting turns over-indented input
into exactly `fn main() {\n    let x: Int = 1\n}\n` via one edit. The same
transcript passes against the seed-built tool and Nova generations 1, 2 and 3.
`nova lsp`/Studio wiring was completed afterwards; see the entry-point section
below.

Host-bundle lookup fixes a relative `NOVA_TOOL_RUNTIME`: `nt_find_gui` now
resolves the runtime path against the working directory before taking its
dirname, so `../../runtime/runtime.c` finds `runtime/gui.m`. Before the fix the
path normalised to `runtime/gui.m` under the project directory, the host object
was skipped and the self-build failed at link with undefined `nova_fs_*`,
`nova_gui_*` and `nova_env` symbols.

The Nova tool now reads process arguments, accepts `build` with no arguments
for a project or `build <source> [output]` for one file, prints `built <path>`,
and returns the native exit status. The single-file fixture exits 7 and the
two-file project imports `math.nova` as `math`, calls `math.double(5)`, builds
`target/nova/cli-project`, and exits 10; `run` rebuilds and propagates that 10, while `check` succeeds over
the flattened project. Project discovery reads `nova.toml` or
`Box.toml`, `[package] name`, and `[bin] path`, and recursively inlines
relative `.nova` imports. Argument access is implemented for Apple hosts
(`_NSGetArgc`/`_NSGetArgv`); other hosts return no arguments. Box dependencies,
standard-library module semantics and Rust-seed fallback remain.

## Parser

```sh
sh examples/nova-tool/tests/parser.sh
```

Result: PASS — passed 51, failed 0; 29-file AST corpus.

Commits: `93d61dc`, `983fd6a`.

## Body sema

```sh
sh examples/nova-tool/tests/sema-ast.sh
```

Result: PASS — passed 39, failed 0.

- Option/Result constructors, built-in methods, exhaustive variant matching
  with payload bindings, and checked `?` propagation are accepted.
- Fixed arrays `[T; N]` are checked in bindings, arguments, returns, indexing,
  assignment and `for`; length and element mismatches report `E0308`.
- Duplicate struct fields/initializers and duplicate enum variants report
  `E0307`.
- Enum constructor and match payload arity report `E0303`; unknown variants
  report `E0305`; incomplete enum matches report `E0310`.
- Exhaustive Int, Bool, String and enum matches are accepted; payload bindings
  carry their declared types into the arm body. String matches reject
  non-literal patterns and missing `_`.
- Duplicate Int/String/enum arms and arms after `_` are accepted like the seed;
  unsupported binding patterns report `E0311`; an incomplete Bool match reports
  `E0310`.
- Mixed arm types report `E0301`, and a Unit block-arm body is checked rather
  than skipped.
- Type mismatches report `E0308` at the offending expression, immutable
  assignment/`push` report `E0301`, and call/field/arity/name errors use
  `E0203`/`E0205`/`E0207`/`E0208` like the seed.

## Declaration sema

```sh
sh examples/nova-tool/tests/sema.sh
```

Result: PASS — passed 4, failed 0.

## Codegen and native execution

```sh
sh examples/nova-tool/tests/codegen.sh
```

Result: PASS — passed 221, failed 0.

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
- List-copy fixture creates two mutable copies of one list, pushes through both,
  and returns 7; without cloning the second `realloc` would use a stale pointer.
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
- List slice fixture copies `values[1..4]`, verifies an empty `[2..2]` slice, and
  uses the copied elements; native 10.
- String transform fixture calls `nova_string_replace` and `nova_string_lower`;
  the transformed lengths sum to 16.
- String bytes fixture verifies `is_empty()` and fills a `List<Int>` with
  `nova_string_bytes`; the first byte and length produce 95.
- String upper fixture uppercases `Hello` and returns its first byte, 72, proving
  the runtime changes bytes rather than only preserving length.
- List ends fixture reads `first()` and `last()` through `nova_list_at`; the
  values 4 and 9 produce 49.
- List contains fixture finds 8 in `[3, 8, 13]`, rejects 4, and returns the end
  sum 16.
- List reverse fixture copies `[1, 2, 3]` backwards; the reversed ends produce 31.
- List sort fixture copies `[3, 1, 2]` into a native qsort-backed result; the
  sorted ends produce 13.
- List sum fixture accumulates `[1, 2, 3, 4]` through `nova_list_at`; result 10.
- List extremes fixture scans `[5, 2, 9]`; min 2 and max 9 produce 29.
- List index_of fixture finds 8 at index 1, misses 4 with -1, and combines the
  results into 11.
- Struct enum fixture packs a flat `Point` and a nested `Labeled(String, Point)`
  into enum slots, reconstructs both bindings, and returns 9.
- Expression-`if` fixture lowers both arms to a merge value and exits 13.
- Qualified-call project fixture imports `math.nova` as `math` and calls
  `math.double(5)`; the project builds and exits 10.
- Builtin fixture combines `char_string(65)` with `json_string`; the resulting
  text length is 5.
- `http_get` fixture emits `call %nova.string @nova_http_get` and clang accepts
  the linked IR without making a network request.
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
  `build-ir` links `runtime/runtime.c` and exits 23. IR lines now go through
  `nova_ir_reset`/`nova_ir_line`/`nova_ir_take`; the stage-1 self-build measured
  1.60s and 1.67 GB peak RSS at that checkpoint instead of 2.48s / 5.84 GB
  before the sink.
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
libc `malloc` and storage is never freed. Mutable list initialization and
reassignment clone the backing array through `nova_list_clone`, but function
arguments and `let` bindings still pass headers by value; nested list ownership
and GC remain missing. Structs are value-only: no generics,
methods, equality, printing or
references. Enum payloads support Int, Bool, String and struct values in codegen;
enum-typed payloads, maps, sets, tuples, arrays and Option/Result remain.
The Nova tool has a project build command, but `nova build` still uses the
Rust seed. This is not a compiler-speed measurement.

## Parser AST arena (bootstrap regression)

Generation 2 of the self-build was killed by the OS (signal 9) after ~107s and
5.68 GB peak RSS. A `sample` of the runaway process showed a 68.7 GB physical
footprint, and the stack mapped to `a_item`/`a_fn`/`a_block`/`a_stmt`/`a_expr`
and `a_add`, not codegen. The cause was value semantics: `a_add` copied both
growing AST lists for every node, so parsing was quadratic. A synthetic
160-function source already allocated 570 MB.

Nodes and links now append to a C-side arena (`nova_ast_*` in
`runtime/runtime.c`) by index; `a_parse` materialises the two lists once at the
end into fresh local lists, which `nova_list_push` grows in place. `a_primary`
reads the in-progress node fields through arena accessors. Only `Int`/`String`
cross the extern boundary, which the Rust seed's sema requires.

```sh
NOVA_TOOL=emit-ir NOVA_TOOL_PATH=/tmp/nova-expanded.nova /tmp/nova-self4
```

Result: PASS — the 15,888-line expanded compiler source parses and generates IR
in 0.25s / 353 MB peak RSS; before the fix the same work was killed at stage 2
after >107s / 5.68 GB. The former Rust seed embedded `runtime.c` with
`include_str!`; today rebuilding `bootstrap/nova` picks up runtime changes.

## Self-build bootstrap

```sh
cd examples/nova-tool
NOVA_TOOL_OUT=/tmp/nova-self4 NOVA_TOOL_RUNTIME=../../runtime/runtime.c \
  ./target/nova/nova-tool build
NOVA_TOOL_OUT=/tmp/nova-self-2c NOVA_TOOL_RUNTIME=../../runtime/runtime.c \
  /tmp/nova-self4 build
NOVA_TOOL_OUT=/tmp/nova-self-3c NOVA_TOOL_RUNTIME=../../runtime/runtime.c \
  /tmp/nova-self-2c build
```

Result: PASS — generation 1 (`/tmp/nova-self4`, 2.15s / 3.08 GB peak RSS),
generation 2 (`/tmp/nova-self-2c`, 2.16s / 2.91 GB) and generation 3
(`/tmp/nova-self-3c`, 2.29s / 2.90 GB) all built. Each generation compiled
`tests/codegen-sample.nova` (exit 9) and passed `tests/lsp.sh`. All three emit
byte-identical IR (`sha256 7e2368452e2535796a3213c92ba4d7e1346b86e1`); the
linked executables differ by 49 bytes of linker metadata, so the fixed point is
at the IR level, not executable bytes. `nova build` still runs the Rust seed
and no Rust crate has been deleted.

The LSP protocol test also passes against generation 1 and generation 2:

```sh
sh examples/nova-tool/tests/lsp.sh /tmp/nova-self
sh examples/nova-tool/tests/lsp.sh /tmp/nova-self-2
sh examples/nova-tool/tests/lsp.sh /tmp/nova-self-3
```

Result: PASS — all three Nova-built binaries returned
`nova-lsp diagnostics and completion ok`.

## Parity audit

`PARITY.md` maps every audited compiler and LSP capability to a stable task ID.
The former Rust LSP header (since deleted) confirmed full-document sync,
diagnostics, completion, hover, definition, document symbols and formatting;
incremental sync, rename, references, semantic tokens, code actions and
cancellation were absent there too.

## Commit history

- Parser AST arena bootstrap-fix checkpoint — codegen 248/0, diff 18/0,
  parser 51/0, sema 4/0, sema-ast 39/0, diff-errors 27/27, CLI 22/0, LSP PASS;
  generations 1–3 build (2.15s/3.08 GB, 2.16s/2.91 GB, 2.29s/2.90 GB), emit
  byte-identical IR, compile the sample (exit 9) and pass `tests/lsp.sh`;
  full-source parse 0.25s / 353 MB instead of an OOM kill.
- Nova LSP hover/definition/symbols/formatting checkpoint (working tree) —
  221/0 codegen, 11/0 CLI, 125/0 toolchain, 64/0 boxes, 10/0 expr, 6/0
  formatter, LSP protocol PASS on the seed-built tool and generations 1/2/3;
  all three generations build themselves and their samples exit 9.

- Nova LSP completion + IR sink checkpoint — 221/0 codegen, 11/0 CLI, 51/0
  parser, 125/0 toolchain, LSP protocol PASS; stage-1 build 1.60s / 1.67 GB.
- Nova LSP diagnostics checkpoint — 221/0 codegen, 11/0 CLI, 125/0 toolchain,
  LSP protocol PASS.
- Stage 1/2/3 Nova-tool bootstrap checkpoint — 221/0 codegen, 11/0 CLI.
- Self-build prerequisites checkpoint — 209/0 codegen, 11/0 CLI.
- Nova CLI build/run/check checkpoint — 184/0 codegen, 9/0 CLI.
- Nova CLI project build checkpoint — 184/0 codegen, 7/0 CLI.
- Nova CLI single-file build checkpoint — 184/0 codegen, 4/0 CLI.
- Struct enum payloads checkpoint — 184/0 codegen, 28/0 sema.
- List index_of checkpoint — 180/0 codegen, 28/0 sema.
- List min/max checkpoint — 177/0 codegen, 28/0 sema.
- List sum checkpoint — 174/0 codegen, 28/0 sema.
- List sort checkpoint — 171/0 codegen, 28/0 sema.
- List reverse checkpoint — 167/0 codegen, 28/0 sema.
- List contains checkpoint — 163/0 codegen, 28/0 sema.
- List first/last checkpoint — 160/0 codegen, 28/0 sema.
- String upper checkpoint — 157/0 codegen, 28/0 sema.
- String is_empty/bytes checkpoint — 153/0 codegen, 28/0 sema.
- String replace/lower checkpoint — 149/0 codegen, 28/0 sema.
- List slices checkpoint — 145/0 codegen, 28/0 sema.
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

## LSP entry point (BOOT-02.05)

```sh
(cd examples/nova-tool && ../../bootstrap/nova build)
sh examples/nova-tool/tests/lsp.sh
sh examples/nova-tool/tests/toolchain.sh
(cd editor && npx tsc --noEmit -p tsconfig.json)
```

Result: PASS. `nova lsp` runs the Nova-built `nova-tool lsp` with inherited stdio and returns
the child's exit code. A framed
`initialize` request through `nova lsp` returns
`"serverInfo":{"name":"nova-lsp","version":"0.2.0"}`, the Nova server's version
(the Rust server reports the crate version), and `tests/toolchain.sh` asserts
that string — passed 126, failed 0. Studio's `novaToolExecutable()` resolves
`NOVA_TOOL_BIN`, a sibling `nova-tool`, or `target/*/nova-tool`, and `LspClient`
spawns it with `lsp`; the editor typechecks clean.

## Module resolution (COMP-01.03)

```sh
(cd examples/nova-tool && ../../bootstrap/nova build)
sh examples/nova-tool/tests/cli.sh
NOVA_TOOL=modules NOVA_TOOL_PATH=$PWD/examples/nova-tool/tests/cli-project/src/main.nova \
  examples/nova-tool/target/nova/nova-tool
(cd examples/nova-tool && NOVA_TOOL_OUT=/tmp/nova-mod-self \
  NOVA_TOOL_RUNTIME=../../runtime/runtime.c ./target/nova/nova-tool build)
```

Result: PASS — passed 22, failed 0. The Nova driver no longer scans lines: it
parses each import, resolves it and expands it transitively. `.nova` paths are
file-relative; a bare name is tried as a `[dependencies]` path dependency (its
`[bin] path`, then `src/lib.nova`/`src/main.nova`), then as a sibling
`<name>.nova`, then as `<name>/src/{lib,main}.nova`. A three-file program
(`main.nova` imports `util`, `util.nova` imports `inner.nova`) exits 8, and a
path dependency exposes `answer()` for exit 42. `std.io` and `std.core` are
accepted (their functions are builtins); `std.bogus`, missing modules,
malformed imports and cycles fail the build with a message naming the importer.
`NOVA_TOOL=modules` prints the graph (`path name=stem decls=N`), which is also
how the test checks that declarations were collected.

The self-built tool (`/tmp/nova-mod-self`, built from its own tree by the
seed-built tool with the new resolver) compiles `tests/codegen-sample.nova`
(exit 9) and passes `tests/lsp.sh`; `tests/codegen.sh` remains 221/0 and
`tests/toolchain.sh` 126/0.

## Rust/Nova differential (DIFF-01.01)

```sh
sh examples/nova-tool/tests/diff.sh
```

Result: PASS — `differential: 14 matched, 0 differed`.

After `codegen-interp` joined the corpus, the same command reports
`differential: 15 matched, 0 differed`.

Each fixture's former Rust output is recorded in `tests/diff-golden.txt`; the
Nova compiler builds the fixture and its native stdout is compared with that
golden. No Rust binary is involved.
The seed's project build requires `fn main()` to return Unit, so the harness
renames the fixture's entry point to `diff_main` and appends

```nova
fn main() {
    println(diff_main())
}
```

The compared value is therefore the computed result, not just the exit status,
and a codegen divergence cannot hide behind equal exit codes.

The corpus covers Int arithmetic, `if`/`while`, `for`, list push and mutation,
structs, `match`, expression `if`, `print`/`println`, short-circuit booleans,
tuples, string methods and mutable `var`. Three existing fixtures are not in
the corpus because the seed frontend rejects them while the Nova frontend
accepts them — `codegen-list` (cannot infer an empty list's element type),
`codegen-list-slice` (a range is only allowed as a `for` iterable) and
`codegen-enum` (`None` is a reserved built-in variant). Those are acceptance
divergences to resolve separately; diagnostic text is `DIFF-01.02`.

## Golden invalid-program diagnostics (DIFF-01.02)

```sh
sh examples/nova-tool/tests/diff-errors.sh
```

Result: PASS — `diagnostics: 27 fixtures compared (27 identical, 0 divergent), 0 mismatched`.

Each fixture is rejected by both drivers: the Rust seed runs `nova check` in a
one-file project, and the Nova frontend runs `NOVA_TOOL=sema-ast` on the same
source. Each diagnostic is normalized to `CODE|message|line:column` so paths,
carets and notes do not affect the comparison, and is compared with a golden
passed to `check_case`. The goldens are per-driver because the two output sets
do not agree; a fixture whose expectations match counts as `identical`, and one
whose expectations differ counts as `divergent`. Changing a driver's
diagnostics fails the suite until its golden is updated deliberately.

All 27 fixtures now match exactly. The aligned vocabulary and spans cover name
and call errors (`E0203`, `E0205`, `E0207`, `E0208`), duplicate declarations
(`E0200`, including the second duplicate-binding diagnostic for a parameter),
immutable assignment/`push` (`E0301`), loop control (`E0302`), type mismatches
at the offending expression (`E0308`), and non-exhaustive matches (`E0310`).
Call arity and unknown functions point at the callee, missing fields at the
struct name, and return/binding mismatches at the value. The eight fixtures
added with `COMP-01.07` cover `None` binding and inference, `Some` arity,
Option and Result exhaustiveness, a `None` payload pattern, and array length
and element types.

Two recovery behaviors were aligned to close the corpus:

- Duplicate Int/String/enum arms and arms after `_` are accepted, matching the
  seed; a duplicate Bool arm reports only the missing `false` as `E0310`.
- The parser carries multiple diagnostics, so a malformed parameter list emits
  the seed's two `E0103` errors (identifier for a parameter name, then `)` to
  close the list) with the same token text and span.

`COMP-01.06` is verified for this corpus. The corpus is not exhaustive, so new
diagnostics still need goldens.

## Option/Result and fixed arrays (COMP-01.07)

```sh
sh examples/nova-tool/tests/sema-ast.sh
# passed 39, failed 0
sh examples/nova-tool/tests/diff-errors.sh
# diagnostics: 27 fixtures compared (27 identical, 0 divergent), 0 mismatched
```

The parser now keeps postfix `?` as an `ak_try` node instead of rejecting it.
Sema checks the operand against the current return type: `Option<T>` unwraps
to `T` in a function returning `Option<_>`, `Result<T, E>` unwraps to `T` in a
function returning `Result<_, E>`, and mismatched error types or return shapes
report `E0306`. The checks cover `Some`/`None`/`Ok`/`Err` construction and
arity, `is_some`/`is_none`/`is_ok`/`is_err`, `unwrap_or`, exhaustive
`Some`/`None` and `Ok`/`Err` matching, payload bindings and bare binding
patterns. A bare binding such as `other => ...` covers the scrutinee, matching
the seed; `None(value)` reports `E0311`.

Fixed arrays are represented by their source type text (`[T; N]`). `sa_array_layout`
extracts the element type and length, `sa_compatible` compares both, and
`sa_array_literal` checks list-shaped literals against an expected array type
in bindings, call arguments and returns. Indexing returns the element type and
`for` iterates arrays; `[Int; 3] = [1, 2]` reports the seed's length message,
and `[Int; 2] = [1, "x"]` reports `E0308` at the element.

Diagnostic wording also matches the seed: `None` assigned to `Int` reports
`expected `Int`, found `Option<_>`` without the usual `type mismatch:` prefix,
and type display inserts a space after `Result` commas (`Result<Int, String>`).

This frontend slice is now paired with native lowering; the next sections record
that evidence. Maps/sets, generics and traits remain; range slicing is still a
Nova-only acceptance divergence. The check workload was not rerun because
coverage has not converged.

## Option/Result and fixed-array codegen (CODEGEN-01.06)

```sh
NOVA_TOOL=build-ir NOVA_TOOL_OUT=/tmp/nova-option-array \
  NOVA_TOOL_PATH=examples/nova-tool/tests/codegen-option-array.nova \
  examples/nova-tool/target/nova/nova-tool
# built /tmp/nova-option-array; process exit 119
sh examples/nova-tool/tests/codegen.sh
# passed 248, failed 0
sh examples/nova-tool/tests/diff.sh
# differential: 18 matched, 0 differed
```

The backend uses the seed's inline layouts:

- `Option<T>` is `{ i1, T }`; `Some` inserts the payload at index 1 and sets the
  tag, while `None` is zeroinitializer with a false tag.
- `Result<T, E>` is `{ i1, T, E }`; `Ok` writes index 1 and `Err` writes index 2,
  with the tag distinguishing them.
- Fixed arrays are `[N x T]`. Literals build an aggregate with `insertvalue`,
  indexing and assignment use a `getelementptr` into an alloca, and bounds
  failures call `nova_panic_bounds`.

Constructors are lowered with expected-type context for `None`, `Ok` and `Err`;
`Some` infers its inner type from the payload. The methods `is_some`, `is_none`,
`is_ok`, `is_err` and `unwrap_or` read the tag/payload directly. `match` handles
`Some`/`None` and `Ok`/`Err` payload bindings plus bare bindings, and `?`
returns `None` or the propagated `Err` payload before extracting the success
value. Arrays support literals, indexing, index assignment, `len`, `for`,
parameters and returns.

`codegen-option-result` is the seed differential fixture: both drivers produce
`82` and its build matches. `codegen-option-array` is intentionally codegen-only
in `tests/diff.sh` because the seed rejects `for` over a fixed array with
`cannot iterate over [Int; 3]`; Nova currently accepts it. Generics and nested
patterns remain; `Slice<T>` views and map/set basics are covered next.

## Slice views (COMP-01.07)

```sh
sh examples/nova-tool/tests/codegen.sh
# passed 248, failed 0 (slice fixture exits 44, slice-for exits 6)
sh examples/nova-tool/tests/diff.sh
# differential: 18 matched, 0 differed
```

`Slice<T>` is `%nova.slice = { ptr, i64 }`. `List<T>.as_slice()` extracts the
list data pointer and length; `[T; N].as_slice()` spills the array and takes a
GEP to element zero. Both produce the same view value. Slice indexing and index
assignment bounds-check against the stored length, `.len()` reads it, and `for`
walks the data pointer. Slice parameters and returns use the by-value
`%nova.slice` ABI.

`codegen-slice` covers list and array views, parameters/returns, indexing,
mutable-view assignment, `.len()` and while iteration; it is in the seed
differential, and both drivers produce `44`. `codegen-slice-for` is
codegen-only because the seed rejects `for` over `Slice<Int>` with `cannot
iterate over Slice<Int>`; Nova currently accepts it. Range slicing
`list[start..end]` remains a separate Nova-only, list-returning operation.

## Map and set basics (COMP-01.07)

```sh
sh examples/nova-tool/tests/codegen.sh
# passed 248, failed 0 (map/set fixture exits 23)
sh examples/nova-tool/tests/diff.sh
# differential: 18 matched, 0 differed
```

`Map<K,V>` and `Set<T>` use `%nova.map = { ptr, ptr, i64, i64, i64, i32 }`.
`map()`/`set()` lower only with a concrete annotation; they insert the runtime
entry size and key kind (Int 0, Bool 1, String 3). `len` calls `nova_map_len`;
`get` calls `nova_map_get` and builds an `Option<V>`; `contains_key`/`contains`
compare the returned pointer; map `insert` calls `nova_map_insert`; set
`insert` calls `nova_map_add` and returns whether the value was new.

`codegen-map-set` exercises a `Map<String, Int>` and `Set<Int>` with insertion,
duplicate insertion, missing and present `get`/`unwrap_or`, membership and
`len`; both drivers produce `23`, and it is in the seed differential. Map/set
literals, iteration, Char keys and generic map/set parameters remain.

## Comparable build and check benchmarks (PERF-01.01/02)

```sh
PERF_SAMPLES=5 PERF_WARMUP=1 sh examples/nova-tool/tests/perf.sh
```

Result: PASS, with this environment and workload:

```
perf: environment: Darwin 27.0.0 arm64; Apple M2 Pro
perf: toolchain: rustc 1.96.0 (ac68faa20 2026-05-25); Apple clang version 21.0.0 (clang-2100.4.1.1); commit 4e4ad38
perf: workload: build examples/nova-tool (87 .nova files, 575535 bytes),
perf:   clean target per run, 5 samples and 1 warmup run(s)
perf: warmup 1: seed 4327.1ms, nova 1812.7ms
perf: seed samples (ms): 4309.6 4313.6 4311.1 4322.6 4341.2  median 4313.6
perf: nova samples (ms): 1806.2 1799.0 1798.7 1844.4 1815.1  median 1806.2
perf: correctness PASS (both artifacts compiled and ran hello-perf)
perf: check workload: check examples/nova-tool (87 .nova files),
perf:   clean target per run, 5 samples and 1 warmup run(s)
perf: check warmup 1: seed 90.6ms, nova 354.7ms
perf: check seed samples (ms): 91.1 94.9 88.0 88.5 88.5  median 88.5
perf: check nova samples (ms): 366.9 357.2 360.0 359.5 359.2  median 359.5
perf: check correctness PASS (both drivers reported no errors)
perf: measurement only; build and check coverage still differs, so no faster-than-Rust claim until the parity gate passes
```

The harness copies `libraries/` and `examples/nova-tool` twice and removes
`target/` before every run. Build wall time covers driver startup, module
expansion, sema, codegen and clang; check wall time covers startup, expansion
and sema. It alternates which driver runs first, requires the built compilers
to compile and run a literal-print program, and requires both check drivers to
report no errors. Those correctness checks are smoke tests: they prove both
paths produced a working result, not that the compilers cover the same
language.

The check workload is not equal work: Nova's sema still covers less than the
seed's, so its 359.5 ms median is about 4× the seed's 88.5 ms and must not be
presented as a fair loss. The build workload remains faster for Nova (1806.2 ms
against 4313.6 ms), but feature coverage still differs there too. Earlier runs
before the interpolation fix measured build medians of 4125.1 and 4137.1 ms for
the seed against 1735.4 and 1747.8 ms for Nova; run-to-run variation is real
and should be reported with any result.

That distinction mattered at the time. The Nova-built artifact was 902,136
bytes against 2,012,020 for the seed's, and a direct probe showed it did not
interpolate strings:

```nova
fn main() {
    let path = "x"
    var count = 7
    let name = "nova"
    println("FORMATTED ${path}")
    println("tokens=${count} name=${name}")
    println("${path}")
}
```

- Seed-built: `FORMATTED x`, `tokens=7 name=nova`, `x`.
- Nova-built: `FORMATTED ${path}`, `tokens=${count} name=${name}`,
  `${path}`.

The Nova tool's own `NOVA_TOOL=check` formatting mode showed the same loss
in-tree (`UNFORMATTED ${path}`). Interpolation is documented in
`docs/language.md` and ADR 0005, so this was a real
`COMP-01.05`/`CODEGEN-01.06` gap, not a test artifact. It has since been fixed;
the timings above still compare compilers with different feature coverage and
must not be published as a speed result.

After the fix, the harness measures 87 files (575,535 bytes): build seed samples
4309.6 4313.6 4311.1 4322.6 4341.2 ms (median 4313.6) and Nova samples 1806.2
1799.0 1798.7 1844.4 1815.1 ms (median 1806.2), with both artifacts compiling
and running the hello program. The Nova-built compiler is 2,069,960 bytes
against 2,012,020 for the seed's.

The check workload now runs too: seed samples 91.1 94.9 88.0 88.5 88.5 ms
(median 88.5) and Nova samples 366.9 357.2 360.0 359.5 359.2 ms (median 359.5),
with both reporting no errors. It is not an equal-work comparison because
Nova's sema covered less than the seed's. With the Rust seed removed, these
seed-vs-Nova numbers are historical and parse/LSP workloads remain unmeasured.

## String interpolation (COMP-01.05)

The parser now splits a string token into literal and `${expr}` parts, keeping
raw text for strings without interpolation. `a_interp_end` finds the closing
brace with nested braces, quoted strings, char literals and line comments in
mind. Sema types an interpolation as `String`, checks each expression part and
rejects non-interpolatable types. Codegen builds the result left to right with
`nova_string_concat`, converting `i64` with `nova_i64_to_string` and `i1`
(zero-extended to `i32`) with `nova_bool_to_string`.

At that checkpoint, all suites passed: codegen 226/0, CLI 22/0, diff 15/0,
diff-errors 19/0 (19 identical, 0 divergent), parser 51/0, sema 4/0, sema-ast 30/0,
toolchain 126/0, boxes 64/0, expr 10/0 and formatter 6/0. `codegen-interp`
returns 19, the length of `FORMATTED x 7 true!`.

A compiler built by the Nova compiler was then used to build another
interpolation fixture. Its output matched the seed byte for byte:

```
p=4
sum=3
esc=${p.x}
struct=9
```

The fixture covers member access (`${p.x}`), arithmetic (`${1 + 2}`), an escaped
`\${` that stays literal, and a struct literal inside the interpolation
(`${P { x: 9 }.x}`). Comment/recovery parity is still open.

## Project-check false positives (COMP-01.02)

`nova check` on `examples/nova-tool` reported 177 `E0301` diagnostics, mostly
`expected List<T>, got List<?>` for typed empty-list bindings, arguments,
fields and returns, plus tuple returns with unknown slots. A bare `?` already
compared as a wildcard, but a wildcard inside `List<?>` or a tuple did not.

`sa_compatible` now compares lists and tuples structurally, treating `?`
elements as unknown, and is used for bindings, assignments, arguments, fields,
returns and list elements. Concrete mismatches still fail.

```sh
(cd examples/nova-tool && ./target/nova/nova-tool check)
# Checked .../src/main.nova: no errors
sh examples/nova-tool/tests/sema-ast.sh
# passed 30, failed 0
```

The suite gained a typed-empty-list program (`List<String>` binding, `List<Int>`
field, argument and return) and a non-empty mismatch that still reports
`E0308`. At that checkpoint, codegen 226/0, diff 15/0, diff-errors 19/19
identical, CLI 22/0 and toolchain 126/0 passed after the diagnostic-vocabulary
change.

## Post-removal traits/closures checkpoint (2026-11-26)

```sh
(cd examples/nova-tool && ../../bootstrap/nova build)
cp examples/nova-tool/target/nova/nova-tool bootstrap/nova
cd examples/nova-tool
for t in codegen diff diff-errors parser sema sema-ast cli lsp boxes formatter toolchain; do
  sh ./tests/$t.sh || exit 1
done
```

Result: PASS — codegen 248/0, diff 18/18, diff-errors 27/27, parser 54/0,
sema 4/0, sema-ast 39/0, CLI 33/0, LSP diagnostics/completion/hover/definition/
symbols/formatting ok, boxes 64/0, formatter 6/0, toolchain 126/0.

The new CLI assertions build and run generic identity (`7`), closure capture,
higher-order/returned/nested closures, trait dispatch and a missing trait method
(`E0201`). The runtime probes printed `17 13 8 8 15 7 5` for closures and `12`,
`rect`, `24` for methods. Parser fixtures cover trait, signature, trait impl and
closure AST rendering. This evidence does not cover generic structs/methods,
`dyn Trait`, generic trait impls or mutable captures.

CI change: the Nova job is `runs-on: macos-15` instead of cancelled `macos-14`.
Run `45782985936` on commit `e1a2f74` passed all four jobs (`nova`, `editor`,
`vscode`, `website`); the Nova job completed its bootstrap build, compiler
suites and toolchain/package suites in 3m35s.
