---
title: CLI reference
description: Commands implemented by the Nova compiler and their current limits.
---

The binary is `nova`. Build it with `cargo build --release -p nova-cli` or run `cargo run -p nova-cli -- <command>`.

## Implemented commands

| Command | Behavior |
|---|---|
| `nova new <name>` | Create a binary project with manifest, source, test and `.gitignore`. |
| `nova init` | Initialize a project in the current directory. |
| `nova run [-- args]` | Build and execute the declared binary. Arguments are accepted, but `main` cannot read them yet. |
| `nova build [--release]` | Emit a native executable under `target/nova/`. |
| `nova check` | Parse, resolve and type-check without running. |
| `nova test` | Discover `@test` functions, compile each into its own process and report status. |
| `nova fmt [--check]` | Conservative indentation, blank-line and trailing-whitespace formatting. |
| `nova clean` | Remove generated artifacts. |
| `nova tree` | Print the project structure. |
| `nova repl` | Interactive evaluation labeled as interpreter-backed; not the native backend. |
| `nova lsp` | Start the Rust bootstrap language server on stdio. |
| `nova doctor` | Report toolchain, runtime ABI, LSP, FreeBuff and platform status. |

`nova box` owns package operations: `add`, `remove`, `update`, `search`, `info`, `fetch`, `tree`, `check`, `lock`, `metadata`, `pack`, `verify` and `unpack`. The top-level `nova add/remove/update` aliases are not commands; unsupported roadmap commands (`bench`, `lint`, `doc`) exit with status 2 and an explicit message.

`nova fmt` is not the Rust formatter: it runs the Nova formatter in
`libraries/nova-fmt` through `nova-tool`. The language server and the compiler
remain the Rust bootstrap.

## Diagnostics

Compiler diagnostics include stable codes, spans and rendered caret ranges. The CLI catches internal failures at its boundary and returns a non-zero exit status for errors. Check `nova doctor` before reporting a toolchain problem.

## Projects

`nova new` and `nova init` still scaffold `nova.toml`, `src/main.nova`, `tests/arithmetic.nova` and `.gitignore`. Boxes projects use `Box.toml`; a build writes the lockfile that matches the manifest (`Box.lock` for `Box.toml`, `nova.lock` for `nova.toml`). Artifacts go to `target/`.

`nova build --locked` fails when that lockfile is absent. Dependencies are imported by their declared name from path or from a fetched registry box pinned in the lock; see [Projects](../projects/).

## Known gaps

- No benchmarks, lint, docs generation or workspace commands.
- Registry publishing and `login` are not implemented; reads need no account.
- No package installation on folder open.
- No machine-readable test report.
- The REPL is not the final execution model.
