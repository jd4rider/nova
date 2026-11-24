# 🦀 Nova

**A statically typed programming language with a Rust bootstrap compiler, an LLVM native backend, a language server, a desktop IDE, a VS Code extension and a Nova package manager.**

The compiler and language server are still the Rust bootstrap in `bootstrap/`.
The formatter, diagnostics and Nova Boxes are Nova implementations in
`libraries/`, run through `nova-tool`; the compiler is not self-hosted yet. See
[the migration plan](docs/rust-migration.md).

[![CI](https://github.com/jd4rider/nova/actions/workflows/ci.yml/badge.svg)](https://github.com/jd4rider/nova/actions/workflows/ci.yml)
[![Website and GitHub Pages](https://github.com/jd4rider/nova/actions/workflows/pages.yml/badge.svg)](https://github.com/jd4rider/nova/actions/workflows/pages.yml)
[![Rust](https://img.shields.io/badge/Rust-1.96-000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Status: working vertical slice](https://img.shields.io/badge/status-working%20vertical%20slice-fabd2f)](ROADMAP.md)

[Website](https://jd4rider.github.io/nova/) · [Playground](https://jd4rider.github.io/nova/playground/) · [Documentation](https://jd4rider.github.io/nova/docs/) · [Nova Studio](https://jd4rider.github.io/nova/studio/) · [VS Code extension](https://jd4rider.github.io/nova/toolchain/vscode/) · [Launch post](https://jd4rider.github.io/nova/blog/building-nova-in-rust/) · [Roadmap](ROADMAP.md) · [GitHub](https://github.com/jd4rider/nova)

Nova is a **functioning native language toolchain** built in Rust: source is lexed, parsed, type-checked, lowered to LLVM IR and linked into a real executable. It is an early vertical slice — not a claim that the language is complete. The supported subset and every known gap are documented.

## Install

### Homebrew: Studio + language together

The macOS cask depends on the language formula, so one command installs both:

```sh
brew install --cask jd4rider/tap/nova-studio
nova --version
open -a "Nova Studio"
```

If Homebrew asks you to trust the tap: `brew trust jd4rider/tap`. Language only:

```sh
brew install jd4rider/tap/nova
```

The tap is [jd4rider/homebrew-tap](https://github.com/jd4rider/homebrew-tap). It currently targets macOS arm64 and still ships `nova 0.1.0`; the tap has not been bumped to 0.2.0 yet, so Homebrew installs the previous build.

### Direct installers

- [Nova Studio v0.2.0 macOS arm64 DMG](https://github.com/jd4rider/nova/releases/download/v0.2.0/Nova-Studio-0.2.0-arm64.dmg)
- [Nova Studio v0.2.0 Windows x64 NSIS EXE](https://github.com/jd4rider/nova/releases/download/v0.2.0/Nova-Studio-0.2.0-x64-setup.exe)
- [macOS arm64 CLI archive](https://github.com/jd4rider/nova/releases/download/v0.2.0/nova-0.2.0-aarch64-apple-darwin.tar.gz)
- [Nova Language VS Code extension](https://github.com/jd4rider/nova/releases/download/v0.2.0/nova-language-0.2.0.vsix) (309 KB)
- [SHA-256 checksums](https://github.com/jd4rider/nova/releases/download/v0.2.0/SHA256SUMS)

These are **unsigned** preview builds. macOS may require right-click → Open; Windows SmartScreen may require More info → Run anyway. The DMG installs Studio only; the Windows installer does not bundle the CLI. The VSIX is also unsigned and is not published to the VS Code Marketplace. See [INSTALL.md](INSTALL.md) for every path.

### VS Code extension

Prefer your own editor? The Nova Language extension adds `.nova` syntax highlighting and launches the same Rust LSP through `nova lsp`:

```sh
code --install-extension nova-language-0.2.0.vsix
```

Download the [VSIX from the v0.2.0 release](https://github.com/jd4rider/nova/releases/download/v0.2.0/nova-language-0.2.0.vsix), or open **Extensions → … → Install from VSIX…** in VS Code. The extension requires VS Code 1.85+ and a `nova` binary for language-server features; set `nova.serverPath` if VS Code cannot find it. See [editors/vscode/README.md](editors/vscode/README.md).

### Browser playground

The [website playground](https://jd4rider.github.io/nova/playground/) runs the real lexer, parser, type checker and formatter in WebAssembly. It checks and formats source without uploading it, but it does **not** execute native programs. Use `nova run` for execution.

### Build from source

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
| Type checking, structs, enums, Option/Result, match, string predicates | **Tested** | `cargo test --workspace` |
| LLVM native codegen, checked arithmetic, bounds | **Tested** | Native codegen tests |
| CLI, formatter, manifests, project scaffolding | **Tested / implemented** | `nova --help`, integration tests |
| Nova Boxes: manifests, SemVer, resolution, archives, fetch | **Implemented** | `examples/nova-tool/tests/boxes.sh` (64 assertions); publishing planned |
| Rust LSP over stdio | **Implemented** | Diagnostics, completion, hover, definition, symbols, formatting |
| VS Code extension | **Implemented** | VSIX installs; shared grammar and live `nova lsp` transcript verified |
| Browser playground | **Implemented** | Real front end compiled to WASM; valid/invalid ABI tests and static route verified |
| Nova Studio desktop IDE | **Implemented** | Typecheck and production build pass; interaction not yet automated |
| Current-line lens, sparkles, synthesized SFX | **Experimental** | Settings toggles, reduced-motion aware |
| Public documentation and website | **Tested** | Astro check, build, link/base checks, grammar and snippet verification |
| Unsigned desktop installers | **Published** | macOS DMG, Windows NSIS EXE, macOS CLI archive, checksums |
| Signed/notarized releases | **Planned** | No Developer ID, notarization or Authenticode |
| Generics, traits, closures, async, DAP | **Partial** | Generic functions and types, static traits, `dyn Trait` dispatch, capturing closures and synchronous async; a scheduler and DAP are absent |

The full matrix is in [ROADMAP.md](ROADMAP.md).

## The Rust pipeline

Nova is not an interpreter and the IDE is not the compiler. The workspace is split into focused Rust crates:

```text
bootstrap/
  nova-lexer          tokens, spans and diagnostics
  nova-parser         recursive descent + Pratt parser
  nova-sema           name resolution and type checking
  nova-codegen-llvm   textual LLVM IR emission
  nova-cli            build, run, check, test, fmt, lsp, tree
  nova-lsp            stdio language server
  nova-wasm           browser front-end ABI
  nova-pkg            manifests, projects and lockfiles
```

Nova emits textual LLVM IR and invokes the host `clang`, so no system LLVM development package is required. See [ADR 0003](docs/adr/0003-llvm-backend.md).

## Nova Studio

Install with Homebrew (`brew install --cask jd4rider/tap/nova-studio`) or build locally:

```sh
cd editor
npm ci --no-audit --no-fund
NOVA_BIN="$PWD/../target/debug/nova" npm run dev
```

Open a folder containing `nova.toml`. Studio provides:

- Monaco editing with Nova syntax highlighting and a VS Code Dark+ theme by default; Gruvbox Dark/Light/High Contrast stay selectable.
- LSP diagnostics, completion, hover, definition and formatting against the real Rust server.
- Reopens the folder from your previous session on launch, or starts cleanly if it was moved or deleted.
- Explorer with live workspace watching: files created or edited outside Studio refresh the tree, clean tabs reload from disk with the caret preserved, and dirty buffers are never overwritten.
- Tab context menu (close, close others, close all, copy path, reveal), Save All, Go to Line and VS Code-style shortcuts.
- Real **Build / Run / Check / Test** tasks using argv arrays and actual compiler output.
- Problems, output, tests, terminal and a built-in **Docs** tab.
- Ripgrep search, quick open, command palette, Vim mode and persisted settings.
- Optional visual effects: a dock-style current-line lens, typing/scroll sparkles and synthesized Web Audio SFX. All are toggleable and respect reduced motion.
- An AI panel with GitHub Copilot device login, inline ghost text, Copilot/Claude/OpenAI-compatible chat, and keep/rollback diff review for proposed file edits. FreeBuff runs in a PTY and is exposed as an experimental scraped chat, with the raw TUI one click away. Keys are encrypted with the OS credential store and never reach the renderer. See [docs/freebuff.md](docs/freebuff.md).

The Electron renderer has no Node access: context isolation and sandbox are on, Node integration is off, CSP is applied, navigation is blocked, paths are confined to the workspace, and preload IPC is narrow. See [ADR 0006](docs/adr/0006-electron-security.md).

Prefer VS Code? The [Nova Language extension](editors/vscode/README.md) provides syntax highlighting and the same Rust LSP without the Studio shell. It is distributed as a local VSIX, not through the Marketplace.

`node-pty` is optional. Without it the terminal reports **pipe mode** and FreeBuff cannot run its TUI; that limitation is shown, not hidden. The FreeBuff chat scrape is heuristic and experimental, not a supported FreeBuff protocol.

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
bootstrap/         Rust bootstrap compiler (being replaced by Nova, phase by phase)
editor/            Electron + React + Monaco desktop IDE
editors/vscode/    VS Code syntax highlighting + LSP client
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

- `String` supports `len`, `is_empty`, `to_string`, `contains`, `starts_with`, `ends_with` and `trim`. Generic functions, structs, enums and inherent methods are monomorphised, `trait`/`impl Trait for Type` perform static conformance, `dyn Trait` boxes a copy of the value behind a per-type vtable, closures capture outer locals by value, and expression/statement macros expand by AST substitution. Still missing: real async scheduling or channels, trait impls on generic types, and registry publishing.
- User modules are loaded with `import "file.nova"`, optionally aliased with `as` for qualified calls and type names. The merged namespace is flat, cycle-checked, and has no package registry.
- Unsigned macOS/Windows installers and a macOS CLI archive are published; there are no signed/notarized builds, Intel/Linux installers, Windows CLI bundle or package registry.
- The VS Code extension is a local, unsigned VSIX; it is not published to the VS Code Marketplace or Open VSX and has no automated Extension Host test.
- The browser playground checks and formats only; native execution, HTTP, modules, user enums/classes and the planned library ecosystem remain unimplemented.
- No benchmark claim; performance work needs a recorded harness.
- No automated GUI interaction suite yet.
- macOS arm64 is tested; Linux and Windows are not verified end to end.
- Project licensing is undecided. Dependencies retain their own licenses; audit before redistribution.

See the [website status page](https://jd4rider.github.io/nova/status/) and [ROADMAP.md](ROADMAP.md) for the maintained source of truth.

## Documentation

- [Install guide and checksums](INSTALL.md)
- [VS Code extension](editors/vscode/README.md)
- [Browser playground](https://jd4rider.github.io/nova/playground/)
- [Homebrew tap](packaging/homebrew/README.md)
- [Language reference](docs/language.md), [grammar](docs/grammar.ebnf), [memory model](docs/memory.md)
- [Nova Studio guide](docs/ide.md)
- [IDE architecture decision](docs/adr/0001-ide-architecture.md)
- [FreeBuff integration decision](docs/adr/0002-freebuff-integration.md)
- [Website and deployment](docs/website.md)
- [Threat model](docs/threat-model.md)
- [Roadmap and feature matrix](ROADMAP.md)

## License

Project licensing is **TBD**. The source is public for evaluation and contribution, but no reuse license is granted until a license file is added. Third-party dependencies keep their own licenses.
