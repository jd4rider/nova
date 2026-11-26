---
title: Language reference
description: Normative reference for the implemented Nova subset.
---

This page is normative for the subset the compiler currently accepts. The [language tour](../tour/) is the tutorial. The formal EBNF lives in the repository at `docs/grammar.ebnf`; it and this page must be updated together. Planned features are marked and are not part of the accepted grammar.

## Lexical rules

- Source is UTF-8. Spans are byte offsets internally; the LSP converts to negotiated UTF-16 positions.
- Line comments start with `//`; block comments use `/* ... */`. Nested block comments are supported by the lexer.
- Identifiers are Unicode XID-style names; variables and functions use `snake_case`, types `PascalCase`.
- Newlines terminate statements. Semicolons are optional separators. Braces delimit blocks.
- Raw strings and escaped sequences are handled by the lexer; see the grammar for the accepted forms.
- Integer literals are i64. Floating literals are f64. Decimal prefixes and signs are not implicit conversions.

## Declarations

```text
program   = { import | function | struct | trait | impl }
function  = "fn" ident [ "<" ident ">" ] "(" params ")" [ "->" type ] block
struct    = "struct" ident "{" { ident ":" type } "}"
trait     = "trait" ident "{" { signature } "}"
signature = "fn" ident "(" "self" [ "," params ] ")" [ "->" type ]
impl      = "impl" [ ident "for" ] ident "{" { function } "}"
import    = "import" ident { "." ident } [ "{" ident { "," ident } "}" ]
```

Function parameters and return types are required when a value is returned. `main` returns `Unit`. Explicit one-parameter generic functions are monomorphized at concrete call sites; generic structs and generic methods are not implemented.

## Statements

`let name = expr`, `var name = expr`, assignment and compound assignment (`=`, `+=`, `-=`, `*=`, `/=`, `%=`), `if`/`else`, `while`, `for name in range_or_list`, `return`, `break`, `continue`, blocks and expression statements.

`let` bindings cannot be assigned. Assignment requires a mutable local or a field/index rooted in one. `return expr` is required when the function's declared return type is non-`Unit`.

## Expressions and precedence

Lowest to highest:

| Precedence | Operators | Associativity |
|---|---|---|
| 1 | `||` | left |
| 2 | `&&` | left |
| 3 | `==`, `!=` | left |
| 4 | `<`, `<=`, `>`, `>=` | left |
| 5 | `..` | non-associative |
| 6 | `+`, `-` | left |
| 7 | `*`, `/`, `%` | left |
| 8 | unary `-`, `!` | prefix |
| 9 | call, field, index, `?` | postfix |

Postfix calls, field access, indexing and `?` bind tighter than arithmetic. Parentheses override precedence. String concatenation uses `+` only when both operands are `String`.

## Types and checking

The checker resolves names lexically, infers local binding types, checks function/struct signatures, validates struct literals, checks mutability and validates exhaustive `Option`/`Result` matches. Recursive struct cycles are rejected.

There are no implicit conversions. A function argument must match its parameter type exactly. Integer arithmetic is checked at runtime; overflow and division by zero abort with a source line.

## Semantics

- `println` writes a newline; `print` does not; `eprintln` writes to stderr.
- `assert(false)` and failed `assert_eq` abort.
- Evaluation order is left-to-right for calls and binary operands; the compiler does not reorder observable effects.
- A failed `?` returns `None` or `Err` from the enclosing function.
- `match` arms are checked for exhaustiveness; a wildcard or binding may be used.

## Memory and runtime

The FFI boundary accepts only Bool, Int, Float, Char and Unit. Arguments and results are copied, generated declarations carry `nounwind`, and no pointer ownership is transferred; unwinding across the boundary is not modeled.

Runtime allocation currently lasts for the process lifetime. There is no reference counting, cycle collector or user-visible free. See [Memory model](../memory/). This is a documented limitation, not a claim of general memory safety.

## Not in the accepted grammar

Not in the accepted grammar: `dyn Trait` dispatch, generic structs/enums/methods, mutable captures, async scheduling/channels, macros, namespaced modules, fixed-width integers, unsafe blocks, `defer` and named/default arguments. Unsupported constructs should produce a diagnostic; if one is silently accepted, treat it as a compiler bug.

Accepted in the current grammar: user enums, non-generic `impl` methods with `self`, structs, tuples, fixed arrays, borrowed `Slice<T>`, explicit one-parameter generic functions, static `trait` declarations with `impl Trait for Type` conformance and concrete method dispatch, value-capturing closures and function values, hashed `Map`/`Set` basics, path imports, and `extern "C"` declarations. Captures are by value: a closure sees the value an outer local held when the closure was created, and mutating the original local afterwards does not change the capture. `let` does not destructure tuples, and slices are not lifetime-checked.
