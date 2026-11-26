# Current Nova checkpoint

Updated: 2026-11-26
Master directive: owner-supplied; parity matrix in `engineering/continuation/`.
Repository / branch: `nova` / `main` (Rust-free commit merged; backup at
`backup/main-pre-rustfree-2026-11-26`)
HEAD: `4e4ad38` (LSP completion + IR sink); the LSP feature-parity and
entry-point wiring checkpoint is uncommitted in the working tree.
Working tree: this checkpoint includes the `editor/` LSP wiring; the playground
wasm remains untouched.

Active milestone: `BOOT-02` / `CODEGEN-01` — self-compilation and LLVM lowering
Active task: `COMP-01.07` is PARTIAL — Option/Result and fixed-array checking
and codegen landed, as did `Slice<T>` views and map/set basics; generic checking remains.
Status: RUST-FREE / PARITY INCOMPLETE — the Rust workspace, all 13 `.rs` files
and the Rust build artifacts were deleted on 2026-11-26. `bootstrap/nova` is a
Nova-built compiler; all eleven Nova suites pass, the website checks pass, and a
fresh checkout builds with no Cargo or rustc. The Nova tool builds itself
through three generations with byte-identical emitted IR and compiles a native
sample;
the Nova LSP covers transport, full-document sync, diagnostics, completion,
hover, definition, document symbols and formatting, and `nova lsp`/Studio launch
it; the driver resolves file, transitive and path-dependency modules; string
interpolation lowers end-to-end (`COMP-01.05`), and `nova check` accepts the
tool's own source; all 27 golden invalid programs match the recorded reference diagnostics.
Registry Box imports resolve through the fetch cache and Box.lock. Remaining
work is language parity: generics/traits/closures (`COMP-01.04`), fuller sema,
`nova test`/REPL and a Nova wasm backend.

## Last verified behavior

- `sh examples/nova-tool/tests/codegen.sh` → passed 248, failed 0.
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
    Native results: Int match 60, Bool/Unit match 9, enum 36, String match 13,
    panic exit 1.
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
  - `print`/`println` lower to `nova_print_i64`, `nova_print_bool` or
    `nova_print_str`; exact stdout and exit code are verified. `panic(String)`
    calls `nova_panic_str` and terminates. `assert(Bool)` branches to
    `nova_assert_fail`; passing and failing exits are verified.
  - String `trim`/`trim_start`/`trim_end` and `contains`/`starts_with`/
    `ends_with` call the runtime; a trimmed search program returns 5.
    `replace(from, to)` and `lower()` also call the runtime (native 16).
    `is_empty()` compares the length and `bytes()` fills a `List<Int>` (native 95).
    `upper()` is backed by `nova_string_upper` (native 72).
  - Enum payloads support Int, Bool and String. Bool is zero-extended/truncated
    in one slot; String occupies pointer/length slots with ptrtoint/inttoptr and
    extractvalue/insertvalue packing. A Text/Label fixture returns 10. Nested
    match expressions merge through per-arm completion blocks.
  - `List<Int> ==`/`!=` compare lengths and elements through `nova_list_at`;
    empty, equal and unequal lists are verified natively (12). `List + List`
    concatenates through `nova_list_at`/`nova_list_push` (11). Slices
    `values[start..end]` build a new list through the same runtime (10);
    `first()`/`last()` read through `nova_list_at` (native 49), and
    `contains(value)` scans with the same runtime check (native 16), and
    `reverse()` copies through the runtime (native 31), and `sort()` uses a
    native int64 comparator (native 13), and `sum()` accumulates values (native 10).
    `min()`/`max()` scan through the same runtime (native 29), and `index_of`
    returns the first index or -1 (native 11). Enum payloads may now be structs;
    recursive slot packing/unpacking handles nested struct fields (native 9).
  - `ptr %arg_env`, `nova_user_<index>` and the `@main` wrapper are emitted.
  - Generic lists carry element metadata; `List<String>` and `List<Struct>`
    literals, indexing, assignment, `push`, `for` and `.len()` use the actual
    element size. Native `List<String>` slice returns 3.
  - Tuple types lower to LLVM literal structs; tuple returns and `.0`/`.1`
    extraction work (native 5).
  - `extern "C"` declarations emit `declare` lines and calls go directly to
    the host symbol; `nova_exit` exits 7 natively.
  - `&&`/`||` short-circuit through branch blocks and `!` lowers to `xor i1`;
    native logical result 3.
  - `String.chars`, `split`, `List<String>.join` and
    `Int`/`Bool`/`String.to_string` lower to runtime calls; native results 3,
    8 and 7.
  - Expression-valued `if`/`else` lowers through branch blocks and a merge
    value; native `if` fixture returns 13.
  - Module-qualified calls such as `gui.read(...)` and `math.double(...)` fall
    back to the flattened global function when the receiver is not a local;
    the CLI project uses an aliased import and exits 10.
  - `%` lowers to `srem`; the builtin fixture exercises it through `json`
    decoding and returns 5.
  - `char_string(Int)` and `json_string(String, String)` lower to
    `nova_char_to_string` and `nova_json_string`; the builtin fixture returns 5.
  - `http_get(String)` lowers to `nova_http_get`; the compile-only fixture
    checks the declaration and link without a network request.
  - The Nova driver detects `nova_gui_`/`nova_fs_`/`nova_process_`/`nova_env`
    symbols, compiles `runtime/gui.m` with `-fobjc-arc` on Darwin and links
    Cocoa/CoreText/CoreGraphics/ImageIO/QuartzCore. This is what lets the
    self-built tool link on macOS.
  - Host-bundle lookup resolves a relative `NOVA_TOOL_RUNTIME` against the
    working directory before taking its dirname. Before this fix,
    `../../runtime/runtime.c` produced a wrong `gui.m` path, the host object
    was silently skipped and self-builds failed at link.
