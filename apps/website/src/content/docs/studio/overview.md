---
title: Nova Studio overview
description: The implemented Electron workbench, its architecture and verified limits.
---

Nova Studio is a standalone Electron workbench using React, strict TypeScript, Vite, Monaco and xterm. It is not a wrapper around an external editor, and the compiler/LSP work without it. If you prefer VS Code, the [Nova Language extension](../../toolchain/vscode/) provides the shared grammar and the same Rust LSP without the Studio shell.

**Verification status:** `npm run typecheck` and `npm run build` pass. The GUI has not yet been exercised by an automated end-to-end test, so this site does **not** publish a screenshot or claim interactive verification that has not happened.

## Architecture

- **Electron 33** with context isolation, sandboxing, no Node integration, CSP, blocked navigation and narrow preload IPC.
- **Monaco** is the editor engine; Nova uses a Monarch grammar and ships a Visual Studio Code Dark+ theme by default, with Gruvbox alternatives.
- **Main process** owns filesystem access, tasks, PTYs, search and the LSP subprocess. Renderer has no Node access.
- **Rust LSP** is started per workspace and reused across open documents.
- **Textual LLVM IR + host Clang** builds native binaries; Studio invokes the actual CLI, it does not simulate build output.

See the repository ADR `docs/adr/0001-ide-architecture.md` for the decision record, candidate risks and measurements.

## Implemented workbench

| Area | Status |
|---|---|
| Explorer, tabs, dirty state, save with external-change confirmation | Implemented |
| Reopens the last workspace on launch | Implemented; missing folders start without a workspace |
| Live workspace watching: external creates/edits refresh the tree; clean tabs reload, dirty buffers are preserved | Implemented |
| Tab actions, Save All, Go to Line, VS Code-style shortcuts | Implemented; no automated GUI interaction test |
| Monaco, Nova syntax highlighting, Dark+ default with Gruvbox Dark/Light/High Contrast | Implemented |
| LSP diagnostics, completion, hover, definition, formatting | Implemented |
| Build / run / check / test tasks with real output | Implemented |
| Problems, output, test panel, integrated terminal | Implemented; terminal is pipe-only without `node-pty` |
| Built-in Docs tab | Implemented; offline quick reference linking to the public docs hub |
| Search, quick open, command palette, settings, Vim mode | Implemented |
| Visual effects and synthesized sound | Experimental; settings toggles, no GUI automation yet |
| Git panel, debugger, test explorer, split groups, multi-window | Planned |
| AI panel | FreeBuff TUI plus streaming chat via Claude or any OpenAI-compatible endpoint; no inline completion or agent edits |

## Visual effects and sound

Settings → **Visual effects & sound** has independent animation and sound toggles plus a volume slider. Preferences persist locally. The current-line lens follows the cursor and Vim motions; typing produces caret sparkles and scrolling adds a sparse trail. Sound is synthesized with Web Audio, not loaded from assets, and is **off by default**. Animated effects respect `prefers-reduced-motion` and the particle layer is pointer-transparent.

This is a visual lens over Monaco's line plate and gutter, not a per-line font-size change: reflowing glyphs would move caret and hit-test geometry. There is no automated interaction test for these effects yet, so the feature is labeled experimental rather than tested.

## Build and run

```sh
cd editor
npm install --cache /tmp/nova-npm-cache
npm run typecheck
npm run build
NOVA_BIN="$PWD/../target/debug/nova" npm run dev
```

Without `node-pty`, the shell terminal reports **pipe mode**. FreeBuff requires a real TTY and will explain that it is unavailable. The AI panel's chat tab does not need a PTY: it streams from a provider you configure, using a key the main process encrypts with the OS credential store.

## Security boundaries

Paths are resolved and checked against the selected workspace, including symlink escape checks. Tasks use argv arrays, not shell concatenation. Workspace trust prompts and extension execution are **not implemented**; do not open hostile workspaces and assume a trust boundary.

## Next

- [Keyboard shortcuts](../shortcuts/)
- [Vim guide](../vim/)
- [FreeBuff limitations](../freebuff/)
