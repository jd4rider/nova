# Nova Studio

**A native desktop workbench for the Nova language: Monaco, a real Rust language server and actual compiler tasks.**

Nova Studio is an Electron + React + strict TypeScript application. It is part of the [Nova repository](https://github.com/jd4rider/nova) and is not a wrapper around VS Code or an AI service.

- Documentation: <https://jd4rider.github.io/nova/studio/overview/>
- Language and compiler: <https://github.com/jd4rider/nova>
- Architecture decision: [`docs/adr/0001-ide-architecture.md`](../docs/adr/0001-ide-architecture.md)

## Install

### Homebrew (macOS arm64)

```sh
brew install --cask jd4rider/tap/nova-studio
```

The cask depends on the `jd4rider/tap/nova` formula, so the compiler/CLI is installed alongside Studio. This is the recommended path.

### Direct installers

- [macOS arm64 DMG](https://github.com/jd4rider/nova/releases/download/v0.1.0/Nova-Studio-0.1.0-arm64.dmg)
- [Windows x64 NSIS EXE](https://github.com/jd4rider/nova/releases/download/v0.1.0/Nova-Studio-0.1.0-x64-setup.exe)
- [Checksums](https://github.com/jd4rider/nova/releases/download/v0.1.0/SHA256SUMS)

The installers are unsigned. The DMG installs Studio only; install the CLI separately with Homebrew. Windows Studio does not bundle the CLI and runs the terminal in pipe mode.

## Run from source

Prerequisites: Node 20+ (tested with Node 26 locally and Node 22 in CI) and a built `nova` binary.

```sh
cargo build -p nova-cli

cd editor
npm ci --no-audit --no-fund
NOVA_BIN="$PWD/../target/debug/nova" npm run dev
```

If npm has a root-owned cache, use `npm ci --cache /tmp/nova-npm-cache`. Electron's postinstall may be blocked by npm policy; run `node node_modules/electron/install.js` if the runtime is missing.

Production and installer checks:

```sh
npm run typecheck
npm run build
npm start        # preview the built app if the Electron runtime is installed

CSC_IDENTITY_AUTO_DISCOVERY=false npm run dist:mac
npm run dist:win
```

Artifacts are written to `release/`. The Windows command disables native-module rebuild and executable signing so it can cross-build without `node-pty`; the installer and app are unsigned.

## Features

- **Monaco editor** with Nova syntax highlighting and Gruvbox Dark, Light and High Contrast themes.
- **Rust LSP** diagnostics, completion, hover, go-to-definition, document symbols and formatting on unsaved buffers.
- **Explorer and tabs** with dirty state and external-change-aware atomic saves.
- **Real tasks**: Build, Run, Check and Test invoke the configured `nova` CLI with argv arrays and show actual output and exit codes.
- **Panels** for Problems, Output, Tests, Terminal and built-in **Docs**.
- **Search** via ripgrep; quick open, command palette and keyboard-first navigation.
- **Vim mode** via `monaco-vim` (partial emulation; see the docs).
- **Visual effects**: current-line lens, typing/scroll sparkles and synthesized Web Audio SFX. Toggle animation and sound independently in Settings; effects respect OS reduced-motion.
- **Dedicated FreeBuff terminal** with an explicit structured-AI boundary.

## Terminal and FreeBuff

`node-pty` is an optional dependency. If its native module is unavailable, the terminal runs in **pipe mode** (no TTY), the status bar reports `pipe`, and FreeBuff’s interactive TUI cannot run. This is an honest limitation, not a simulated terminal.

Structured FreeBuff chat, agents and inline ghost text are disabled because the installed CLI exposes only an interactive terminal. Nova does not scrape terminal output or automate authentication.

## Security

- Context isolation and renderer sandbox enabled.
- Node integration disabled; renderer has no Node access.
- CSP applied; navigation and new windows blocked.
- Narrow, validated preload IPC.
- Filesystem access is workspace-confined after symlink resolution.
- Tasks use argv arrays, not shell string concatenation.

See [`docs/adr/0006-electron-security.md`](../docs/adr/0006-electron-security.md). Workspace trust prompts and extension execution do not exist yet; do not treat the current model as a hostile-workspace sandbox.

## Known gaps

- No automated Electron end-to-end interaction suite yet.
- No Git panel, debugger/DAP, test explorer, split editor groups or multi-window.
- Vim mode is partial, not Neovim.
- macOS and Windows installers are published unsigned; no Developer ID/notarized or Authenticode build exists.
- The Windows installer does not include the language, and its terminal is pipe-only.
- Tauri 2 and native Rust alternatives were evaluated on paper but not prototyped; see ADR 0001.

## License

Nova Studio is currently `UNLICENSED` while the repository license is undecided. Third-party dependencies retain their licenses.
