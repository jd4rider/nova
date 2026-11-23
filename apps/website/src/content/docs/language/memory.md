---
title: Memory model
description: The current runtime allocation policy, checks and explicit limitations.
---

## Current model

Nova compiles values natively and links a small C runtime. Scalars and structs use native layouts. Strings and lists are runtime-managed handles.

Allocation currently lasts for the **process lifetime**. The runtime does not reclaim memory before exit. This is safe for short-lived programs and examples, but long-running allocation-heavy programs will grow until they terminate. It is not a general-purpose production memory model.

- No reference counting.
- No cycle collector.
- No finalizers.
- No user-visible `free`.
- No move-only resources.
- No threads, atomics or data-race surface.
- No FFI, so there is no C-unwind boundary.

## Checks

- Integer add, subtract, multiply, divide and remainder are checked. Overflow, division by zero and `i64::MIN / -1` abort with a source line.
- List indexing is bounds checked; failure aborts.
- Definite initialization and mutability are checked before code generation.
- Panics abort. There is no recovery or unwinding.

The compiler emits LLVM IR and invokes the host Clang; the LLVM verifier runs. The runtime ABI is versioned (`ABI_VERSION = 1`), and `nova doctor` reports it.

## Why this is documented so bluntly

A safer model — deterministic destruction, compiler-inserted reference counting, copy-on-write collections, weak references, cycle behavior and retain/release elimination — is a substantial design. It must be implemented and tested before any memory-safety claim beyond the current subset is made.

The current guarantee is narrow: safe source cannot free runtime handles, so the language subset does not expose use-after-free for those handles. Leaks and process-lifetime growth are real limitations.

## Next memory milestone

1. Define destruction and ownership rules.
2. Implement reference counting and explicit cycle documentation.
3. Add tests for aliasing, retain/release balance, cycles and sanitizer runs.
4. Design move-only resources and FFI ownership before exposing either.
5. Evaluate escape analysis and borrowing only after the default model is sound.
