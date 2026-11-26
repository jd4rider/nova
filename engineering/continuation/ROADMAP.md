# Self-hosting roadmap

Status vocabulary: `TODO`, `READY`, `IN_PROGRESS`, `PARTIAL`, `BLOCKED`,
`VERIFIED`, `DEFERRED`. Only `VERIFIED` means acceptance evidence passed.
The detailed matrix is `PARITY.md`.

| ID | Milestone | Status | Depends on |
|---|---|---|---|
| BOOT-01 | Preserve and verify the Rust stage-0 seed | VERIFIED | — |
| COMP-01 | Nova compiler frontend: lexer, parser, sema, modules | PARTIAL | BOOT-01 |
| CODEGEN-01 | Nova LLVM IR lowering and native execution | PARTIAL | COMP-01 parser |
| CLI-01 | Nova build/run driver | IN_PROGRESS | CODEGEN-01.04 |
| DIFF-01 | Rust/Nova differential validation | compile/run harness (`tests/diff.sh`, 18/0) and golden invalid-program diagnostics (`tests/diff-errors.sh`, 27 fixtures, 27 identical, 0 divergent) | CLI-01, COMP-01 |
| LSP-01 | Nova language server, including current Rust surface | PARTIAL | COMP-01, CODEGEN-01, stdio |
| PERF-01 | Measured compiler/LSP performance at parity | PARTIAL — build/check harness measured (`tests/perf.sh`: build seed 4313.6 ms vs Nova 1806.2 ms; check seed 88.5 ms vs Nova 359.5 ms, 5 samples); check coverage differs and no speed claim is made until parity | CLI-01, LSP-01 |
| BOOT-02 | Stage-1/2/3 self-compilation and Nova-built LSP | PARTIAL | CLI-01 |
| REL-01 | Clean-machine distribution without a Rust SDK | TODO | BOOT-02 |

## Current verified slice

- AST parser 29/29 corpus files.
- Body sema 39/0, including duplicate struct/enum checks, enum payload
bindings, String literal patterns, Int/Bool/String/enum exhaustiveness, built-in
Option/Result constructors, methods, match and `?`, and fixed-array length and
element checks. Typed empty lists are accepted in typed contexts, and `nova
check` reports no errors on the Nova tool's own source.
- Codegen 248/0 across Int/control flow, runtime-linked lists, String
operations/match, print/println, assert, value structs and enums, generic
lists, tuples, fixed arrays, slice views, map/set basics, Option/Result,
externs and logical operators; native binaries exit 9, 2, 5, 7, 10, 11, 12,
13, 14, 17, 36, 44, 23, 119, and the
focused method/list/tuple fixtures listed in `EVIDENCE.md`.
- Nova CLI project build/run/check: `nova.toml`, `[bin] path`, relative,
  extensionless, transitive and path-dependency imports, `target/nova/<name>`
  and native status propagation (22/0).
- Nova LSP slice: framed stdio transport, initialize, full-document sync,
  `publishDiagnostics`, completion, hover, definition, document symbols and
  formatting; `tests/lsp.sh` passes against the seed-built tool and Nova-built
  generations 1, 2 and 3. `nova lsp` and Studio launch the Nova server
  (`BOOT-02.05`), with the Rust server only as a missing-tool fallback.
- Codegen appends IR through a runtime sink and the parser appends AST nodes to
  a C arena, avoiding value-semantic list copies on both hot paths; stage-1
  self-build measures 2.15s and 3.08 GB peak RSS.
- `tests/diff-errors.sh` pins the seed's and Nova's diagnostics for 27 invalid
  programs. All 27 now match exactly, including duplicate-arm acceptance,
  malformed-parameter recovery, Option binding/inference/arity/exhaustiveness
  and array length/element checks, so diagnostic parity is verified for this
  corpus rather than assumed.
- String interpolation lowers end-to-end: literal, Int, Bool and String parts
  concat through `nova_string_concat`; escapes, member access and nested struct
  literals match the seed. `tests/codegen.sh` 248/0 and `tests/diff.sh` 18/0;
  a Nova-built compiler prints the interpolated values correctly.
- Option/Result, fixed arrays and slice views are checked and lowered:
  constructors and arity, `is_some`/`is_none`/`is_ok`/`is_err`/`unwrap_or`,
  exhaustive match with payload bindings, checked `?`, inline tagged layouts,
  `[T; N]` literals with bounds-checked indexing/assignment, `len`, `for`,
  parameters and returns, and `%nova.slice` views from `as_slice()` with
  indexing/assignment, `len`, `for`, parameters and returns. `Map`/`Set`
  basics use `%nova.map` with annotated `map()`/`set()`, `len`, lookups,
  membership and inserts. Generics, map/set literals, iteration and expanded
  key types remain.
- `tests/perf.sh` compares clean builds and full-project checks of
  `examples/nova-tool` through both drivers. 5 samples: build seed median
  4313.6 ms vs Nova 1806.2 ms; check seed median 88.5 ms vs Nova 359.5 ms; both
  correctness checks pass. No speed claim: language coverage still differs,
  check is not equal work, and parse/LSP are not comparable.
- Self-build passes through three generations: the Rust-built tool builds
  `/tmp/nova-self4`, that tool builds `/tmp/nova-self-2c`, and the second builds
  `/tmp/nova-self-3c`. Each generation compiles the native sample (exit 9) and
  passes `tests/lsp.sh`. All three emit byte-identical IR; linked executables
  differ by 49 bytes of linker metadata, so the fixed point is at the IR level,
  not executable bytes. The former Rust seed is gone; `bootstrap/nova` and
  `nova build` are in use.
- No claim of full compiler/LSP parity or speed.

## Critical path

CODEGEN-01.04 ABI → CODEGEN-01.09 runtime-linked clang invocation →
CLI-01.01 driver →
COMP-01.03 modules → LSP-01.01/02 transport/diagnostics (done) →
LSP-01.03 completion (done) → hover/definition/formatting → DIFF-01 →
BOOT-02 → PERF-01.

Do not delete `bootstrap/` before BOOT-02 passes.
