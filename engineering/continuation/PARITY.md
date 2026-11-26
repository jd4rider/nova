# Compiler and LSP parity matrix

Evidence date: 2026-11-25; Rust seed removed 2026-11-26, so the reference paths
below are historical and no Rust binary is available to rerun. The Rust reference was audited from source; its full
baseline suite was not rerun in this session. Status: `VERIFIED`, `PARTIAL`,
`TODO`, `BLOCKED`. Every missing row has a stable task ID in `TASKS.md`.

## Compiler

| Capability | Rust reference | Nova status | Task |
|---|---|---|---|
| Lexer, tokens, spans, interpolation | `bootstrap/nova-lexer` (894 lines) | PARTIAL — Nova lexer tokenizes the corpus; interpolation is parsed and lowered (literal/Int/Bool/String parts concat through `nova_string_concat`, escapes and nested expressions match the seed); comment/recovery parity remains | COMP-01.05 |
| Parser, AST, recovery | `bootstrap/nova-parser` (2,447 lines) | PARTIAL — AST parses 29/29 corpus files; no recovery/incomplete-source mode | COMP-01.01, COMP-01.02 |
| Name resolution / declarations | `bootstrap/nova-sema` (6,278 lines) | PARTIAL — functions, structs, enums, scopes | COMP-01.02 |
| Type checking / inference | `bootstrap/nova-sema` | PARTIAL — Int/String/Bool, String concat/equality and literal patterns, calls, struct fields and duplicate-field checks, enum variants/payloads and duplicate checks, returns, match arm/payload types and Int/Bool/String/enum exhaustiveness; typed empty lists (`List<?>`) and unknown tuple slots are accepted structurally; built-in Option/Result constructors, methods, exhaustive match and checked `?`; fixed arrays `[T; N]` in bindings, arguments, returns and `for`; `Slice<T>` views from lists and arrays with indexing/assignment, `len`, `for`, parameters and returns; annotated `map()`/`set()`, `len`, `get`, `contains_key`/`contains` and `insert` for Int/Bool/String keys; explicit generic functions, non-generic methods, static traits and closures are checked; generic structs/methods and `dyn Trait` are not | COMP-01.02, COMP-01.04, COMP-01.07 |
| Modules / imports / Box resolution | `bootstrap/nova-cli`, `nova-pkg` | present for `.nova` paths, extensionless sibling modules, module directories, transitive imports and `[dependencies]` path deps; std imports validated, missing/malformed/cyclic imports diagnosed. Registry-only boxes still resolve through the Rust driver | COMP-01.03 |
| CLI build driver | `bootstrap/nova-cli` | PARTIAL — Nova `build` discovers `nova.toml`/`Box.toml`, reads `[bin] path`, inlines relative `.nova` imports, builds to `target/nova/<name>` and returns the native status; `build <source> [output]` handles one file. `run` executes the built binary and propagates its status, and `check` runs body sema with the same module resolution; registry Box dependencies and seed fallback remain | CLI-01.01 |
| Generics, methods, traits, closures | removed reference | PARTIAL — explicit single-parameter generic functions are monomorphized from `name<Type>(...)`; non-generic inherent and static trait methods with `self`, trait conformance and concrete dispatch, value-capturing closures and named function values work. Generic structs/enums/methods, `dyn Trait` and generic trait impls remain TODO | COMP-01.04 |
| Diagnostic codes/spans/recovery | `bootstrap/nova-diag`, sema | VERIFIED for the 27-program corpus — all 27 match the seed's code/message/span exactly (`tests/diff-errors.sh`); duplicate arms are accepted and malformed parameters emit the seed's two `E0103` diagnostics | COMP-01.06 |
| Int arithmetic | Rust codegen | PARTIAL — `+ - * /`, no overflow/div-zero checks | CODEGEN-01.01, CODEGEN-01.07 |
| Control flow | Rust codegen | PARTIAL — `if`/`else`, `while`, range/list/array/slice `for`, nested loops, `break`/`continue`, Int/Bool/String/enum `match`, String literal patterns, enum payload bindings and `_`, comparisons; short-circuit `&&`/`||` and unary `!`; sema rejects nested patterns, accepts duplicate/wildcard-shadowed arms like the seed and checks Int/Bool/String/enum exhaustiveness | CODEGEN-01.03, CODEGEN-01.08 |
| Strings, lists, maps, sets | Rust runtime | PARTIAL — string literals/by-value params/returns/`.len()`/concat/equality and literal match patterns, plus trim/contains/starts_with/ends_with/replace/lower/upper/is_empty/bytes; element-aware list literals including `List<String>`/`List<Struct>`, by-value params/returns, `.len()`, indexing/assignment/push/`for` through `nova_list_at`/`nova_list_push` with the linked C runtime, and structural `List<Int>` equality/concatenation start..end slicing, first/last, contains, index_of, reverse, sort, sum, min and max; `chars`, `split`, `join` and `to_string`; basic maps/sets with annotated empty constructors, lookups, membership and inserts; no ownership | CODEGEN-01.06 |
| Structs, enums, tuples, arrays | Rust runtime | PARTIAL — value structs lower to `%nova.struct.*`; enums lower to `%nova.enum.*` with an `i32` tag and Int/Bool/String/struct payload slots, construction, payload binding and exhaustive match; tuples lower to LLVM literal structs with `.0`/`.1` extraction; fixed arrays lower inline as `[N x T]` with literals, bounds-checked indexing/assignment, `len`, `for`, parameters and returns | CODEGEN-01.06 |
| Slice views | Rust runtime | PARTIAL — `List<T>.as_slice()` and `[T; N].as_slice()` lower to `%nova.slice { ptr, i64 }`; bounds-checked indexing and assignment, `len`, `for`, parameters and returns; native 44 and slice-for 6, seed differential 18/0 | COMP-01.07, CODEGEN-01.06 |
| Map/Set basics | Rust runtime | PARTIAL — annotated `map()`/`set()`, `len`, `get`/`contains_key`/`contains`/`insert` for Int/Bool/String keys; `%nova.map` runtime calls; native 23 and seed differential 18/0; no literals, iteration or Char keys | COMP-01.07 |
| Option/Result/error propagation | Rust runtime | PARTIAL — constructors, methods, exhaustive match and `?` are checked and lowered as inline `{ i1, T }` / `{ i1, T, E }` values; native fixture exits 119 and the seed differential matches 18/0 | CODEGEN-01.06, COMP-01.07 |
| FFI and runtime ABI | Rust codegen + `runtime/` | PARTIAL — seed-shaped `ptr %arg_env`, `nova_user_<index>`, Int/Unit/String/List signatures, entry wrapper, extern declarations/calls and linked `nova_list_at`/`nova_list_push`; broader runtime ABI TODO | CODEGEN-01.04 |
| Clang linking from the compiler | Rust CLI | VERIFIED for the supported slice — `build-ir` invokes clang with quoted paths and checks status | CODEGEN-01.09 |
| Optimization parity | Rust LLVM pipeline | TODO | CODEGEN-01.10 |
| Differential Rust/Nova tests | `bootstrap/nova-cli` | compile/run harness (`tests/diff.sh`, 18 programs, stdout match) and golden diagnostics harness (`tests/diff-errors.sh`, 27 invalid programs, 27 identical, 0 divergent; both drivers pinned) | DIFF-01.01, DIFF-01.02 |
| Stage 1/2/3 bootstrap | Rust seed | VERIFIED (IR fixed point) — the Nova tool builds itself through three generations; each binary compiles the native sample (exit 9) and passes `tests/lsp.sh`, and generations 1–3 emit byte-identical IR. Linked executables differ by 49 bytes of linker metadata, so no byte-identical executable is claimed | BOOT-02.01–03 |
| Nova compiler built by itself | — | PARTIAL — the driver/parser/sema/codegen tool self-builds and includes the LSP diagnostics/completion slice; full compiler parity and LSP feature parity remain | BOOT-02.04 |
| Comparable performance | Rust seed | PARTIAL — `tests/perf.sh` measures clean builds of `examples/nova-tool` (seed median 4313.6 ms vs Nova 1806.2 ms over 5 samples) and full-project checks (seed 88.5 ms vs Nova 359.5 ms); not a speed claim: the compilers cover different language subsets, check coverage still differs, and parse/LSP workloads are not comparable yet | PERF-01.01, PERF-01.02 |

