# nova-tui

An interactive ANSI terminal UI toolkit written in Nova, plus a Bubble Tea-style runtime. Renderers build panels, lists, tables, progress bars, text wrapping and screen-control sequences as strings; `bubble.nova` owns the terminal and drives a model/update/view loop.

The library has no package dependencies. The native runtime supplies raw-mode input, key polling, terminal size, mouse and bracketed-paste events, polled resize events, Unicode display-width/cluster/clipping/wrapping, HTTP and minimal JSON extraction; the rest is Nova.

```sh
cd examples/tui
../../target/debug/nova build
./target/nova/tui-demo

cd examples/bible
../../target/debug/nova build
./target/nova/nova-bible
```

The dashboard animates a tick and reacts to arrows, the wheel, clicks, paste and resize. The Bible app has pickers for translation, book, chapter and verse, live book search, whole-chapter loading, mouse focus and wheel scrolling, and fetches from `bible-api.com` when you press Enter. Both print one static frame when stdin or stdout is not a TTY.

## The runtime

```nova
import "../../../libraries/nova-tui/src/bubble.nova" as bubble

fn update(model: Model, event: bubble.Event) -> (Model, bubble.Cmd) { ... }
fn view(model: Model) -> String { ... }

fn main() {
    bubble.run(initial(), update, view, bubble.settings(160))
}
```

- `bubble.Event`: `KeyEvent(Int)`, `MouseEvent(code, x, y)`, `PasteEvent(String)`, `ResizeEvent(width, height)`, `TickEvent`.
- `bubble.Cmd`: `IdleCmd`, `QuitCmd`, `AfterCmd(milliseconds)`.
- `bubble.settings(tick_ms)` enables mouse, paste and the alternate screen; `bubble.plain()` disables them.
- Commands are synchronous: `update` finishes its work before the next frame. There is no async runtime.
- Variant constructors are global in this Nova build, so Bubble's variants carry their enum name.

## Scope

- 256-color ANSI, UTF-8 box drawing, ANSI-aware width, grapheme clusters and word wrapping.
- Raw-mode input for arrows, Home/End, Page Up/Down, Insert/Delete, Tab, Enter, Escape and UTF-8 characters.
- SGR/X10 mouse press, release, wheel and drag events with cell coordinates.
- Bracketed paste buffered as a `String` plus a codepoint iterator, and polled resize events.
- `http_get`, `json_string` and `char_string` builtins over `curl` and a small JSON string extractor.
- POSIX verified on macOS. The Windows console branch exists but is untested.
- No focus events, mouse modifiers, full UAX #29 segmentation or registry packaging.
- No cursor/selection text editing; the Bible demo appends and backspaces.

See `apps/website/src/content/docs/libraries/tui.md` for the full guide.
