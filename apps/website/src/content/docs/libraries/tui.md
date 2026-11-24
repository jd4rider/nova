---
title: Nova TUI
description: A terminal UI toolkit and Elm-style runtime written in Nova, with mouse, paste and resize events, Unicode layout, HTTP/JSON builtins and a Bible reader.
---

**Status: experimental source library.** Nova TUI is a small terminal toolkit in the repository, not a package-registry dependency. The API can change. Both demos are built and run in a PTY with the current `nova` binary.

Nova TUI is a Charm-style toolkit written in Nova itself. Components compose panels, lists, tables, progress bars, text wrapping and screen-control sequences as ordinary `String` values. `bubble.nova` adds a Bubble Tea-style runtime: the app supplies a model, an update function and a view function, and the runtime owns the terminal. There is no third-party package.

## Run the demos

From the repository root:

```sh
cd examples/tui
../../target/debug/nova build
./target/nova/tui-demo
```

```sh
cd examples/bible
../../target/debug/nova build
./target/nova/nova-bible
```

The dashboard animates a tick-driven spinner, and the Bible app fetches passages from `bible-api.com`. When stdin or stdout is not a TTY, either binary prints one frame and exits, which is what automated checks capture.

## The Bubble-style runtime

`libraries/nova-tui/src/bubble.nova` is the Elm architecture with the terminal plumbing attached:

```nova
import std.io { println }
import "../../../libraries/nova-tui/src/bubble.nova" as bubble

struct Model {
    count: Int
}

fn update(model: Model, event: bubble.Event) -> (Model, bubble.Cmd) {
    return match event {
        bubble.Event.KeyEvent(code) => on_key(model, code),
        bubble.Event.MouseEvent(code, x, y) => on_mouse(model, code, x, y),
        bubble.Event.PasteEvent(text) => on_paste(model, text),
        bubble.Event.ResizeEvent(width, height) => on_resize(model, width, height),
        bubble.Event.TickEvent => (Model { count: model.count + 1 }, bubble.Cmd.AfterCmd(250))
    }
}

fn view(model: Model) -> String {
    return "ticks: ${model.count}"
}

fn main() {
    bubble.run(Model { count: 0 }, update, view, bubble.settings(250))
}
```

| Piece | Meaning |
|---|---|
| `bubble.Event` | `KeyEvent(Int)`, `MouseEvent(code, x, y)`, `PasteEvent(String)`, `ResizeEvent(width, height)`, `TickEvent`. |
| `bubble.Cmd` | `IdleCmd` (block for the next event), `QuitCmd`, `AfterCmd(milliseconds)` (redraw after a delay). |
| `bubble.run(init, update, view, settings)` | Enters raw mode and the alternate screen, redraws on every event, and restores the terminal on exit. |
| `bubble.settings(tick_ms)` | Mouse, bracketed paste and the alternate screen on; `tick_ms` is the initial read timeout (`-1` blocks). |
| `bubble.plain()` | Same, with the protocols off and no alternate screen. |
| `bubble.row_hit(row, first_row, count)` | Maps a mouse row to a list index, or `-1` when outside. |
| `bubble.previous_index` / `next_index` | Clamped list navigation. |
| `bubble.is_quit_key(code)` | `q`, `Q`, `Esc` or Ctrl-C. |

Variant constructors are global in this Nova build, so Bubble's variants carry their enum name (`KeyEvent`, `IdleCmd`) instead of the bare `Key`/`None` used by `tui.Key` and `Option`. Commands run synchronously: `update` performs its work before returning, so a network fetch blocks the frame. There is no goroutine equivalent, no command batching and no independent subscription list.

## The Bible reader

`examples/bible` is a real application of the runtime. It has pickers for translation, book, chapter and verse, live book search, and loads whole chapters as well as single verses. Selecting `John 3` and pressing Enter requests:

```text
https://bible-api.com/john+3?translation=web
```

A verse selection appends `:16`. The response is JSON; the app reads three fields and renders a scrollable passage:

```nova
let body = http_get("https://bible-api.com/" + reference + "?translation=" + version)
let text = json_string(body, "text")
let name = json_string(body, "translation_name")
let verses = count_verses(body)
```

`count_verses` is ordinary Nova: it removes every `"verse"` key and divides the byte difference by the key length, so the verse picker knows how many verses the chapter has. No JSON parser is involved.

Pickers:

| Pane | Options | Notes |
|---|---|---|
| Translation | WEB, KJV, ASV, BBE, Darby, YLT, DRB, OEB | Sent as the `translation` query parameter. |
| Book | all 66 books | A static list of names and chapter counts; no network needed to browse. |
| Chapter | 1 … the book's chapter count | Changing the book clamps the chapter. |
| Verse | `whole`, 1 … the loaded chapter's verse count | `whole` loads the chapter; the count comes from the previous load. |
| Search | any text | Filters all 66 books by name as you type (`lower()` makes it case-insensitive); ↑/↓ move between matches. |
| Mouse | click, wheel | Clicking a picker row focuses that pane; the wheel moves the focused picker or scrolls the passage when the pointer is below them. |

