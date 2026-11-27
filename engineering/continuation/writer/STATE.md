# Nova Writer — current state

Updated: 2026-11-26
Branch: `main` (Writer landed; v3 typography is the current slice).
Installed app: `/Applications/Nova Writer.app` — version 0.1.0,
`dev.nova.writer.native`, ad-hoc signed.

## What exists and works

A native, Nova-built Writer window opens over a GUI-independent document core.
It is not a mock and not a browser canvas.

- **Document core** (`libraries/nova-document/`): versioned `Document` with
  stable paragraph IDs, grapheme offsets, validated insert/delete/split/merge,
  paragraph styles, alignment and direction, bounded Document-based undo/redo,
  outline, counts, and a checksummed `.novadoc` v3 format with atomic save and
  v1/v2 migration.
- **Text** (`text.nova`): extended-grapheme segmentation implemented in Nova
  (combining marks, skin tones, flags, ZWJ families), insert, backspace,
  forward delete, escaping, word count and case-aware find.
- **Layout** (`layout.nova`): CoreText-measured word wrapping, hard breaks for
  long tokens and CJK runs, auto/LTR/RTL paragraph alignment, direction-aware
  caret geometry, A4 pagination with one-inch margins and grapheme
  hit-testing.
- **Store** (`store.nova`): temp write → verify → atomic rename; checksum
  validation rejects corruption, unknown records and garbage.
- **Recovery** (`recovery.nova`): atomic `.novadoc` autosave snapshots under
  `~/Library/Application Support/Nova Writer/Recovery`, epoch-named; startup
  restores valid snapshots as dirty tabs; save and clean close remove them.
- **Host** (`runtime/gui.m`, `libraries/nova-gui`): styled CoreText runs
  (sans/serif/mono, bold/italic/underline), measured advances, atomic rename,
  epoch time and file deletion.
- **App** (`writer/src/main.nova`): tabs, ribbon, ruler, styles/direction and
  outline sidebar, pages, caret, selection, keyboard/mouse editing, clipboard,
  IME composition, undo/redo, save/open, autosave, crash restore, zoom,
  themes, paper size, accessibility labels, autoclose/snapshot hooks.
- **Packaging** (`writer/scripts/install-macos.sh`): builds, renders the icon,
  creates and signs `Nova Writer.app`, installs to `/Applications`.
- **Tests**: `writer/tests/core.sh` (124 assertions, headless),
  `writer/tests/recovery.sh` (two-process crash recovery),
  `writer/tests/ime.sh` (14 IME composition assertions) and
  `writer/tests/smoke.sh` (real window + PNG snapshot).

## Evidence (latest run)

- Core: `failures=0`, 124 PASS, exit 0.
- Crash recovery: `PASS recovery survives a process exit`; a second process
  loaded `Crash Report` / `Unsaved work survives a crash.` from the file the
  first process left behind.
- App restore: installed bundle restored the crash document, tab
  “Crash Report”, status “Recovered 1 unsaved document”; snapshot 132,828
  bytes.
- Autosave: recovery-file mtime advanced `1795758920` → `1795758922` during an
  app run, and the rewritten file still loaded.
- IME: `sh writer/tests/ime.sh` → 14 PASS, 0 FAIL, exit 0. The snapshot
  (332,535 bytes) shows “にほ” underlined inline with “Nova Writer” pushed
  right; commit inserts “日本” exactly once; Escape clears the composition and
  a later composition still commits.
- Migration: v1 and v2 fixtures load as v3, preserving IDs/text; v1 gets
  language `en`, v2 keeps its language, and both get auto direction. V3
  requires `lang` and `dir`; missing fields and unknown versions are rejected.
- Typography: mixed Latin/Hebrew/Arabic/CJK/emoji measures through CoreText;
  auto direction detects Hebrew, explicit LTR overrides it, an RTL paragraph
  is right-aligned with a right-to-left caret, and a CJK paragraph wraps with
  every line inside the column.
- Host: `clang -fobjc-arc -O2 -c runtime/gui.m` exit 0; one pre-existing
  `accessibilityPerformPress` warning.
- App build: `built .../writer/target/nova/nova-writer`, exit 0.
- Smoke: `PASS writer window rendered`, exit 0.
- Install: `Installed /Applications/Nova Writer.app`, exit 0;
  `codesign --verify --strict` passes.
- Normal installed launch: exit 0; snapshot 2360×1640, 344,420 bytes.

## Honest limits

- No DOCX/ODT/RTF/HTML/PDF import or export yet.
- No inline bold/italic runs, character styles or list numbering; styles are
  paragraph-level.
- No tables, images, headers/footers, footnotes, sections or TOC.
- No accessibility text ranges.
- Bidi runs are shaped by CoreText within a line; caret and selection geometry
  use the paragraph direction, so mixed-direction caret positions inside one
  line are approximate. CJK breaks are grapheme-based, not full UAX #14
  kinsoku; there is no hyphenation dictionary.
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

**WRI-06.003 — inline bold/italic runs** is next. WRI-04.002 landed:
`.novadoc` v3 stores per-paragraph auto/LTR/RTL direction, v1/v2 files
migrate on load, the sidebar exposes direction, and the core suite covers
mixed-script fallback, RTL caret geometry and CJK wrapping.

After that: WRI-05.005 incremental layout, WRI-07.004 save panel (host work),
WRI-07.005 recovery prompt/discard.

## Working-tree state

Writer is committed on `main`; the installed app is rebuilt from the current
tree after each slice. Unrelated Browser/Terminal/engine changes remain
uncommitted and untouched.