- `sh examples/nova-tool/tests/parser.sh` → passed 51, failed 0.
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 28, failed 0.
- `sh examples/nova-tool/tests/sema.sh` → passed 4, failed 0.
- `sh examples/nova-tool/tests/cli.sh` → passed 22, failed 0.
  - Module resolution: `import "util"` loads `util.nova` (extensionless), a
    nested `import "inner.nova" as inner` loads transitively, and the program
    exits 8. A `[dependencies] lib = { path = "../lib" }` import loads the
    dependency's `[bin] path` and exits 42.
  - Diagnostics: `import std.bogus` reports `unknown standard module`, a
    missing `import "nope"` reports `cannot resolve module`, and `a.nova`
    importing `b.nova` back reports `cyclic module import`.
  - `NOVA_TOOL=modules` prints each module path with `name=` and `decls=`.
- Self-build after the module resolver: the seed-built tool built
  `/tmp/nova-mod-self` from its own tree; it built `tests/codegen-sample.nova`
  (exit 9) and passed `tests/lsp.sh`.
- `sh examples/nova-tool/tests/diff.sh` → 15 matched, 0 differed.
  - Each fixture is built twice as a one-file project (Rust seed CLI and
    Nova-built driver). Because the seed requires `fn main()` to return Unit,
    the fixture's entry point is renamed and wrapped in a Unit `main` that
    prints its Int result; the harness compares the printed value.
  - Matched programs cover Int arithmetic, `if`/`while`, `for`, list push and
    mutation, structs, `match`, expression `if`, printing, short-circuit
    booleans, tuples, strings, interpolation and `var`.
  - Both drivers reject `fn main() -> Int` in project builds; that is why the
    wrapper exists, and it is not itself compared as a diagnostic.
- `sh examples/nova-tool/tests/lsp.sh` → PASS.
  - `nova-tool lsp` speaks Content-Length-framed JSON-RPC over stdio.
  - `initialize` advertises full-document sync, completion (`.`/`$` triggers),
    hover, definition, document symbols and whole-document formatting;
    `didOpen`/`didChange`/`didSave` publish diagnostics, and `didClose` clears
    them.
  - Diagnostics come from the Nova parser/sema slice; the invalid-buffer
    fixture reports `E0301` and the fixed buffer reports none.
  - Completion returns keywords, builtins, and declared functions, structs and
    enums from the current buffer; the protocol test checks `helper`, `main`,
    `fn` and `println` after an edit.
  - Hover at a usage renders the resolved declaration signature
    (`fn helper(value: Int) -> Int`, `field x: Int`); definition returns the
    declaration URI/range; `documentSymbol` returns functions, structs, enums
    and impls with fields/variants/methods as children.
  - `textDocument/formatting` returns one full-document edit from the Nova
    formatter and no edits when the buffer is already formatted.
