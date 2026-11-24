# nova-tui

A small ANSI terminal UI library written in Nova. It renders panels, lists, tables, progress bars, spinners and badges as strings, then writes them with `print`.

The library has no runtime package dependencies. It uses `usleep` and `fflush` from the C runtime through Nova's scalar FFI for the spinner; everything else is Nova.

```sh
cd examples/tui
../../target/debug/nova run
```

## Scope

- 256-color ANSI, UTF-8 box drawing and a terminal that understands carriage returns.
- String padding is byte-based. The included components assume single-column text.
- No raw-mode input, key events, mouse events, alternate scroll regions, Unicode display-width calculation or Windows terminal layer.
- `style`, `paint`, `panel`, `list_panel`, `table`, `progress`, `banner` and the screen-control functions return `String` values, so they can be composed or tested without a terminal.

See `apps/website/src/content/docs/libraries/tui.md` for the full guide.
