# Nova Writer — current state

Updated: 2026-11-26
Branch: `selfhost/pure-nova` (working tree; no Writer commit yet)
Installed app: `/Applications/Nova Writer.app` — version 0.1.0,
`dev.nova.writer.native`, ad-hoc signed.

## What exists and works

A native, Nova-built Writer window opens over a GUI-independent document core.
It is not a mock and not a browser canvas.

- **Document core** (`libraries/nova-document/`): versioned `Document` with
  stable paragraph IDs, grapheme offsets, validated insert/delete/split/merge,
  paragraph styles and alignment, bounded Document-based undo/redo, outline,
  counts, and a checksummed `.novadoc` v1 format with atomic save.
- **Text** (`text.nova`): extended-grapheme segmentation implemented in Nova
  (combining marks, skin tones, flags, ZWJ families), insert, backspace,
  forward delete, escaping, word count and case-aware find.
- **Layout** (`layout.nova`): CoreText-measured word wrapping, hard breaks for
  long tokens, A4 pagination with one-inch margins, caret geometry and
  grapheme hit-testing.
- **Store** (`store.nova`): temp write → verify → atomic rename; checksum
  validation rejects corruption, unknown records and garbage.
- **Recovery** (`recovery.nova`): atomic `.novadoc` autosave snapshots under
  `~/Library/Application Support/Nova Writer/Recovery`, epoch-named; startup
  restores valid snapshots as dirty tabs; save and clean close remove them.
- **Host** (`runtime/gui.m`, `libraries/nova-gui`): styled CoreText runs
  (sans/serif/mono, bold/italic/underline), measured advances, atomic rename,
  epoch time and file deletion.
- **App** (`writer/src/main.nova`): tabs, ribbon, ruler, styles/outline
  sidebar, pages, caret, selection, keyboard/mouse editing, clipboard, IME
  composition, undo/redo, save/open, autosave, crash restore, zoom, themes,
  paper size, accessibility labels, autoclose/snapshot hooks.
- **Packaging** (`writer/scripts/install-macos.sh`): builds, renders the icon,
  creates and signs `Nova Writer.app`, installs to `/Applications`.
- **Tests**: `writer/tests/core.sh` (82 assertions, headless),
  `writer/tests/recovery.sh` (two-process crash recovery),
  `writer/tests/ime.sh` (14 IME composition assertions) and
  `writer/tests/smoke.sh` (real window + PNG snapshot).

## Evidence (latest run)

- Core: `failures=0`, 82 PASS, exit 0.
- Crash recovery: `PASS recovery survives a process exit`; a second process
  loaded `Crash Report` / `Unsaved work survives a crash.` from the file the
  first process left behind.
- App restore: installed bundle restored the crash document, tab
  “Crash Report”, status “Recovered 1 unsaved document”; snapshot 127,530
  bytes.
- Autosave: recovery-file mtime advanced `1795758920` → `1795758922` during an
  app run, and the rewritten file still loaded.
- IME: `sh writer/tests/ime.sh` → 14 PASS, 0 FAIL, exit 0. The snapshot
  (332,535 bytes) shows “にほ” underlined inline with “Nova Writer” pushed
  right; commit inserts “日本” exactly once; Escape clears the composition and
  a later composition still commits.
- Host: `clang -fobjc-arc -O2 -c runtime/gui.m` exit 0; one pre-existing
  `accessibilityPerformPress` warning.
- App build: `built .../writer/target/nova/nova-writer`, exit 0.
- Smoke: `PASS writer window rendered`, exit 0.
- Install: `Installed /Applications/Nova Writer.app`, exit 0;
  `codesign --verify --strict` passes.
- Normal installed launch: exit 0; snapshot 2360×1640, 329,789 bytes.

## Honest limits

- No DOCX/ODT/RTF/HTML/PDF import or export yet.
- No inline bold/italic runs, character styles or list numbering; styles are
  paragraph-level.
- No tables, images, headers/footers, footnotes, sections or TOC.
- No accessibility text ranges.
- IME composition is painted from a temporary document/layout; the real
  document is unchanged until the input method commits.
- Save As has no native save panel; Save writes the current path and a new
  document uses the folder picker with a unique filename.
- No recovery prompt or discard command: startup auto-restores every valid
  recovery file (up to six) as a dirty tab. Saving removes that tab’s file.
- Autosave rewrites the whole document every 4 seconds; no incremental save,
  external-change detection or encryption. Recovery files are local plain
  `.novadoc` under Application Support.
- Full relayout on every edit.
- Tab bar shows at most six tabs; no overflow control.
- GUI typing/selection/clipboard are wired but not automated; only launch and
  rendering are machine-verified.

## Active slice

**WRI-02.005 — schema migration** is next. Only `.novadoc` v1 exists; the
loader dispatches on the version header so v2 can be added without breaking
v1 files.

After that: WRI-04.002 bidi fixtures, WRI-07.004 save panel (host work),
WRI-07.005 recovery prompt/discard, WRI-06 inline runs.

## Working-tree state

Modified: `runtime/gui.m`, `libraries/nova-gui/src/gui.nova`.
New: `libraries/nova-document/`, `writer/`,
`engineering/continuation/writer/`.
Unrelated Browser/engine/render/metal/speech changes are untouched.