- `sh examples/nova-tool/tests/toolchain.sh` → passed 126, failed 0.
- Self-build, generation 1: `./target/nova/nova-tool build` with
  `NOVA_TOOL_OUT=/tmp/nova-self4` produced a native compiler; it compiled
  `tests/codegen-sample.nova` (exit 9) and passes `tests/lsp.sh`.
- Generation 2 (`/tmp/nova-self-2c`) and generation 3 (`/tmp/nova-self-3c`)
  also build the project (2.16s/2.91 GB and 2.29s/2.90 GB peak RSS); each
  compiles the sample (exit 9) and passes `tests/lsp.sh`.
- All three generations emit byte-identical IR for the compiler
  (`sha256 7e2368452e2535796a3213c92ba4d7e1346b86e1`). Linked executables
  differ by 49 bytes of linker metadata, so the fixed point is at the
  emitted-IR level, not executable bytes.
- The LSP protocol test also passes against `/tmp/nova-self`,
  `/tmp/nova-self-2` and `/tmp/nova-self-3`, including completion.
- The stage-1 self-build is measured at 2.15s and 3.08 GB peak RSS.
- Mach-O confirmed on this Apple Silicon host.

## Work completed since previous checkpoint

- Fixed the parser's quadratic AST build: `a_add` copied the whole
  `nodes`/`links` lists per node through Nova's value semantics, so the
  compiler's own 15,888-line expanded source needed tens of GB and generation 2
  was killed (signal 9) after ~107s. Nodes and links now append to a C-side
  arena (`nova_ast_*`) by index, and `a_parse` materialises the lists once at
  the end. Full-source parse: 0.25s / 353 MB instead of >107s / 5.68 GB.
- Qualified module calls after import flattening; sema and codegen agree.
- Expression-valued `if`/`else` lowering with line/kind diagnostics.
- Modulo lowering and native builtin support for `char_string`, `json_string`
  and `http_get`.
- GUI/fs/process host linking in the Nova driver, mirroring the former Rust bootstrap.
- Mutable list bindings and list reassignment clone their backing storage via
  `nova_list_clone`; `cg_struct_cycle` builds a fresh `List<String>` before
  recursion instead of pushing through a shallow list-header copy.
- A regression fixture pushes through two copies of the same list and returns 7.
- New native fixtures: `codegen-if-expr.nova`, `codegen-builtins.nova`,
  `codegen-http-get.nova`; the CLI project now uses an aliased qualified call.
- Full codegen/CLI/parser/sema/sema-ast suites remain green at 221/11/50/28/4;
  toolchain is 126/0.
- Added `libraries/nova-lsp/src/lsp.nova`: framed stdio transport, initialize,
  shutdown, full-document sync and semantic diagnostics. It is wired into
  `nova-tool lsp` and exercised by `tests/lsp.sh`.
- Added runtime stdin primitives `nova_stdin_read_line` and
  `nova_stdin_read_bytes`; the Nova driver embeds `runtime/runtime.c`, so rebuild
  `bootstrap/nova` after runtime changes.
- The LSP protocol test passes against the seed-built tool, `/tmp/nova-self`
  (generation 1), `/tmp/nova-self-2` (generation 2) and `/tmp/nova-self-3`
  (generation 3), including completion.
- Added document storage and completion: keywords, builtins, declared
  functions/structs/enums, and `.` member items.
- Added hover, definition and document symbols over the Nova AST: word spans,
  declaration lookup (top-level items over parameters/fields/bindings),
  signatures, declaration ranges and a symbol tree with children.
- Added `textDocument/formatting` backed by `fmt_source`; already-formatted
  buffers produce no edits.
- Fixed `nt_find_gui` so a relative `NOVA_TOOL_RUNTIME` still finds and links
  `runtime/gui.m`; self-builds with the documented relative path now link.
- Replaced `cg_line`'s per-line `List<String>` clone with a runtime IR sink
  (`nova_ir_reset`/`nova_ir_line`/`nova_ir_take`) and joined function bodies
  once. At that checkpoint stage-1 peak RSS fell from 5.84 GB to 1.67 GB
  (measured); later map/set and AST-arena work put the current stage-1 build at
  3.08 GB.
