# Executable tasks

Stable IDs are permanent. A parent is not VERIFIED until its required children
and integration gate pass. The full matrix is `PARITY.md`.

## COMP-01 — Nova compiler frontend

- COMP-01.01 — AST parser corpus: **VERIFIED**. 51/0, 29-file corpus. The
  parser appends nodes and links to a C arena and materialises them once, so
  parsing the compiler's 15,888-line expanded source takes 0.25s / 353 MB
  instead of a quadratic build that was OOM-killed.
- COMP-01.02 — Body semantic slice: **PARTIAL**. 39/0; declarations, scopes,
  expression/return types, calls, fields (including duplicate declarations and
  initializers), enums (duplicate/unknown variants and payload arity),
  control-flow conditions, match pattern validation, payload bindings,
  block-arm checking and Int/Bool/enum exhaustiveness. Typed empty-list
  initializers, arguments, fields and returns are accepted through structural
  wildcard compatibility (`List<?>` and tuple slots), while concrete
  mismatches still fail; `nova check` is clean on the Nova tool's own source.
  39/0.
- COMP-01.03 — Module/import resolution: **VERIFIED** for file and path-dep
  modules. The Nova driver resolves `.nova` paths, extensionless sibling
  modules, module directories (`src/lib.nova`, `src/main.nova`), transitive
  imports and `[dependencies]` path dependencies; it validates std imports
  (`std.io`, `std.core` only) and diagnoses missing, malformed and cyclic
  imports in terms of the importing file. `NOVA_TOOL=modules` prints the
  module graph with per-module declaration counts. Evidence: `tests/cli.sh`
  33/0 (extensionless + transitive three-file program exits 8; path dep exits
  42; four diagnostics). Registry-only Box imports resolve through
  `nt_registry_module` from the fetch cache and `Box.lock`.
- COMP-01.04 — Generics, methods, traits, closures: **PARTIAL**. Explicit
  single-parameter generic functions (`fn id<T>(x: T) -> T` called as
  `id<Int>(7)`) are monomorphized before sema/codegen. Non-generic `impl Type`
  and `impl Trait for Type` methods take `self`, check trait conformance and
  dispatch concretely at compile time. Capturing closures and named function
  values lower to `%nova.fn { fn, env }`; captures are copied by value, and
  higher-order, returned and nested closures work. Evidence: `tests/cli.sh`
  33/0 (generic identity, closure captures/higher-order/returned/nested,
  trait dispatch and missing-method `E0201`), `tests/parser.sh` (trait,
  signature, trait impl and closure AST), `tests/codegen.sh` 248/0. Generic
  structs/enums/methods, inference without explicit `<Type>`, `dyn Trait`,
  generic trait impls and mutable captures remain TODO.
- COMP-01.05 — Lexer parity, comments/interpolation/recovery: **PARTIAL**.
  String interpolation is lowered end-to-end: the parser splits literal and
  `${expr}` parts (nested braces, quoted strings and escapes handled), sema
  types the result as String and rejects non-interpolatable parts, and codegen
  concatenates literal, Int, Bool and String pieces through
  `nova_string_concat`. A Nova-built compiler now prints `p=4`, `sum=3`,
  `esc=${p.x}` and `struct=9` exactly like the seed. Evidence:
  `tests/codegen.sh` 248/0 (`codegen-interp` exits 19) and `tests/diff.sh`
  18/0. Comment/recovery parity remains.
- COMP-01.06 — Diagnostic parity: codes, spans, incomplete-source recovery:
  **VERIFIED** for the 27-program corpus. `tests/diff-errors.sh` now reports
  27 identical, 0 divergent. Nova matches the seed's `E0200` duplicate
  function/parameter (including the second duplicate-binding diagnostic),
  `E0203` unknown name/function, `E0205` non-function calls, `E0207` field
  errors, `E0208` arity, `E0301` immutable assignment/`push`, `E0302` loop
  control, `E0308` type mismatch at the offending expression and `E0310`
  non-exhaustive match. Duplicate Int/String/enum arms and wildcard-shadowed
  arms are accepted like the seed; malformed parameter lists emit both of the
  seed's `E0103` diagnostics. The corpus is not exhaustive.
