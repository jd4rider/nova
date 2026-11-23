---
title: Standard library
description: The small standard-library surface currently implemented by Nova, and what is planned.
---

**Status: minimal and manually documented.** There is no generated API index yet. This page lists only facilities exercised by the compiler/tests. If a signature is not listed here, do not assume it exists.

## Builtins

| Name | Behavior |
|---|---|
| `print(value)` | Write a value to standard output without a trailing newline. |
| `println(value)` | Write a value and a newline to standard output. |
| `eprintln(value)` | Write a value and a newline to standard error. |
| `assert(condition)` | Abort with a source line if the condition is false. |
| `assert_eq(left, right)` | Abort with a source line if values differ. |
| `panic(message)` | Abort with a message and source line. |

Builtins are available to checked programs; `import std.io { println }` and `import std.core { assert }` are recognized. An import does not currently provide a user module graph.

## Types and core operations

- `String`: UTF-8, immutable value semantics, `+` concatenation, `${...}` interpolation.
- `List<T>`: literals, indexing with bounds checks, iteration with `for`, `push(value)`, `len()`.
- `Option<T>`: `None`, `Some(value)`, exhaustive `match`, `?` propagation.
- `Result<T, E>`: `Ok(value)`, `Err(error)`, exhaustive `match`, `?` propagation.
- Structs: field access, literals, mutation through mutable roots.

Byte and Unicode-scalar string indexes, slicing, iterators, maps, sets, tuples, fixed-width integers and arrays are **not implemented**.

## Modules

Only standard import names are recognized for the builtin surface. A complete module index, signatures, platform notes and generated API docs are **planned**. The current truth is the compiler source and its tests, not an aspirational API listing.

## Planned

Formatting, math, iterators, filesystem/path/env/process/time, JSON, CSV, regex, encoding, random, networking/HTTP/TLS, async/channels, logging and testing libraries. Crypto/TLS must use audited platform or library facilities; Nova will not invent cryptography.

## Errors and platform

Runtime failures abort with a source location. There is no exception recovery, filesystem API or networking API to document yet. Allocation is process-lifetime; see [Memory model](../language/memory/).
