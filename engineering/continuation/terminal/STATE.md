# Nova Terminal checkpoint

Updated: 2026-11-26
Repository / branch: `nova` / `main`
Active milestone: TERM-02 streaming parser
Active task: TERM-02.001 chunk-invariance fixtures (partial UTF-8/CSI/OSC
buffering landed; the full byte-split corpus remains)
Status: IN_PROGRESS — TERM-00 is VERIFIED; the core write-path and control-byte
defects it found are repaired and the app is installed.

## What changed in this session

- `libraries/nova-term/src/term.nova`
  - Replaced `var xs = t.field` mutation (which clones under current Nova list
    semantics) with direct aggregate writes `t.field[i] = value`.
  - Replaced prompt marks with a fixed 512-slot ring plus `mcount`; no
    unbounded `push` remains in the core.
  - Replaced `\u{...}` escapes with `char_string(...)`; the lexer does not
    decode them, so ESC was arriving as literal `u{1b}`.
  - Replaced `graphemes()` with `chars()` (the codegen has no `graphemes`
    method); cells are scalar-based today.
  - Added `char_code(String) -> Int` to the language (runtime, codegen and
    sema) as the inverse of `char_string`. `tm_code_of` now uses it instead of
    hand-decoding UTF-8 bytes. BEL had decoded as 0, so `ESC]0;title BEL` never
    closed the OSC; the next `ESC[K` lost its bracket and painted a literal
    `K` after the prompt. Non-ASCII prompt glyphs were dropped for the same
    reason.
  - `tm_feed` holds a trailing incomplete UTF-8 scalar or CSI/OSC sequence in
    a pending register and parses it with the next pty read. Every feed ends
    at a clean parser boundary, so a read split or resize cannot strand
    parser state; `poll` stores the returned Term.
- `libraries/nova-codegen/src/llvm.nova`
  - `cg_index_assign` accepts a list-valued aggregate expression and writes
    through the shared list handle (`cg_aggregate_index_assign`). `push`
    still requires a mutable local, so growth is not silently lost.
- `libraries/nova-check/src/sema.nova`
  - Compares the trailing name of qualified imported types and allows an
    index write through an aggregate field without a mutable local receiver.
    `nova check` on the terminal project is now clean.
- `examples/nova-term/src/main.nova`
  - `set_term` replaces tabs by value because field assignment through a list
    index is unsupported.
  - Tab bar reserves the traffic-light corner (92 px), only the `+` button
    spawns a tab, and empty strip presses drag the window.
  - Click, double-click word, triple-click line and drag selection; ⌘C copies
    the selection when one exists, otherwise the screen.
  - Drawing is demand-driven (`needs_draw`); idle no longer repaints at the
    event-loop rate.
  - Plain keys arrive through AppKit's input context as `event_text`, not key
    codes. The app now routes that composed UTF-8 to the active pty, or to the
    palette/search field, so ordinary typing and IME commits work.
  - The event loop drains input before drawing and coalesces a burst into one
    frame. A timestamped `NOVA_GUI_TRACE` run recorded nine keys → nine pty
    writes in the same millisecond and the first frame ~14 ms later; Enter echo
    was also ~14 ms. The later ~195 ms burst is oh-my-posh rebuilding its
    prompt, not terminal input.
- `libraries/nova-term/src/term.nova`
  - Cell runs now draw at fixed cell x positions, with bold/italic/underline
    styles, dim blending and a solid inverse cursor; wide glyphs fall back to
    per-cell draws so columns do not drift.
  - Scripted `dump` command; Bool comparison in `assert_alt` rewritten for the
    current comparison surface.
- `runtime/gui.m`
  - The child shell gets an explicit identity (`TERM=xterm-256color`,
    `TERM_PROGRAM=NovaTerminal`, `TERM_PROGRAM_VERSION`, `COLORTERM=truecolor`,
    UTF-8 `LANG`) and inherited Warp/iTerm/VS Code/kitty plus `NOVA_TERM_*`
    variables are dropped. `NOVA_PTY_TRACE=<path>` records raw pty reads.
- `examples/nova-term/install-macos.sh`, `packaging/icon.html`, `VERSION`
  - Uses the current Nova tool, renders an icon, writes a full Info.plist,
    ad-hoc signs and installs to `/Applications`. The headless Chrome render
    uses a private `--user-data-dir`, so a concurrent installer or preview
    cannot make the icon step fail.