- COMP-01.07 — Full type system (collections, Option/Result, arrays/slices):
  **PARTIAL**. Built-in `Option`/`Result` construction and arity (`Some`,
  `None`, `Ok`, `Err`), methods (`is_some`, `is_none`, `is_ok`, `is_err`,
  `unwrap_or`), exhaustive `Some`/`None` and `Ok`/`Err` matching with payload
  bindings and bare binding patterns, recursive Option/Result compatibility,
  and checked `?` propagation including error-type mismatch. Fixed arrays
  `[T; N]` are checked in bindings, arguments, returns and `for`; length and
  element mismatches are diagnosed. Evidence: `tests/sema-ast.sh` 39/0 and
  `tests/diff-errors.sh` 27/27 identical, including Option binding/inference/
  arity/exhaustiveness and array length/element fixtures. Slice views are
  checked for `List<T>` and `[T; N]` through `as_slice()`, with indexing,
  assignment, `len`, `for`, parameters and returns. Codegen lowers
  `Some`/`None`/`Ok`/`Err`, methods, match and `?` as `{ i1, T }` and
  `{ i1, T, E }`, fixed arrays as inline `[N x T]`, and slice views as
  `%nova.slice { ptr, i64 }` with the same operations. `Map<K,V>` and
  `Set<T>` are checked with annotated `map()`/`set()`, `len`, `get`,
  `contains_key`/`contains` and `insert` for Int/Bool/String keys. Maps/sets
  lower to `%nova.map` with the `nova_map_*` runtime; map/set literals,
  iteration, Char keys and generics/traits remain.
- COMP-01.08 — Incomplete-source AST/sema for the LSP: **TODO**.
- COMP-01.09 — FFI/module runtime semantics: **TODO**.

## CODEGEN-01 — Nova LLVM IR lowering

- CODEGEN-01.01 — Straight-line Int functions: **VERIFIED**. 15/0 evidence.
- CODEGEN-01.02 — Native execution through clang: **VERIFIED**. Native results
  9, 2 and 17; Mach-O on this host.
- CODEGEN-01.03 — Control flow and comparisons: **VERIFIED**. `if`/`else`,
  `while`, `icmp`, labels; loop/branch binary exits 17.
- CODEGEN-01.05 — Immutable-binding SSA lowering: **VERIFIED**. No `alloca` for
  params/`let`; mutable fixture exits 2.
- CODEGEN-01.04 — Runtime ABI alignment: **PARTIAL**. Int/Unit/String/List
  signatures and calls use `ptr %arg_env`, `nova_user_<index>`, `void` returns,
  `%nova.string` and `%nova.list` by value, and the `@main` wrapper; list
  `nova_list_at`/`nova_list_push` calls and the linked C runtime are verified.
  Non-generic methods pass `self` by value as the first parameter; closures use
  `%nova.fn` plus a `ptr %arg_env` capture environment and indirect calls.
  Evidence: 248/0, `tests/cli.sh` 33/0.
- CODEGEN-01.06 — Strings, lists, maps, sets, structs, enums, tuples, arrays,
  slices, Option/Result: **PARTIAL**. String literals, parameters/returns and `.len()`  are verified; `List<Int>` literals (including empty), `.len()`, by-value
  parameters/returns and runtime-linked indexing/assignment/push are verified.
  `nova_list_at` supplies bounds enforcement and `nova_list_push` supplies
  growth; both survive a return. Value
  structs are verified with nested fields, literals, reads, field assignment,
  by-value parameters/returns and recursive-layout rejection. Enums are
  verified with tags, Int/Bool payloads, construction, payload bindings, wildcard
  arms, exhaustive matching and by-value parameters/returns. Lists index, assign
  and push through `nova_list_at`/`nova_list_push`; literals use malloc-backed
  storage, `==`/`!=` compare lengths/elements, `+` concatenates, and slicing copies a
  start..end range. Native expression-valued `if`/`else`, qualified module
  calls, `%`, `char_string`, `json_string` and `http_get` are also lowered.
  Fixed arrays lower inline as `[N x T]` with bounds-checked indexing and
  assignment, `len`, `for`, parameters and returns. Option/Result lower as
  `{ i1, T }` and `{ i1, T, E }` with constructors, methods, match and `?`.
  Slice views from lists and arrays lower to `%nova.slice` with `as_slice`,
  indexing/assignment, `len`, `for`, parameters and returns. Map/Set basics
  lower to `%nova.map` with annotated empty constructors, `len`, lookups and
  inserts through `nova_map_insert`/`nova_map_get`/`nova_map_add`/
  `nova_map_len`. Non-generic `impl`/trait methods and value-capturing
  closures/function values lower through the same call ABI. Enum-typed payloads,
  nested/tuple-typed payloads, map/set literals, iteration and Char keys remain.
  String concatenation, equality and literal
  match patterns are verified through the runtime. String interpolation is
  verified: literal, Int, Bool and String parts concat through
  `nova_string_concat`, with escapes, member access and nested struct literals
  matching the seed (`codegen-interp` exits 19; `tests/diff.sh` 18/0).
