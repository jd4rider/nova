---
title: Studio shortcuts
description: Implemented menu accelerators and editor shortcuts.
---

Use **Cmd** on macOS and **Ctrl** on other platforms.

| Action | Shortcut |
|---|---|
| Open folder | `Cmd/Ctrl+O` |
| New file | `Cmd/Ctrl+N` |
| Save | `Cmd/Ctrl+S` |
| Save all | `Cmd/Ctrl+Alt+S` |
| Close tab | `Cmd/Ctrl+W` |
| Quick open | `Cmd/Ctrl+P` |
| Command palette | `Cmd/Ctrl+Shift+P` |
| Find in files | `Cmd/Ctrl+Shift+F` |
| Toggle sidebar | `Cmd/Ctrl+B` |
| Toggle bottom panel | `Cmd/Ctrl+`` |
| Explorer view | `Cmd/Ctrl+Shift+E` |
| Go to line | `Ctrl+G` |
| Build | `Cmd/Ctrl+Shift+B` |
| Run | `F5` |
| Format document | `Shift+Alt+F` |
| Cancel task | Run menu |
| Settings | View → Settings |
| Toggle Vim mode | View → Toggle Vim Mode |
| Documentation panel | View → Documentation |
| Toggle visual effects | View → Toggle Visual Effects |
| Toggle sound effects | View → Toggle Sound Effects |

Monaco supplies its normal multi-cursor, find/replace, undo, folding, bracket matching and snippet keybindings. The app-level listener handles quick open, palette, save and the panel toggle even when focus is in Monaco.

## Quick open and palette

Up/Down moves the selection; Enter opens or runs it; Escape closes. Quick open searches filenames, not file contents. Workspace content search is in the Search activity view and uses ripgrep.

## Terminal

The bottom panel has Shell and FreeBuff buttons. FreeBuff requires both the `freebuff` executable and a PTY. Without `node-pty`, the shell is clearly labeled pipe mode. The AI panel has its own FreeBuff tab that runs the same CLI, plus a chat tab that streams from Claude or an OpenAI-compatible endpoint.

## Caveat

Shortcuts are documented from the implemented menu and renderer handlers. Automated keyboard interaction tests are not present yet.
