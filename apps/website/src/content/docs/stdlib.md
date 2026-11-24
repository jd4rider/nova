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

Builtins are available to checked programs; `import std.io { println }` and `import std.core { assert }` are recognized. User files can also be loaded with path imports, documented under Modules.

## Types and core operations

- `String`: UTF-8, immutable value semantics, `+` concatenation, `${...}` interpolation.
- `List<T>`: literals, indexing with bounds checks, iteration with `for`, `push(value)`, `len()`.
- `Option<T>`: `None`, `Some(value)`, exhaustive `match`, `?` propagation.
- `Result<T, E>`: `Ok(value)`, `Err(error)`, exhaustive `match`, `?` propagation.
- Structs: field access, literals, mutation through mutable roots.
- Enums: unit or multi-field variants, exhaustive `match`, and non-generic `impl Type` methods with `self`.
- Tuples: inline values, `.0` access, tuple patterns and field mutation.
- Arrays: `[T; N]`, compile-time length, bounds-checked indexing, `.len()` and `.as_slice()`.
- `Slice<T>`: borrowed `{ptr, len}` view with bounds-checked indexing and `.len()`.
- `Map<K, V>` / `Set<T>`: hashed collections built with `map()` / `set()` and a type annotation; open addressing with FNV-1a key hashing; keys limited to `Int`, `Bool`, `Char` and `String`.
- Generic structs, enums and inherent methods: `struct Pair<T>`, `enum Boxed<T>` and `impl<T> Pair<T>`, monomorphised from concrete annotations.
- `fn(...) -> T`: function values for named functions and capturing closures; captures copy the outer value at closure creation.

Byte and Unicode-scalar string indexes, general string slicing, iterators, fixed-width integers, mutable captures and lifetime-checked references are **not implemented**.

## Modules

Standard imports expose the builtin surface. `import "util.nova"` flattens another source file into the current program with cycle checks; `import "util.nova" as util` additionally qualifies its calls and type names. The underlying namespace is still flat, so duplicate unqualified names remain errors. Package resolution, a registry and a generated module index are **planned**. The current truth is the compiler source and its tests, not an aspirational API listing.

## Planned

Formatting, math, iterators, filesystem/path/env/process/time, JSON, CSV, regex, encoding, random, networking/HTTP/TLS, async/channels, logging and testing libraries. Crypto/TLS must use audited platform or library facilities; Nova will not invent cryptography.

## Errors and platform

Runtime failures abort with a source location. There is no exception recovery, filesystem API or networking API to document yet. Allocation is process-lifetime; see [Memory model](../language/memory/).
