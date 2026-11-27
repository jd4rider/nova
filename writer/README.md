# Nova Writer

A native word processor written in Nova. The window, document model, text
editing, layout, pagination, persistence, recovery and app logic are Nova
source; the only non-Nova code is the audited macOS GUI host (`runtime/gui.m`)
that wraps AppKit/CoreText, which the directive permits as a native OS service.

## Status

Version 0.1.0 — real document model, measured typography with mixed-script
fallback, bidi paragraph direction, inline bold/italic, incremental layout,
native Save As, paginated A4 pages, editing, selection, IME composition,
undo/redo, save/open, autosave, crash recovery with a native recovery prompt,
zoom, themes and an installable app. DOCX/ODT/PDF, tables, images,
headers and character styles are not implemented yet. See
`engineering/continuation/writer/` for the ledger.

## Build, test, install

```sh
# Compiler (if not already built)
(cd examples/nova-tool && ../../bootstrap/nova build)

# Headless core: 156 assertions
sh writer/tests/core.sh

# Two-process crash recovery
sh writer/tests/recovery.sh

# IME marked text: commit and Escape cancellation
sh writer/tests/ime.sh

# Native Save As: confirm and cancel
sh writer/tests/save-panel.sh

# Native recovery prompt: recover, keep, discard, explicit discard
sh writer/tests/recovery-prompt.sh

# Build the app
(cd writer && ../examples/nova-tool/target/nova/nova-tool build)

# Launch the real window, snapshot, exit
sh writer/tests/smoke.sh

# Build, sign and install /Applications/Nova Writer.app
sh writer/scripts/install-macos.sh
```

The installer is ad-hoc signed for local use. Set
`NOVA_WRITER_INSTALL_DIR="$HOME/Applications"` to install elsewhere.

## Run

```sh
writer/target/nova/nova-writer
```

Optional environment hooks:

- `NOVA_WRITER_SNAPSHOT=/path.png` — write a PNG after the first frame.
- `NOVA_WRITER_AUTOCLOSE=1` — quit after the snapshot.
- `NOVA_WRITER_DEMO=0` — start with a blank document.
- `NOVA_WRITER_THEME=light|dark|gruvbox` — initial theme.
- `NOVA_WRITER_RECOVERY_DIR=/path` — override the recovery directory.
- `NOVA_WRITER_NO_RECOVERY=1` — skip the startup prompt and restore (used by
  the smoke and Save As tests).
- `NOVA_WRITER_COMMAND_PROBE=discard_recovery` — run one command at startup;
  used to test the discard confirmation.
- `NOVA_GUI_ALERT_PROBE=primary|secondary|tertiary|dismiss` — stop a native
  alert for automated tests; accepts a comma-separated sequence.
- `NOVA_WRITER_TRACE=1` — log preedit/commit/escape and recovery decisions to
  stdout.
- `NOVA_WRITER_SNAPSHOT_PREEDIT=1` — with `NOVA_WRITER_SNAPSHOT`, capture the
  frame while marked text is active.

## Autosave and recovery

Dirty tabs are written atomically to
`~/Library/Application Support/Nova Writer/Recovery` at most every four
seconds, and immediately after the first edit. Files are ordinary `.novadoc`
documents named `recovery-<epoch-ms>-<tab>.novadoc`.

On launch, if snapshots exist, a native alert asks Recover, Keep for Now or
Discard. Recover restores up to six valid files as dirty tabs; Keep (or Escape)
leaves them on disk; Discard removes only those snapshots. When snapshots stay
pending, a Discard button appears in the ribbon and asks for confirmation.
Saving a tab removes its recovery file, as does closing a clean tab. Closing a
dirty tab is blocked until it is saved.

## IME composition

Marked text from the macOS input context is painted inline: Writer builds a
temporary document/layout for the preedit, underlines its range and pushes the
following text, but does not touch the saved document. `insertText:` commits the
string exactly once; Escape clears the host composition and leaves the document
unchanged. A real Japanese/Chinese/Korean input source is a manual check; the
automated test drives the same `NSTextInputClient` entry points with
`NOVA_GUI_IME_PROBE`.

## Shortcuts

| Command | Shortcut |
|---|---|
| New / Open / Save | Cmd+N / Cmd+O / Cmd+S |
| Save As | Cmd+Shift+S |
| Undo / Redo | Cmd+Z / Cmd+Shift+Z |
| Bold / Italic | Cmd+B / Cmd+I |
| Select all / Copy / Cut / Paste | Cmd+A / Cmd+C / Cmd+X / Cmd+V |
| Move by grapheme / line | Left/Right / Up/Down |
| Line start/end | Home / End |
| Scroll by view | PageUp / PageDown |
| Split paragraph / merge | Enter / Backspace at start |
| Delete forward | Delete |

Shift extends the selection. Click and drag selects; Shift-click extends from
the caret.

## Native format

`.novadoc` v4 is a deterministic UTF-8 container: magic header, version,
title, language, next paragraph ID, tab-separated paragraph records (ID, kind,
alignment, direction, escaped text), inline run records and a 31-based
polynomial body checksum. Version 1 migrates with language `en` and auto
direction; version 2 keeps its language and gains auto direction; version 3
keeps direction and starts with no inline runs. All are rewritten as v4 on the
next save. Save writes `<path>.tmp`, verifies it, then renames atomically.
Unknown records, bad checksums, missing v4 fields and malformed data are
rejected. A ZIP container with styles and media is a later migration; the
loader dispatches on the header.

## Packages

- `libraries/nova-document/` — `text.nova`, `document.nova`, `layout.nova`,
  `store.nova`, `recovery.nova`. GUI-independent and reusable by future Nova
  Office apps.
- `writer/src/main.nova` — the application.
- `writer/tests/core.sh` — headless acceptance (156 assertions).
- `writer/tests/recovery.sh` — crash recovery across two processes.
- `writer/tests/ime.sh` — IME commit and Escape cancellation (14 assertions).
- `writer/tests/save-panel.sh` — native Save As confirm and cancel.
- `writer/tests/recovery-prompt.sh` — recover, keep, discard and explicit
  discard through the native alert.
- `writer/tests/smoke.sh` — native window launch/render.
- `writer/scripts/install-macos.sh` — bundle and install.

## Limits

No DOCX/ODT/RTF/HTML/PDF, tables, images, headers/footers, footnotes,
sections, character styles, list numbering, comments, revisions, or
accessibility text ranges. Save As uses the native macOS save panel; recovery
asks before restoring and keeps unreadable or over-limit snapshots pending for
the discard command. Autosave rewrites the whole document every four seconds,
with no external-change detection or encryption. Unchanged paragraphs keep their measured lines; only edited or new
paragraphs are reshaped, though pagination still copies line boxes. These are
tracked in
`engineering/continuation/writer/BLOCKERS.md`, not hidden.
