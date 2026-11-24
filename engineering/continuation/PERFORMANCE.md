# Performance gate

Goal: the Nova compiler and LSP should be faster than the Rust toolchain on
comparable workloads **after feature parity**. This is a requirement, not a
current result.

## No claim yet

The Nova compiler cannot build a project end to end, and there is no Nova LSP.
Comparing the current parser/sema/codegen slices with the complete Rust
compiler would compare different amounts of work and would be invalid.

No faster-than-Rust claim may be published until the workloads below are
measured on the same machine with recorded inputs and outputs.

## Measurement rules

- Record hardware, OS, compiler/LLVM versions, build flags, input files, warmup,
  sample count and whether the run is cold or warm.
- Compare equal work: parse vs parse, check vs check, build vs build, LSP
  diagnostics/completion vs the same protocol requests.
- Keep correctness checks alongside timings; a faster wrong result is not a
  result.
- Publish raw samples and losses, not only best or median values.
- Separate compiler time, linker time and runtime. Native runtime speed does not
  prove compiler speed.

## Current evidence

The codegen slice now keeps immutable parameters and `let` bindings in SSA
instead of spilling them to `alloca`; `tests/codegen.sh` asserts that the
straight-line fixture emits no stack allocation. This is a generated-code
improvement, not a compiler-speed measurement.

Host used for current development: Apple M2 Pro, 10 cores; Apple clang 21;
rustc/cargo 1.96.0. These are environment facts, not benchmark results.

## Planned workloads

- Rust stage-0 vs Nova: parse 29-file corpus, check the same fixture, build the
  same fixture, clean vs incremental.
- LSP: startup, full diagnostics, completion and hover over identical
  workspaces and protocol transcripts.
- Generated binaries: identical arithmetic/collection workloads, measured
  separately from compile time.

Add the harness only when the Nova path can run the same workload; otherwise it
would benchmark stubs and invite false claims.
