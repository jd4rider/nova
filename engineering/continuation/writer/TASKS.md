# Nova Writer tasks

Stable IDs are permanent. Statuses are evidence-based. See `EVIDENCE.md`.

## WRI-00 — Ecosystem audit and architecture

- **WRI-00.001 — Reuse audit: VERIFIED.** Read `runtime/gui.m`,
  `runtime/runtime.c`, `libraries/nova-gui`, `libraries/nova-boxes`,
  `browser/`, `examples/nova-studio`; recorded in `AUDIT.md`.
- **WRI-00.002 — Architecture decisions: VERIFIED.** `DECISIONS.md` records
  the document model, position units, native format, undo strategy,
  typography boundary, recovery design and no-fake-controls rule.
- **WRI-00.003 — Compiler capability probe: VERIFIED.** Probe confirmed
  `chars()` and `List<String>.join`, independent snapshots, no `graphemes()`,
  and rejected qualified struct literals. Probe removed after recording.
- **WRI-00.004 — Qualified generic fields: VERIFIED.** `cg_unqualified` now
  strips a module prefix inside `List<...>`/`Option<...>` without truncating
  the wrapper, so `List<wd.Document>` is `List<Document>`. The compiler
  regression fixture returns 7, and the Writer core test declares a
  `List<Document>` field and writes through it.

## WRI-01 — Native shell

- **WRI-01.001 — Nova-built Writer window: VERIFIED.** Native window launches,
  draws, accepts input, closes; installed bundle and PNG snapshot exist.
- **WRI-01.002 — Document tabs: PARTIAL.** Create, close, switch, dirty
  markers and titles work. At most six tabs are visible; no overflow control
  or multi-window support.
- **WRI-01.003 — Command routing and shortcuts: VERIFIED.** Every visible
  command executes real document behavior; unsupported commands are absent.
- **WRI-01.004 — Headless test target: VERIFIED.** `writer/tests/core.sh`
  runs 147 assertions without a window and exits non-zero on failure.

## WRI-02 — Document core

- **WRI-02.001 — Typed document tree: VERIFIED.** Versioned document with
  paragraph IDs, block kinds, text and stable document positions.
- **WRI-02.002 — Transactional edits: VERIFIED.** Insert, delete, split,
  merge, style change and replace-selection as validated operations.
- **WRI-02.003 — Undo/redo: VERIFIED.** Bounded 100-entry history, grouping
  and selection restoration; core `Document` history tested. The app uses
  serialized snapshots (DEC-WRI-006).
- **WRI-02.004 — Structural validation: VERIFIED.** Rejects invalid
  positions, empty paragraph IDs, duplicate IDs and unknown records.
- **WRI-02.005 — Schema migration: VERIFIED.** `.novadoc` v4 adds persisted
  inline runs. Version 1 migrates with language `en` and auto direction;
  version 2 keeps its language and gains auto direction; version 3 keeps
  direction and starts with no runs. All rewrite as v4 on save; missing
  fields and unknown versions are rejected.

## WRI-03 — Text editing

- **WRI-03.001 — Grapheme-aware insertion/deletion: VERIFIED.** Cursor
  movement and deletion stop on extended grapheme clusters; tested.
- **WRI-03.002 — Selection: PARTIAL.** Anchor/focus range, replace selection,
  select all, click, drag, Shift-click and Shift+arrows are wired. Not
  machine-tested; no double-click word selection.
- **WRI-03.003 — Clipboard: PARTIAL.** Plain-text copy/cut/paste through the
  system clipboard, with CRLF normalization. Not machine-tested; no rich text.
- **WRI-03.004 — IME composition: VERIFIED.** Host marked text is painted
  inline through a temporary layout, committed once by `insertText:`, and
  cleared on Escape. `writer/tests/ime.sh` drives both paths and passes 14
  assertions.
- **WRI-03.005 — Accessibility text ranges: PARTIAL.** Buttons, tabs, styles,
  outline and status are exposed; the page has no AX range model.

## WRI-04 — Typography

- **WRI-04.001 — Styled CoreText runs: VERIFIED.** Sans/serif/mono, bold,
  italic and underline shape and measure through CoreText; rendered in the
  snapshot.
