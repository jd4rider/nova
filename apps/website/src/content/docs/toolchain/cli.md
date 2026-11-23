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
| `nova lsp --stdio` | Start the Rust language server on stdio. |
| `nova doctor` | Report toolchain, runtime ABI, LSP, FreeBuff and platform status. |

Unsupported roadmap commands (`bench`, `lint`, `doc`, `add`, `remove`, `update`) exit with status 2 and an explicit message. They are not silently ignored.

## Diagnostics

Compiler diagnostics include stable codes, spans and rendered caret ranges. The CLI catches internal failures at its boundary and returns a non-zero exit status for errors. Check `nova doctor` before reporting a toolchain problem.

## Projects

`nova new` and `nova init` create `nova.toml`, `src/main.nova`, `tests/arithmetic.nova`, `nova.lock` when needed and `.gitignore`. Artifacts go to `target/`.

## Known gaps

- No benchmarks, lint, docs generation, registry commands or workspace commands.
- No package installation on folder open.
- No machine-readable test report.
- The REPL is not the final execution model.