- Replaced the driver's import inliner with a module resolver: `.nova` paths,
  extensionless sibling modules, module directories, transitive imports, path
  dependencies and std validation, with missing/malformed/cyclic diagnostics.
  `NOVA_TOOL=modules` prints the module graph and declaration counts.
- Added `tests/diff.sh` and froze the former Rust compiler's native results in
  `tests/diff-golden.txt`; the Nova compiler matches all 18.
- `nova check` is clean on the Nova tool's own source. Structural wildcard
  compatibility for `List<?>` and tuple slots removed 177 false `E0301`
  diagnostics (typed empty-list bindings, arguments, fields and returns)
  without accepting concrete mismatches. Evidence: `tests/sema-ast.sh` 39/0.
- String interpolation is parsed into literal/`${expr}` parts and lowered to
  `nova_string_concat` with `nova_i64_to_string`/`nova_bool_to_string`
  conversions. A Nova-built compiler matches the seed on member access,
  arithmetic, escapes and struct literals inside `${...}`. Evidence:
  `tests/codegen.sh` 248/0, `tests/diff.sh` 18/0.
- Added `tests/diff-errors.sh`, the golden invalid-program diagnostics
  harness: 27 fixtures run through the seed's `nova check` and the Nova
  `sema-ast` frontend, normalized to `CODE|message|line:column`, and compared
  with per-driver goldens. 27/0 compared, 27 identical, 0 divergent.
- Extended `tests/perf.sh` with a full-project check workload alongside the
  build benchmark: identical copies, clean target per run, alternating order,
  1 warmup + 5 samples. Latest (87 files): build seed median 4313.6 ms vs Nova
  1806.2 ms; check seed median 88.5 ms vs Nova 359.5 ms; both artifacts built,
  ran hello and reported no check errors. Recorded as measurements, not speed
  claims.
- Added frontend `Option`/`Result` semantics: constructors, built-in methods,
  exhaustive match with payload bindings and bare binding patterns, recursive
  type compatibility, and checked `?` propagation. Added fixed-array `[T; N]`
  checking for bindings, arguments, returns, indexing, assignment and `for`.
  The parser now preserves postfix `?` as `ak_try`. Codegen lowers both as
  inline tagged values (`{ i1, T }`, `{ i1, T, E }`) and arrays as `[N x T]`
  with bounds-checked indexing/assignment, `len`, `for`, parameters and
  returns. Slice views from lists and fixed arrays lower to `%nova.slice`
  (`{ ptr, i64 }`) with `as_slice`, bounds-checked indexing/assignment, `len`,
  `for`, parameters and returns. `Map<K,V>`/`Set<T>` basics lower to
  `%nova.map` with annotated `map()`/`set()`, `len`, `get`,
  `contains_key`/`contains` and `insert` for Int/Bool/String keys. Evidence:
  sema-ast 39/0; diff-errors 27/27 identical; codegen 248/0; diff 18/0.
  Generics, map/set literals, iteration and Char keys remain.

## Incomplete work / known failures

- Rust removal is complete: no Cargo workspace or `.rs` file remains, and
  `bootstrap/nova` builds the toolchain without a Rust toolchain. The remaining
  incompleteness is language parity, not a Rust stage-0 dependency.
- `BOOT-02.04` now covers the Rust LSP feature set (transport, sync,
  diagnostics, completion, hover, definition, document symbols, formatting)
  and is built by the self-hosted compiler. `nova lsp` and Studio launch that
  server (`BOOT-02.05`); there is no Rust fallback.
- Mutable list initialization and reassignment clone the backing array, but
  function arguments and `let` bindings still pass headers by value; nested
  list ownership and GC are unresolved.
- List literals allocate with libc `malloc`; storage is never freed and
  ownership/GC is unresolved.
- Registry Box dependencies resolve through `nt_registry_module` from the
  fetched cache and `Box.lock`. Std imports are validated but remain compiler
  builtins.
- Sema lacks generics, traits, methods and full diagnostics; the Nova LSP
  covers transport, full-document sync, diagnostics and completion.
- Overflow, division-by-zero and alignment checks remain.
- Match lacks nested enum/struct patterns and enum-typed payloads. Map/set
  coverage excludes literals, iteration, Char keys and non-Int/Bool/String
  keys.
