---
title: Nova GUI
description: A native immediate-mode GUI written in Nova, with a CoreGraphics rasterizer, CoreText glyphs and a polled event loop.
---

**Status: experimental, macOS only.** `libraries/nova-gui` gives a Nova program a real window, text and input. It is the toolkit the [Nova Studio rewrite](../../studio/nova-studio/) is built on, and it was written from the ground up because Nova had no graphical layer at all: the toolchain could print to a terminal or compile to WebAssembly, and nothing in between.

## How it fits together

Three pieces, no external dependencies:

- **`runtime/gui.m`** — the macOS host. It creates one `NSWindow`, owns a `CGBitmapContext` framebuffer, draws glyphs with CoreText, queues input events, composites an optional offscreen Metal frame and writes PNGs for tests.
- **`libraries/nova-gui/src/gui.nova`** — the Nova API. `extern "C"` declarations plus a theme, an `Event` struct and geometry helpers.
- **The compiler** — `nova-codegen-llvm` compiles and links the host only when a program's resolved externs mention `nova_gui_`, `nova_fs_`, `nova_process_` or `nova_env`, and links `Cocoa`, `CoreText`, `CoreGraphics` and `ImageIO`. A program that never touches the GUI does not pay for AppKit, and the WebAssembly target is untouched.

The host occupies the same position the terminal bindings occupy: Nova has no way to pass a function pointer back into C, so input is **polled** rather than delivered through callbacks. A program is a loop.

```nova
import "../../libraries/nova-gui/src/gui.nova" as gui

fn main() {
    if !gui.open("Hello", 640, 360) {
        return
    }
    let theme = gui.dark()
    var running = true
    while running {
        gui.clear(theme.bg)
        gui.text(24, 24, "Hello, Nova", 22, theme.fg_bright)
        gui.mono(24, 60, "drawn by CoreText", 13, theme.aqua)
        gui.present()

        let event = gui.next_event()
        if event.kind == gui.event_close() || event.code == 113 {
            running = false
        }
        gui.sleep(8)
    }
    gui.close()
}
```

## Coordinates and pixels

Everything is immediate mode: a frame is whatever the loop draws between `begin` and `present`, and there is no retained widget tree to reconcile.

- Coordinates are **points from the top-left**, like CSS or a canvas. The host flips the CoreGraphics transform once, so drawing code never thinks about AppKit's bottom-left origin.
- The backing bitmap is sized by the display's scale factor, so on a Retina screen the PNG above is 1280×640 for a 640×320 window and text stays sharp. Callers never pass a pixel ratio.
- Colours are packed `0xRRGGBB` integers built by `gui.rgb(r, g, b)`. `gui.rect_alpha` adds a 0–255 alpha for overlay scrims.

## Metal frame compositing

A GUI app can pair nova-gui with `libraries/nova-render`: create an offscreen session with `metal_offscreen`, fill its bounded draw list, render at the backing scale and call `gui.metal_frame(x, y, w, h)`. The frame is composited into the same sRGB bitmap as the chrome, so `gui.save_png` includes it and the app's own grid or labels can be drawn on top.

The handoff is an internal weak symbol, not a new Nova type: GUI-only programs still link without Metal, and no pointer or callback crosses the FFI. The UI itself remains CPU-rasterized; only the viewport comes from the GPU.

## API

| Area | Calls |
|---|---|
| Window | `open(title, w, h)`, `close()`, `is_open()`, `width()`, `height()`, `set_title(text)` |
| Pickers | `pick_file()`, `pick_folder()` — native `NSOpenPanel`; `save_panel(name, dir)` — native `NSSavePanel`; all return `""` on cancel |
| Clipboard | `clipboard()`, `set_clipboard(text)` — the system pasteboard |
| Terminal | `term_start(cmd, dir)`, `term_poll()`, `term_running()`, `term_clear()` — a real PTY (libc `posix_openpt` + `fork`) read by a host thread, so a child program sees a terminal |
| Terminal input | `term_send(code)` — write a byte (`13` Enter, `127` Backspace) to the running PTY |
| Terminal pool | `pty_open(slot, cmd, dir, cols, rows)`, `pty_poll(slot)`, `pty_running(slot)`, `pty_write(slot, code)`, `pty_write_text(slot, text)`, `pty_resize(slot, cols, rows)`, `pty_status(slot)`, `pty_close(slot)` — up to eight ptys, each a real terminal with its own window size |
| Frame | `begin()`, `clear(color)`, `present()`, `save_png(path)`, `sleep(ms)`, `scale()`, `metal_frame(x, y, w, h)` |
| Shapes | `rect`, `round_rect`, `stroke`, `line`, `rect_alpha` |
| Text | `text` (system font), `mono` (monospaced), `text_fmt` (sans/serif/mono with bold/italic/underline bits), `text_width`, `text_fmt_width`, `mono_width`, `advance(size)`, `font_sans/font_serif/font_mono`, `style_bold/style_italic/style_underline` |
| Mono font | `mono_font()`, `set_mono_font(family)`, `mono_fonts()` — the family defaults to an installed Nerd Font (`NOVA_MONO_FONT` overrides it), `mono_fonts()` lists every installed monospaced family, and setting one invalidates the host's glyph cache |
| Clipping | `clip(x, y, w, h)`, `clip_reset()` |
| Input | `next_event()` returning `Event { kind, code, x, y, mods, delta }`; `typed_text()` and `marked_text()` for composed and in-progress IME text |
| Events | `event_key`, `event_mouse_down`, `event_mouse_up`, `event_mouse_move`, `event_drag`, `event_scroll`, `event_resize`, `event_close` |
| Keys | `key_enter`, `key_tab`, `key_escape`, `key_backspace`, `key_delete`, `key_left`, `key_right`, `key_up`, `key_down`, `key_home`, `key_end`, `key_page_up`, `key_page_down` |
| Modifiers | `mod_command`, `mod_shift`, `mod_option`, `mod_control` |
| Theme | `dark()` (VS Code Dark+), `light()`, `gruvbox()`, `rgb`, `white` |
| Workspace | `read`, `write`, `exists`, `is_dir`, `list`, `cwd`, `env`, `run`, `exit_status`, `rename`, `delete_file`, `sha256` |

