# Performance gate

Goal: the Nova compiler and LSP should be faster than the Rust toolchain on
comparable workloads **after feature parity**. This is a requirement, not a
current result.

## No claim yet

The Nova compiler can build its own project and lower string interpolation,
but feature parity is incomplete. `tests/perf.sh` measures a clean build and a
full-project check of `examples/nova-tool` from identical sources. Latest
medians: build seed 4313.6 ms vs Nova 1806.2 ms; check seed 88.5 ms vs Nova
359.5 ms. Neither is a claim: the compilers still cover different language
subsets (generics, floats, maps and more remain), so the two paths do not do
equal work. Parse and LSP workloads wait for parity and are not measured.

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

`tests/perf.sh` builds and checks `examples/nova-tool` (87 `.nova` files,
575,535 bytes) from two identical copies with a clean `target/` per run,
alternating which driver runs first, 1 warmup + 5 recorded samples. It also
requires both built artifacts to compile and run a hello program, and both
check drivers to report no errors. Results on the host below:

| workload | driver | samples (ms) | median |
|---|---|---|---|
| build | Rust seed | 4309.6 4313.6 4311.1 4322.6 4341.2 | 4313.6 ms |
| build | Nova | 1806.2 1799.0 1798.7 1844.4 1815.1 | 1806.2 ms |
| check | Rust seed | 91.1 94.9 88.0 88.5 88.5 | 88.5 ms |
| check | Nova | 366.9 357.2 360.0 359.5 359.2 | 359.5 ms |

Earlier runs before the interpolation fix measured build medians of 4125.1 and
4137.1 ms for the seed against 1735.4 and 1747.8 ms for Nova; run-to-run
variation is real and should be reported with any result.

Status: the Rust seed was removed on 2026-11-26. These seed-vs-Nova numbers are
historical, not a faster-than-Rust result. Both artifacts are about 2 MB
(2,069,960 for the Nova-built compiler against 2,012,020 for the seed's), but
the compilers still cover different language subsets. Check is now measured but
is not equal work: Nova's sema covers less, so its 359.5 ms cannot be read as a
fair loss next to the seed's 88.5 ms. Parse and LSP remain excluded: the seed
has no parse-only command, and `nova lsp` prefers the Nova server, so the Rust
LSP cannot be selected.

Host used for current development: Apple M2 Pro, 10 cores, Darwin 27.0.0
arm64; Apple clang 21.0.0; commit `4e4ad38`. Rust 1.96.0 was used for the
historical seed measurements. These are environment facts, not benchmark
results.

## Planned workloads

- Build (done): Rust stage-0 vs Nova, same `examples/nova-tool` sources (87
  files), clean target each run, 1 warmup + 5 samples (`tests/perf.sh`).
- Check (done, coverage differs): same fixture, 1 warmup + 5 samples; both
  report no errors, but Nova's sema covers less, so the timing is not equal
  work.
- Parse: blocked until the seed exposes a parse-only command or the comparison
  uses a protocol both servers answer identically.
- LSP: startup, full diagnostics, completion and hover over identical
  workspaces and protocol transcripts; the paired Rust comparison is no longer
  possible, so measure the Nova server on its own.
- Generated binaries: identical arithmetic/collection workloads, measured
  separately from compile time.

Add a workload only when the Nova path can run the same work; otherwise it
would benchmark stubs and invite false claims.
