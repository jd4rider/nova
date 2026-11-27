# Nova Terminal continuation

Durable state for Nova Terminal, the native Nova terminal emulator in
`examples/nova-term` over the headless core in `libraries/nova-term`.

## How to resume

1. Read `STATE.md`, then `ROADMAP.md`, `TASKS.md`, `DECISIONS.md`,
   `BLOCKERS.md`, `EVIDENCE.md` and `AUDIT.md`.
2. Run `git status --short`, `git branch --show-current` and
   `git log --oneline -5`. Preserve unrelated Browser, engine and Writer
   working-tree changes.
3. Reproduce the last checkpoint before editing:
   `(cd examples/nova-tool && ../../bootstrap/nova build)` then
   `sh examples/nova-term/tests/term-ui.sh`.
4. Continue the active task in `STATE.md`, or take the highest-priority READY
   slice in `TASKS.md`.
5. Record commands, results and files changed in `STATE.md` and `EVIDENCE.md`
   before stopping.

## Build and test

```sh
(cd examples/nova-tool && ../../bootstrap/nova build)
sh examples/nova-term/install-macos.sh
sh examples/nova-term/tests/term-ui.sh
```

`install-macos.sh` builds with `examples/nova-tool/target/nova/nova-tool`
(override with `NOVA_BIN`), renders `packaging/icon.html` with headless Chrome,
builds the `.icns`, ad-hoc signs and installs `/Applications/Nova Terminal.app`.
The checked-in `bootstrap/nova` is the compiler seed; its codegen predates
imported-struct field support, so it cannot build this app directly.

## Scripted UI protocol

`NOVA_TERM_SCRIPT=<file>` runs commands and exits, printing `PASS`/`FAIL` for
`assert_*` lines. `NOVA_TERM_SHOT=<png>` saves the first frame. Supported
commands include `newtab`, `cmd`, `click`, `type`, `enter`, `feed`, `wait`,
`shot`, `dump`, `assert_has`, `assert_cell`, `assert_colour`, `assert_alt`,
`assert_tabs`, `assert_tab_title`, `assert_tab_cwd` and `assert_title`.
`<E>` in a `feed` payload becomes a real ESC byte.

## Namespace rule

This directory owns terminal milestones (`TERM-*`). Do not overwrite the
compiler `engineering/continuation/` state, `engine/`, `browser/` or `writer/`.
