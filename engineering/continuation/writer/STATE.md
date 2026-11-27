# Nova Writer — current state

Updated: 2026-11-27
Branch: `main` (Writer landed; v4 inline runs and incremental layout are the
current slice).
Installed app: `/Applications/Nova Writer.app` — version 0.1.0,
`dev.nova.writer.native`, ad-hoc signed.

## What exists and works

A native, Nova-built Writer window opens over a GUI-independent document core.
It is not a mock and not a browser canvas.

- **Document core** (`libraries/nova-document/`): versioned `Document` with
  stable paragraph IDs, grapheme offsets, validated insert/delete/split/merge,
  paragraph styles, alignment, direction and inline bold/italic runs, bounded
  Document-based undo/redo, outline, counts, and a checksummed `.novadoc` v4
  format with atomic save and v1/v2/v3 migration.
- **Text** (`text.nova`): extended-grapheme segmentation implemented in Nova
  (combining marks, skin tones, flags, ZWJ families), insert, backspace,
  forward delete, escaping, word count and case-aware find.
- **Layout** (`layout.nova`): CoreText-measured word wrapping over styled
  segments, hard breaks for long tokens and CJK runs, auto/LTR/RTL paragraph
  alignment, direction-aware caret geometry, A4 pagination with one-inch
  margins, grapheme hit-testing and a paragraph-level cache that reshapes only
  changed or new paragraphs before repaginating.
- **Store** (`store.nova`): temp write → verify → atomic rename; checksum
  validation rejects corruption, unknown records and garbage.
- **Recovery** (`recovery.nova`): atomic `.novadoc` autosave snapshots under
  `~/Library/Application Support/Nova Writer/Recovery`, epoch-named; startup
  restores valid snapshots as dirty tabs; save and clean close remove them.
- **Host** (`runtime/gui.m`, `libraries/nova-gui`): styled CoreText runs
  (sans/serif/mono, bold/italic/underline), measured advances, atomic rename,
  epoch time and file deletion.
- **App** (`writer/src/main.nova`): tabs, ribbon with Bold/Italic, ruler,
  styles/direction and outline sidebar, pages, caret, selection, keyboard/mouse
  editing, clipboard, IME composition, undo/redo, save/open, autosave, crash
  restore, zoom, themes, paper size, accessibility labels, autoclose/snapshot
  hooks.
- **Packaging** (`writer/scripts/install-macos.sh`): builds, renders the icon,
  creates and signs `Nova Writer.app`, installs to `/Applications`.
- **Tests**: `writer/tests/core.sh` (156 assertions, headless),
  `writer/tests/recovery.sh` (two-process crash recovery),
  `writer/tests/ime.sh` (14 IME composition assertions) and
  `writer/tests/smoke.sh` (real window + PNG snapshot).

## Evidence (latest run)

- Core: `failures=0`, 156 PASS, exit 0.
- Crash recovery: `PASS recovery survives a process exit`; a second process
  loaded `Crash Report` / `Unsaved work survives a crash.` from the file the
  first process left behind.
- App restore: installed bundle restored the crash document, tab
  “Crash Report”, status “Recovered 1 unsaved document”; snapshot 132,828
  bytes.
- Autosave: recovery-file mtime advanced `1795758920` → `1795758922` during an
  app run, and the rewritten file still loaded.
- IME: `sh writer/tests/ime.sh` → 14 PASS, 0 FAIL, exit 0. The snapshot
  shows “にほ” underlined inline with “Nova Writer” pushed
  right; commit inserts “日本” exactly once; Escape clears the composition and
  a later composition still commits.
- Migration: v1, v2 and v3 fixtures load as v4, preserving IDs/text; v1 gets
  language `en`, v2 keeps its language, and both get auto direction; v3 keeps
  direction and starts with no runs. V4 requires `lang` and `dir`; missing
  fields and unknown versions are rejected.
- Inline runs: bold and italic ranges survive save/reopen, toggle off, combine,
  and shift correctly through insert, delete, split and merge. Layout emits
  styled line segments and the bold range has measurable width.
- Incremental layout: the cache reports `measured=1, reused=79` after a
  one-paragraph edit in an 80-paragraph fixture; cached output matches full
  layout. Ten iterations took 2,265ms full vs 56ms incremental (2,499ms vs
  30ms when nothing changed) on this host.
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
- Normal installed launch: exit 0; snapshot 2360×1640, 343,002 bytes.

## Honest limits

- No DOCX/ODT/RTF/HTML/PDF import or export yet.
- Inline bold/italic is implemented; character styles, underline UI and list
  numbering are not. Inline runs are painted as separate CoreText segments, so
  mixed bidi plus inline style remains approximate.
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
- Layout reuses measured paragraphs but still copies every line box during
  pagination; the cache is in memory only.
- Tab bar shows at most six tabs; no overflow control.
- GUI typing/selection/clipboard are wired but not automated; only launch and
  rendering are machine-verified.

## Active slice

**WRI-07.004 — native save panel** is next (host work). WRI-05.005 landed:
`layout.nova` caches measured paragraphs by stable ID and content, the app
reuses unchanged lines after every edit, and an 80-paragraph benchmark measured
a one-paragraph update at 56ms versus 2,265ms for a full relayout.

After that: WRI-07.005 recovery prompt/discard, WRI-06.002 list numbering,
WRI-05.002 sections/columns.

## Working-tree state

Writer is committed on `main`; the installed app is rebuilt from the current
tree after each slice. Unrelated Browser/Terminal/engine changes remain
uncommitted and untouched.
