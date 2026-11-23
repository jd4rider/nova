# Nova language reference (implemented subset)

Nova is statically typed and native-compiled. Source is UTF-8 with byte-offset spans internally; the LSP negotiates UTF-16 positions.

## Declarations

```nova
import std.io { println }

struct User {
    name: String
    age: Int
}

fn greet(user: User) -> String {
    return "Hello, ${user.name}!"
}
```

Functions require parameter and return types. Struct fields require types. `let` bindings are immutable, `var` mutable. Local types are inferred. Variable and function names use `snake_case`; types use `PascalCase`. Statements end at newline or `;`; braces delimit blocks. `else` may follow a closing brace on the next line.

## Types

Implemented: `Bool`, `Int` (i64), `Float` (f64), `Char`, `String` (UTF-8), `Unit`, `List<T>`, `Option<T>`, `Result<T, E>`, structs, and nominal user enums.

`Option<T>` is `None` or `Some(value)`; `Result<T,E>` is `Ok(value)` or `Err(error)`. `?` propagates `None`/`Err` from the current function. Match must be exhaustive over these variants.

```nova
enum Shape {
    Circle(Int)
    Square(Int)
    Empty
}
```

A variant takes zero or one payload. Variant constructors are global in this build (`Circle(2)`, `Empty`), so variant names must be unique across enums, and `Some`, `None`, `Ok` and `Err` are reserved.

Implemented `String` methods: `len`, `is_empty`, `to_string`, `contains`, `starts_with`, `ends_with` and `trim`. `List` has `len`, `is_empty`, `get` and `push`.

Planned but rejected by the checker today: fixed-width integers, `USize`, `f32`, arrays/slices, tuples, `Map`, `Set`, generics, traits, methods on user types, closures, variants with multiple payload fields.

## Expressions and control flow

```nova
fn main() {
    let name = "World"
    var total = 0
    for i in 0..5 {
        total += i
    }
    while total > 3 {
        total -= 1
    }
    if total == 3 {
        println("three")
    } else {
        println("not three")
    }
    let items = [1, 2, 3]
    for item in items {
        println(item)
    }
}
```

Ranges are exclusive (`..`); inclusive ranges are not implemented. `if` is usable as an expression only in expression position supported by the current grammar. Arithmetic is checked; overflow, division by zero, and out-of-bounds list access abort with a diagnostic message rather than wrapping silently.

String interpolation uses `"${expr}"`. Supported builtins: `print`, `println`, `eprintln`, `assert`, `assert_eq`, `panic`. No user-defined operators, implicit conversions, truthiness, null, or exceptions.

## Pattern matching

```nova
fn first(items: List<Int>) -> Option<Int> {
    if items.length() == 0 {
        return None
    }
    return Some(items[0])
}

fn main() {
    match first([4, 5]) {
        Some(value) => println(value)
        None => println("empty")
    }
}
```

Patterns include `_`, identifiers, literals, `None`, `Some(...)`, `Ok(...)`, `Err(...)`, and user enum variants such as `Circle(radius)` or `Empty`. A bare identifier that names a unit variant is a variant pattern, not a new binding. No guards, or-patterns, or nested payload patterns yet.

## Limitations and semantics

- No user modules or path imports; `import std.io` is recognized for the builtins.
- No generics, despite `<T>` parsing experiments; such code is rejected rather than miscompiled.
- No FFI, unsafe, async, threads, closures, or debug information.
- Memory allocation currently lasts for the process lifetime; see [memory.md](memory.md).
- Panics abort. There is no unwind-based recovery.
- `main` does not yet receive command-line arguments.
- Formal newline/ambiguity conformance tests are partial; the grammar in [grammar.ebnf](grammar.ebnf) describes the implemented subset.

Run `nova check` for diagnostics and `nova fmt` for conservative formatting.
