---
title: Install Nova
description: Install Nova Studio and the Nova language with Homebrew, DMG, Windows EXE, VS Code extension or source.
---

**Status: v0.1.0 preview.** macOS arm64 installers and the CLI archive have been built and smoke-tested locally. The Windows x64 NSIS installer was cross-built and structurally verified, but has not been launched on Windows. No artifact is signed or notarized.

## Homebrew: Studio + language

The public tap installs both packages together:

```sh
brew install --cask jd4rider/tap/nova-studio
nova --version
open -a "Nova Studio"
```

The cask depends on `jd4rider/tap/nova`, so the language is installed automatically. If Homebrew asks you to trust the tap, run `brew trust jd4rider/tap`. For the language alone:

```sh
brew install jd4rider/tap/nova
```

Nova invokes Clang to link native programs; install the Xcode Command Line Tools if needed (`xcode-select --install`).

## Direct installers

- [Nova Studio macOS arm64 DMG](https://github.com/jd4rider/nova/releases/download/v0.1.0/Nova-Studio-0.1.0-arm64.dmg) — drag to Applications.
- [Nova Studio Windows x64 EXE](https://github.com/jd4rider/nova/releases/download/v0.1.0/Nova-Studio-0.1.0-x64-setup.exe) — NSIS installer.
- [macOS CLI archive](https://github.com/jd4rider/nova/releases/download/v0.1.0/nova-0.1.0-aarch64-apple-darwin.tar.gz).
- [VS Code extension VSIX](https://github.com/jd4rider/nova/releases/download/v0.1.0/nova-language-0.1.0.vsix) — syntax highlighting and LSP client.
- [SHA-256 checksums](https://github.com/jd4rider/nova/releases/download/v0.1.0/SHA256SUMS).

The DMG installs Studio only. The Windows installer does not bundle the language. Use Homebrew or build the CLI from source. The apps are unsigned; macOS may require right-click → Open and Windows SmartScreen may require More info → Run anyway.

## VS Code extension

Prefer Visual Studio Code? Install the local VSIX instead of Studio:

```sh
code --install-extension nova-language-0.1.0.vsix
```

Or open **Extensions → … → Install from VSIX…**. The extension provides `.nova` syntax highlighting and, when the `nova` CLI is installed, diagnostics, completion, hover, definitions, symbols and formatting through `nova lsp`. It does not bundle the compiler.

If VS Code cannot find `nova`, set `nova.serverPath` to the binary's absolute path. The extension is an unsigned VSIX, is not on the VS Code Marketplace or Open VSX, and has no automated Extension Host test yet. See [VS Code extension](../../toolchain/vscode/) for details.

## Requirements

- Rust stable (tested with 1.96) and Cargo for source builds
- Clang or Apple Clang on `PATH` (tested with Apple clang 21)
- Node.js 20+ and npm only if you want to build Nova Studio, the VS Code extension or the website
- A 64-bit macOS, Linux or Windows host for local compilation; only macOS arm64 is currently tested

You do **not** need a system LLVM installation. Nova emits textual LLVM IR and asks the host Clang to optimize, verify and link it. The backend tradeoff is recorded in ADR `0003-llvm-backend.md` in the repository.

## Build from source

```sh
git clone https://github.com/jd4rider/nova.git
cd nova
cargo build --release -p nova-cli
export PATH="$PWD/target/release:$PATH"
nova --version
nova doctor
```

The source is public at <https://github.com/jd4rider/nova>. `nova doctor` is the authoritative check for the compiler, Clang, runtime ABI and language server.

## Verify the compiler

```sh
nova new hello
cd hello
nova run
nova test
nova fmt --check
```

`nova run` should print `Hello, Nova!`. If it does not, read [Troubleshooting](../troubleshooting/).

## Build Nova Studio

```sh
cd editor
npm install --cache /tmp/nova-npm-cache
npm run typecheck
npm run build
NOVA_BIN="$PWD/../target/release/nova" npm run dev
```

If npm blocks Electron's postinstall script, run `node node_modules/electron/install.js` and `node node_modules/esbuild/install.js` manually. `node-pty` is optional; without it the terminal runs in pipe mode, and FreeBuff's terminal remains unavailable because it requires a TTY.

macOS and Windows Studio installers are published as unsigned preview artifacts. The VS Code extension is built from `editors/vscode`:

```sh
cd editors/vscode
npm ci --no-audit --no-fund
npm run package
```

There is no signed/notarized build, Intel Mac build, Linux desktop package, bundled Windows CLI or Marketplace extension yet.

## Next

- [Hello, world](../hello-world/)
- [Your first project](../first-project/)
- [Language tour](../../language/tour/)
