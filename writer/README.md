# Nova Writer

A native word processor written in Nova. The window, document model, text
editing, layout, pagination, persistence, recovery and app logic are Nova
source; the only non-Nova code is the audited macOS GUI host (`runtime/gui.m`)
that wraps AppKit/CoreText, which the directive permits as a native OS service.

## Status

Version 0.1.0 — real document model, measured typography with mixed-script
fallback, bidi paragraph direction, inline bold/italic and incremental
layout, paginated A4 pages,
editing, selection, IME composition, undo/redo, save/open, autosave, crash
recovery, zoom, themes and an installable app. DOCX/ODT/PDF, tables, images,
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
- `NOVA_WRITER_NO_RECOVERY=1` — skip startup restore (used by the smoke test).
- `NOVA_WRITER_TRACE=1` — log preedit/commit/escape events to stdout.
- `NOVA_WRITER_SNAPSHOT_PREEDIT=1` — with `NOVA_WRITER_SNAPSHOT`, capture the
  frame while marked text is active.

## Autosave and recovery

Dirty tabs are written atomically to
`~/Library/Application Support/Nova Writer/Recovery` at most every four
seconds, and immediately after the first edit. Files are ordinary `.novadoc`
documents named `recovery-<epoch-ms>-<tab>.novadoc`.

On launch, every valid recovery file (up to six) becomes a dirty tab, replacing
the demo/blank tab; the status bar reports how many were recovered. Saving a
tab removes its recovery file, as does closing a clean tab. Closing a dirty tab
is blocked until it is saved. There is no confirmation dialog or “discard
recovery” command yet; recovery is automatic and visible in the status bar.

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
- `writer/tests/smoke.sh` — native window launch/render.
- `writer/scripts/install-macos.sh` — bundle and install.

## Limits

No DOCX/ODT/RTF/HTML/PDF, tables, images, headers/footers, footnotes,
sections, character styles, list numbering, comments, revisions, or
accessibility text ranges. Save As has no native save panel; a new document
uses the folder picker and a unique name. Autosave rewrites the whole document
every four seconds, with no external-change detection or encryption, and
recovery auto-restores without a prompt or discard command. Unchanged
paragraphs keep their measured lines; only edited or new paragraphs are
reshaped, though pagination still copies line boxes. These are tracked in
`engineering/continuation/writer/BLOCKERS.md`, not hidden.
