---
title: Cookbook
description: Recipes using only implemented Nova features.
---

Only implemented syntax appears here. Planned recipes such as HTTP services, JSON processing, concurrency and C interop are intentionally absent until the language supports them.

## Structs and formatting

```nova
struct Point {
    x: Int
    y: Int
}

fn show(point: Point) -> String {
    return "(${point.x}, ${point.y})"
}

fn main() {
    let origin = Point { x: 0, y: 0 }
    println(show(origin))
}
```

## Lists and ranges

```nova
fn main() {
    var values = [1, 2, 3]
    values.push(4)
    var total = 0
    for value in values {
        total += value
    }
    println(total)
}
```

## Optional lookup

```nova
fn first(items: List<Int>) -> Option<Int> {
    if items.len() == 0 {
        return None
    }
    return Some(items[0])
}

fn main() {
    match first([7, 8]) {
        Some(value) => println(value)
        None => println("empty")
    }
}
```

These are small, readable programs. They are not a standard-library tour: the current library surface is minimal. See [Status](../status/) for collection and stdlib coverage.