`present()` pumps the platform queue with `nextEventMatchingMask:` and sends each event, then blits the framebuffer into the view. Printable keys arrive as their Unicode scalar; everything else uses the codes above, which match `nova-tui` where the two overlap.

**An idle loop must still call `pump()`.** `pump` drains events without redrawing; `present` drains and redraws. A program that only touches AppKit when it draws stops answering the window server the moment it goes idle, and its layer transaction never commits — macOS then shows a blank window with a spinning cursor. The pattern is:

```nova
while running {
    if dirty {
        gui.begin()
        draw()
        gui.present()
        dirty = false
    } else {
        gui.pump()
        gui.sleep(8)
    }
    let event = gui.next_event()
    // handle one event, set dirty
}
```

## Workspace access and tasks

Nova's standard library is still a placeholder — `std.io` and `std.core` only — so a GUI program that edits a project needs files and processes from somewhere. The macOS host provides them:

- `read`/`write`/`exists`/`is_dir`/`list`/`cwd` operate on paths as given. `list` returns one `d<TAB>name` or `f<TAB>name` line per child, directories first, with hidden files skipped.
- `run(command, directory)` runs a shell command and returns combined output; `exit_status()` returns the exit code.
- `env(name)` reads an environment variable, and `cwd()` reports the working directory.

These are host functions, not a sandbox: the caller decides which paths are allowed. They are linked only into programs that use nova-gui.

## Text across the FFI boundary

Drawing text and reading files both need strings in and out of C, so `extern "C"` now accepts `String` alongside `Bool`, `Int`, `Float`, `Char` and `Unit`. On both sides a `String` is `{ const char *ptr; int64_t len; }` — a borrowed view, never freed by the host. Lists, maps and user structs still cannot cross. The [language tour](../../language/tour/) documents the boundary.

## IME and marked text

The macOS host implements `NSTextInputClient`, so a Japanese, Chinese or Korean input source can compose in front of a Nova app. `gui.marked_text()` returns the in-progress composition; `insertText:` clears it and delivers one `event_text`; `cancelOperation:` (Escape) clears it and delivers `key_escape`. The host owns the marked string; the app decides what to paint and when to insert it into its own model.

[Nova Writer](../../apps/nova-writer/) uses this to paint the preedit inline through a temporary layout, commit exactly once and cancel on Escape. `writer/tests/ime.sh` drives the same two entry points with `NOVA_GUI_IME_PROBE` and asserts both paths.

## Verifying a frame

GUI programs are hard to check from a terminal, so the host can write its framebuffer to a PNG:

```nova
gui.present()
gui.save_png("dist/frame.png")
```

`examples/gui-smoke` is a forty-line program that draws a heading, a panel, a button and a measured string, saves the frame and exits. Diffing that PNG, or reading it back, verifies the rasterizer without a screen recorder or accessibility permissions.

## Limits

- **macOS only.** There is no X11, Wayland or Windows host yet; the build simply does not link the GUI on other targets.
- **The UI is CPU-rasterized.** Every chrome frame is drawn on the CPU into a bitmap. An optional offscreen Metal frame can be composited for a viewport, but there is no general GPU UI path.
- **IME is host-owned.** Marked text is exposed through `marked_text()` and committed through `insertText:`; there is no document text-range model, so a screen reader still sees a page label rather than editable ranges.
- **No arbitrary images, gradients, paths, rotation or multi-window support.** `rect`, `round_rect`, `stroke`, `line`, `text` and clipping are the vocabulary; the one image path is the Metal viewport frame.
- **No high-contrast or reduced-motion query.** The host can publish an accessibility tree of pushed elements, but it is not a document model.
- **Text is measured per call.** There is no glyph-run cache; a frame with thousands of short runs will spend most of its time in CoreText.
- The event queue holds 1024 events and drops the newest when full.

The [Studio rewrite](../../studio/nova-studio/) is the reference application: it uses the whole surface above.

## Key releases and sound

`event_key_up()` pairs with `event_key()`, so a program can tell a tap from a
hold. The host maps both through the same key-code table, and reports the
modifier set on each.

`gui.sound(frequency, milliseconds, volume)` plays a synthesized tone: the host
builds a small mono WAV in memory (sine plus a second harmonic, 6 ms attack,
exponential decay) and hands it to `NSSound`, so a bundle needs no audio files
and several tones may overlap. Volume is 0–100; the call is fire-and-forget.
