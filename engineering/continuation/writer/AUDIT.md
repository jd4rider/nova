# Nova Writer ecosystem audit

Date: 2026-11-26
Platform: macOS ARM64, Apple clang, checked-in Nova bootstrap
Repository: `nova`
Scope: what actually exists in this tree that Writer can reuse, and what is
missing. Names and APIs below were read from source, not assumed.

## Toolchain

| Item | State | Evidence |
|---|---|---|
| Self-hosted compiler | present | `bootstrap/nova` is a Mach-O arm64 executable; `nova build <file> <out>` compiles and links natively |
| LLVM/Clang pipeline | present | driver emits IR and invokes clang; GUI programs link `runtime/gui.m` and Cocoa/CoreText |
| Test harness | present | `nova test` discovers `@test` functions; headless project binaries also work (browser tests) |
| Modules | present | relative `.nova` imports, extensionless modules, path dependencies |
| Language limits | real | no generic structs, no `dyn Trait`, no string slicing; `graphemes()` availability is verified by `writer/probe.nova` |

## GUI host (`runtime/gui.m`, `libraries/nova-gui`)

| Capability | State | Notes |
|---|---|---|
| One native window | verified | `gui.open/close/width/height`, resize events |
| Immediate-mode framebuffer | verified | rect, rounded rect, stroke, line, clip, present |
| Text drawing | verified | CoreText `CTLine`; sans/serif/mono families, size, colour, bold/italic/underline via `nova_gui_text_fmt` |
| Text measurement | verified | `gui.text_width`, `gui.mono_width`, `gui.advance` |
| Input polling | verified | key, key-up, mouse down/up/move/drag, scroll, resize, close, repaint |
| Composed text / IME | verified | `event_text`, `typed_text`, `marked_text`; Writer paints/commits/cancels marked text and `writer/tests/ime.sh` passes 14 assertions |
| Clipboard | verified | system clipboard get/set |
| Accessibility tree | verified | pushed elements, roles, labels, values, selection; `AXPress` maps to click |
| Filesystem | verified | read/write text and bytes, exists, is_dir, mkdir, list, cwd, sha256 |
| Native open panel | verified | `gui.pick_file`, `gui.pick_folder` (open only) |
| Native save panel | verified | `gui.save_panel(name, dir)` wraps `NSSavePanel`; Writer Save As confirm/cancel passes in `writer/tests/save-panel.sh` |
| Native alert | verified | `gui.alert(title, message, primary, secondary, tertiary)` wraps a three-button `NSAlert`; Writer's recovery prompt passes recover/keep/discard and the explicit discard command |
| Native application menu | partial | fixed app/file menu and tab context menu; no app-defined menu model |
| Appearance | verified | light/dark switch for native chrome |
| Multi-window | **missing** | one window per process |

## Text and Unicode

| Capability | State | Notes |
|---|---|---|
| UTF-8 `String` | verified | `len` is bytes, `chars()` scalars, `graphemes()` clusters |
| Grapheme splitting | runtime exists | `nova_string_graphemes`; exposed as `graphemes()` in Nova (probe verifies) |
| String editing primitives | **missing** | no slice/insert/delete; editing must rebuild from `graphemes()` + `join("")` |
| Shaping / bidi | via CoreText only | host draws a complete line; Nova cannot query glyph runs, bidi levels or fallback per run |
| Line breaking | **missing** | `String.wrap` is terminal-column based, not document typography; Writer implements measured wrapping |
| Font discovery | **missing** | only monospaced family enumeration exists |

## Archives, XML, PDF

| Capability | State | Notes |
|---|---|---|
| `.box` archive | verified | gzip stream of stored deflate blocks around ustar tar, CRC32, path traversal refusal |
| ZIP container | **missing** | no ZIP central directory or deflate; DOCX/ODT packages need a new Box |
| Deflate (real compression) | **missing** | stored blocks only; `.box` does not shrink |
| XML parser | **missing** | no tokenizer, DOM or namespace handling |
| PDF writer | **missing** | no PDF objects, fonts or content streams |
| RTF | **missing** | none |
| HTML parsing | **missing** | `nova-web` builds trusted HTML strings only |
| Markdown | **missing** | no parser |

## Reusable Nova code

- `libraries/nova-gui` — window, drawing, input, fs, clipboard, accessibility.
- `libraries/nova-boxes/src/archive.nova` — byte helpers, CRC32, tar/gzip, safe path checks. Writer's package code should reuse the byte/CRC ideas, not duplicate blindly.
- `libraries/nova-fmt` — source text scanning patterns for line/word work.
- `browser/app/src/main.nova` — a working native app loop: tabs, hit-testing, accessibility, keyboard routing, theme tokens, snapshot testing.
- `browser/tests/*` — headless test project layout and shell harness style.
- `examples/nova-studio/src/main.nova` — text buffer editing with graphemes, caret, selection and line model.

## What Writer must add

1. A GUI-independent document model with stable paragraph/block IDs and
   grapheme offsets.
2. Transactional edits, bounded undo/redo and structural validation.
3. A versioned native document format with save/open and migration.
4. A measured line-breaking and pagination engine with hit-testing and caret
   geometry.
5. A document canvas: paper, margins, rulers, zoom, scrolling, selection and
   caret.
6. Font family/weight/style drawing in the Nova host (CoreText), used by the
   canvas but owned by Nova layout decisions.
7. Later: ZIP/XML for DOCX/ODT, PDF export, tables, images, comments,
   revisions, spellcheck.

## Deliberately absent substitutes

- No Electron, browser contenteditable, embedded LibreOffice/Word or hidden
  office engine.
- No Rust/Cargo/Node/Python dependency for Writer or its core.
- No claim of DOCX/PDF support before adapters and fixtures exist.

## Baseline commands run for this audit

| Command | Result |
|---|---|
| `git status`, `git log` | working tree has unrelated Browser/engine changes; preserved |
| `./bootstrap/nova build writer/probe.nova /tmp/nova-writer-probe` | see `EVIDENCE.md` |
| source reads of `runtime/gui.m`, `runtime/runtime.c`, `libraries/nova-gui`, `libraries/nova-boxes` | APIs above |
