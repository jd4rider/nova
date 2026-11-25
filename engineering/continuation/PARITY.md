# Compiler and LSP parity matrix

Evidence date: 2026-11-25. The Rust reference was audited from source; its full
baseline suite was not rerun in this session. Status: `VERIFIED`, `PARTIAL`,
`TODO`, `BLOCKED`. Every missing row has a stable task ID in `TASKS.md`.

## Compiler

| Capability | Rust reference | Nova status | Task |
|---|---|---|---|
| Lexer, tokens, spans, interpolation | `bootstrap/nova-lexer` (894 lines) | PARTIAL — Nova lexer tokenizes the corpus | COMP-01.05 |
| Parser, AST, recovery | `bootstrap/nova-parser` (2,447 lines) | PARTIAL — AST parses 29/29 corpus files; no recovery/incomplete-source mode | COMP-01.01, COMP-01.02 |
| Name resolution / declarations | `bootstrap/nova-sema` (6,278 lines) | PARTIAL — functions, structs, enums, scopes | COMP-01.02 |
| Type checking / inference | `bootstrap/nova-sema` | PARTIAL — Int/String/Bool, String concat/equality and literal patterns, calls, struct fields and duplicate-field checks, enum variants/payloads and duplicate checks, returns, match arm/payload types and Int/Bool/String/enum exhaustiveness; no generics/traits | COMP-01.02, COMP-01.04 |
| Modules / imports / Box resolution | `bootstrap/nova-cli`, `nova-pkg` | TODO | COMP-01.03 |
| CLI build driver | `bootstrap/nova-cli` | PARTIAL — Nova `build` discovers `nova.toml`/`Box.toml`, reads `[bin] path`, inlines relative `.nova` imports, builds to `target/nova/<name>` and returns the native status; `build <source> [output]` handles one file. `run` executes the built binary and propagates its status, and `check` runs body sema; Box dependencies, std-module semantics and seed fallback remain | CLI-01.01 |
| Generics, methods, traits, closures | Rust compiler | TODO | COMP-01.04, CODEGEN-01.10 |
| Diagnostic codes/spans/recovery | `bootstrap/nova-diag`, sema | PARTIAL — E03xx slice; no full parity | COMP-01.06 |
| Int arithmetic | Rust codegen | PARTIAL — `+ - * /`, no overflow/div-zero checks | CODEGEN-01.01, CODEGEN-01.07 |
| Control flow | Rust codegen | PARTIAL — `if`/`else`, `while`, range/list `for`, nested loops, `break`/`continue`, Int/Bool/String/enum `match`, String literal patterns, enum payload bindings and `_`, comparisons; short-circuit `&&`/`||` and unary `!`; sema rejects nested patterns and duplicate/unreachable arms and checks Int/Bool/String/enum exhaustiveness | CODEGEN-01.03, CODEGEN-01.08 |
| Strings, lists, maps, sets | Rust runtime | PARTIAL — string literals/by-value params/returns/`.len()`/concat/equality and literal match patterns, plus trim/contains/starts_with/ends_with/replace/lower/upper/is_empty/bytes; element-aware list literals including `List<String>`/`List<Struct>`, by-value params/returns, `.len()`, indexing/assignment/push/`for` through `nova_list_at`/`nova_list_push` with the linked C runtime, and structural `List<Int>` equality/concatenation start..end slicing, first/last, contains, index_of, reverse, sort, sum, min and max; `chars`, `split`, `join` and `to_string`; no maps, sets or ownership | CODEGEN-01.06 |
| Structs, enums, tuples, arrays | Rust runtime | PARTIAL — value structs lower to `%nova.struct.*`; enums lower to `%nova.enum.*` with an `i32` tag and Int/Bool/String/struct payload slots, construction, payload binding and exhaustive match; tuples lower to LLVM literal structs with `.0`/`.1` extraction; arrays TODO | CODEGEN-01.06 |
| Option/Result/error propagation | Rust runtime | TODO | CODEGEN-01.06 |
| FFI and runtime ABI | Rust codegen + `runtime/` | PARTIAL — seed-shaped `ptr %arg_env`, `nova_user_<index>`, Int/Unit/String/List signatures, entry wrapper, extern declarations/calls and linked `nova_list_at`/`nova_list_push`; broader runtime ABI TODO | CODEGEN-01.04 |
| Clang linking from the compiler | Rust CLI | VERIFIED for the supported slice — `build-ir` invokes clang with quoted paths and checks status | CODEGEN-01.09 |
| Optimization parity | Rust LLVM pipeline | TODO | CODEGEN-01.10 |
| Differential Rust/Nova tests | — | TODO | DIFF-01.01, DIFF-01.02 |
| Stage 1/2/3 bootstrap | Rust seed | PARTIAL — the Nova tool builds itself through three generations and each binary compiles the native sample (exit 9); hashes differ, so no byte-identical fixed point is claimed | BOOT-02.01–03 |
| Nova compiler built by itself | — | PARTIAL — the driver/parser/sema/codegen tool self-builds and includes the LSP diagnostics/completion slice; full compiler parity and LSP feature parity remain | BOOT-02.04 |

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
| hover, definition, document symbols | present | TODO | LSP-01.04 |
| formatting | present | TODO | LSP-01.05 |
| incremental sync / incomplete source | absent in Rust | TODO | LSP-01.06 |
| rename, references, semantic tokens, code actions, cancellation | absent in Rust | TODO | LSP-01.07 |
| protocol transcript tests | partial unit tests | present for transport/sync/diagnostics/completion (`tests/lsp.sh`); remaining feature transcripts TODO | LSP-01.08 |
| Studio integration | Rust server | TODO — `nova lsp` still launches Rust; the Nova server is `nova-tool lsp` | LSP-01.08, BOOT-02.05 |

