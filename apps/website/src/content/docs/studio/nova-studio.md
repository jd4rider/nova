---
title: Nova Studio in Nova
description: The workbench rebuilt in Nova on nova-gui — same layout and palette, with the gaps listed honestly.
---

**Status: experimental, macOS only.** `examples/nova-studio` is Nova Studio rewritten in Nova. It draws the workbench itself through [nova-gui](../../libraries/gui/): no Electron, no React, no Monaco, no web view. The Electron application described in the [overview](../overview/) still exists and is unchanged; this page documents the Nova implementation.

## Run it

```sh
cargo build -p nova-cli
cd examples/nova-studio
../../target/debug/nova build
NOVA_BIN="$(cd ../.. && pwd)/target/debug/nova" \
NOVA_STUDIO_ROOT="$(cd ../.. && pwd)" \
./target/nova/nova-studio
```

| Variable | Meaning |
|---|---|
| `NOVA_STUDIO_ROOT` | Workspace folder shown in the explorer. Defaults to the working directory. |
| `NOVA_STUDIO_OPEN` | File to open in a tab at launch. |
| `NOVA_BIN` | Compiler used by Build, Check and Run. Defaults to `nova` on `PATH`. |
| `NOVA_STUDIO_SHOT` | Writes the first frame to a PNG and exits. Used for verification. |
| `NOVA_STUDIO_TASK` | Runs `build`, `check` or `test` at launch. Used for verification. |

The **File** menu carries **Open File…** (`⌘O`) and **Open Folder…** (`⇧⌘O`), both opening the native macOS panel; the workspace therefore comes from the picker when `NOVA_STUDIO_ROOT` is unset. Point `NOVA_BIN` at a freshly built compiler: a stale `nova` on `PATH` will produce real compiler errors in the output panel.

## Install as an application

```sh
cd examples/nova-studio
./install-macos.sh
```

This builds the workbench, renders the icon from `icon.html` with headless Chrome, assembles `Nova Studio (Nova).app` under `dist/`, copies it to `/Applications` (or `~/Applications` when that is not writable) and writes a `nova-studio` launcher to `~/.local/bin`.

The bundle name carries the `(Nova)` suffix so it sits next to the Electron app instead of replacing it. Install over the Electron one with:

```sh
APP_NAME="Nova Studio" ./install-macos.sh
```

Double-clicking the icon opens the workbench on the folder the app was installed from — the installer bakes that path into the bundle, along with the compiler it was built against. `NOVA_STUDIO_ROOT` and `NOVA_BIN` still win when set.

The `nova-studio` command opens whatever folder you run it from, which is the convenient way to use it on a project outside this repository:

```sh
cd ~/code/my-project
nova-studio
```

The bundle is unsigned. A locally built app is not quarantined, but copying it to another machine will require right-click → Open the first time.

## What matches the Electron Studio

- **Layout and palette.** The same 35px title bar with a centred command pill, 48px activity bar, 300px explorer, 35px tabs, 22px breadcrumbs, 240px bottom panel and 22px status bar. The same Dark+ colours, byte for byte from `styles.css`, plus Gruvbox as an alternative.
- **Explorer.** Real directory listing of the workspace, directories first, lazy expansion, click to open a file, active file highlighted.
- **Tabs.** Dirty state, close button, active-tab accent, per-tab scroll and caret position.
- **Editor.** Line numbers, current-line highlight, indent guides, bracket-pair matching under the caret, a gutter marker on declarations, the enclosing `fn` marked along the left edge and named in the breadcrumb, and a minimap down the right edge. Syntax highlighting follows Dark+ scopes: block and line comments, strings with escapes, numbers, declarations, control flow, types and function names.
- **Selection and clipboard.** Drag to select, `⇧`-click to extend, `⌘A` selects all, and `⌘C` / `⌘X` / `⌘V` go through the system clipboard. Typing or `Delete` replaces the selection.
- **Find and replace.** `⌘F` opens an in-editor bar: type to highlight every match, `Enter`/`⇧Enter` to step, `Tab` to the replace field, `⌘Enter` to replace all.
- **Multi-cursor.** `⌘D` drops another caret on the next occurrence of the word under the caret (or of the selection); typing and `Backspace` apply at every caret and each caret is drawn in place. `Esc` or a click returns to one caret.
- **Terminal panel.** Build and Check no longer block: the command runs on a host thread and its output streams into a TERMINAL tab in the bottom panel while the window stays responsive. The host allocates a real PTY (`posix_openpt` + `fork`), so click the panel and type — keystrokes go to the shell as they are sent, `Esc` returns to the editor.
- **Folding.** Declaration lines (`fn`, `struct`, `impl`, `enum`) get a gutter marker; click it to collapse the block, which shows `⋯ n lines`, and click the `+` to reopen.
- **Vim mode.** On by default and toggled from Settings or the palette. NORMAL, INSERT and VISUAL, with `h j k l`, `w b e`, `0 $`, `gg G`, `x`, `dd`, `yy`, `p`, `u`, `i a I A o O`, `v`. The status bar shows the mode and NORMAL draws a block caret.
- **Zoom and effects.** `⌘+`/`⌘-`/`⌘0` resize the editor; typing throws a short-lived spark trail at the caret.
- **Icons.** Material-style file and folder icons, coloured by type, and vector activity-bar icons, all drawn with line primitives because nova-gui ships no icon font.
- **Save.** `⌘S` writes the buffer and clears the dirty dot; `⇧⌘S` saves every dirty tab.
- **Go to file.** `⌘P` lists every file in the workspace (skipping `target`, `node_modules`, `dist`, `out`, `release`) and filters as you type.
- **Command palette.** `⇧⌘P` with the same commands the Electron menu exposes, including Build, Run, Check, toggle sidebar and panel, documentation, settings and quit.
- **Tasks.** Build, Check and Run shell out to the real compiler in the workspace and print its output verbatim with the exit code. Run builds first, then opens `target/nova/<package>` in Terminal, so a TUI program gets a real terminal instead of a frozen window.
- **Panels.** Problems, Output and the built-in Docs quick reference.
- **Status bar.** Workspace, last status message, line/column, encoding, theme and tool name.
- **Settings.** `⌘,` switches between Dark+ and Gruvbox and changes the editor font size, applied immediately.
- **Welcome screen** when no tab is open, listing the shortcuts.