- CODEGEN-01.07 — Overflow, division-by-zero, bounds and alignment checks:
  **PARTIAL**. List, fixed-array and slice indexing/assignment route through
  `nova_list_at`/`nova_panic_bounds`; overflow,
  division-by-zero and alignment checks remain.
- CODEGEN-01.08 — `for`, `match`, `break`/`continue`, short-circuit booleans:
  **PARTIAL**. Range/list/array/slice/nested `for`, `break`/`continue`, Int/Bool
  `match` expressions, Option/Result matches with payload bindings, enum
  variants with Int payload bindings, and Unit block arms are verified natively.
  Sema rejects string/nested patterns and duplicate or unreachable arms, checks
  arm/binding types and enforces Int/Bool/enum exhaustiveness. An unmatched or
  empty match calls `nova_panic_str`; String match uses literal patterns.
  Short-circuit booleans are verified; nested patterns remain.
- CODEGEN-01.09 — Clang invocation and linking from the Nova tool: **VERIFIED**.
  `build-ir` writes IR, invokes clang with quoted paths, checks exit status and
  produces a native binary and compiles/links `runtime/runtime.c` for list
  runtime calls. Evidence: `tests/codegen.sh` → 248/0.
- CODEGEN-01.10 — Optimization pipeline parity: **TODO**.

## CLI-01 — Self-hosted driver

- CLI-01.01 — `nova build` via the Nova frontend/codegen with seed fallback:
  **IN_PROGRESS**. The Nova tool now accepts `build` with no arguments for a
  project or `build <source> [output]` for one file. It discovers
  `nova.toml`/`Box.toml`, reads `[bin] path`, recursively inlines relative
  `.nova` imports, builds to `target/nova/<name>`, and returns the native
  status. `run` executes the built binary and propagates its exit status;
  `check` runs body sema over the flattened project. Box dependencies,
  standard-library module semantics, and seed fallback remain. Evidence:
  `tests/cli.sh` → 33/0.

## LSP-01 — Nova language server

- LSP-01.01 — Stdio/JSON-RPC streaming primitive: **VERIFIED**. Content-Length
  framing over stdio, including a blank-line/EOF distinction; evidence:
  `tests/lsp.sh`.
- LSP-01.02 — initialize, capabilities, document sync, diagnostics: **VERIFIED**
  for the Nova sema slice. `initialize` advertises full-document sync;
  `didOpen`/`didChange`/`didSave` publish diagnostics and `didClose` clears
  them. Evidence: `tests/lsp.sh` against the seed-built tool and two Nova-built
  generations.
- LSP-01.03 — Completion: **VERIFIED** for the current buffer. Keywords,
  builtins, declared functions/structs/enums and `.` member items; evidence:
  `tests/lsp.sh` (seed-built and generations 1/2/3).
- LSP-01.04 — Hover, definition, document symbols: **VERIFIED**. Word lookup
  at the request position resolves top-level declarations, parameters, fields,
  variants and bindings; hover renders the declaration signature, definition
  returns its URI/range and `documentSymbol` returns the item tree with fields,
  variants and impl methods as children. Evidence: `tests/lsp.sh`.
- LSP-01.05 — Whole-document formatting: **VERIFIED**. `textDocument/formatting`
  runs the Nova `fmt_source` and returns one full-document `TextEdit`; already
  formatted buffers return no edits. Evidence: `tests/lsp.sh`.
- LSP-01.06 — Incremental sync, stale documents, incomplete source, error
  recovery: **TODO** (beyond the current Rust server).
- LSP-01.07 — Rename, references, semantic tokens, code actions, cancellation:
  **TODO** (beyond the current Rust server).
- LSP-01.08 — Protocol transcript tests and Studio integration: **VERIFIED**.
  `tests/lsp.sh` asserts diagnostics, completion, hover, definition, document
  symbols and formatting against the Nova-built tool and generations 1/2/3;
  `tests/toolchain.sh` drives the `nova lsp` entry point and asserts the Nova
  server's version. `nova lsp` spawns the Nova server and Studio prefers it.

## DIFF-01 — Differential validation