- **WRI-04.002 — Fallback/bidi fixtures: VERIFIED.** `.novadoc` v4 stores
  auto/LTR/RTL per paragraph; auto uses the first strong character. Layout
  aligns and maps the caret from the paragraph direction, CoreText falls back
  per glyph, and CJK text wraps at grapheme boundaries without overflowing.
  The core suite includes Hebrew, Arabic, Japanese, Chinese, Korean and emoji
  fixtures. Hyphenation is still absent.

## WRI-05 — Paginated layout

- **WRI-05.001 — Measured line breaking: VERIFIED.** Wraps by CoreText
  advance width; no line exceeds the text column.
- **WRI-05.002 — Page fragmentation: PARTIAL.** A4, one-inch margins,
  paragraph spacing and overflow pagination work. Sections, columns,
  headers/footers and footnotes are absent.
- **WRI-05.003 — Hit-testing and caret geometry: VERIFIED.** Click maps to a
  grapheme offset; caret and selection rectangles come from layout boxes.
- **WRI-05.004 — Zoom and scroll: VERIFIED.** Discrete zoom, visible-page
  virtualization, wheel scrolling and PageUp/PageDown.
- **WRI-05.005 — Incremental layout: TODO.** Full relayout per edit.

## WRI-06 — Styles and formatting

- **WRI-06.001 — Named paragraph styles: PARTIAL.** Title, Heading 1,
  Heading 2, Body, Quote and List with real metrics. No character styles,
  inheritance or style sets.
- **WRI-06.002 — Lists and numbering: TODO.**
- **WRI-06.003 — Inline bold/italic runs: VERIFIED.** Paragraphs store
  non-overlapping grapheme ranges with style bits. Bold/Italic toggle a
  selection or set the style for the next typed text; insert/delete/split/
  merge preserve runs; `.novadoc` v4 persists them. Layout measures and draws
  styled segments, and the core suite covers save/reopen, toggle, combination
  and edit propagation.

## WRI-07 — Native persistence

- **WRI-07.001 — `.novadoc` v4 save/open: VERIFIED.** Deterministic UTF-8
  format, checksum, validation, atomic temp-then-rename write, reopen and
  corrupt-file rejection tested. Version 1, 2 and 3 files migrate on load.
- **WRI-07.002 — Migration: VERIFIED.** See WRI-02.005: v1, v2 and v3
  fixtures migrate to v4, v4 requires `lang` and `dir`, and unknown versions
  are rejected.
- **WRI-07.003 — Autosave and crash recovery: VERIFIED.** Atomic snapshots
  every 4 seconds under Application Support, epoch-named; startup restores up
  to six valid files as dirty tabs; save and clean close remove them. A
  two-process crash test and installed-app restore pass.
- **WRI-07.004 — Save As dialog: BLOCKED.** No `NSSavePanel` bridge. Save
  writes the current path; a new document uses the native folder picker and a
  unique filename, never overwriting.
- **WRI-07.005 — Recovery prompt/discard UI: TODO.** Startup auto-restores
  without a confirmation dialog because the host has no alert bridge. There
  is no “discard recovery” command.

## WRI-14 — Professional UX

- **WRI-14.001 — Ribbon, sidebar, outline, ruler: PARTIAL.** Real command
  surface with text buttons; no icons, compact toolbar or command palette.
- **WRI-14.002 — Shortcuts: VERIFIED.** Cmd+N/O/S/Shift+S/Z/Shift+Z/A/C/X/V,
  arrows, Home/End, PageUp/PageDown, Backspace/Delete/Enter.
- **WRI-14.003 — Accessibility: PARTIAL.** AX labels and roles for controls;
  no text ranges or screen-reader document navigation.

## WRI-20 — Distribution

- **WRI-20.001 — Local bundle and install: VERIFIED.** Icon, Info.plist,
  ad-hoc signature, `/Applications/Nova Writer.app`, installed launch.
- **WRI-20.002 — Developer ID signing/notarization: BLOCKED.** No
  credentials and no release authorization.

## Verification limits

Recovery save/load, a two-process crash, app startup restore, the first
autosave write, IME marked-text commit/cancel, mixed-script fallback, bidi
caret geometry, CJK wrapping and inline run propagation are machine-tested.
Interactive typing, selection, clipboard, zoom, theme, native file panels, the
sidebar direction control, the Bold/Italic buttons, the 4-second autosave
cadence over a long session, and recovery cleanup after save are wired but not
automated. Do not mark them VERIFIED until an automated GUI test exists.
