---
title: Troubleshooting
description: Diagnose toolchain, Studio and website problems with real error paths.
---

## Start with `nova doctor`

```sh
nova doctor
```

It prints the Nova version, executable path, OS, runtime ABI, Clang version, LLVM version when available, Node, FreeBuff, language-server status and known FreeBuff limitations. Report its output when filing a toolchain issue.

## `no C toolchain found`

Nova links through Clang. Install Xcode command line tools or Clang and confirm:

```sh
clang --version
```

Set `NOVA_CC` to an explicit compiler path if needed. A system LLVM installation is not required; the host Clang supplies LLVM and the platform linker.

## `nova: command not found`

Use the Cargo binary directly or add it to `PATH`:

```sh
cargo build --release -p nova-cli
export PATH="$PWD/target/release:$PATH"
```

## Studio cannot find the compiler

Pass an absolute path when starting Studio:

```sh
NOVA_BIN="$PWD/target/debug/nova" npm run dev
```

Without `nova.toml`, Studio opens and highlights files but reports that LSP startup and tasks are unavailable.

## npm cache permission errors

Do not change system ownership unless you understand the cause. Use a temporary cache:

```sh
npm install --cache /tmp/nova-npm-cache
```

## Electron binary missing

If npm blocked install scripts, run the local installer:

```sh
cd editor
node node_modules/electron/install.js
```

The terminal reports **pipe mode** when `node-pty` is unavailable. That is an honest fallback, not a full TTY. FreeBuff requires a PTY and is disabled without it.

## The website build fails on a base path

Build with the same base used by deployment and run the verifier:

```sh
cd apps/website
BASE_PATH=/nova/ SITE_URL=https://example.github.io npm run build
BASE_PATH=/nova/ npm run test:site
```

The verifier rejects root-relative links that escape the configured base and checks representative routes and search assets.