- DIFF-01.01 — Frozen-reference compile/run harness: **VERIFIED**.
  `tests/diff.sh` builds 18 fixtures with the Nova compiler, wraps each
  Int-returning entry point in a Unit `main` that prints the result, and
  compares native stdout with `tests/diff-golden.txt`. 18/0 matched, covering arithmetic, control flow,
  `for`, list push/mutation, structs, match, expression `if`, printing,
  short-circuit booleans, tuples, strings, interpolation, `var`, Option/Result,
  slice views and map/set basics. Known acceptance divergences (seed rejects while Nova
  accepts): empty-list inference (`codegen-list`), range slicing
  (`codegen-list-slice`), the reserved `None` variant (`codegen-enum`), `for`
  over a fixed array (`codegen-option-array`) and `for` over a slice
  (`codegen-slice-for`).
- DIFF-01.02 — Golden invalid-program diagnostics comparison: **VERIFIED**.
  `tests/diff-errors.sh` runs 27 invalid programs through the Nova frontend's
  `NOVA_TOOL=sema-ast`, normalizes each diagnostic to
  `CODE|message|line:column`, and compares it with the frozen Rust-reference
  goldens. 27/0 compared, 27 identical, 0 divergent. The goldens pin today's
  output so a diagnostic change in either driver fails the suite; all 27 now
  match, so `COMP-01.06` is verified for this corpus.

## PERF-01 — Performance at parity

- PERF-01.01 — Benchmark harness: **PARTIAL**. `tests/perf.sh` now measures
  the Nova compiler alone with a clean target, prints raw samples and medians,
  and requires the artifact to compile and run a hello program. The former
  seed-vs-Nova comparison is historical after the 2026-11-26 Rust removal
  (`PERFORMANCE.md`); no speed claim is made across different language subsets.
- PERF-01.02 — Check-workload benchmark: **PARTIAL**. The same harness checks
  `examples/nova-tool` through both drivers with a clean target per run,
  alternating order, 1 warmup + 5 samples, and requires both to report no
  errors. Latest: seed median 88.5 ms (91.1 94.9 88.0 88.5 88.5), Nova median
  359.5 ms (366.9 357.2 360.0 359.5 359.2). This is not equal work: Nova's sema
  covers less than the seed's, so the check timing is a measurement, not a
  speed result. Parse and LSP remain unmeasured because the seed has no
  parse-only command and `nova lsp` always prefers the Nova server.

## BOOT-02 — Self-compilation

- BOOT-02.01 — Generation 1 from the seed: **VERIFIED**. The Rust-built
  `nova-tool` built `/tmp/nova-self4` (2.15s / 3.08 GB peak RSS); it compiled
  the sample (exit 9) and passes `tests/lsp.sh`.
- BOOT-02.02 — Generation 2 from generation 1: **VERIFIED**. `/tmp/nova-self4`
  built `/tmp/nova-self-2c` (2.16s / 2.91 GB peak RSS); its sample exited 9 and
  it passes `tests/lsp.sh`.
- BOOT-02.03 — Generation 3 and comparison: **VERIFIED (IR fixed point)**.
  `/tmp/nova-self-2c` built `/tmp/nova-self-3c` (2.29s / 2.90 GB); its sample
  exited 9 and it passes `tests/lsp.sh`. Generations 1–3 emit byte-identical IR
  (`sha256 7e236845…`); linked executables differ by 49 bytes of linker
  metadata, so no byte-identical executable is claimed. The parser AST arena
  removed the quadratic build that killed generation 2.
- BOOT-02.04 — Build the Nova LSP with the self-hosted compiler: **VERIFIED**.
  `nova-tool lsp` contains transport, sync, diagnostics, completion, hover,
  definition, document symbols and formatting, and `/tmp/nova-self`,
  `/tmp/nova-self-2` and `/tmp/nova-self-3` all pass `tests/lsp.sh`. The
  `nova lsp` entry point remains under `BOOT-02.05`.
- BOOT-02.05 — Studio uses the Nova-built LSP: **VERIFIED**. The Rust CLI's
  `nova lsp` spawns `nova-tool lsp` and only falls back to
  `nova_lsp::run_stdio` when the tool is missing. Studio's `novaToolExecutable`
  prefers `NOVA_TOOL_BIN`, then a `nova-tool` sibling or `target/*/nova-tool`,
  and `LspClient` spawns it with `lsp`. Evidence: `tests/toolchain.sh`
  initialize returns `"version":"0.2.0"`; `tsc --noEmit` passes.