## What is not ported yet

These are gaps, not hidden behaviour:

- **Split editor.** `⌘\` (or *Toggle split editor* in the palette) shows two panes side by side on the same workbench; each pane remembers its own tab, the unfocused pane renders dimmed without a caret, and clicking a pane focuses it. Panes hide below 520px of width.
- **Go to definition.** `F12` (or `⌘`-click a symbol, or *Go to definition*) jumps to the `fn`, `struct`, `enum` or `impl` that declares the word under the caret — in the current file, then across every `.nova` file in the workspace, opening the file if needed. A status message reports what it found.
- **No language server.** There is no completions, hover or inline diagnostics; the Problems panel is intentionally empty. The Rust LSP is a separate process that speaks JSON-RPC over stdio, and the host has no asynchronous channel.
- **No terminal sessions.** The PTY panel runs one shell at a time and keeps no scrollback beyond the visible buffer; Run still launches Terminal for TUI programs.
- **No AI panel.** No Copilot, no FreeBuff chat, no diff review. The host cannot stream a network response.
- **No search across files.** Quick open and the palette exist; workspace search does not.
- **No multi-cursor.** Selection, clipboard, find/replace and folding exist; multiple simultaneous carets do not.
- **Folding is declaration-level only.** `fn`, `struct`, `impl` and `enum` blocks fold; arbitrary brace regions do not.
- **Unclosed dirty tabs are discarded** on close instead of prompting.
- **Settings are not persisted** between launches.
- **Plugin contributions are commands only.** See [Plugins](../plugins/) for the manifest format and its limits.
- **Closing with ⌘Q leaves the window state unsaved**; there is no session restore.

## Shortcuts

| Action | Shortcut |
|---|---|
| Open file / folder | `⌘O` / `⇧⌘O` |
| Save / Save all | `⌘S` / `⇧⌘S` |
| Zoom in / out / reset | `⌘+` / `⌘-` / `⌘0` |
| Find / replace | `⌘F` |
| Copy / cut / paste | `⌘C` / `⌘X` / `⌘V` |
| Select all | `⌘A` |
| Select next occurrence | `⌘D` |
| Fold or unfold a block | click the gutter marker |
| Build / Check | `⌘B` / `⇧⌘B` |
| Build and run in Terminal | `⌘R` |
| Close tab | `⌘W` |
| Go to file | `⌘P` |
| Command palette | `⇧⌘P` |
| Toggle sidebar | `⇧⌘E` |
| Toggle bottom panel | ``⌘` `` |
| Settings | `⌘,` |
| Quit | `⌘Q` |

Overlays close with `Esc`; `↑`/`↓` move the selection and `Enter` commits it.

## How it is verified

- `nova check` passes on the whole program.
- The workbench was driven with `NOVA_STUDIO_SHOT` and the resulting frames were inspected: chrome, explorer, welcome screen, an open Nova file with highlighting, and the Output panel after a real `check` task.
- The process and file helpers behind Build were checked directly: a command's exit code and output cross back to Nova, and a write/read round-trip preserves content.

`tests/studio-ui.sh` drives the workbench with synthetic clicks, drags and keystrokes: it writes a script of events, the app feeds each one through the same `click`/`drag`/`key` handlers the window uses, and the script asserts on the resulting model (selection text, clipboard contents, find match count, fold count, visible row count, caret count, line contents and streamed terminal output). It runs ten cases — select, find, fold, multi-cursor, streamed build output, go-to-definition, split panes, typed terminal input, emulator cells/colours and cross-file references — and prints `passed n, failed 0`. What it does not cover is AppKit's own event delivery — real mouse and keyboard capture is still exercised by hand.
## Next

- [nova-gui library guide](../../libraries/gui/)
- [Studio overview (Electron)](../overview/)
- [Keyboard shortcuts (Electron)](../shortcuts/)
