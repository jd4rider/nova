---
title: Nova Terminal
description: A native terminal emulator written in Nova — pty tabs, ANSI colour, scrollback, a command palette and prompt blocks.
---

**Status: experimental, macOS only.** `examples/nova-term` is a standalone terminal emulator written in Nova on [nova-gui](../../libraries/gui/). The pty lives in the host, the escape-sequence parser lives in `libraries/nova-term`, and the window is drawn cell by cell by Nova itself. There is no Electron, no web view and no ncurses.

## Run it

```sh
cargo build -p nova-cli
cd examples/nova-term
../../target/debug/nova build
NOVA_TERM_ROOT="$PWD" ./target/nova/nova-term
```

| Variable | Meaning |
|---|---|
| `NOVA_TERM_ROOT` | Folder new tabs start in. Defaults to the working directory. |
| `NOVA_TERM_SCRIPT` | Runs a scripted UI test, then exits (see [Tests](#tests)). |
| `NOVA_TERM_SHOT` | Writes the first frame to a PNG and exits. |

## What it does

- **Real pty tabs.** Each tab is its own pty slot in the host (eight available). The shell sees a terminal: it prompts, colours, reads keys and receives `SIGWINCH` when the window resizes.
- **ANSI / VT100 screen.** SGR colours (16, 256 and 24-bit), cursor addressing, erase, insert/delete line and character, scroll regions, save/restore cursor, tab stops, wrapping, the alternate screen (`ESC[?1049h`) and OSC window titles.
- **Scrollback.** 2000 lines per tab, wheel or `PageUp`/`PageDown`, `Cmd+F` searches it and marks each hit on the left edge.
- **Prompt blocks.** A 2px rail marks every prompt line (OSC 133 `A`, or a line beginning with `$`, `❯` or `%`), so commands are easy to find while scrolling.
- **Command palette.** `Cmd+K` filters *new tab, close tab, clear, copy, paste, find, theme, larger/smaller text, scroll to top/bottom*.
- **Themes and zoom.** Dark+ and Gruvbox palettes, 10–22pt text; the grid and the pty resize together.
- **Copy and paste** through the system clipboard.

## Keys

| Action | Key |
|---|---|
| New tab / close tab | `Cmd+T` / `Cmd+W` |
| Select tab | `Cmd+1` … `Cmd+8` |
| Command palette | `Cmd+K` |
| Find in scrollback | `Cmd+F` |
| Copy screen / paste | `Cmd+C` / `Cmd+V` |
| Clear screen | `Cmd+L` |
| Larger / smaller text | `Cmd+` / `Cmd-` |
| Scroll | wheel, `PageUp`, `PageDown` |
| Close palette or search | `Esc` |

## Architecture

```
keyboard ──► key_text ──► pty_write(slot) ──► /bin/sh -c "$SHELL"
                                                    │
screen ◄── parse ◄── pty_poll(slot) ◄────────────────┘
```

- `crates/nova-runtime/src/gui.m` owns a pool of eight ptys: `nova_pty_open`, `nova_pty_poll`, `nova_pty_write`, `nova_pty_write_text`, `nova_pty_resize`, `nova_pty_close`, `nova_pty_status`. Each slot has a reader thread appending to a locked buffer; Nova polls it once per frame.
- `libraries/nova-term` is the emulator: a ring of rows (a linefeed copies one row, not the grid), a ring of scrollback lines, a VT parser and a renderer that batches same-colour runs into single draw calls.
- The app is immediate mode: poll every live pty, feed the bytes, draw the grid, poll one event.

### The nova-term library

```nova
import "../libraries/nova-term/src/term.nova" as vt

var t = vt.tm_new(120, 30)
vt.tm_feed(t, gui.pty_poll(0))
vt.tm_draw(t, 16, 8, 1200, 640, 13, vt.tm_palette_dark())
```

`tm_feed` mutates the terminal in place; `tm_resize`, `tm_scroll`, `tm_all_text`, `tm_find`, `tm_cell_at`, `tm_key_text` and `tm_palette_gruvbox()` cover the rest. Nova parameters are immutable, so the library keeps its state in shared lists rather than struct fields — worth knowing before extending it.

The parser needed raw Unicode scalars, so this work also added **`String.chars()`** to the language: unlike `graphemes()`, which glues ANSI sequences onto the following cluster for styled-text measurement, `chars()` returns one element per scalar and keeps escape bytes intact.

## Tests

```sh
cd examples/nova-term
sh tests/term-ui.sh
```

The harness feeds synthetic ANSI through the real parser and drives the real pty: colour indices, cursor addressing, alternate-screen restore, 256-colour, `pwd` through a shell, tab create/close and OSC titles. It prints `passed n, failed 0`.

## Install as an application

```sh
cd examples/nova-term
./install-macos.sh
```

Assembles `Nova Terminal.app` under `dist/`, copies it to `/Applications` (or `~/Applications`) and writes a `nova-term` launcher to `~/.local/bin` that opens the folder you run it from.

## Honest gaps

- **No splits or windows.** Tabs only; one window.
- **No mouse reporting.** Full-screen programs that expect mouse events fall back to the keyboard.
- **No DSR replies** (`ESC[6n`), and no character sets beyond UTF-8.
- **No reflow on resize:** a narrower grid wraps new output, older lines keep their original break.
- **Scrollback is plain text**, so scrolled-off colour is lost.

Compared with Warp it is a fraction of the feature set and has no AI. Compared with Ghostty it is not in the same performance class: Nova draws text through CoreText and re-renders in Nova code, whereas Ghostty uses Metal. What it does prove is that the language, the host and `nova-gui` are enough to build a working terminal.
