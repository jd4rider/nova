# Nova Writer decisions

## DEC-WRI-001 — Paragraph list with stable IDs and grapheme offsets

Status: accepted.

### Context

Nova has value-semantic structs and lists, no string slicing, no generic
structs and no `dyn Trait`. A giant mutable string is both prohibited by the
directive and impossible to index cheaply in this language today.

### Decision

- A document is a versioned `Document` containing `List<Paragraph>`.
- Every paragraph has a stable integer `id` that is never reused.
- A position is `{ paragraph index, grapheme offset }`; offsets are measured in
  extended grapheme clusters, not bytes or scalars.
- Edits are pure functions over `Document` that return a new `Document`.
- Inline runs are stored as ranges over grapheme offsets, so text remains the
  canonical content and formatting cannot desynchronise it.

### Alternatives rejected

- One mutable string: no stable anchors, quadratic edits, prohibited shape.
- Rope/piece table now: the language lacks the ownership and generic
  containers to implement one safely; paragraph lists are the measured,
  working structure for this slice.
- Byte offsets as the public position: breaks combining marks and emoji, and
  the directive requires grapheme-aware editing.

### Consequences

- Editing a paragraph rebuilds its text from `graphemes()`; acceptable for
  ordinary documents, profiled before any performance claim.
- Paragraph IDs survive edits, so comments/bookmarks can anchor later.
- A rope or piece table can replace the internal text storage without changing
  the public position type.

## DEC-WRI-002 — `.novadoc` v2 is a deterministic text container

Status: accepted for WRI-07.001/WRI-02.005; ZIP container is a later
migration.

### Context

The directive prefers a ZIP package with manifest, document, styles and media.
The tree has gzip/tar `.box` archives but no ZIP central directory and no
deflate. Implementing ZIP+XML+deflate before any document can be saved would
block the first vertical slice and risk a half-valid container.

### Decision

- v2 is a UTF-8, line-oriented container with a magic header, version,
  document metadata including a BCP-47 language, paragraph records and a
  31-based polynomial body checksum. It detects truncation and corruption, not
  tampering; the format makes no cryptographic claim.
- Version 1 files migrate on load: the missing language defaults to `en`, and
  the next save writes v2.
- Records are escaped, bounded and validated on load; unknown record kinds,
  missing v2 fields and unknown versions are rejected rather than silently
  ignored.
- Writes go to `<path>.tmp`, are verified, then renamed atomically.
- A later v3 may wrap the same parts in a ZIP container; the loader dispatches
  on the magic header, so v1 and v2 files remain readable.

### Consequences

- Real save/reopen/migration testing exists now.
- Media and style parts are not packaged yet; they are explicit TODO items, not
  hidden claims.
- DOCX/ODT work still requires a new ZIP/XML Box (`@nova/office-xml`).

## DEC-WRI-003 — Bounded snapshot undo

Status: accepted.

### Context

Correct undo is an acceptance gate. Inverse-edit undo needs careful
composition and is easy to get subtly wrong while the model is still moving.

### Decision

- Each committed transaction pushes the previous `Document` onto a bounded
  history (100 entries) with the selection before/after.
- Redo is a second stack cleared by a new edit.
- Consecutive typing is coalesced into one transaction by the app layer.
- History stores complete documents, not serialized text, so IDs and
  formatting survive.

### Consequences

- Memory is proportional to document size × history depth; bounded and
  measurable.
- The core library stores `Document` snapshots. The Writer app still stores
  bounded serialized snapshots (DEC-WRI-006); the compiler now supports a
  `List<Document>` field, so the app can switch later. Both paths are tested.
- A later slice can replace snapshots with inverse edits behind the same API
  without changing tests.

## DEC-WRI-004 — CoreText is the font backend; Nova owns layout

Status: accepted.

### Context

The host draws a whole line with CoreText but exposes only system/mono families
and no weight or style. Writer needs serif body text, bold, italic and
underline. Writing a shaper in Nova would violate the directive's instruction to
use established text services.

### Decision

- Extend the Nova GUI host with `nova_gui_text_fmt` and
  `nova_gui_text_fmt_width`: family (sans/serif/mono), size, colour, and style
  bits (bold, italic, underline).
- CoreText performs shaping, fallback and glyph metrics; Nova decides line
  breaks, pagination, runs and painting order.
- The host returns only measured advances and draws; it owns no document state.

### Consequences

- Bold/italic/underline become real, not simulated.
- Bidi and complex-script runs are only as good as a single CoreText line;
  explicit bidi paragraph layout is a later WRI-04 task.
- Other Nova apps keep the existing API; the new functions are additive.

