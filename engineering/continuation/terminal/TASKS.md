# Nova Terminal tasks

Stable IDs. A parent is not VERIFIED until its acceptance gate passes.

## TERM-00 — Audit and baseline

- TERM-00.001 — Inventory and gap matrix: **VERIFIED**. `AUDIT.md` records the
  host PTY, `libraries/nova-term`, app shell, compiler constraints and missing
  capabilities. Sources were read, not assumed.
- TERM-00.002 — Build, package and launch: **VERIFIED**.
  `examples/nova-tool/target/nova/nova-tool` builds the app;
  `install-macos.sh` renders `packaging/icon.html`, creates
  `NovaTerminal.icns`, ad-hoc signs and installs
  `/Applications/Nova Terminal.app`; the installed binary was launched and a
  frame was captured. Evidence in `EVIDENCE.md`.
- TERM-00.003 — Repair the screen-state write path: **VERIFIED**. The compiler
  rejected writes through struct fields and every `var cells = t.cells` copy
  was discarded, so ANSI output never reached the grid. `cg_index_assign`
  now writes through aggregate list handles; the core uses direct field
  writes. Evidence: `term-ui.sh` 15/0, `codegen.sh` 258/0.
- TERM-00.005 — Sema agrees with codegen on imported types and field writes:
  **VERIFIED**. `nova check` reports no errors on the terminal project;
  sema-ast 39/0 and diff-errors 27/27 unchanged.
- TERM-00.004 — Real control bytes in the test protocol: **VERIFIED**.
  `\u{...}` is not decoded by the Nova lexer; `tm_escape`/`tm_control_text`
  now use `char_string(...)`. ANSI and OSC title fixture failures cleared.

## TERM-01 — PTY lifecycle

- TERM-01.001 — Real interactive shell: **VERIFIED** for start, input, output
  and close. `wait 1200; type pwd; enter` prints the working directory in the
  scripted run, and System Events typed a real command into the installed app
  (`touch /tmp/nova-typed-ok`) which created the file.
- TERM-01.002 — Initial size, resize and exit status: **PARTIAL**. `TIOCSWINSZ`
  is set at open and on resize with `SIGWINCH`; the reader thread reaps the
  child and stores the status. Automated resize and status assertions are TODO.
- TERM-01.003 — Job control, Ctrl-C/Ctrl-Z, foreground groups, password
  prompts: **TODO**. No scripted evidence yet.
- TERM-01.004 — Close vs terminate policy: **TODO**. Closing a tab sends
  `SIGHUP` immediately; no running-job confirmation.

## TERM-02 — Streaming parser

- TERM-02.001 — Chunk-invariance fixtures: **PARTIAL**. `tm_feed` now keeps
  incomplete UTF-8 scalars and CSI/OSC prefixes in a pending register, and
  scripted cases cover a split U+E0B0, BEL-terminated OSC and split
  `ESC[2K`. Feeding every split point of a representative corpus and
  comparing against whole-buffer parsing remains.
- TERM-02.002 — Malformed and oversized input recovery: **TODO**. Add bounded
  OSC/DCS payload tests and unknown-sequence recovery fixtures.
- TERM-02.003 — DCS and device replies: **TODO**. DCS is currently swallowed;
  DSR/DA replies are absent.

## TERM-03 — Screen state

- TERM-03.001 — Alternate screen preserves primary history: **VERIFIED** for
  enter/leave with `1049h`/`1049l`; the primary row survives.
- TERM-03.002 — DEC private modes: **PARTIAL**. Cursor visibility and alternate
  screen are handled; origin mode, autowrap toggle, application cursor/keypad,
  bracketed paste, focus reporting and mouse modes are not.
- TERM-03.003 — Cursor style and saved cursor: **PARTIAL**. Coordinates are
  saved; attributes and style are not.

## TERM-04 — Scrollback and Unicode

- TERM-04.001 — Bounded history: **VERIFIED** for line and mark rings
  (2000 lines, 512 marks); no unbounded push remains in the core.
- TERM-04.002 — Reflow and logical lines: **TODO**. Resize keeps the bottom
  rows and does not reflow.
- TERM-04.003 — Grapheme, wide-cell and emoji policy: **PARTIAL**. BMP
  scalars now decode, store and render (U+E0B0 verified), and split UTF-8
  bytes are buffered. Wide cells, combining marks and emoji width are TODO.

## TERM-05 — Input

- TERM-05.001 — Key encoding: **PARTIAL**. Arrows, Home/End, PageUp/Down,
  Delete, F-keys, Ctrl and Option are encoded; application modes are ignored.
- TERM-05.002 — Mouse reporting: **TODO**. No X10/SGR mouse sequences are
  emitted yet.
- TERM-05.004 — Local selection and copy: **VERIFIED** for click, double-click
  word, triple-click line, drag and ⌘C; scripted assertions cover word and
  drag selection. Mouse reporting to the child is still TERM-05.002.
- TERM-05.003 — IME and composed text: **PARTIAL**. Plain keys and IME commits
  arrive as `event_text` and are written to the pty; the candidate-window and
  marked-text path exists in the host but has not been exercised with a real
  IME.

## TERM-06 — Rendering

- TERM-06.001 — Correct grid frame: **PARTIAL**. Verified by scripted cell and
  colour assertions plus captured frames. Runs draw at fixed cell positions;
  bold/italic/underline/dim and a solid inverse cursor are rendered. No visual
  golden comparison yet.
- TERM-06.003 — Idle redraw: **VERIFIED** for the app loop. The frame is drawn
  only when bytes, input, resize or selection change it; the idle loop pumps
  and sleeps instead of repainting. No CPU measurement recorded yet.
- TERM-06.002 — Performance baseline: **TODO**. No recorded startup, idle CPU,
  latency or sustained-output numbers.

## TERM-07 — TUI compatibility

- TERM-07.001 — vim, less, top, tmux, SSH: **TODO**. Run each interactively
  and record exact versions and failures.

## TERM-08 — Tabs and splits

- TERM-08.001 — Independent tabs: **VERIFIED** for create, close and count
  (scripted 1 → 2 → 1). Independent output per tab follows from per-slot ptys.
  The first tab starts at 92 px so AppKit's traffic lights keep their corner;
  only the `+` control creates a tab and empty strip presses drag the window.
- TERM-08.002 — Split panes: **TODO**.
- TERM-08.003 — Multiple windows: **BLOCKED** by the single-window GUI host.

## TERM-09 — Profiles and settings

- TERM-09.001 — Atomic persisted settings and profiles: **TODO**.

## TERM-10 — Search and navigation

- TERM-10.001 — Literal scrollback search: **PARTIAL**. `tm_find` exists; the
  find bar, match navigation and regex are TODO.
- TERM-10.002 — OSC 8 links: **TODO**.

## TERM-11 — Shell integration

- TERM-11.001 — Opt-in reversible integration: **TODO**.

## TERM-12 — Security

- TERM-12.001 — Bounds: **PARTIAL**. Scrollback, marks, draw runs and parser
  parameter slots are bounded.
- TERM-12.002 — Clipboard/OSC/hyperlink policy and fuzzing: **TODO**.

## TERM-13..18 — Later gates

Advanced protocols, accessibility, widget extraction, performance endurance,
signed release and cross-platform adapters are `TODO`/`DEFERRED` in
`ROADMAP.md`.
