# Memory, runtime and ABI

Nova compiles to native code and links a small C runtime. The runtime ABI is versioned (`ABI_VERSION = 1`); `nova doctor` checks compatibility where possible.

## Current model

- Scalars and structs are values with native layouts.
- Strings and lists are managed runtime handles. Current allocation is **process-lifetime**: memory is not reclaimed before exit.
- There is no user-visible `free`, no reference counting, no cycle collector, and no finalizers. Programs that allocate in long loops will grow until termination. This is a documented limitation, not a safety claim.
- Safe code cannot free runtime-managed memory, so use-after-free of these handles is not exposed by the current language subset.
- Bounds checks and checked arithmetic are always emitted. Failure aborts with a message; no unwinding or recovery.
- Panic policy is abort. There is no FFI, so no C-unwind boundary exists yet.
- No threads, atomics, `Send`/`Sync`, or data-race surface exists yet.
- The standard library is minimal: formatting and assertions are builtins; collections beyond `List` are absent.

## Not implemented

Deterministic destruction, compiler-inserted reference counting, copy-on-write collections, weak references, move-only resources, escape analysis, allocator selection, ASan/TSan test runs, FFI ABI/ownership rules, debug symbols, and reproducible-build guarantees.

## Roadmap

Do not claim cycle collection, zero-overhead ownership, or general memory safety until implemented and tested. The next memory milestone is deterministic destruction with reference counting, explicit cycle behavior, retain/release elimination, and sanitizer-backed tests.
