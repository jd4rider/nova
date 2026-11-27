# Nova Terminal roadmap

Status vocabulary: `TODO`, `READY`, `IN_PROGRESS`, `PARTIAL`, `BLOCKED`,
`VERIFIED`, `DEFERRED`. Only `VERIFIED` means the acceptance evidence passed.

| ID | Milestone | Status | Evidence / gap |
|---|---|---|---|
| TERM-00 | Existing implementation audit | VERIFIED | `AUDIT.md`; build, install, launch and 15/0 UI tests recorded in `EVIDENCE.md` |
| TERM-01 | PTY lifecycle | PARTIAL | Real `/bin/zsh` under a POSIX pty; child slave gets `TIOCSWINSZ` (`stty size` 34x133), resize and SIGHUP, exit status; job control not yet scripted |
| TERM-02 | Streaming parser | PARTIAL | ESC/CSI/OSC state machine with chunk-preserving state; DSR/DA/size replies added; DCS, split-byte and malformed-input fixtures remain |
| TERM-03 | Screen state | PARTIAL | Primary/alternate screens, cursor, scrolling, eras, regions, insert/delete, SGR 16/256/truecolor; origin/autowrap/app-cursor/focus modes remain |
| TERM-04 | Scrollback and Unicode | PARTIAL | 2000-line ring, fixed 512-mark ring, 96-cell draw runs; no reflow, no grapheme-aware selection, no wide-cell policy |
| TERM-05 | Input | PARTIAL | Arrow/Home/End/Delete/F-keys, Ctrl/Option mapping, paste, scrollback keys, left-button mouse press/release/drag/wheel in X10/SGR; no right/middle, IME automation or host-click fixture |
| TERM-06 | Rendering | PARTIAL | Immediate-mode grid, runs, dirty-frame loop, Retina via host, software fallback; no GPU path, no glyph atlas, no measured benchmarks |
| TERM-07 | TUI compatibility | TODO | Not yet exercised against vim, less, top, tmux or SSH |
| TERM-08 | Tabs and splits | PARTIAL | Up to 8 tabs with close/new/select and independent ptys; no split panes, no multiple windows |
| TERM-09 | Profiles and settings | TODO | Theme toggle and font cycling are in memory only; no persisted profiles |
| TERM-10 | Search and navigation | PARTIAL | Literal find over scrollback and prompt rail marks; no find bar UI or regex, no OSC 8 links |
| TERM-11 | Shell integration | TODO | OSC 133 prompt marks are consumed, but no opt-in installer |
| TERM-12 | Security | TODO | Bounded scrollback/marks and no command execution from output, but clipboard/OSC payload policy and fuzzing are absent |
| TERM-13 | Advanced protocols | TODO | No Kitty keyboard, synchronized output, graphics protocols or capability matrix |
| TERM-14 | Accessibility and polish | TODO | Host accessibility primitives exist; terminal exposes none yet |
| TERM-15 | Nova ecosystem integration | TODO | `libraries/nova-term` is importable, but the embeddable widget and Studio integration are not verified |
| TERM-16 | Performance and endurance | TODO | No recorded latency, idle CPU or sustained-output measurements |
| TERM-17 | Packaging and release | PARTIAL | Local icon, bundle and ad-hoc signature install verified; no notarization or release artifact |
| TERM-18 | Cross-platform expansion | DEFERRED | macOS ARM64 only; Windows ConPTY and Linux adapters not started |

## Current slice

TERM-00 audit is complete and the critical core defects found by it are
repaired: shared list writes in the screen buffer now persist, scripted ESC
bytes are real control bytes, pty winsize reaches the child, mouse reporting
covers the left button, and DSR/DA replies unblock vim-style probes. The next
slice is TERM-02.001 parser chunk-invariance fixtures, because every later
screen and TUI claim depends on it.

## Gates

- A milestone is VERIFIED only with a command, result and artifact.
- Do not advertise xterm-256color parity, right/middle mouse, link support or
  splits before their fixtures pass.
- The PTY host is `runtime/gui.m`; the parser and screen are Nova. Do not
  replace them with a third-party terminal or a mock.
