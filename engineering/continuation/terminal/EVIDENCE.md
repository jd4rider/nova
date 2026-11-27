# Nova Terminal evidence

All entries are from this working tree on 2026-11-26, macOS ARM64.

## Baseline audit

| Command | Result |
|---|---|
| `(cd examples/nova-term && ../../bootstrap/nova build)` | FAIL: `error: unsupported type vt.Term on field term` — the seed codegen cannot key imported struct fields |
| `(cd examples/nova-term && ../../examples/nova-tool/target/nova/nova-tool build)` | FAIL: `error: in tm_finish_osc: unsupported method graphemes` |
| `"vt.Term".split(".")` probe built with the seed | PASS: `len=2 part=vt part=Term` (split is not the cause) |
| `var xs = s.xs; xs[0] = 9` probe | PASS, prints `local=1` — the mutable binding cloned the list and the write was discarded |
| `s.xs[0] = 5` probe | FAIL before repair: `index assignment requires a mutable list variable` |
| `xs.push(4)` on a parameter | FAIL: `push requires a mutable list variable` |
| Scripted ANSI probe before repair | `TEXT[u{1b}[31mredu{1b}[0m…]` — `\u{1b}` was not a real ESC |

## Repairs

- `cg_index_assign` routes a non-local list receiver through
  `cg_aggregate_index_assign`, which stores the handle and writes with
  `nova_list_at`. Aggregate `push` is deliberately rejected (a grown header
  cannot be stored back through a read-only parameter).
- `term.nova` writes directly through `t.regs`, `t.params`, `t.cells`,
  `t.sb` and the fixed `t.marks` ring.
- `tm_escape` and `tm_control_text` use `char_string(...)`.
- Sema compares the trailing name of a qualified imported type, so
  `vt.Term`/`Term`, `gui.Theme`/`Theme` and `vt.Palette`/`Palette` agree, and
  an index write through an aggregate field no longer reports E0301 for the
  immutable receiver.

## Verified commands

| Command | Result |
|---|---|
| `(cd examples/nova-tool && ../../bootstrap/nova build)` | PASS: built `examples/nova-tool/target/nova/nova-tool` |
| `(cd examples/nova-term && ../../examples/nova-tool/target/nova/nova-tool build)` | PASS: built `target/nova/nova-term` |
| `sh examples/nova-term/tests/term-ui.sh` | PASS: `passed 47, failed 0` (mouse, replies, alt-view, default-home and earlier parser cases) |
| `sh examples/nova-tool/tests/codegen.sh` | PASS: `passed 264, failed 0` (adds `char_code` and the entry-block slot guard) |
| `sh examples/nova-tool/tests/sema-ast.sh` | PASS: `passed 39, failed 0` |
| `sh examples/nova-tool/tests/diff-errors.sh` | PASS: 27 fixtures matched, 0 mismatched |
| `(cd examples/nova-term && ../../examples/nova-tool/target/nova/nova-tool check)` | PASS: `Checked …/src/main.nova: no errors` |
| Direct build of `codegen-list-field-write.nova` | PASS: exit 9 (`buffer.cells[1]` written through the field) |
| `codegen-builtins.nova` | PASS: exit 5 with `char_code("A")` and `char_code(char_string(57520))` |
| `sh examples/nova-term/install-macos.sh` | PASS: installed `/Applications/Nova Terminal.app`, version 0.1.0, `NovaTerminal.icns`, ad-hoc signed; headless Chrome uses a private profile |
| `NOVA_TERM_SHOT=/tmp/nova-term-live.png <installed binary>` | PASS: PNG written (88,105 bytes) |
| `NOVA_TERM_SCRIPT` with `wait 1800; shot /tmp/nova-term-live2.png` | PASS: PNG written (100,335 bytes) |
| `open -a "Nova Terminal"` + `pgrep -fl` | PASS: installed app process running |
| System Events keystrokes into installed app | PASS: `touch /tmp/nova-typed-ok` + Return created the file |

## Prompt glyph and OSC repair

Raw capture (`NOVA_PTY_TRACE`) of the zsh prompt ended with:

```
… U+E0B0  ESC[0m  ESC]0;nova-term BEL  space  ESC[K  ESC[?2004h
```

There is no ASCII `K` in the pty stream. The parser produced one: `tm_code_of`
returned 0 for BEL and for every non-ASCII scalar, so the OSC never closed; the
following `ESC` entered the OSC-escape state, which swallowed `[` and then
painted `K` in the ground state. The same decoder dropped the U+E0B0 chevron.
The parsed row now ends at column 62 with `238,130,176` (U+E0B0), the cursor is
in column 64, and `cells 0` contains no `75`. A split U+E0B0 fed as `\356` then
`\202\260` lands as one cell, and a BEL-terminated title is captured without
being painted. `tm_feed` also holds incomplete CSI/OSC prefixes, so a read
split inside `ESC[K` cannot leak the final byte.

