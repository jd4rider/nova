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
    Circle(Int)
    Square(Int)
    Empty
}

fn area(shape: Shape) -> Int {
    match shape {
        Circle(radius) => radius * radius * 3
        Square(side) => side * side
        Empty => 0
    }
}

fn main() {
    println(area(Circle(2)))
    println(area(Empty))
}
```

A variant takes zero or one payload. Constructors are global (`Circle(2)`, `Empty`), so variant names must be unique across enums; `Some`, `None`, `Ok` and `Err` are reserved. Match must cover every variant or end with `_`.

## Not implemented

Implemented on `String`: `len`, `is_empty`, `to_string`, `contains`, `starts_with`, `ends_with` and `trim`. Still planned: generics, variants with multiple payload fields, traits and impls, methods on user types, closures, tuples, maps, sets, arrays/slices, async, FFI, macros, named/default arguments, user modules and a package registry.

These features are rejected with a diagnostic rather than silently compiled to something else. See [Status](../../status/) and the [reference](../reference/).