- `examples/nova-tool/tests/codegen-list-field-write.nova`,
  `tests/codegen.sh`
  - Pin the aggregate index-write path.

## Last verified behavior

- `(cd examples/nova-tool && ../../bootstrap/nova build)` → built the current
  compiler tool.
- `(cd examples/nova-term && ../../examples/nova-tool/target/nova/nova-tool build)`
  → `built …/target/nova/nova-term`.
- `sh examples/nova-term/tests/term-ui.sh` → passed 25, failed 0.
  Covers ANSI colours and cells, a real zsh `pwd`, tabs, OSC title, alternate
  screen history, font discovery and struct/list behavior, plus tab-bar
  click rules, double-click word selection and drag selection. New cases pin
  a split U+E0B0 glyph, a BEL-terminated OSC, a split `ESC[2K` and composed
  text reaching the pty.
- Live keyboard: the installed app was frontmost and System Events typed
  `touch /tmp/nova-typed-ok` plus Return; the file appeared. This exercises the
  real keyDown → input context → `event_text` → pty path that scripted tests
  bypass.
- `NOVA_GUI_TRACE` steady-state frame cost: draw 8–9 ms, present 2–3 ms. The
  first frame is 33 ms draw + 29 ms present (font/cache warmup). Nine queued
  keys produced nine pty writes in one millisecond and one frame ~14 ms later.
- Live prompt capture with `NOVA_PTY_TRACE` ends in
  `U+E0B0 ESC[0m OSC 0;nova-term BEL space ESC[K`; the parsed row ends at
  U+E0B0 with the cursor in the following cell and no stray `K`.
- `sh examples/nova-tool/tests/codegen.sh` → passed 259, failed 0, including
  the field index-write fixture (exit 9) and the `char_code` round-trip in
  `codegen-builtins.nova` (exit 5).
- `sh examples/nova-tool/tests/sema-ast.sh` → passed 39, failed 0.
- `sh examples/nova-tool/tests/diff-errors.sh` → 27 fixtures matched, 0
  mismatched.
- `(cd examples/nova-term && ../../examples/nova-tool/target/nova/nova-tool check)`
  → `Checked …/src/main.nova: no errors`.
- `sh examples/nova-term/install-macos.sh` → installed
  `/Applications/Nova Terminal.app`, version 0.1.0, ad-hoc signed, icon
  `NovaTerminal.icns`.
- Captured frames: `/tmp/nova-term-live.png` (startup) and
  `/tmp/nova-term-live2.png` (after `wait 1800`).

## Known failures and limits

- The checked-in `bootstrap/nova` cannot build the app: its codegen lacks
  imported-type field support. Build with the current tool as the installer
  does. `nova check` is clean with the current tool.
- No mouse reporting, splits, multi-window, IME automation, reflow, search UI,
  persistence, shell-integration installer or TUI compatibility suite yet.
- Scripted tests cover the parser and one real shell, not interactive vim,
  tmux, SSH or job control.
- The installed bundle is ad-hoc signed, not Developer ID signed/notarized.
- Rendering is CoreGraphics/CoreText into a CPU bitmap; there is no Metal or
  GPU path. Steady-state frames are 8–9 ms draw + 2–3 ms present, and input
  is coalesced, but a GPU glyph atlas is future work.
- No sustained-output, memory-endurance or long-session latency numbers have
  been recorded; do not claim them.

## Next exact action

1. Add `tests/parser-chunks.sh` (or extend the scripted harness) that feeds a
   representative DEC/CSI/OSC corpus at every byte split, compares the
   resulting cells/attributes/cursor with whole-buffer parsing, then add
   malformed and oversized OSC fixtures. The pending register now makes the
   two split regressions pass; the exhaustive corpus is still missing.
2. Implement DSR/DA replies and bounded DCS handling in `tm_feed` so vim,
   less and tmux probes have the replies they expect.
3. Then run the TERM-07 interactive probes and record exact versions.

## Dependencies

- Compiler: current `examples/nova-tool/target/nova/nova-tool`.
- Host: `runtime/gui.m` PTY pool (8 slots), `libraries/nova-gui`.
- Unrelated working-tree changes in Browser, engine, Writer and scene-gui are
  preserved and were not edited.