Child environment after the host change: `TERM=xterm-256color`,
`TERM_PROGRAM=NovaTerminal`, `TERM_PROGRAM_VERSION=0.1.0`,
`COLORTERM=truecolor`; inherited `WARP_*`, `NOVA_TERM_*`, iTerm/VS Code/kitty
variables are removed before `exec`.

## Input latency and frame cost

A timestamped `NOVA_GUI_TRACE` run (2026-11-26, installed binary) recorded nine
queued keys, nine `pty write` calls and the first `present` inside one
millisecond band; the next frame landed ~14 ms after the keys. Enter produced
a pty write immediately and the shell echo ~14 ms later. After the command, a
335-byte burst arrived ~195 ms later: that is oh-my-posh rebuilding the prompt,
not terminal input latency.

Steady-state frame cost: `draw` 8–9 ms, `present` 2–3 ms. The first frame is
33 ms draw + 29 ms present (font/line-cache warmup). The renderer is
CoreGraphics/CoreText into a CPU bitmap; no GPU/Metal path exists yet.

## Keyboard path

`NovaGuiView keyDown:` sends non-command keys through `interpretKeyEvents`;
AppKit answers with `insertText:`, which queues `NOVA_GUI_TEXT` (kind 11). The
app loop originally handled only `NOVA_GUI_KEY`, so shortcuts worked and plain
letters vanished. The loop now consumes `event_text()` and writes the composed
UTF-8 to the active pty (or the palette/search field). Verified in the installed
app by typing `touch /tmp/nova-typed-ok` and Return with System Events: the
file was created.

## Pty size, replies and mouse

- The child saw `stty size` = `0 0` when only the master received `TIOCSWINSZ`.
  Setting the size on the opened slave too makes it report `34 133` on the
  1120x720 window; `LINES`/`COLUMNS` are dropped from the inherited child
  environment. This is the layout failure that made full-screen programs spill
  or leave the bottom blank.
- `CSI 6n`/`?6n`, `CSI 5n`, primary/secondary DA and `CSI 18t` are queued and
  written back to the pty. `vim -u NONE` now paints `VIM - Vi IMproved` and its
  `~` rows instead of leaving a blank alternate screen.
- Mouse modes 1000/1002/1003 and SGR 1006/X10 encoding have twelve assertions:
  left press/release, drag, wheel, shift/control modifiers, mode shutdown, and
  no bare-motion reports in mode 1000. Entering/leaving the alternate screen
  resets the view and mouse modes.

## Scripted UI coverage (47 assertions)

- ANSI: `31m` red cell, `92m` bright green cell, `2;1H` cursor address,
  `38;5;196` indexed colour.
- Mouse: SGR press/release/drag/wheel, X10 bytes, modifiers and mode shutdown.
- Replies: cursor-position DSR, status DSR, primary/secondary DA and `CSI 18t`.
- Alternate screen: view reset on entry and mouse-mode reset on leave.
- Default home: without `NOVA_TERM_ROOT`, the first tab starts in `$HOME`;
  the installed app's `pwd` returned `/Users/jonathanforrider`.
- Shell: real zsh prints the working directory after `pwd`.
- Tabs: 1 → 2 → 1 after new/close.
- Title: OSC `0;my-shell` captured from a real ESC/ST sequence.
- Alternate screen: `1049h`/`1049l` restores the primary row and text.
- Font discovery contains a Nerd Font family.
- Struct literals, list push and tab cwd/title behavior.
- Mouse: pressing empty tab strip and the right side does not create a tab;
  the `+` control does; double-click selects a word and click-drag selects a
  range (asserted through the copy text).
- Chunks/Unicode: U+E0B0 split across two feeds becomes one cell; a
  BEL-terminated OSC is captured and not painted; a split `ESC[2K` erases the
  line instead of printing `2K`.
- Text events: the composed-text handler sends a whole command to a real pty
  and the shell prints it.

UI slice (2026-11-26):

- `install-macos.sh` reinstalled `/Applications/Nova Terminal.app`; the bundle
  was quit and relaunched after the relaunch race.
- Captured frame `/tmp/nova-term-live4.png` shows the first tab starting after
  the 92 px traffic-light inset, a separate `+`, and a correctly aligned zsh
  prompt; the PNG is the Nova framebuffer, so AppKit's window buttons are not
  in it.
- The app no longer repaints on every idle event-loop tick; `needs_draw` is
  set by pty bytes, input, resize and selection.

