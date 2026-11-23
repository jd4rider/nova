---
title: Hello, world
description: Compile and run a complete Nova program.
---

This complete program is compiled in the website's snippet check on every CI run.

```nova check title="src/main.nova"
import std.io { println }

fn main() {
    println("Hello, Nova!")
}
```

Create a project and run it:

```sh
nova new hello
cd hello
nova run
```

The compiler writes a native executable to `target/nova/hello` and executes it. `nova build` produces the binary without running it.

## What the program demonstrates

- `import std.io { println }` declares the standard IO symbols used by the program.
- `fn main()` is the entry point.
- `println` is a builtin that appends a newline and writes to standard output.
- Strings are UTF-8 and immutable values.

The checker requires parameter types and return types. Local bindings infer their type. There is no implicit null, truthiness or numeric narrowing.

## Next

- [Your first project](../first-project/)
- [Language tour](../../language/tour/)
