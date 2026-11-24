---
title: Nova TUI
description: A real terminal UI library written in Nova, with runnable output and clearly stated limits.
---

**Status: experimental source library.** Nova TUI is a small ANSI renderer in the repository, not a package-registry dependency. The API can change. The demo is checked and run with the current `nova` binary; keyboard interaction is not implemented.

Nova TUI is a Charm-style toolkit written in Nova itself. It composes panels, lists, tables, progress bars, spinners and screen-control sequences as ordinary `String` values, then writes them with `print`. There is no hidden rendering runtime and no third-party package.

## Run the demo

From the repository root:

```sh
cd examples/tui
../../target/debug/nova build
./target/nova/tui-demo
```

The first frame is a spinner. After it finishes, the same process prints this dashboard (colors omitted here):

```text
  ✦ NOVA TUI
    a Charm-style terminal toolkit written in Nova
──────────────────────────────────────────────────────────────
╭─ RENDERED IN NOVA ─────────────────────────────────────────╮
│ > bordered panels and titles                               │
│   colored 256-color text                                   │
│   tables, badges and rules                                 │
│   progress bars and spinners                               │
│   alternate-screen helpers                                 │
│   no runtime dependencies                                  │
╰────────────────────────────────────────────────────────────╯

────────────────────────────────────────────────
component       status      shape
────────────────────────────────────────────────
Panel           ready       width-aware
Table           ready       list<row>
Progress        ready       integer math
Spinner         ready       10 frames
────────────────────────────────────────────────

native build  ██████████████████████████████████ 100%
docs coverage ███████████████████████████░░░░░░░ 80%
raw input     ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ 0%

   TESTED   EXPERIMENTAL   ANSI 256 · UTF-8 box drawing

  Press Ctrl-C to exit; keyboard events are not implemented.
```

## Import it

The library lives at `libraries/nova-tui/src/tui.nova`. Nova has no registry or installed-package resolution yet, so import it by path:

```nova
import std.io { println }
import "../../../libraries/nova-tui/src/tui.nova" as tui

fn main() {
    let colors = tui.theme()
    let card = tui.panel("Build", ["compile: ok", "link: ok"], 44, colors)
    println(card.render())
}
```

The path is relative to the importing source file, not the shell's working directory. Use the [demo source](https://github.com/jd4rider/nova/blob/main/examples/tui/src/main.nova) as the working reference.

## API

All renderers return a `String`; none of them writes to stdout. That makes layouts composable and testable without a terminal.

| Function | Result |
|---|---|
| `style(fg, bg, bold, dim)` | A 256-color style. Use `-1` for no color. |
| `paint(style, text)` | Text wrapped in ANSI SGR codes, with a reset. |
| `theme()` | The default accent, muted, text, success, warning and danger palette. |
| `panel(title, lines, width, theme)` | A bordered panel with a title and `List<String>` body. |
| `list_panel(title, items, selected, width, theme)` | A panel with a `>` marker on one row. |
| `row(cells)` / `table(headers, rows, widths, theme)` | A fixed-width table with one `List<Int>` width per column. |
| `progress(label, value, total, width, theme)` | A percentage progress bar. |
| `spinner_frame(index)` / `spinner_line(label, index, theme)` | One deterministic frame from a ten-frame spinner. |
| `badge(label, style)` | A filled inline label. |
| `banner(title, subtitle, width, theme)` / `rule(label, width, style)` | Headers and horizontal rules. |
| `clear()`, `home()`, `move_to(row, column)` | Cursor/screen escape strings. |
| `enter_alternate()`, `leave_alternate()`, `hide_cursor()`, `show_cursor()` | Alternate-screen and cursor helpers. |
| `sleep_ms(milliseconds)`, `flush()` | Scalar FFI calls to `usleep` and `fflush` for timed output. |

`panel`, `table` and `progress` have `.render()` methods. The method bodies that need multiple statements use explicit `;` separators because of the current parser behavior inside `impl` blocks.

## How it works

`Panel` calculates a visible width, pads each line with spaces and emits UTF-8 box-drawing characters. `Table` pads each cell to its declared width and draws separators from the same width sum. `Progress` uses integer arithmetic, clamps the filled portion, and guards division by zero. `paint` only prepends ANSI when a style is actually set, so plain output stays plain.

The spinner is the only timed behavior. `examples/tui` prints a carriage return, flushes, sleeps 70 ms and advances the frame. There is no scheduler or event loop behind it.

## Limits

These are current limits, not roadmap promises:

- **No raw-mode input.** Nova has no terminal API, key events, mouse events or signal handling. `Ctrl-C` is handled by the terminal, not by this library. The alternate-screen helpers only emit escape sequences.
- **No Unicode display width.** String padding uses UTF-8 byte length. Box glyphs, the spinner and the demo labels are single-column, but arbitrary CJK text or emoji will misalign.
- **No clipping or wrapping.** Content wider than its column or panel will break the layout. There is no substring or general string-slicing API to truncate it.
- **POSIX-oriented.** The timed demo uses `usleep` and `fflush` through scalar C FFI. It was checked on macOS. A Windows port needs a platform-specific timer.
- **No capability detection.** The output assumes a terminal with 256-color ANSI, UTF-8 and carriage-return support.
- **No package install.** Path imports are the current dependency mechanism. There is no registry, version resolution or lockfile entry for this library.
- **No stable layout engine.** Widths are explicit integers. There are no constraints, flex layout, scroll regions or resize events.

The [standard library page](../../stdlib/) records the wider API surface that does and does not exist. If you need an interactive TUI today, the input side requires a separate C shim and terminal raw-mode work that Nova does not expose yet.