## LSP

Rust LSP reference: `bootstrap/nova-lsp/src/lib.rs` (856 lines). Its own header
lists full-document sync, syntax/type diagnostics, completion, hover,
definition, document symbols and whole-document formatting as implemented.
Incremental sync, rename, semantic tokens, code actions, references, inlay hints,
workspace indexing and cancellation are absent even in Rust.

| LSP capability | Rust | Nova | Task |
|---|---|---|---|
| stdio/JSON-RPC transport | present | present — Content-Length framing, `tests/lsp.sh` | LSP-01.01 |
| initialize/capabilities | present | present — advertises full-document sync | LSP-01.02 |
| full document sync | present | present — didOpen/didChange/didSave/didClose | LSP-01.02 |
| diagnostics | present | present — Nova parser/sema diagnostics, cleared on close | LSP-01.02 |
| completion | present | present — keywords, builtins, declarations and `.` members | LSP-01.03 |
| hover, definition, document symbols | present | present — AST word spans, declaration lookup, signature hover, definition range and symbol tree | LSP-01.04 |
| formatting | present | present — whole-document `TextEdit` from the Nova formatter | LSP-01.05 |
| incremental sync / incomplete source | absent in Rust | TODO | LSP-01.06 |
| rename, references, semantic tokens, code actions, cancellation | absent in Rust | TODO | LSP-01.07 |
| protocol transcript tests | partial unit tests | present for transport/sync/diagnostics/completion/hover/definition/symbols/formatting (`tests/lsp.sh`) and the `nova lsp` entry point (`tests/toolchain.sh`) | LSP-01.08 |
| Studio integration | removed reference | present — `nova lsp` runs the Nova server; Studio prefers `nova-tool` and falls back to `nova lsp` | LSP-01.08, BOOT-02.05 |

