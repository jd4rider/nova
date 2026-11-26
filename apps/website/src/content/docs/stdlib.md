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
| `term_paste()` | Return and clear the text captured by the last bracketed paste, or `""` when nothing is buffered. |
| `http_get(url)` | Fetch a URL with the host `curl` (no shell) and return the body, or `""` on failure. Blocks; POSIX only. |
| `json_string(json, key)` | Decode the last `"key": "value"` string in a JSON document, with escapes. Not a JSON parser. |
| `char_string(code)` | One-character `String` from a Unicode scalar, for text entry from key codes. |
| `http_listen(port)` | Bind a blocking HTTP/1.1 listener on `127.0.0.1:port`. `0` on success, `-1` on failure. POSIX only. |
| `http_accept()` | Parse the next request and block until one arrives. `1` on success, `-1` on error. |
| `http_method()` | Method of the current request, e.g. `"GET"`. |
| `http_path()` | Path of the current request, without the query string. |
| `http_query()` | Raw query string, e.g. `"id=3&mood=calm"`. |
| `http_body()` | Body of the current request, up to 4 MiB. |
| `http_header(name)` | Header value by case-insensitive name, or `""`. |
| `http_form(name)` | Body field (or query parameter when the body is empty), percent-decoded. |
| `http_respond(status, content_type, body)` | Write a response with `Content-Length` and `Connection: close`, then close the client. |
| `http_close()` | Close the listener; `0` when one was open. |

Builtins are available to checked programs; `import std.io { println }` and `import std.core { assert }` are recognized. User files can also be loaded with path imports, documented under Modules.

## Types and core operations

- `String`: UTF-8, immutable value semantics, `+` concatenation, `${...}` interpolation, `len`, `is_empty`, `contains`, `starts_with`, `ends_with`, `trim`, `replace(from, to)`, `lower()`, `display_width`, `clip(width)`, `wrap(width)` and `graphemes()`. `lower()` folds ASCII letters only; it is not full Unicode case mapping. The layout methods measure terminal columns, ignore ANSI CSI/OSC sequences, count common CJK/emoji ranges as two columns, keep extended grapheme clusters (combining marks, flags, ZWJ emoji) intact and return `String` or `List<String>`.
- `List<T>`: literals, indexing with bounds checks, iteration with `for`, `push(value)`, `len()`.
- `Option<T>`: `None`, `Some(value)`, exhaustive `match`, `?` propagation.
- `Result<T, E>`: `Ok(value)`, `Err(error)`, exhaustive `match`, `?` propagation.
- Structs: field access, literals, mutation through mutable roots.
- Enums: unit or multi-field variants, exhaustive `match`, and non-generic `impl Type` methods with `self`.
- Tuples: inline values, `.0` access, tuple patterns and field mutation.
- Arrays: `[T; N]`, compile-time length, bounds-checked indexing, `.len()` and `.as_slice()`.
- `Slice<T>`: borrowed `{ptr, len}` view with bounds-checked indexing and `.len()`.
- `Map<K, V>` / `Set<T>`: hashed collections built with `map()` / `set()` and a type annotation; open addressing with FNV-1a key hashing; keys limited to `Int`, `Bool` and `String`.
- Generic functions: explicit one-parameter functions such as `fn id<T>(x: T) -> T`, monomorphised from `id<Int>(...)`. Generic structs, enums and methods are not implemented.
- `fn(...) -> T`: function values for named functions and capturing closures; captures copy the outer value at closure creation.
- Traits: static `trait` declarations and `impl Trait for Type` conformance with concrete method dispatch. `dyn Trait` is not implemented.

Byte and Unicode-scalar string indexes, general string slicing, iterators, fixed-width integers, mutable captures and lifetime-checked references are **not implemented**. `clip`, `wrap` and `graphemes` are layout operations, not general indexing; they do not expose byte offsets, and cluster segmentation is an approximation rather than full UAX #29.

## Modules

Standard imports expose the builtin surface. `import "util.nova"` flattens another source file into the current program with cycle checks; `import "util.nova" as util` additionally qualifies calls, type names, struct literals, enum constructors and patterns (`util.Point { ... }`, `util.Event.Key(code)`). A module imported through two different relative paths is expanded once. The underlying namespace is still flat, so duplicate unqualified names, and variant names shared by two enums, remain errors. Package resolution, a registry and a generated module index are **planned**. The current truth is the compiler source and its tests, not an aspirational API listing.

## Planned

Formatting, math, iterators, filesystem/path/env/process/time, a real JSON parser, CSV, regex, encoding, random, an HTTP client with header/TLS control, async/channels, logging and testing libraries. Crypto/TLS must use audited platform or library facilities; Nova will not invent cryptography.

## Errors and platform

Runtime failures abort with a source location. There is no exception recovery or filesystem/path/env API yet; `http_get` and the `http_*` server builtins are the only networking surface, and both are POSIX only. Allocation is process-lifetime; see [Memory model](../language/memory/).
