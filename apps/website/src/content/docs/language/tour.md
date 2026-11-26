---
title: Language tour
description: The implemented Nova subset, with a complete checked example.
---

Nova is intentionally small today. This page describes only syntax and behavior the compiler accepts. Planned syntax is listed at the end and is not presented as executable.

## A complete program

```nova check title="examples/tour.nova"
import std.io { println }

struct User {
    name: String
    age: Int
}

fn describe(user: User) -> String {
    return "${user.name} (${user.age})"
}

fn main() {
    let user = User { name: "Ada", age: 36 }
    var total = 0
    for i in 0..4 {
        total += i
    }
    println(describe(user))
    println(total)
}
```

## Bindings

`let` creates an immutable binding. `var` creates a mutable one. Local types are inferred; parameters, return types and struct fields are annotated.

```nova
let name = "World"
var count = 0
count += 1
```

Reassigning a `let` is a type error. Shadowing is lexical and explicit. There is no implicit conversion between `Int` and `Float`, no truthiness, and no null.

## Scalars and strings

Implemented types: `Bool`, `Int` (i64), `Float` (f64), `Char`, `String`, `Unit`, `List<T>`, `Option<T>`, `Result<T, E>` and structs.

Strings are UTF-8 byte sequences with immutable value semantics. Interpolation uses `${expr}` and supports `Bool`, `Int`, `Float`, `Char` and `String`. Byte and Unicode-scalar indexing APIs are **planned**; do not assume constant-time Unicode indexing.

## Control flow

```nova
fn main() {
    var total = 0
    for i in 0..5 {
        total += i
    }
    while total > 6 {
        total -= 1
    }
    if total == 6 {
        println("six")
    } else {
        println("not six")
    }
}
```

Ranges are exclusive (`..`). Inclusive ranges, `break` values, `defer` and expressions in every position are not all implemented; the grammar page in the repository records the accepted subset.

## Collections and errors

```nova
fn first(items: List<Int>) -> Option<Int> {
    if items.len() == 0 {
        return None
    }
    return Some(items[0])
}

fn main() {
    match first([3, 4]) {
        Some(value) => println(value)
        None => println("empty")
    }
}
```

`Option` and `Result` have exhaustive matches over `None`, `Some`, `Ok` and `Err`. `?` propagates a compatible `None` or `Err`. Panics are unrecoverable and abort.

## Implemented builtins

`print`, `println`, `eprintln`, `assert`, `assert_eq`, `panic`. Methods currently include list `push`, `len`, `get` and the operations exercised by the compiler tests. A comprehensive standard-library index is **planned**; the current module surface is minimal.

## Enums

```nova
enum Shape {
    Rect(Int, Int)
    Circle(Int)
    Empty
}

impl Shape {
    fn area(self) -> Int {
        match self {
            Rect(width, height) => width * height
            Circle(radius) => radius * radius * 3
            Empty => 0
        }
    }
}

fn main() {
    println(Rect(2, 3).area())
    println(Empty.area())
}
```

Variants may carry positional payload fields. Constructors are global (`Rect(2, 3)`, `Empty`), so variant names must be unique across enums; `Some`, `None`, `Ok` and `Err` are reserved. `impl Type { fn name(self) ... }` adds methods to a struct or enum; the first parameter must be `self`. Static `impl Trait for Type` conformance and concrete method dispatch work for non-generic types. `dyn Trait`, generic methods, static methods and trait impls on generic types are not implemented. Match must cover every variant or end with `_`.

## Tuples, arrays and slices

```nova
fn total(values: Slice<Int>) -> Int {
    var i = 0
    var sum = 0
    while i < values.len() {
        sum += values[i]
        i += 1
    }
    return sum
}

fn main() {
    let pair = (1, "two")
    println(pair.0)
    match (3, 4) {
        (left, right) => println(left + right)
    }
    var numbers: [Int; 3] = [1, 2, 3]
    numbers[1] = 9
    println(total(numbers.as_slice()))
}
```

Tuples are inline values: construction, `.0`-style access, tuple-typed returns, tuple patterns in `match`, and mutation of tuple fields through a mutable root. `let (a, b) = ...` destructuring is not implemented. Arrays have a compile-time length and bounds-checked indexing. `Slice<T>` is a borrowed `{ptr, len}` view created by `.as_slice()` on an array or list; it does not own storage, so do not return a slice of a local across its lifetime.

## Beyond the core

- Capturing closures (`|x: Int| x + base`) capture outer locals by value, including nested captures and closures returned from functions; named function values use the type `fn(Int) -> Int`.
- Hashed `Map<K, V>` and `Set<T>` created with `map()` / `set()` plus a type annotation; `insert`, `get`, `contains`, `contains_key` and `len` are supported. Keys are limited to `Int`, `Bool` and `String`.
- Explicit one-parameter generic functions are monomorphised at concrete call sites. Generic structs, enums and methods are not implemented.
- `trait` declarations and `impl Trait for Type` provide conformance checking and concrete method dispatch. `dyn Trait`, trait objects and trait impls on generic types are not implemented.
- Async syntax, tasks, channels and a scheduler are not implemented.
- `extern "C" fn` declarations bind directly to linked platform symbols. The boundary is copied values only: Bool, Int, Float, Char, Unit, and `String`, which is the same `{ const char *ptr; int64_t len; }` view on both sides (C must not free it). Lists, maps and user structs have no C representation, and declarations are emitted `nounwind`; pointer ownership transfer and unwinding across the boundary are not supported.
- `import "util.nova"` loads another source file into the same flat module namespace; `import "util.nova" as util` additionally allows qualified calls (`util.add(2, 3)`) and qualified type names (`util.User`). Imports are cycle-checked. The merged namespace is still flat, so duplicate unqualified names are errors, and a package registry is not implemented.

## Not implemented

Implemented on `String`: `len`, `is_empty`, `to_string`, `contains`, `starts_with`, `ends_with` and `trim`. Still planned: mutable captures, `dyn Trait`, generic structs/methods, trait impls on generic types, async, macros and named/default arguments.

These features are rejected with a diagnostic rather than silently compiled to something else. See [Status](../../status/) and the [reference](../reference/).
