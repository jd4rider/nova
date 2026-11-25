# Compiler and LSP parity matrix

Evidence date: 2026-11-24. The Rust reference was audited from source; its full
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
| Generics, methods, traits, closures | Rust compiler | TODO | COMP-01.04, CODEGEN-01.10 |
| Diagnostic codes/spans/recovery | `bootstrap/nova-diag`, sema | PARTIAL — E03xx slice; no full parity | COMP-01.06 |
| Int arithmetic | Rust codegen | PARTIAL — `+ - * /`, no overflow/div-zero checks | CODEGEN-01.01, CODEGEN-01.07 |
| Control flow | Rust codegen | PARTIAL — `if`/`else`, `while`, range/list `for`, nested loops, `break`/`continue`, Int/Bool/String/enum `match`, String literal patterns, enum payload bindings and `_`, comparisons; sema rejects nested patterns and duplicate/unreachable arms and checks Int/Bool/String/enum exhaustiveness; no short-circuit booleans | CODEGEN-01.03, CODEGEN-01.08 |
| Strings, lists, maps, sets | Rust runtime | PARTIAL — string literals/by-value params/returns/`.len()`/concat/equality and literal match patterns, plus trim/contains/starts_with/ends_with/replace/lower; malloc-backed `List<Int>` literals, by-value params/returns, `.len()`, and indexing/assignment/push through `nova_list_at`/`nova_list_push` with the linked C runtime, and structural `List<Int>` equality/concatenation and start..end slicing; no maps, sets or ownership | CODEGEN-01.06 |
| Structs, enums, tuples, arrays | Rust runtime | PARTIAL — value structs lower to `%nova.struct.*`; enums lower to `%nova.enum.*` with an `i32` tag and Int/Bool/String payload slots, construction, payload binding and exhaustive match; tuples and arrays TODO | CODEGEN-01.06 |
| Option/Result/error propagation | Rust runtime | TODO | CODEGEN-01.06 |
| FFI and runtime ABI | Rust codegen + `runtime/` | PARTIAL — seed-shaped `ptr %arg_env`, `nova_user_<index>`, Int/Unit/String/List signatures, entry wrapper and linked `nova_list_at`/`nova_list_push`; broader runtime ABI TODO | CODEGEN-01.04 |
| Clang linking from the compiler | Rust CLI | VERIFIED for the supported slice — `build-ir` invokes clang with quoted paths and checks status | CODEGEN-01.09 |
| Optimization parity | Rust LLVM pipeline | TODO | CODEGEN-01.10 |
| Differential Rust/Nova tests | — | TODO | DIFF-01.01, DIFF-01.02 |
| Stage 1/2/3 bootstrap | Rust seed | TODO | BOOT-02.01–03 |
| Nova compiler built by itself | — | TODO | BOOT-02.04 |

## LSP

Rust LSP reference: `bootstrap/nova-lsp/src/lib.rs` (856 lines). Its own header
lists full-document sync, syntax/type diagnostics, completion, hover,
definition, document symbols and whole-document formatting as implemented.
Incremental sync, rename, semantic tokens, code actions, references, inlay hints,
workspace indexing and cancellation are absent even in Rust.

| LSP capability | Rust | Nova | Task |
|---|---|---|---|
| stdio/JSON-RPC transport | present | TODO | LSP-01.01 |
| initialize/capabilities | present | TODO | LSP-01.02 |
| full document sync | present | TODO | LSP-01.02 |
| diagnostics | present | TODO | LSP-01.02 |
| completion | present | TODO | LSP-01.03 |
| hover, definition, document symbols | present | TODO | LSP-01.04 |
| formatting | present | TODO | LSP-01.05 |
| incremental sync / incomplete source | absent in Rust | TODO | LSP-01.06 |
| rename, references, semantic tokens, code actions, cancellation | absent in Rust | TODO | LSP-01.07 |
| protocol transcript tests | partial unit tests | TODO | LSP-01.08 |
| Studio integration | Rust server | TODO | LSP-01.08, BOOT-02.05 |

## Verified Nova slices

- AST parser: `tests/parser.sh` → 50/0, 29-file corpus.
- Body sema: `tests/sema-ast.sh` → 28/0; String literal patterns duplicate struct fields/initializers
  and enum variants are checked, enum constructor/pattern arity and payload
  binding types are checked, match patterns are validated, block-arm bodies are
  checked, mixed arm types are errors, and Int/Bool/enum matches must be
  exhaustive (`E0311`–`E0313`).
- Codegen: `tests/codegen.sh` → 149/0; SSA params/`let`; seed-shaped
  `ptr %arg_env`/`nova_user_N` ABI; Int, Unit/void, `%nova.string` and
  `%nova.list` declarations; string literals and `.len()`; heap-backed
  `List<Int>` literals/params/returns, runtime-linked indexing/assignment/push,
  `.len()`; range/list/nested `for` with `break`/`continue`; Int/Bool
  `match` expressions, enum tags/Int/Bool payloads and Unit block arms; unmatched
  matches call the runtime panic; value structs with nested fields,
  construction, reads, assignment and by-value parameters/returns; `build-ir`
  invokes clang. Native results 9, 2, 17, 4, 5, 23, 13, 16, 10, 45, 8, 30, 60,
  9, struct 14, enum 36 and Unit `main` 0; String match 13; print stdout and
  exit 7; assert exit 5 and failure exit 1; list equality 12; list concat 11; String methods 5; Bool enum 7; String enum 10; list slice 10; String transform 16; panic exit 1; Mach-O confirmed on this host.

## Rule

A row moves to `VERIFIED` only after the same program's valid path, invalid path
and regression path are tested against the Rust reference where practical.
