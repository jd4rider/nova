# ADR 0005: Language subset and semantics

## Status

Accepted.

## Decision

- Braces, optional semicolons, newline-terminated statements, `snake_case` functions/variables, PascalCase types.
- `let` immutable, `var` mutable. Locals infer types; parameters and returns require annotations (public returns).
- No implicit null, truthiness, numeric narrowing, or arbitrary conversions.
- Checked integer arithmetic in all profiles; explicit runtime trap on overflow/division by zero. No wrapping operators yet.
- UTF-8 strings with immutable value semantics. Byte indexing and Unicode scalar APIs are not yet exposed, so no false constant-time indexing claim.
- `Option<T>` and `Result<T,E>` with `?` propagation; exhaustive matches.
- Runtime panics are unrecoverable and abort.

## Implemented syntax/semantics

Imports (std only), functions, structs, `let`/`var`, if/else, while, for over ranges and lists, break/continue, return, match, string interpolation, calls, struct literals, field access, list literals/indexing, builtins (`println`, `print`, `eprintln`, `assert`, `assert_eq`, `panic`).

## Not implemented

Generics, user enums, traits/impls, methods, closures, tuples, maps/sets, arrays/slices, defer, named/default arguments, macros, user modules, async, FFI, unsafe, overloads, exceptions.

## Consequences

The compiler rejects unsupported syntax with diagnostics rather than silently miscompiling. The IDE advertises only capabilities backed by these semantics.
