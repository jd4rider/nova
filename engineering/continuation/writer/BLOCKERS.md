# Nova Writer blockers

## External blockers

None for the current core/shell/recovery slice. The host has the Nova compiler,
clang, CoreText and the native GUI.

## Owner-authorization boundaries

- Do not push, publish, deploy, sign/notarize or change repository visibility
  without explicit approval.
- Do not delete or rewrite unrelated working-tree changes (Browser, engine,
  render, metal, speech).
- Do not claim DOCX/ODT/PDF compatibility before adapters and independent
  validation exist.

## Internal capability gaps (tracked, not excuses)

| Gap | Effect | Task |
|---|---|---|
| App undo still uses serialized snapshots | compiler now supports `List<Document>`; app can switch later | DEC-WRI-006; not user-visible |
| Index assignment through a struct field is rejected | session replacement goes through `wr_put` with a local list copy | compiler parity |
| No native save panel | Save As cannot exist; Save writes the current path | WRI-07.004 |
| No alert/confirmation bridge | recovery auto-restores without a prompt and has no discard command | WRI-07.005 |
| No app-defined native menu model | only the fixed host app menu; commands live in the ribbon | host work |
| No ZIP central directory/deflate | DOCX/ODT/native ZIP container blocked | WRI-10 prerequisite |
| No XML parser | OOXML/ODF blocked | WRI-10 prerequisite |
| No PDF writer | PDF export/print blocked | WRI-11 |
| No string slice/insert | editing rebuilds paragraphs from graphemes | DEC-WRI-001 |
| No accessibility text ranges | screen readers see a document label, not ranges | WRI-03.005 |
| One window, at most six visible tabs | no multi-window or tab overflow | WRI-01.002 |
| No incremental layout | every edit relayouts the document | WRI-05.005 |
| No automated GUI input harness | typing/selection/clipboard are not machine-verified | WRI-01 test harness |
| No generic structs / `dyn Trait` | flat paragraph model, static dispatch | compiler parity work |

## Resolved

- **No font family/weight API.** Styled CoreText runs and measured advances
  were added (DEC-WRI-004), so serif body, bold, italic and underline are real.
- **No native test target.** A headless core binary with 124 assertions exists.
- **No installable bundle.** `/Applications/Nova Writer.app` builds, signs and
  launches.
- **No autosave or crash recovery.** `recovery.nova`, a two-process crash test
  and an installed-app restore now cover dirty documents.
- **No IME marked text.** The host exposes `marked_text()` and clears it on
  commit or Escape; Writer paints the composition inline and commits once.
  `writer/tests/ime.sh` passes 14 assertions.
- **No bidi direction or CJK fixture.** `.novadoc` v3 stores per-paragraph
  auto/LTR/RTL direction; the core suite covers RTL alignment/caret geometry,
  mixed-script fallback and CJK wrapping.

## Known correctness limits

- Snapshot undo is bounded at 100 transactions and is memory-heavy for very
  large documents; the app reparses on undo.
- Pagination is single-column A4 with fixed margins; sections, columns,
  headers/footers and footnotes are not implemented.
- CoreText shapes bidi runs, but caret and selection geometry use one
  paragraph direction; mixed-direction caret positions inside a line are
  approximate. CJK breaks are grapheme-based, not full UAX #14 kinsoku, and
  there is no hyphenation dictionary.
- Autosave rewrites the full document every 4 seconds. There is no incremental
  save, external-change detection or encryption; recovery files are plain
  `.novadoc` under Application Support.
- Startup auto-restores recovery files without a prompt; a save removes that
  tab’s recovery file, but there is no “discard recovery” command.
- IME composition is painted from a temporary document/layout, so it reflows
  while composing but is not part of the document until commit.
- Save As uses a folder picker and never overwrites; choosing an existing file
  name creates a new unique name instead.
- AppKit prints a benign `ApplePersistenceIgnoreState` notice at launch; window
  state restoration is not managed.
