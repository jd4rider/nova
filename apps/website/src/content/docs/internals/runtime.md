---
title: Runtime and ABI
description: The C runtime, checked helpers, abort policy and allocation limits.
---

Nova links a small C runtime (`crates/nova-runtime/src/runtime.c`) embedded by the compiler. The runtime ABI is versioned (`ABI_VERSION = 1`); `nova doctor` reports it.

## Responsibilities

- UTF-8 string construction, concatenation, printing and length
- dynamically sized lists with bounds information
- checked integer helpers using LLVM overflow intrinsics
- division-by-zero and remainder checks
- bounds and assertion failures with source lines
- process startup support

## Failure policy

Overflow, division by zero, bounds failures, failed assertions and explicit `panic` flush standard output, print a diagnostic to standard error and call `exit(1)`. There is no unwinding and no recoverable exception mechanism.

## Allocation

Allocation currently uses process-lifetime allocation. There is no free, reference counting, cycle collector or finalizer. This prevents the current safe subset from exposing use-after-free of runtime handles, but it leaks until exit and is not suitable for arbitrary long-running workloads.

## ABI and portability

Generated calls use a C-compatible ABI, and user code can declare raw `extern "C"` functions. There is still no ownership or unwind contract at that boundary. Only macOS arm64 is currently tested. No sanitizer run, thread model, C-unwind boundary or cross-platform ABI matrix exists yet.

## Reproducibility

No reproducible-build guarantee is claimed. Builds depend on the host Clang, linker and system libraries. Debug symbols are not yet emitted by the compiler pipeline even when the linker accepts a debug flag.

## Next

Design ownership/destruction, implement reference counting and cycle documentation, add sanitizer-backed tests, then expose FFI only with explicit ownership rules and no-unwind guarantees.