## Verified Nova slices

- AST parser: `tests/parser.sh` → 51/0, 29-file corpus.
- Body sema: `tests/sema-ast.sh` → 39/0; typed empty lists are accepted in typed contexts, and String literal patterns duplicate struct fields/initializers
  and enum variants are checked, enum constructor/pattern arity and payload
  binding types are checked, match patterns are validated, block-arm bodies are
  checked, mixed arm types are errors, and Int/Bool/String/enum matches must be
  exhaustive (`E0311`–`E0313`). Option/Result constructors, methods, payload
  bindings and `?` are checked, fixed arrays are checked for length and element
  type in bindings, arguments and returns, `Slice<T>` views are checked from
  `as_slice()` on lists and arrays, and `Map`/`Set` basics are checked with
  Int/Bool/String keys and annotated empty constructors.
- Nova LSP: `tests/lsp.sh` → PASS; framed stdio, initialize, full-document
  sync, diagnostics and completion, plus hover (`fn helper(value: Int) -> Int`,
  `field x: Int`), definition (`helper` resolves to line 0), document symbols
  (functions, structs, enums with field/variant children) and formatting (messy
  indentation normalized to one full-document edit). The invalid-buffer fixture
  reports `E0301`; the fixed buffer and `didClose` report none; completion
  includes `helper`, `main`, `fn` and `println`. Passes against the seed-built
  tool and `/tmp/nova-self`/`/tmp/nova-self-2`/`/tmp/nova-self-3`.
- Codegen: `tests/codegen.sh` → 248/0; the runtime IR sink avoids cloning the
  growing line list, and stage-1 self-build measures 2.15s / 3.08 GB peak RSS;
  SSA params/`let`; seed-shaped
  `ptr %arg_env`/`nova_user_N` ABI; Int, Unit/void, `%nova.string` and
  `%nova.list`, `%nova.slice` and `%nova.map` declarations; string literals and `.len()`;
  heap-backed
  `List<Int>` literals/params/returns, runtime-linked indexing/assignment/push,
  `.len()`; range/list/slice/nested `for` with `break`/`continue`; Int/Bool
  `match` expressions, enum tags/Int/Bool payloads and Unit block arms; unmatched
  matches call the runtime panic; value structs with nested fields,
  construction, reads, assignment and by-value parameters/returns; `build-ir`
  invokes clang. Native results 9, 2, 17, 4, 5, 23, 13, 16, 10, 45, 8, 30, 60,
  9, struct 14, enum 36 and Unit `main` 0; String match 13; print stdout and
  exit 7; assert exit 5 and failure exit 1; list equality 12; list concat 11; String methods 5; Bool enum 7; String enum 10; list slice 10; String transform 16; String bytes 95; String upper 72; list first/last 49; list contains 16; list reverse 31; list sort 13; list sum 10; list min/max 29; list index_of 11; struct enum 9; expression `if` 13; list copy 7; builtins 5; Option/Result/array 119; slice 44; slice-for 6; map/set 23; `http_get` IR compiles; panic exit 1; Mach-O confirmed on this host.
- String interpolation: `tests/codegen.sh` 248/0 (`codegen-interp` exits 19);
  `tests/diff.sh` 18/0. A Nova-built compiler matches the seed on `p=${p.x}`,
  `sum=${1 + 2}`, escaped `\${p.x}` and `${P { x: 9 }.x}`.
- Build and check benchmarks: `tests/perf.sh` builds and checks
  `examples/nova-tool` (87 files) from identical copies with a clean target and
  1 warmup + 5 samples. Build: Seed 4309.6 4313.6 4311.1 4322.6 4341.2 ms
  (median 4313.6); Nova 1806.2 1799.0 1798.7 1844.4 1815.1 ms (median 1806.2);
  both artifacts compile and run a hello program. Check: seed 91.1 94.9 88.0
  88.5 88.5 ms (median 88.5); Nova 366.9 357.2 360.0 359.5 359.2 ms (median
  359.5); both report no errors. Not a speed claim: language coverage still
  differs, check is not an equal-work comparison, and parse/LSP are not
  comparable.

## Rule

A row moves to `VERIFIED` only after the same program's valid path, invalid path
and regression path are tested against the Rust reference where practical.
