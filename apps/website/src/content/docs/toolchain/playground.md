---
title: Browser playground
description: The real Nova parser, type checker and formatter compiled to WebAssembly.
---

The [Nova playground](../../playground/) is a static page that compiles the same Rust front end used by the CLI and language server to WebAssembly.

It is a **check and format** environment, not an interpreter. LLVM emission, Clang linking and the native runtime stay on the host, so execution remains `nova run` or Nova Studio.

## What is real

- `nova-lexer` tokenisation and diagnostics
- `nova-parser` syntax and recovery diagnostics
- `nova-sema` type checking, name resolution and exhaustiveness checks
- `nova-fmt` formatting
- UTF-16 line/character positions for the editor
- The same source examples and grammar as the rest of the project

The page fetches a local `playground/nova.wasm` file and calls a small exported ABI (`nova_alloc`, `nova_check`, `nova_result_len`, `nova_free`). Your source is not uploaded to a server.

## Build locally

The checked-in WASM is generated from the repository:

```sh
rustup target add wasm32-unknown-unknown
cd apps/website
npm run build:playground
npm run test:playground
```

`build:playground` builds `crates/nova-wasm` and copies `nova_wasm.wasm` to `public/playground/nova.wasm`. `test:playground` instantiates the module in Node, checks that it has no imports, verifies a passing program and asserts that a bad type annotation produces `E0308`.

## Limits

The WASM build intentionally excludes native code generation. It does not execute programs, perform I/O, handle HTTPS, link user files or expose a package registry. Those require compiler and runtime work that is tracked on the [status page](../../status/) and [roadmap](../../roadmap/).

The example gallery currently demonstrates functions, structs, lists, loops, `Option`, `Result`, matching and a deliberate type error. User enums, classes/traits, generics, modules and the planned TUI/GUI/WASM libraries are not implemented yet.
