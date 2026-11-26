# Nova Studio

**A native desktop workbench for the Nova language: Monaco, the Nova language server and actual compiler tasks.**

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

### Prefer VS Code?

Install the [Nova Language extension](../editors/vscode/README.md) for `.nova` syntax highlighting and the same Nova LSP without Studio. The extension is a local VSIX, not a Marketplace listing.

## Run from source

Prerequisites: Node 20+ (tested with Node 26 locally and Node 22 in CI) and a built `nova` binary.

```sh

cd editor
npm ci --no-audit --no-fund
NOVA_BIN="$PWD/../bootstrap/nova" npm run dev
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

- **Monaco editor** with Nova syntax highlighting, the bundled JetBrains Mono Nerd Font and Material Icon Theme file/folder icons. The UI and editor default to a Visual Studio Code Dark+ look, with Gruvbox Dark, Light and High Contrast still selectable.
- **Nova LSP** diagnostics, completion, hover, go-to-definition, document symbols and formatting on unsaved buffers. Studio runs the Nova-built server (`nova-tool lsp`) and falls back to `nova lsp` when it is not installed.
- **Remembered workspace**: the last opened folder is reopened at launch when it still exists; missing or moved folders fall back to an empty workbench.
- **Explorer and tabs** with dirty state, a live recursive workspace watcher (external creates and edits refresh the tree; clean tabs reload, dirty buffers do not), and external-change-aware atomic saves.
- **Tab actions**: context menu for close/close others/close all, copy path and reveal; Save All; Go to Line; VS Code-style shortcuts.
- **Real tasks**: Build, Run, Check and Test invoke the configured `nova` CLI with argv arrays and show actual output and exit codes.
- **Panels** for Problems, Output, Tests, Terminal and built-in **Docs**.
- **Search** via ripgrep; quick open, command palette and keyboard-first navigation.
- **Vim mode** via `monaco-vim` (partial emulation; see the docs).
- **Visual effects**: current-line lens, typing/scroll sparkles and synthesized Web Audio SFX. Toggle animation and sound independently in Settings; effects respect OS reduced-motion.
- **AI panel** with two tabs: a real FreeBuff CLI session reconstructed as chat, and streaming chat through the official GitHub Copilot SDK, Claude, or any OpenAI-compatible endpoint (GitHub Models, OpenAI, OpenRouter, local Ollama). Copilot uses GitHub device sign-in; API keys are encrypted with the OS credential store in the main process.

## Terminal and FreeBuff

`node-pty` is an optional dependency. If its native module is unavailable, the terminal runs in **pipe mode** (no TTY), the status bar reports `pipe`, and FreeBuff’s interactive TUI cannot run. This is an honest limitation, not a simulated terminal.

FreeBuff 0.2.36 exposes an interactive TUI only: `--print` and `-p` are rejected and stdin just starts the TUI. Studio runs the real CLI in a PTY, reconstructs its screen with a headless terminal, and extracts the transcript above the composer. This is a version-specific scrape, not a FreeBuff API; the Raw TUI remains available when the heuristic is wrong. FreeBuff cannot provide inline ghost text because the installed CLI has no completion channel. Copilot chat/auth/models use the official `@microsoft/copilot-sdk`; Copilot inline completion uses the Copilot language server.

## Security

- Context isolation and renderer sandbox enabled.
- Node integration disabled; renderer has no Node access.
- CSP applied; navigation and new windows blocked.
- Narrow, validated preload IPC.
- Filesystem access is workspace-confined after symlink resolution.
- Tasks use argv arrays, not shell string concatenation.
- AI keys are encrypted with the OS credential store in the main process; the renderer never receives them and they are never written to the workspace.

See [`docs/adr/0006-electron-security.md`](../docs/adr/0006-electron-security.md). Workspace trust prompts and extension execution do not exist yet; do not treat the current model as a hostile-workspace sandbox.

## Known gaps

- No automated Electron end-to-end interaction suite yet.
- No Git panel, debugger/DAP, test explorer, split editor groups or multi-window.
- Vim mode is partial, not Neovim.
- macOS and Windows installers are published unsigned; no Developer ID/notarized or Authenticode build exists.
- No automated tests cover the AI panel or a live provider call; adapters are exercised against a local SSE server and the app is checked by typecheck and build.
- FreeBuff inline completion is not available; only its scraped chat is supported by the current CLI. Copilot inline completion depends on the Copilot language server and the account authorized through the SDK.
- The Windows installer does not include the language, and its terminal is pipe-only.
- Tauri 2 and native Rust alternatives were evaluated on paper but not prototyped; see ADR 0001.

## License

Nova Studio is currently `UNLICENSED` while the repository license is undecided. Third-party dependencies retain their licenses.
