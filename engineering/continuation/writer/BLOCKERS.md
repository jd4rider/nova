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
| No app-defined native menu model | only the fixed host app menu; commands live in the ribbon | host work |
| No ZIP central directory/deflate | DOCX/ODT/native ZIP container blocked | WRI-10 prerequisite |
| No XML parser | OOXML/ODF blocked | WRI-10 prerequisite |
| No PDF writer | PDF export/print blocked | WRI-11 |
| No string slice/insert | editing rebuilds paragraphs from graphemes | DEC-WRI-001 |
| No accessibility text ranges | screen readers see a document label, not ranges | WRI-03.005 |
| One window, at most six visible tabs | no multi-window or tab overflow | WRI-01.002 |
| No automated GUI input harness | typing/selection/clipboard are not machine-verified | WRI-01 test harness |
| No generic structs / `dyn Trait` | flat paragraph model, static dispatch | compiler parity work |

## Resolved

- **No font family/weight API.** Styled CoreText runs and measured advances
  were added (DEC-WRI-004), so serif body, bold, italic and underline are real.
- **No native test target.** A headless core binary with 156 assertions exists.
- **No installable bundle.** `/Applications/Nova Writer.app` builds, signs and
  launches.
- **No autosave or crash recovery.** `recovery.nova`, a two-process crash test
  and an installed-app restore now cover dirty documents.
- **No IME marked text.** The host exposes `marked_text()` and clears it on
  commit or Escape; Writer paints the composition inline and commits once.
  `writer/tests/ime.sh` passes 14 assertions.
- **No bidi direction or CJK fixture.** `.novadoc` v4 stores per-paragraph
  auto/LTR/RTL direction; the core suite covers RTL alignment/caret geometry,
  mixed-script fallback and CJK wrapping.
- **No inline formatting.** `.novadoc` v4 stores bold/italic run ranges;
  insert/delete/split/merge preserve them and the core suite covers them.
- **No incremental layout.** `layout.nova` caches measured paragraphs by ID and
  content; only changed/new paragraphs are reshaped. An 80-paragraph benchmark
  measured a one-paragraph edit at 56ms vs 2,265ms full.
- **No native save panel.** The host exposes `nova_gui_save_panel`; Writer's
  Save As uses the panel URL and suggested `.novadoc` name, and
  `writer/tests/save-panel.sh` covers confirm and cancel.
- **No alert/confirmation bridge.** The host exposes a native three-button
  alert; Writer asks before restoring, keeps snapshots on dismissal, and has a
  conditional discard command. `writer/tests/recovery-prompt.sh` covers all
  four paths.

## Known correctness limits

- Snapshot undo is bounded at 100 transactions and is memory-heavy for very
  large documents; the app reparses on undo.
- Pagination is single-column A4 with fixed margins; sections, columns,
  headers/footers and footnotes are not implemented. The layout cache avoids
  reshaping unchanged paragraphs but still copies every line box per pass.
- CoreText shapes bidi runs, but caret and selection geometry use one
  paragraph direction; mixed-direction caret positions inside a line are
  approximate. Inline runs are painted as separate CoreText segments, so
  kerning across a style boundary and mixed bidi plus inline styling are
  approximate. CJK breaks are grapheme-based, not full UAX #14 kinsoku, and
  there is no hyphenation dictionary.
- Autosave rewrites the full document every 4 seconds. There is no incremental
  save, external-change detection or encryption; recovery files are plain
  `.novadoc` under Application Support.
- The recovery prompt is modal and restores at most six valid snapshots.
  Unreadable snapshots and valid files beyond that limit stay pending for the
  explicit discard command. Saving a restored tab removes its file.
- IME composition is painted from a temporary document/layout, so it reflows
  while composing but is not part of the document until commit.
- AppKit prints a benign `ApplePersistenceIgnoreState` notice at launch; window
  state restoration is not managed.
