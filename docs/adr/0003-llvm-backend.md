# ADR 0003: Textual LLVM IR plus host Clang

## Status

Accepted.

## Context

The directive asks for a maintained LLVM binding and pinned LLVM. `inkwell`/`llvm-sys` couple the build to a specific libLLVM version and installation. On the test machine multiple Homebrew LLVM versions exist, and building against the wrong one is a common failure.

## Decision

Generate textual LLVM IR from typed Nova IR and invoke the host `clang` to assemble, optimize (`-O0`/`-O2`/`-O3`), verify, and link against the native runtime. Do not use `inkwell` or link libLLVM directly.

## Consequences

- The compiler uses whatever Clang/LLVM the developer has; `nova doctor` reports it.
- LLVM's verifier and optimizer still run, so this is a real native backend.
- IR text generation is fast enough for the current language subset.
- We remain responsible for IR correctness; malformed IR is reported via Clang diagnostics.
- Debug metadata, LTO flags, and target triples are not yet exposed.
- If tighter LLVM APIs become necessary, the backend can be migrated behind the existing `generate_ir`/`compile` boundary.

## Alternatives rejected

- `inkwell`: version coupling and build complexity.
- Custom machine code: excessive scope and poor optimization.
- Transpiling to C: hides semantics and diagnostics; not a real native compiler pipeline.