- The differential harness covers 18 programs both frontends accept; five
  fixtures with acceptance divergence (empty-list inference, range slicing,
  `None`, `for` over a fixed array and `for` over a slice) remain. `tests/diff-errors.sh` matches all 27 invalid programs exactly;
  that corpus is not exhaustive, so newly added diagnostics still need
  goldens. Build and check times are measured by `tests/perf.sh` but are not a
  speed claim: the compilers still cover different language subsets, and the
  check path is not equal work (seed 88.5 ms vs Nova 359.5 ms, 5 samples).
  `nova check` is clean on the Nova tool's own source, but its sema still covers
  less than the seed's.

## Resume here

1. Continue `COMP-01.04` with generics, traits, methods and closures; then
   close the remaining map/set literal, iteration and key-type gaps. Rerun the
   check benchmark once coverage converges. Parse and LSP workloads remain
   unmeasured.
2. Port `nova test` and the REPL, which were lost with the Rust CLI, and add
   their tests to CI.
3. Add a Nova wasm backend so the playground artifacts can be rebuilt without
   the former Rust front end.
4. Build non-macOS bootstrap artifacts before advertising Linux/Windows CLI
   downloads; the checked-in bootstrap is macOS arm64 only.

## Validation

- Command: `(cd examples/nova-tool && ../../bootstrap/nova build)`
- Result: PASS
- Command: `sh examples/nova-tool/tests/codegen.sh`
- Result: PASS (248/0; `codegen-interp` exits 19)
- Command: `sh examples/nova-tool/tests/cli.sh`
- Result: PASS (22/0)
- Command: `sh examples/nova-tool/tests/diff.sh`
- Result: PASS (18 programs matched, 0 differed)
- Command: `sh examples/nova-tool/tests/diff-errors.sh`
- Result: PASS (27 fixtures compared, 27 identical, 0 divergent, 0 mismatched)
- Command: `(cd examples/nova-tool && ./target/nova/nova-tool check)`
- Result: PASS (`Checked .../src/main.nova: no errors`; previously 177
  false-positive `E0301` diagnostics)
- Command: Nova-built compiler compiled an interpolation fixture and ran it
- Result: PASS (output matched the seed: `p=4`, `sum=3`, `esc=${p.x}`,
  `struct=9`)
- Command: `PERF_SAMPLES=5 PERF_WARMUP=1 sh examples/nova-tool/tests/perf.sh`
- Result: PASS (build seed 4313.6 ms vs Nova 1806.2 ms; check seed 88.5 ms vs
  Nova 359.5 ms over 5 samples; both correctness checks passed; measurements
  only, not a speed claim)
- Command: self-build after the module resolver, then sample + LSP
- Result: PASS (`/tmp/nova-mod-self`, sample exit 9, LSP PASS)
- Command: `sh examples/nova-tool/tests/lsp.sh`
- Result: PASS (diagnostics, completion, hover, definition, document symbols
  and formatting; seed-built, `/tmp/nova-self`, `/tmp/nova-self-2`,
  `/tmp/nova-self-3`)
- Command: `sh examples/nova-tool/tests/toolchain.sh`
- Result: PASS (126/0)
- Command: `sh examples/nova-tool/tests/parser.sh`
- Result: PASS (51/0)
- Command: `sh examples/nova-tool/tests/sema-ast.sh`
- Result: PASS (39/0)
- Command: `sh examples/nova-tool/tests/sema.sh`
- Result: PASS (4/0)
- Command: `NOVA_TOOL=emit-ir` over the 15,888-line expanded compiler source
- Result: PASS (0.25s, 353 MB peak RSS; previously killed by OOM at stage 2)
- Command: stage 1/2/3 self-build (`/tmp/nova-self4`, `/tmp/nova-self-2c`,
  `/tmp/nova-self-3c`) + sample and LSP runs
- Result: PASS (each builds the project, compiles the sample with exit 9 and
  passes `tests/lsp.sh`; emitted IR is byte-identical across generations,
  linked executables differ by 49 linker-metadata bytes)
- Command: stage-1 self-build under `/usr/bin/time -l`
- Result: PASS (2.15s, 3.08 GB peak RSS)

## Blockers and approvals

- No external blocker for the remaining self-hosting work.
- Push, publish, deployment and branch deletion require owner authorization.
- List ownership/copy semantics are a known correctness limit; do not claim full
  runtime parity.
- The Nova LSP matches the Rust server's feature set and is what `nova lsp`
  and Studio launch; the Rust server remains only as a fallback when
  `nova-tool` is missing.
