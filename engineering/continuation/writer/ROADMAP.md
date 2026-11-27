# Nova Writer roadmap

Status vocabulary: `TODO`, `READY`, `IN_PROGRESS`, `PARTIAL`, `BLOCKED`,
`VERIFIED`, `DEFERRED`. Only `VERIFIED` means the acceptance evidence passed.

The directive's milestone IDs are stable. A milestone is a backlog container,
not a single session.

| ID | Deliverable | Status | Depends on |
|---|---|---|---|
| WRI-00 | Ecosystem audit and architecture | VERIFIED | — |
| WRI-01 | Native shell: window, command surface, tabs, harness | PARTIAL | WRI-00 |
| WRI-02 | Document core: typed tree, stable ranges, transactions, undo | PARTIAL | WRI-00 |
| WRI-03 | Text editing: Unicode, selection, clipboard, IME, caret | PARTIAL | WRI-02 |
| WRI-04 | Typography: shaping, fallback, bidi, line-break fixtures | PARTIAL | WRI-03 |
| WRI-05 | Paginated layout: pages, margins, zoom, hit-testing | PARTIAL | WRI-04 |
| WRI-06 | Styles and formatting: named styles, inheritance, lists | PARTIAL | WRI-05 |
| WRI-07 | Native persistence: save/open, migration, autosave, recovery | PARTIAL | WRI-02 |
| WRI-08 | Tables and media | TODO | WRI-06 |
| WRI-09 | Sections and publishing: headers, footers, notes, TOC | TODO | WRI-08 |
| WRI-10 | DOCX import/export | TODO | WRI-07, WRI-09 |
| WRI-11 | PDF and printing | TODO | WRI-05 |
| WRI-12 | ODT, RTF, Markdown, HTML, text with loss reports | TODO | WRI-10 |
| WRI-13 | Review tools: comments, revisions, accept/reject | TODO | WRI-09 |
| WRI-14 | Professional UX: ribbon, panels, templates, shortcuts | PARTIAL | WRI-01 |
| WRI-15 | Language tools: spellcheck, search, dictionaries, counts | TODO | WRI-03 |
| WRI-16 | Advanced publishing: equations, shapes, citations, merge | TODO | WRI-09 |
| WRI-17 | Automation: native API, headless CLI, permissioned plugins | TODO | WRI-02 |
| WRI-18 | Optional collaboration | DEFERRED | WRI-17 |
| WRI-19 | Hardening: fuzz, recovery, performance, accessibility | TODO | WRI-10..WRI-17 |
| WRI-20 | Distribution: signed/notarized release where credentials exist | PARTIAL | WRI-19 |
| WRI-21 | Portability: Windows/Linux adapters and builds | DEFERRED | WRI-20 |

## Current slice

The vertical slice, autosave/crash recovery and IME marked-text composition are
landed and installed: a native Writer window over a GUI-independent document
core with measured pagination, editable paragraphs, bounded undo, a checksummed
`.novadoc` round trip, recovery files that survive a process crash, and inline
composition that commits once or cancels on Escape.

Next: **WRI-02.005 schema migration** — only `.novadoc` v1 exists; the loader
dispatches on the version header. Then WRI-04.002 bidi fixtures, WRI-07.004
save panel, WRI-07.005 recovery prompt/discard, WRI-06 inline runs.

## Critical path

WRI-00 → WRI-02 model/transactions → WRI-07 save/open → WRI-03 editing →
WRI-04 typography → WRI-05 pagination → WRI-01 shell integration → WRI-06
styles → WRI-09 sections → WRI-10 DOCX → WRI-11 PDF → hardening.

## Gate policy

- A milestone is VERIFIED only with a command, result and artifact/fixture.
- A visible command either executes real behavior or is absent/disabled with a
  reason. No placeholder buttons.
- Platform GUI checks run locally on this host; absent CI GUI runners are
  recorded as unverified, never as passed.