## Stack-exhaustion crash (2026-11-27)

Report `nova-term-2026-11-27-005927.ips`: `EXC_BAD_ACCESS` / `SIGSEGV`,
`KERN_PROTECTION_FAILURE` at `0x16b037fe0`, 32 bytes below the bottom of the
main thread's 8 MB stack, while CoreText drew glyphs for `nova_gui_text_fmt`.
The fault was the first write after the stack pointer had already walked out of
its region, not a bad text argument.

Cause: `libraries/nova-codegen/src/llvm.nova` emitted `alloca` at the current
insertion point. `var handled = 0` / `var running = true` in the app's main
loop, and parser locals in `tm_feed`'s byte switch, therefore allocated a fresh
slot on every iteration. Nothing reclaimed them, so an idle terminal lost stack
continuously and died during a later draw.

| Measurement (`NOVA_GUI_TRACE`, idle) | Before | After |
|---|---|---|
| stack used over 16–20 s | 31 → 87 KB, +9.5 KB per 2.7 s | 22 KB, flat |
| projected life of 8176 KB | ~37 min (crash report: launch 00:15, death 00:59) | unbounded |
| `alloca` inside loop bodies in `nova-term.ll` | 74 in 17 functions | 0 |

Repair: all slots use `cg_entry_slot` (entry block); `cg_for_range`,
`cg_for_array`, `cg_for_slice`, `cg_for_list`, `cg_var`, `cg_try` and
`cg_match` no longer emit an `alloca` where they are reached. Total alloca
count is unchanged (683), only their placement moved.

### Regression guard

`examples/nova-tool/tests/codegen-loop-slots.nova` declares slots in every shape
that leaked: `var` in `while`, `var` in an `if` inside a loop, `var` and `match`
in `for`, and a list temporary in a loop. `codegen.sh` then fails if any
`alloca` appears after a function's first block label — the entry block is the
only unlabelled block — and a hand-written `.ll` containing a late `alloca`
proves the check fires instead of passing vacuously.

| Command | Result |
|---|---|
| `(cd examples/nova-tool && ../../bootstrap/nova build)` | PASS: built the current tool with the repair |
| `(cd examples/nova-term && ../../examples/nova-tool/target/nova/nova-tool build)` | PASS: built `target/nova/nova-term`; `nova-term.ll` has 0 loop-body allocas |
| idle run, `NOVA_GUI_TRACE` | PASS: `stack: used=22KB` unchanged across every sample (was +9.5 KB/2.7 s) |
| `sh examples/nova-term/tests/term-ui.sh` | PASS: `passed 47, failed 0` |
| `sh examples/nova-tool/tests/codegen.sh` | PASS: `passed 264, failed 0` (259 before the guard) |
| `sh examples/nova-term/install-macos.sh` | PASS: reinstalled `/Applications/Nova Terminal.app`, ad-hoc signed |
| installed binary, idle run with `NOVA_GUI_TRACE` | PASS: `stack: used=22KB`, flat |

### The same defect in Nova Browser

`~/Library/Logs/DiagnosticReports/nova-browser-2026-11-26-032901.ips`
("Thread stack size exceeded", fault 80 bytes below the stack) is the same bug;
see `../browser/BLOCKERS.md`. Rebuilt against the repaired tool:

| Command | Result |
|---|---|
| `(cd browser && ../examples/nova-tool/target/nova/nova-tool build)` | PASS: built `target/nova/nova-browser` |
| IR scan of `browser/target/nova/nova-browser.ll` | PASS: 0 `alloca` after a function's first block label, 1818 allocas total |
| `sh browser/scripts/install-macos.sh` | PASS: installed `/Applications/Nova Browser.app` 0.9.11, ad-hoc signed, `codesign --verify --strict` ok |
| installed binary, 60 s idle with `NOVA_GUI_TRACE` | PASS: 15 samples, all `stack: used=23KB` |
| `sh browser/tests/address.sh` | PASS: `failures=0` |

Not verified: a 40-minute-plus live session (the leak is measured as zero, not
waited out), and the full Browser suite rather than the address slice.

## Not verified

- Right/middle mouse buttons and a physical host-click automation fixture;
  protocol and app encoding are tested, but synthetic AppKit clicks were not
  delivered by accessibility automation.
- IME, splits, multi-window, reflow, search UI, settings persistence, shell
  integration, OSC 8 links and bounded DCS handling.
- less, top, tmux, SSH, job control, Ctrl-C/Ctrl-Z and password prompts.
  Vim's alternate screen is the only full-screen TUI checked through the
  scripted cell dump.
- Performance, idle CPU, sustained output and memory endurance.
- Developer ID signing and notarization.