## DEC-WRI-005 — No fake controls

Status: accepted.

Every visible command executes real behavior. A command that cannot work yet
is absent from the ribbon, or present but disabled with a visible reason and
recorded in `BLOCKERS.md`. This applies to Save As (no save panel), DOCX
export, PDF export, comments and collaboration until their slices land.

## DEC-WRI-006 — App undo history uses serialized snapshots

Status: accepted as a compiler-limit workaround.

### Context

The app needs per-tab undo history. At the time, the compiler rejected a
`List<Document>` struct field with `unsupported type Document> on field undo`
and index assignment through a struct field. Both were codegen bugs in type
normalization and field writes; WRI-00.004 fixes the qualified-generic part,
and the field-write path now has its own regression.

### Decision

- The app keeps bounded `List<String>` serialized snapshots (100) per tab and
  reparses on undo/redo.
- Session replacement goes through `wr_put`, which copies the list to a local
  variable before index assignment.
- The core `@nova/document` API keeps Document-based history and remains
  independently tested.

### Consequences

- Undo validates the checksum and reparses the document; acceptable for the
  current document sizes and measurable later.
- Snapshot strings preserve IDs, styles and alignment for the life of the
  session.
- The compiler now supports `List<Document>` fields; the app can switch back
  without changing user-visible behavior, but the serialized path remains
  because it is already tested and bounded.

## DEC-WRI-007 — Install each phase to /Applications

Status: accepted (owner requirement).

After every working Writer phase, run `writer/scripts/install-macos.sh` so the
owner can test `/Applications/Nova Writer.app`. Installation is local and
ad-hoc signed; it is not a release, notarization or deployment. Build artifacts
stay out of version control (`writer/.gitignore`).

## DEC-WRI-008 — Autosave is a full `.novadoc` snapshot every 4 seconds

Status: accepted.

### Context

A crash must not destroy unsaved work, and the app has no alert/confirmation
bridge. Recovery files must be inspectable, validated and safe to rewrite while
the document is being edited.

### Decision

- Recovery files live under
  `~/Library/Application Support/Nova Writer/Recovery`, overridable with
  `NOVA_WRITER_RECOVERY_DIR` for tests.
- Names are `recovery-<epoch-ms>-<tab-id>.novadoc`, so a directory listing sorts
  oldest to newest without a stat call.
- Every dirty tab writes atomically (temp, verify, rename) at most every 4000ms;
  the first edit after a save or restore writes immediately.
- Startup loads every valid recovery file (up to six) as a dirty tab, replaces
  the demo/blank tab, and reports “Recovered N unsaved document(s)”.
- Saving removes that tab’s recovery file; closing a clean tab removes it.
  Closing a dirty tab remains blocked until it is saved.
- No confirmation dialog: the host exposes no alert bridge, so the status bar
  makes the restore explicit instead of pretending to ask.

### Consequences

- A crash, force-quit or power loss leaves a valid document that the next launch
  restores; a two-process test and an installed-app restore verify this.
- Autosave cost is one full serialization and write per dirty tab per 4 seconds;
  this must be measured and replaced with an incremental journal for very large
  documents.
- Recovery files are plain local `.novadoc` files, not encrypted. They live in
  the user’s Application Support directory and are removed on save.
- A future WRI-07.005 adds a prompt and an explicit discard command without
  changing the storage format.

## DEC-WRI-009 — The host owns marked text; Writer owns the commit

Status: accepted.

### Context

AppKit delivers an IME composition as `setMarkedText:…` updates followed by
`insertText:…` or `cancelOperation:`. The composition must not enter the
document until it is committed, and a cancelled composition must disappear
without leaving text behind.

### Decision

- `runtime/gui.m` owns the in-progress `NSMutableString` and exposes it through
  `nova_gui_marked_text()` / `gui.marked_text()` as a read-only string.
- Writer paints the composition from a temporary document and layout, so the
  preedit takes real space, wraps and pushes the following text while the saved
  document remains untouched. Its range is underlined in the canvas.
- `insertText:` clears the host store and emits one text event; Writer inserts
  that string once. It does not also insert the marked text.
- `cancelOperation:` clears the host store before emitting Escape; Writer then
  clears its status and leaves the document unchanged.
- `NOVA_GUI_IME_PROBE=1` (or `cancel`) drives the same two entry points for the
  automated test; a real input source remains a manual check.

### Consequences

- The document model never sees uncommitted text, and undo snapshots are not
  polluted by composition updates.
- Composition reflows inline but costs a temporary full layout while active;
  this is acceptable for ordinary documents and measurable later.
- A cancelled composition leaves no recovery or document artifact.
