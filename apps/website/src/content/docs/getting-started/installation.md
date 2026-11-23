---
title: Install Nova
description: Build the Nova compiler and language server from source on macOS.
---

**Status: tested on macOS arm64.** Windows and Linux builds have not been verified. Nova Studio and the compiler do not yet have signed installers or published release assets.

## Requirements

- Rust stable (tested with 1.96) and Cargo
- Clang or Apple Clang on `PATH` (tested with Apple clang 21)
- Node.js 20+ and npm only if you want to build Nova Studio or the website
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

No signed `.dmg`, `.exe` or `.AppImage` is published. Do not treat a local build as a release.

## Next

- [Hello, world](../hello-world/)
- [Your first project](../first-project/)
- [Language tour](../../language/tour/)