Keys:

- **↑/↓** or **j/k** change the selection in the focused pane
- **←/→** or **Tab/Shift-Tab** switch panes (translation, book, chapter, verse)
- **Enter** loads the current selection
- **c** loads the whole chapter regardless of the verse selection
- **/** searches books: typing filters the list by name, **↑/↓** move between matches, **Enter** loads the chapter, **Esc** cancels
- **:** opens a free-text reference (`john 3:16`, `romans 8:28-30`); **Enter** loads it, **Esc** cancels
- **Page Up/Down**, the **wheel** and **j/k** in the passage scroll it
- **click** a picker row to focus it; the **wheel** moves the focused picker, or scrolls the passage when the pointer is over it
- **paste** fills the free-text reference
- **resize** reflows the pickers and passage
- **Esc** or **Ctrl-C** exits and restores the terminal

`bible-api.com` is free and needs no key. It is rate limited (15 requests per 30 seconds per address), so the app fetches only on Enter. The header names the translation that produced the text, and the World English Bible is public domain.

## Import it

The libraries live under `libraries/nova-tui/src/`. Nova has no registry, so import by path relative to the importing source file:

```nova
import std.io { println }
import "../../../libraries/nova-tui/src/tui.nova" as tui

fn main() {
    let colors = tui.theme()
    let size = tui.terminal_size()
    let card = tui.text_panel(
        "Build",
        "compile: ok | link: ok | terminal: ${size.width}x${size.height}",
        44,
        colors
    )
    println(card.render())
}
```

The path is resolved relative to the importing file, not the shell's working directory. A module imported twice through different relative paths is expanded once.

## Components

Every renderer returns a `String`; none writes to stdout, so layouts are composable and testable without a terminal.

| Function | Result |
|---|---|
| `style(fg, bg, bold, dim)` | A 256-color style. Use `-1` for no color. |
| `paint(style, text)` | Text wrapped in ANSI SGR codes, with a reset. |
| `theme()` | The default accent, muted, text, success, warning and danger palette. |
| `panel(title, lines, width, theme)` | A bordered panel with a title and `List<String>` body. |
| `text_panel(title, text, width, theme)` | A panel that wraps plain text to the inner width. |
| `list_panel(title, items, selected, width, theme)` | A panel with a `>` marker on one row. |
| `row(cells)` / `table(headers, rows, widths, theme)` | A fixed-width table with one `List<Int>` width per column. |
| `progress(label, value, total, width, theme)` | A percentage progress bar. |
| `spinner_frame(index)` / `spinner_line(label, index, theme)` | One deterministic frame from a ten-frame spinner. |
| `badge(label, style)` | A filled inline label. |
| `banner(title, subtitle, width, theme)` / `rule(label, width, style)` | Headers and horizontal rules. |
| `clear()`, `home()`, `move_to(row, column)` | Cursor/screen escape strings. |
| `enter_alternate()`, `leave_alternate()`, `hide_cursor()`, `show_cursor()` | Alternate-screen and cursor helpers. |
| `mouse_on()`, `mouse_off()`, `paste_on()`, `paste_off()` | Enable or disable mouse reporting and bracketed paste. |
| `sleep_ms(milliseconds)`, `flush()` | Scalar FFI calls to `usleep` and `fflush`. |

## Terminal input

| Function | Result |
|---|---|
| `is_tty()` | True when both stdin and stdout are terminals. |
| `terminal_width()` / `terminal_height()` | Current columns/rows, with an 80×24 fallback. |
| `terminal_size()` | A `TerminalSize { width, height }` value. |
| `enter_raw()` / `leave_raw()` | Switch between raw and restored modes; both return success. |
| `read_key(timeout_ms)` | Block for an event, or wait up to the timeout. Returns a Unicode scalar or a key code; `-1` is a timeout, `-2` is EOF. |
| `decode_key(code)` | Convert a `read_key` result into the `Key` enum. |
| `mouse_x()` / `mouse_y()` | 1-based cell coordinates of the most recent mouse event. |
| `paste_len()` / `paste_next()` | Character count and codepoint iterator for the last paste. |
| `paste_text()` | The whole last paste as a `String`, clearing the buffer. |

`Key` covers arrows, Home/End, Page Up/Down, Insert/Delete, Back Tab, Enter, Tab, Backspace, Escape, Quit, Resize, mouse presses and releases, wheel directions, drags, Paste, character input and timeout.

Raw mode disables canonical input, echo and signal generation, so Ctrl-C arrives as code `3`. The runtime registers an `atexit` restore hook; the demo also restores explicitly. An uncatchable kill can still leave the terminal in raw mode.

### Mouse, paste and resize

Enable the protocols before reading and disable them on exit. Event codes are stable integers:

| Code | Meaning |
|---|---|
| `1001`–`1011` | Arrows, Home, End, Page Up/Down, Delete, Insert, Back Tab |
| `1020` | Resize; read `terminal_size()` for the new size |
| `2000`–`2002` | Left, middle and right button press |
| `2003` | Release |
| `2004`/`2005` | Wheel up/down |
| `2006`/`2007` | Wheel left/right |
| `2010`–`2012` | Drag with the left, middle or right button |
| `3000` | Bracketed paste; the text is buffered until `paste_text()` or `paste_next()` consumes it |

Mouse reporting uses SGR coordinates (`ESC [ < b ; x ; y M`) and also understands legacy X10 reports. Coordinates are cells, not pixels; modifier bits are parsed but discarded.

Resize is reported without a signal handler: `read_key` polls the size in 50 ms slices and returns `1020` when it changes, so a blocked read still wakes on a resize.

Bracketed paste reads until `ESC [ 201 ~`, folds CRLF to LF, and keeps the text in a process-lifetime buffer. `paste_text()` is bound to the compiler builtin `term_paste()` because scalar `extern "C"` declarations cannot return a `String`.

## Unicode text

| Method | Result |
|---|---|
| `display_width()` | Terminal columns, ignoring ANSI CSI/OSC sequences. Wide CJK/emoji ranges count as 2; combining marks and zero-width controls count as 0. |
| `clip(width)` | Prefix that fits in `width` columns, never splitting a grapheme cluster. ANSI sequences are preserved and a reset is appended when styles were present. |
| `wrap(width)` | `List<String>` split at word boundaries when possible, hard-breaking long words. |
| `graphemes()` | `List<String>` of extended grapheme clusters, with ANSI escapes kept attached. |
| `replace(from, to)` | Replaces every occurrence of a substring. |

Clusters cover combining marks, variation selectors, emoji skin tones, keycaps, regional-indicator flags and ZWJ sequences such as `👨‍👩‍👧`, which measure as one two-column cluster rather than five scalars.

## HTTP, JSON and characters

These builtins are implemented in the native runtime, so a program needs no package:

| Builtin | Result |
|---|---|
| `http_get(url)` | Response body, or `""` on failure. Runs the host `curl` directly (no shell), with a 20 s timeout. POSIX only. Used by `examples/bible`. |
| `json_string(json, key)` | Decodes the last `"key": "value"` string in the document and its JSON escapes. Not a parser: numbers, arrays, objects and nesting are not decoded. |
| `char_string(code)` | A one-character `String` from a Unicode scalar, for text entry from key codes. |

`http_get` blocks until the request finishes. There is no connection reuse, header control, TLS policy, streaming or async request, and the Windows build returns an empty string.

## Verified behavior

`cargo test --workspace` covers the compiler, including the new qualified-name resolution. The interactive paths were exercised through real PTYs on macOS:

- resize to 100×30 and 90×30 → the status line reports the new size
- wheel, click and bracketed paste → selection moves, coordinates and pasted character count are reported
- `q` and `Esc` → the process exits `0` and restores the terminal
- the Bible app fetched `John 3:16` from `bible-api.com`, rendered the passage, and exited on `Esc`

The non-TTY path is a separate static run for both examples.

## Limits

- **Cluster segmentation is an approximation.** No full UAX #29 tables, emoji tag sequences, Indic conjunct rules or East Asian ambiguous-width locale rules.
- **Input is a key stream plus a few protocols.** Focus events, Alt combinations, kitty keyboard protocols and mouse modifier bits are not delivered.
- **Mouse coordinates are unvalidated cells.** The runtime reports what the terminal sent; applications map rows onto components with helpers such as `row_hit`.
- **Paste is buffered text.** A paste completes when the terminator arrives (2 s per-byte timeout), CRLF is folded to LF, and the next paste replaces the buffer.
- **Resize is polled** at up to 50 ms latency and reports only the size, not a diff.
- **Commands are synchronous.** A fetch blocks rendering; there is no async runtime, command batching or subscription model.
- **Text input is minimal.** There is no cursor, selection, word motion or input method support; the Bible app appends characters and backspaces one cluster.
- **JSON extraction is not a parser.** `json_string` handles one top-level string field; it cannot read numbers, arrays or nested objects.
- **HTTP depends on curl.** POSIX only, no Windows path, no TLS configuration of its own.
- **Variant constructors are global.** Two enums cannot share a variant name, which is why Bubble prefixes its variants.
- **Windows is untested.** The console branch exists but the PTY evidence here is POSIX.
- **No package install.** Path imports are the current dependency mechanism.
