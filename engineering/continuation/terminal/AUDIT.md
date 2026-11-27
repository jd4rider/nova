# Nova Terminal audit (TERM-00)

Date: 2026-11-26
Platform: macOS ARM64, Apple clang, self-hosted Nova compiler
Sources read: `runtime/gui.m`, `libraries/nova-gui/src/gui.nova`,
`libraries/nova-term/src/term.nova`, `examples/nova-term/src/main.nova`,
`examples/nova-term/tests/term-ui.sh`, `libraries/nova-codegen/src/llvm.nova`,
`runtime/runtime.c`.

## Architecture as found

```text
runtime/gui.m pty pool (8 slots)
  fork/exec /bin/sh -c "<shell> -i", setsid + TIOCSCTTY, TIOCSWINSZ
  reader thread -> locked byte buffer -> nova_pty_poll()
        |
        v
libraries/nova-term (headless)
  tm_feed() streaming parser: ground/ESC/CSI/OSC/OSC-ESC states
  register list + cell ring + scrollback ring + fixed mark ring
  tm_draw() immediate-mode runs through libraries/nova-gui
        |
        v
examples/nova-term
  App/Tab structs, per-tab pty slot, immediate-mode loop, command palette,
  literal scrollback find, scripted test protocol
```

Boundaries are sensible: byte transport is the host, parser and screen are
Nova and importable, and the app is a view. Seven independent sessions are
possible (8 slots, one tab starts on slot 0). The app is not a shell wrapper:
it launches the user's `$SHELL` under a real pty.

## Verified working

- Build with the current Nova tool; install and launch as a signed-local app.
- Real zsh start, keystrokes, output, resize plumbing and SIGHUP close.
- ANSI SGR 16/256/truecolor, cursor addressing, erase, insert/delete, regions.
- Primary and alternate screens; alternate leave restores primary cells.
- OSC 0/2 title capture; OSC 133 `A` prompt marks consumed.
- 2000-line scrollback, 512-mark ring, literal find, prompt rail.
- Tabs: create, close, select, per-slot ptys.
- Font discovery and cycling; Dark+/Gruvbox palettes.
- Scripted harness with cell, colour, text, tab, title and alt assertions.

## Defects found and repaired in this session

1. **Screen writes were discarded.** Every core setter did
   `var cells = t.cells; cells[i] = v`; current Nova clones a list on a mutable
   binding, so the clone was thrown away. The compiler also rejected writes
   through a field. Fixed by aggregate index assignment in codegen and direct
   field writes in the core.
2. **`\u{...}` is not a lexer escape.** `tm_escape()` returned literal
   `u{1b}`, so every scripted CSI/OSC was text. Fixed with `char_string`.
3. **`graphemes()` is not implemented in codegen.** The core used it in five
   places. Switched to `chars()`; cells are scalar-based, not grapheme-based.
4. **Unbounded marks.** Prompt marks used `push` on a shared list, which is not
   observable through a parameter and can grow forever. Replaced with a fixed
   512-slot ring and a count register.
5. **The app did not compile with either available compiler** (seed: imported
   struct field; current tool: graphemes / field assignment). It now compiles
   and the UI suite runs.
6. **Sema disagreed with codegen about imported type spelling.** Method
   signatures used `Term` while fields used `vt.Term`, producing E0308, and
   the aggregate index write produced E0301 on the immutable receiver. Sema
   now compares trailing names and permits field index writes. `nova check` is
   clean.

## Missing or partial (priority order)

1. Parser chunk-invariance and malformed-input fixtures (TERM-02).
2. DSR/DA replies, DCS bounds, DEC modes beyond cursor visibility and alt
   screen (TERM-02/03) — required by vim/less/tmux.
3. Reflow and grapheme/wide-cell selection (TERM-04).
4. Mouse reporting, selection, IME automation (TERM-05).
5. TUI probes: vim, less, top, tmux, SSH, job control (TERM-07).
6. Splits, settings/profiles, search UI, links (TERM-08/09/10).
7. Shell integration installer (TERM-11).
8. Security policy for OSC 52/hyperlinks and fuzzing (TERM-12).
9. Performance measurement and release hardening (TERM-16/17).

## Compiler constraints that shape the design

- `var x = list` copies; aggregate index writes share the handle; `push`
  needs a mutable local.
- No `graphemes()`, no string slicing, no field assignment through a list
  index, no generic structs/methods.
- Scalar-only FFI; the host exposes pty operations by slot and string handles.

These are recorded rather than hidden. The terminal avoids unsupported
constructs instead of mocking their behavior.