## Verified Nova slices

- AST parser: `tests/parser.sh` → 51/0, 29-file corpus.
- Body sema: `tests/sema-ast.sh` → 28/0; String literal patterns duplicate struct fields/initializers
  and enum variants are checked, enum constructor/pattern arity and payload
  binding types are checked, match patterns are validated, block-arm bodies are
  checked, mixed arm types are errors, and Int/Bool/enum matches must be
  exhaustive (`E0311`–`E0313`).
- Nova LSP: `tests/lsp.sh` → PASS; framed stdio, initialize, full-document
  sync, diagnostics and completion. The invalid-buffer fixture reports `E0301`;
  the fixed buffer and `didClose` report none; completion includes `helper`,
  `main`, `fn` and `println`. Passes against the seed-built tool and
  `/tmp/nova-self`/`/tmp/nova-self-2`/`/tmp/nova-self-3`.
- Codegen: `tests/codegen.sh` → 221/0; the runtime IR sink avoids cloning the
  growing line list, and stage-1 self-build measures 1.60s / 1.67 GB peak RSS;
  SSA params/`let`; seed-shaped
  `ptr %arg_env`/`nova_user_N` ABI; Int, Unit/void, `%nova.string` and
  `%nova.list` declarations; string literals and `.len()`; heap-backed
  `List<Int>` literals/params/returns, runtime-linked indexing/assignment/push,
  `.len()`; range/list/nested `for` with `break`/`continue`; Int/Bool
  `match` expressions, enum tags/Int/Bool payloads and Unit block arms; unmatched
  matches call the runtime panic; value structs with nested fields,
  construction, reads, assignment and by-value parameters/returns; `build-ir`
  invokes clang. Native results 9, 2, 17, 4, 5, 23, 13, 16, 10, 45, 8, 30, 60,
  9, struct 14, enum 36 and Unit `main` 0; String match 13; print stdout and
  exit 7; assert exit 5 and failure exit 1; list equality 12; list concat 11; String methods 5; Bool enum 7; String enum 10; list slice 10; String transform 16; String bytes 95; String upper 72; list first/last 49; list contains 16; list reverse 31; list sort 13; list sum 10; list min/max 29; list index_of 11; struct enum 9; expression `if` 13; list copy 7; builtins 5; `http_get` IR compiles; panic exit 1; Mach-O confirmed on this host.

## Rule

A row moves to `VERIFIED` only after the same program's valid path, invalid path
and regression path are tested against the Rust reference where practical.
