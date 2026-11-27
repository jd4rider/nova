# Nova Writer evidence

All results below were produced on this host on 2026-11-26 (macOS, Apple
Silicon). Commands are reproducible from the repository root.

## 1. Headless core acceptance

```sh
sh writer/tests/core.sh
```

Result: exit 0; 82 `PASS` lines; final line `failures=0`.

Covers: grapheme segmentation (combining marks, skin tones, flags, ZWJ),
insert/delete, escaping, word count, find, document validation, stable IDs,
split/merge, cross-paragraph delete, styles/alignment, outline, Document-based
undo/redo and the 100-entry bound, `.novadoc` serialize/parse, checksum and
unknown-record rejection, measured layout, line-width bound, caret geometry,
hit-testing, long-word hard break, multi-page pagination, atomic save, reopen,
corrupt-file rejection, and recovery save/load/list/newest/remove/reject.

## 2. Host compile

```sh
clang -fobjc-arc -O2 -c runtime/gui.m -o /tmp/nova-gui-check.o
```

Result: exit 0. Two warnings: `accessibilityPerformPress` conflicts with the
AppKit protocol declaration, and `LSCopyDefaultHandlerForURLScheme` is
deprecated. The first is pre-existing; the second comes from unrelated Browser
work. Neither is introduced by Writer.

## 3. App build

```sh
cd writer && ../examples/nova-tool/target/nova/nova-tool build
```

Result: exit 0; `built .../writer/target/nova/nova-writer`.

## 4. Native window smoke test

```sh
sh writer/tests/smoke.sh
```

Result: exit 0; `PASS writer window rendered /tmp/nova-writer-smoke.png`.

The snapshot shows the real window: A4 page, serif body text, bold headings,
italic quote, ruler, tabs, styles list, outline, caret and status counts
(Words 251, Characters 1485, Page 1/1, Zoom 100%, Saved).

Artifact: `writer/dist/snapshot.png` (2360×1640, 329,789 bytes). `dist/` is
generated and gitignored; regenerate with the smoke test.

## 5. Crash recovery across processes

```sh
sh writer/tests/recovery.sh
```

Result: exit 0.

```
wrote /tmp/nova-writer-recovery-crash/recovery-1795758890442-crash.novadoc
recovered=Crash Report
text=Unsaved work survives a crash.
PASS recovery survives a process exit
```

The `write` process created unsaved work, wrote only the recovery file and
exited without a normal save. A second process found the newest recovery file,
validated its checksum and loaded the title and text.

## 6. App startup restore

```sh
NOVA_WRITER_RECOVERY_DIR=/tmp/nova-writer-app-recovery \
NOVA_WRITER_AUTOCLOSE=1 \
NOVA_WRITER_SNAPSHOT=/tmp/nova-writer-recovered.png \
writer/target/nova/nova-writer
```

Result: exit 0; snapshot 127,530 bytes. The snapshot shows the tab
“Crash Report” with a dirty dot, the recovered body text, and the status
“Recovered 1 unsaved document”. The demo tab was replaced by the recovered
document.

Artifact: `writer/dist/recovered.png` (gitignored, regenerate by seeding a
recovery file and launching with the environment above).

## 7. Autosave writes while the app runs

The recovery file was seeded, its mtime recorded, then the app was launched
with the same recovery directory:

```
before=1795758920 after=1795758922
AUTOSAVE_WROTE=yes
recovered=Crash Report
text=Unsaved work survives a crash.
```

The mtime advanced during the app run, and the rewritten file still passed the
checksum and loaded. This confirms the first autosave after restore, not the
4-second cadence over a long editing session.

## 8. Bundle install

```sh
sh writer/scripts/install-macos.sh
```

Result: exit 0; `Installed /Applications/Nova Writer.app`;
`Version 0.1.0 · dev.nova.writer.native · ad-hoc signed`.

```sh
codesign --verify --strict "/Applications/Nova Writer.app"
```

Result: exit 0 (`SIGN_OK`).

## 9. Installed bundle launch and restore

Normal launch with an empty recovery directory: exit 0; 329,789-byte snapshot.

Installed recovery launch with a seeded crash file: exit 0; 127,530-byte
snapshot showing “Crash Report” and “Recovered 1 unsaved document”. This
proves the installed bundle contains the recovery code, not just the build
tree binary.

Artifact: `writer/dist/recovered-installed.png` (gitignored).

## 10. IME marked-text composition

```sh
sh writer/tests/ime.sh
```

Result: exit 0; 14 `PASS`, 0 `FAIL`.

The host probe drove the same two `NSTextInputClient` entry points AppKit uses:
`setMarkedText:selectedRange:replacementRange:` for the composition and
`insertText:replacementRange:` for the commit. The app logged
`writer: preedit=にほ` while composing, then `writer: committed=日本` and
`writer: paragraph=日本Nova Writer` after the commit. The commit line appeared
exactly once. The preedit snapshot (332,535 bytes) shows “にほ” underlined
inline with “Nova Writer” pushed to the right.

The same test ran a cancellation pass: `ime: cancelled has=0`, then
`writer: escape`, then `writer: composition cleared`, then a second composition
that committed normally. The cancelled text never reached the document.

The installed bundle was also launched with `NOVA_GUI_IME_PROBE=1` and produced
`ime: marked=にほ has=1`, `writer: preedit=にほ`, `ime: committed=日本 has=0`,
`writer: committed=日本` and `writer: paragraph=.*日本`. This proves the
installed app contains the composition path, not only the build-tree binary.

Artifact: `writer/dist/ime-preedit.png` (gitignored).

## 11. Compiler fix: module-qualified generic fields

Before the fix, a Writer field declared as `List<wd.Document>` failed with:

```
error: unsupported type Doc> on field docs
```

`cg_unqualified` split on every dot, so `List<wd.Document>` became
`Document>` and the `List<` wrapper was lost. The fix strips only the module
prefix immediately before each dot, keeping the generic wrapper:

```sh
NOVA_TOOL=emit-ir NOVA_TOOL_PATH=examples/nova-tool/tests/codegen-list-qualified.nova \
  examples/nova-tool/target/nova/nova-tool > /tmp/qualified.ll
clang -x ir /tmp/qualified.ll -x none runtime/runtime.c -o /tmp/qualified -lm
/tmp/qualified
```

Result: IR emitted, clang accepted it, and the program returned 7. The Writer
core test now declares `struct DocStack { docs: List<wd.Document> }`, writes
through `stack.docs[0]`, and passes. The standalone fixture lives at
`examples/nova-tool/tests/codegen-list-qualified.nova`.

## Verified vs not verified

Verified by machine: document model and edits, grapheme boundaries, undo/redo
invariants, serialization/checksum, atomic store, layout measurement and
pagination, hit-testing, recovery save/load/list/remove, a two-process crash,
app startup restore, the first autosave write, IME marked-text composition and
Escape cancellation through the host probe, native window launch and rendering,
bundle signing and installation.

Not machine-verified: interactive typing, selection, clipboard, zoom, theme
switching, save/open through the native panels, a real Japanese/Chinese/Korean
input source (the probe drives the same `NSTextInputClient` entry points),
accessibility navigation, the 4-second autosave cadence over a long edit
session, and recovery cleanup after a save. These are wired to real code but
require manual or future automated GUI tests; they are not claimed as tested.

Not implemented: DOCX/ODT/RTF/HTML/PDF, tables, images, headers/footers,
footnotes, inline runs, comments/revisions, recovery prompt/discard UI,
external-change detection, encryption, performance baselines.
