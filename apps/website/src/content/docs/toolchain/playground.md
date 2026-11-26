---
title: Browser playground
description: The real Nova parser, type checker and formatter compiled to WebAssembly.
---

The [Nova playground](../../playground/) is a static page that runs a checked-in WebAssembly build of the front end. Rebuilding that artifact still needs a Nova wasm backend.

It is a **check and format** environment, not an interpreter. LLVM emission, Clang linking and the native runtime stay on the host, so execution remains `nova run` or Nova Studio.

## What is real

- `nova-lexer` tokenisation and diagnostics
- `nova-parser` syntax and recovery diagnostics
- `nova-sema` type checking, name resolution and exhaustiveness checks
- `nova-fmt` formatting (the Nova formatter compiled to wasm)
- UTF-16 line/character positions for the editor
- The same source examples and grammar as the rest of the project

The page fetches a local `playground/nova.wasm` file and calls a small exported ABI (`nova_alloc`, `nova_check`, `nova_result_len`, `nova_free`). Your source is not uploaded to a server.

## Build locally

The checked-in WASM artifacts are verified without rebuilding:

```sh
cd apps/website
npm run test:playground
```

Rebuilding them requires a Nova wasm backend, which is still a tracked gap.

There is no Nova wasm build yet: the removed Rust front end produced the checked-in `public/playground/nova.wasm`. `test:playground` instantiates that frozen module in Node, checks that it has no imports, verifies a passing program and asserts that a bad type annotation produces `E0308`.

## Limits

The WASM build intentionally excludes native code generation. It does not execute programs, perform I/O, handle HTTPS, link user files or expose a package registry. Those require compiler and runtime work that is tracked on the [status page](../../status/) and [roadmap](../../roadmap/).

The example gallery demonstrates functions, structs, lists, loops, `Option`, `Result`, matching and a deliberate type error. The checked-in wasm predates the native compiler's latest work and may lag its subset; use `nova check` for the authoritative parser, checker and formatter. The [TUI library](../../libraries/tui/) is an experimental source-imported library; real concurrency and GUI/WASM application libraries are not implemented.
