# ADR 0004: C runtime with process-lifetime allocation

## Status

Accepted for the current milestone; explicitly provisional.

## Context

Nova needs strings, lists, bounds checks, checked arithmetic traps, and startup support now. A full reference-counting or ownership model is a larger design that must be sound before it is exposed.

## Decision

Ship a small C runtime (`crates/nova-runtime/src/runtime.c`) embedded with `include_str!` and compiled by Clang. Allocation currently uses process-lifetime allocation. All safe-language values are immutable or uniquely owned; there is no user-visible manual free. Checked arithmetic and bounds failures call a runtime trap that writes a message and aborts.

## Consequences

- Long-running allocation-heavy programs leak memory until exit. This is documented, not hidden.
- No use-after-free is possible for runtime-managed values in the current subset.
- The ABI is versioned (`ABI_VERSION = 1`); CLI and runtime mismatch is detected.
- Deterministic destruction/ARC, cycles, weak references, finalization, and move-only resources remain unimplemented.
- Sanitizer and concurrency testing are future work; there is no thread support yet.

## Next

Design and test deterministic destruction and reference counting with cycle documentation before claiming a memory-safety guarantee beyond the current subset.
