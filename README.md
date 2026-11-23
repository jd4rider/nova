# 🦀 Nova

**A statically typed programming language with a Rust compiler, an LLVM native backend, a language server and a desktop IDE.**

[![CI](https://github.com/jd4rider/nova/actions/workflows/ci.yml/badge.svg)](https://github.com/jd4rider/nova/actions/workflows/ci.yml)
[![Website and GitHub Pages](https://github.com/jd4rider/nova/actions/workflows/pages.yml/badge.svg)](https://github.com/jd4rider/nova/actions/workflows/pages.yml)
[![Rust](https://img.shields.io/badge/Rust-1.96-000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Status: working vertical slice](https://img.shields.io/badge/status-working%20vertical%20slice-fabd2f)](ROADMAP.md)

[Website](https://jd4rider.github.io/nova/) · [Documentation](https://jd4rider.github.io/nova/docs/) · [Nova Studio](https://jd4rider.github.io/nova/studio/) · [Launch post](https://jd4rider.github.io/nova/blog/building-nova-in-rust/) · [Roadmap](ROADMAP.md) · [GitHub](https://github.com/jd4rider/nova)

Nova is a **functioning native language toolchain** built in Rust: source is lexed, parsed, type-checked, lowered to LLVM IR and linked into a real executable. It is an early vertical slice — not a claim that the language is complete. The supported subset and every known gap are documented.

## Try it

Requirements: Rust 1.96+, Cargo, and a C compiler/linker (`clang`). Node 20+ is only needed for Nova Studio and the website.

```sh
git clone https://github.com/jd4rider/nova.git
cd nova
cargo build --workspace
cargo test --workspace

cargo run -p nova-cli -- new hello
cd hello
../target/debug/nova run
```

Installed binary: `target/debug/nova`.

```sh
nova new my-app       # create a project
nova check            # type-check
nova build            # emit a native executable
nova run              # build and run
nova test             # run project tests
nova fmt              # format
nova lsp              # start the Rust language server
nova doctor           # inspect the local toolchain
```

## A small Nova program

```nova
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
    for i in 0..3 {
        total += i
    }
    println(describe(user))
    println(total)
}
```

`nova run` prints:

```text
Ada (36)
3
```

## What works today

| Area | State | Evidence |
|---|---|---|
| Lexer, parser, spans, recovery, interpolation | **Tested** | `cargo test -p nova-lexer -p nova-parser` |
| Type checking, structs, Option/Result, match | **Tested** | `cargo test --workspace` |
| LLVM native codegen, checked arithmetic, bounds | **Tested** | Native codegen tests |
| CLI, formatter, manifests, project scaffolding | **Tested / implemented** | `nova --help`, integration tests |
| Rust LSP over stdio | **Implemented** | Diagnostics, completion, hover, definition, symbols, formatting |
| Nova Studio desktop IDE | **Implemented** | Typecheck and production build pass; interaction not yet automated |
| Current-line lens, sparkles, synthesized SFX | **Experimental** | Settings toggles, reduced-motion aware |
| Public documentation and website | **Tested** | Astro check, build, link/base checks, grammar and snippet verification |
| Generics, traits, closures, async, DAP | **Planned** | Explicitly rejected or absent |
| Signed installers and release binaries | **Not published** | Source build only |

The full matrix is in [ROADMAP.md](ROADMAP.md).

## The Rust pipeline

Nova is not an interpreter and the IDE is not the compiler. The workspace is split into focused Rust crates:

```text
crates/
  nova-lexer          tokens, spans and diagnostics
  nova-parser         recursive descent + Pratt parser
  nova-sema           name resolution and type checking
  nova-ir             typed intermediate representation
  nova-codegen-llvm   textual LLVM IR emission
  nova-runtime        native support library
  nova-cli            build, run, check, test, fmt, lsp, tree
  nova-lsp            stdio language server
  nova-pkg            manifests, projects and lockfiles
  nova-fmt            conservative formatter
```

Nova emits textual LLVM IR and invokes the host `clang`, so no system LLVM development package is required. See [ADR 0003](docs/adr/0003-llvm-backend.md).

## Nova Studio

```sh
cd editor
npm ci --no-audit --no-fund
NOVA_BIN="$PWD/../target/debug/nova" npm run dev
```

Open a folder containing `nova.toml`. Studio provides:

- Monaco editing with Nova syntax highlighting and Gruvbox Dark/Light/High Contrast.
- LSP diagnostics, completion, hover, definition and formatting against the real Rust server.
- Explorer, tabs, dirty state and external-change-aware saves.
- Real **Build / Run / Check / Test** tasks using argv arrays and actual compiler output.
- Problems, output, tests, terminal and a built-in **Docs** tab.
- Ripgrep search, quick open, command palette, Vim mode and persisted settings.
- Optional visual effects: a dock-style current-line lens, typing/scroll sparkles and synthesized Web Audio SFX. All are toggleable and respect reduced motion.
- A dedicated FreeBuff terminal. Structured FreeBuff chat/agent/ghost text is deliberately disabled because the installed CLI exposes only an interactive TUI. See [docs/freebuff.md](docs/freebuff.md).

The Electron renderer has no Node access: context isolation and sandbox are on, Node integration is off, CSP is applied, navigation is blocked, paths are confined to the workspace, and preload IPC is narrow. See [ADR 0006](docs/adr/0006-electron-security.md).

`node-pty` is optional. Without it the terminal reports **pipe mode** and FreeBuff cannot run its TUI; that limitation is shown, not hidden.

## Website and documentation

The site is a static **Astro 7 + Starlight** application in `apps/website`, with shared Nova grammar highlighting, Pagefind search, a docs hub and a blog. It supports both a custom domain/root deployment and a GitHub Pages project base such as `/nova/`.

Production: **https://jd4rider.github.io/nova/**

```sh
cd apps/website
npm ci --no-audit --no-fund
npm run check
npm run build
npm run test

# project-base validation
BASE_PATH=/nova/ SITE_URL=https://jd4rider.github.io npm run build
BASE_PATH=/nova/ SITE_URL=https://jd4rider.github.io npm run test:site
```

The Pages workflow is [`.github/workflows/pages.yml`](.github/workflows/pages.yml). It builds the compiler for checked snippets, freezes the website lockfile, runs Astro checks, verifies root and `/nova/` link bases, uploads the static artifact and deploys with the official Pages actions.

## Project layout

```text
crates/            Rust compiler, CLI, formatter, LSP and package model
editor/            Electron + React + Monaco desktop IDE
apps/website/      Astro/Starlight site, docs and blog
examples/          Runnable Nova programs
grammar/           Shared TextMate grammar for the site
docs/              Architecture, language, IDE and security notes
.github/workflows/ CI and GitHub Pages
```

## Development checks

```sh
cargo fmt --check
cargo test --workspace

cd editor
npm run typecheck
npm run build

cd ../apps/website
npm run check
npm run build
npm run test
```

The website checks include real compiler invocations for documentation snippets. There is no fake download, screenshot, benchmark or AI capability on the site.

## Limitations

- No generics, user enums with payloads, traits, methods, closures, async, FFI or macros.
- No user module graph beyond standard imports.
- No package registry, signed binaries, installers or published releases.
- No benchmark claim; performance work needs a recorded harness.
- No automated GUI interaction suite yet.
- macOS arm64 is tested; Linux and Windows are not verified end to end.
- Project licensing is undecided. Dependencies retain their own licenses; audit before redistribution.

See the [website status page](https://jd4rider.github.io/nova/status/) and [ROADMAP.md](ROADMAP.md) for the maintained source of truth.

## Documentation

- [Language reference](docs/language.md), [grammar](docs/grammar.ebnf), [memory model](docs/memory.md)
- [Nova Studio guide](docs/ide.md)
- [IDE architecture decision](docs/adr/0001-ide-architecture.md)
- [FreeBuff integration decision](docs/adr/0002-freebuff-integration.md)
- [Website and deployment](docs/website.md)
- [Threat model](docs/threat-model.md)
- [Roadmap and feature matrix](ROADMAP.md)

## License

Project licensing is **TBD**. The source is public for evaluation and contribution, but no reuse license is granted until a license file is added. Third-party dependencies keep their own licenses.
