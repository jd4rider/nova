# Nova Browser checkpoint

Updated: 2026-11-26
Branch: `main`
Last known good compiler checkpoint: `cfa00e3` (before browser work)
Browser: **0.7.6 installed at `/Applications/Nova Browser.app`**
Browser working tree: uncommitted; see `git status`
Active gate: G0 WebKit; G1 WebCore not started
Active task: **WB-04.002 — downloads** (0.7.5 persisted completed downloads;
0.7.6 lets the shelf forget them)

## Completed

- Audited Nova GUI, FFI, WebKit availability, networking, Boxes, LSP/Studio and
  build linking; recorded `AUDIT.md`.
- Added `runtime/webkit.m` (thin WKWebView host), `nova_gui_native_view()`,
  `libraries/nova-webview`, `browser/engine-api` and the WebKit adapter.
- Added a native Nova GUI shell, trusted offline fixture and headless lifecycle.
- Extended the compiler linker to compile/link `webkit.m` and WebKit only when
  `nova_webview_` is referenced.
- Added composed UTF-8 text events, an editable address bar, URL normalization,
  clipboard editing, page/chrome focus handoff and live Cmd+L evidence.
- Added per-tab WKWebView instances, tab strip, create/close/select, closed-tab
  restore, visibility switching, navigation generations and active-navigation
  stale-callback guards.
- Added a repeatable macOS bundle installer: icon render, `.icns`, plist,
  resource fixture, ad-hoc signature, `/Applications` copy.
- **0.4.0 — daily-driver chrome pass**
  - `gui.now()` monotonic clock and eased hover/focus energies (≈100 ms),
    pointer tracking, hover on tabs/close/buttons/suggestions, blinking caret,
    loading bar driven by WebKit's `estimatedProgress`, stop-instead-of-reload
    while loading.
  - Address autocomplete: `browser/history` ranks local bookmarks and visits for
    the typed prefix, Up/Down/Enter/Tab/Escape, click to open, scroll to move the
    highlight. Proportional address text instead of monospace.
  - `browser/history`: escaped tab-separated profile records, bounded history,
    bookmark identity/dedupe, pure ranking, Chrome JSON and Netscape HTML import.
  - Google import: Cmd+Shift+I and `--import [path]`, idempotent, temp-profile
    tested; Cmd+D toggles a bookmark with a notice.
  - Switchable horizontal/vertical tabs (Cmd+Shift+E, toolbar toggle, persisted
    in `settings.tsv`); page rectangle and hit boxes follow the layout.
  - WB-03.003 error surface: page view hidden on failure, Nova-drawn panel with
    category, WebKit's description, attempted URL, Try Again/Back, and an
    explicit no-certificate-bypass statement. `browser/status` classifies
    offline/dns/certificate/unreachable/blocked.
  - Failure snapshot and both layouts captured for visual review.
- Verified address 25/0, library 0 failures, lifecycle 25/0, `--import` twice
  (new=2 then new=0), all three layouts/panels snapshot, installed bundle.
- **0.5.0 — downloads, session restore, opt-in search, faster frames**
  - Real `WKDownload` support: `WKDownloadDelegate` in `runtime/webkit.m`, a
    response policy that turns `Content-Disposition: attachment` and
    non-displayable MIME types into downloads, and a polled registry
    (name/path/received/expected/state). Nova draws the shelf: per-row name,
    byte progress, cancel while running, reveal when finished, dismiss.
  - End-to-end test `browser/tests/downloads.sh`: loopback HTTP server sends an
    attachment; WebKit writes it; the file is 5120 bytes. `--download <url>`
    runs the same path headless.
  - Session restore: `session.tsv` records navigable tabs plus the active index;
    restart reopens them (`NOVA_BROWSER_NO_RESTORE=1` opts out). Only allowed
    http(s)/about URLs are recorded.
  - Opt-in search: settings key `search` cycles off → DuckDuckGo → Google →
    Brave with Cmd+Shift+S. While off, plain text is still rejected rather than
    guessed. Queries are percent-encoded in UTF-8.
  - Performance: `fx_step` is a single allocation-free pass; tab titles are
    cached for 500 ms; frames are skipped while nothing changed, so an idle
    window only polls events. `NOVA_BROWSER_BENCH=N` reports frame cost.
  - Downloads button, scheme dot, tooltips and byte formatting follow the
    existing chrome language; start fixture rewritten with the 0.5.0 keys.
- **0.5.1 — address selection geometry**
  - The select-all chip was 8 px left of the text and ignored the field's right
    padding; it now starts on the first glyph and clamps to the text area.
  - Gruvbox's `selection` token equals the border colour, so the chip read as a
    border. It is blended towards the blue and the text stays bright.
  - Long URLs: the field scrolls with the caret (select-all keeps the start
    visible). Ink is clipped 16 px from the field edge while the chip runs 4 px
    past that, so the last glyph never sits on the chip's rounded corner -- the
    remaining "unhighlighted character" was the final glyph touching, and in
    the narrow-window case spilling past, the chip edge.
  - Clicking the field now places the caret at the nearest character boundary
    instead of always selecting all; Cmd+L still selects everything.
  - Visual-check hook `NOVA_BROWSER_FOCUS_ADDRESS` (1 = current URL, other
    value = seed text) makes the selected state snapshot-testable.
- **0.5.2 — address select-all tail**
  - A URL wider than the field now scrolls to show its end when everything is
    selected (Chrome behaviour), so the tail is no longer cut mid-glyph at the
    ink limit. The chip spans the visible run in that case instead of hanging
    off the left edge.
- **0.6.0 — editing, IME, tabs**
  - `Address` now carries an `anchor`, so a selection is a real range instead of
    a boolean. Typing, Backspace and Delete replace the selected text; arrows
    collapse to an edge; Cmd+L selects everything.
  - Because there is a range, double-click selects the word (a separator run or
    the token around it) and triple-click selects the URL. Option+Left/Right
    jump by token boundary, Option+Backspace/Delete delete one.
  - IME: `NovaGuiView` is an `NSTextInputClient`; unmodified keys go through
    `interpretKeyEvents:`, marked text is held in the host and painted after
    the caret with a rule under it, and the candidate window anchors to the
    toolbar. Editing selectors are translated back to Nova key codes, and
    word selectors re-emit Option+arrow so the app's model stays authoritative.
  - Tabs reorder by drag (4 px threshold, orange insertion marker at the drop
    slot) and the horizontal strip scrolls to keep the active tab visible once
    tabs no longer fit, with the strip clipped at both ends.
  - Performance: the merged history/bookmark list and its ranking were rebuilt
    every frame while the field was open; they now rebuild only when the query
    or the store changes. `address_caret_at` no longer re-measures every prefix
    (quadratic on long URLs).
- **0.6.1 — launch hang after a crash**
  - AppKit's persistent-UI restorer raised a modal "restore windows?" alert
    after the earlier crash. It runs a nested loop behind the window, so the
    app looked hung and never reached its first frame.
  - Persistent UI is now off (`ApplePersistenceIgnoreState` set before
    `NSApplication` exists), any `dev.nova.browser.native.savedState` is removed
    at start, the window is not restorable, `applicationShouldHandleReopen:`
    answers `NO`, and save/restore state both return `NO`.
  - A 0.25 s modal guard aborts a system modal that would otherwise freeze the
    loop; the app's own `NSOpenPanel` sets a flag so it is never dismissed.
  - Installed launch: 1.2 s and 0.9 s, `state=ready`, twice in a row (was: no
    exit after 30 s).
- Verified 0.5.0: address 38/0, library 72 assertions 0 failures, lifecycle
  25/0, downloads 5120/5120 bytes, `--import` 2→0→1, bench 120 frames avg 8 ms
  (worst 203 ms first frame), `fx_step_2000` 4 ms; installed 0.5.0 plist,
  valid ad-hoc signature, icon present, horizontal/vertical launch snapshots.

- **0.7.0 — IME automation, tab menu, stack headroom**
  - `tests/ime.sh` drives the two `NSTextInputClient` entry points AppKit uses
    (`setMarkedText:…`, `insertText:…`) behind `NOVA_BROWSER_IME_PROBE` and
    asserts the marked text, Nova's preedit trace and the committed text in the
    address field. 4 PASS.
  - Tab context menu: right-clicking a tab (both layouts) asks the host for a
    native `NSMenu` with New Tab, Duplicate Tab, Close Tab and Close Other Tabs;
    the choice is polled like every other input event. Duplicate and Close
    Others are implemented in the tab model and covered by `tests/tabs.sh`
    (5 PASS), including the menu's item set and disabled state.
  - Stack headroom instrumentation (`nova_gui_stack_used/peak/headroom`) and a
    periodic trace line; `NOVA_BROWSER_BENCH` reports the peak. The 03:29 crash
    report's fault address sits below the 8 MB stack with only three Nova
    frames, and no function reserves more than 64 KB, so the loop's own usage is
    measured: 13 KB peak at 120 and at 2000 frames.
  - Bench `avg=4ms worst=179ms fx_step_2000=3ms stack=13KB/8176KB`.
  - Verified: address 38/0, history 72/0, downloads PASS, lifecycle 0 failures,
    tabs 5/5, IME 4/4; installed 0.7.0, launches 1.3 s and 1.0 s, signature
    valid, plist 0.7.0.

- **0.7.1 — grapheme-aware address editing**
  - Cursor positions stay scalar offsets, but keys and clicks now move on
    extended grapheme-cluster boundaries: Left/Right, Backspace/Delete, click
    carets and word snapping all stop on a cluster start. One Backspace removes
    "e"+U+0301, "👨‍👩‍👧", "🇺🇸" and "👍🏽" whole instead of one scalar.
  - `browser/address` owns the segmentation: the bundled self-hosted compiler
    has no `graphemes()` method, so clusters are grouped from `chars()` using
    the runtime splitter's rule set, exposed as `cluster_starts`,
    `cluster_count`, `char_left`, `char_right` and `snap`. 18 new assertions,
    56 address PASS total.
  - Segmentation covers combining marks (U+0300–036F, U+1AB0–1AFF,
    U+1DC0–1DFF, U+20D0–20FF, U+FE20–FE2F), variation selectors, keycaps, emoji
    skin tones, regional-indicator pairs and ZWJ sequences. Indic conjuncts,
    Arabic/Hebrew vowel marks and Hangul jamo are not joined, matching the
    runtime.

- **0.7.2 — chrome accessibility tree**
  - `NovaGuiView` now reports itself as an `AXGroup` and builds
    `NSAccessibilityElement` children from a list the app pushes while drawing.
    Tabs carry `AXTab` with `selected`, the address field is an `AXTextField`
    with the current URL as its value, and the back/forward/reload/star/
    downloads/docs/layout/new-tab controls plus the `WEBKIT` badge are exposed
    as buttons and static text. Frames are converted from the top-left drawing
    space to screen coordinates.
  - `AXPress` on any element emits the click event at its centre, so VoiceOver
    reuses the existing hit-testing instead of a parallel action path.
  - `NOVA_GUI_AX_DUMP=1` walks the same children AppKit receives and prints
    them once; `tests/accessibility.sh` checks the 11 elements, their roles,
    selection, value, non-empty frames and that the dump stays opt-in.
  - The bundled compiler miscompiles an extern call taking two strings plus a
    trailing integer (the second string and the integer arrive as garbage), so
    value and selection are attached by separate one-argument calls.

- **0.7.3 — download pause/resume**
  - Clicking a running row pauses it: the partial file stays and WebKit's
    `cancel:` completion supplies resume data. Clicking the paused row calls
    `WKWebView.resumeDownloadFromResumeData:`, which re-enters the delegate and
    keeps writing into the same destination instead of allocating a new name.
    A failed transfer with resume data retries the same way.
  - Rows whose origin sent no byte-range support report "cannot resume" rather
    than silently restarting from zero. The registry keeps paused and resumable
    entries when finished ones are cleared.
  - The download entry holds the originating `WKWebView` strongly because
    `WKDownload.webView` is nil when a main-frame navigation becomes a
    download; it is released on finish.
  - `--download` with `NOVA_BROWSER_DOWNLOAD_PAUSE=1` is the automated probe:
    pause at the first bytes, wait for resume data, resume, require the full
    file. `tests/downloads.sh` serves 51,200 bytes over a Range-capable,
    validator-bearing loopback server and also checks the collision name.

- **0.7.4 — shelf and suggestion accessibility**
  - The accessibility tree now follows the whole chrome: the open download
    shelf exposes its close control, the empty-state message, and per-row
    Pause/Resume/Retry rows plus Cancel/Show-in-Finder buttons named with the
    file; autocomplete rows are `AXLink` children whose label is the title or
    URL and whose value is the target URL.
  - `NOVA_BROWSER_DOWNLOADS_OPEN=1` opens the shelf before the first frame so
    the empty shelf's controls are testable without a live transfer.
  - `tests/accessibility.sh` is now 16 checks: base chrome 11 elements, the open
    shelf (13 children), and a seeded bookmark producing exactly one link row
    with its URL, plus the opt-in guard.

- **0.7.5 — download history across restart**
  - A completed transfer is written once to `<profile>/downloads.tsv` as
    `bytes<TAB>when<TAB>escaped path<TAB>escaped name`, both from the frame loop
    and from the `--download` path, which exits before the loop.
  - On launch the last 20 completed files return as shelf rows with a negative
    ident, so they can never collide with a live WebKit download; a history row
    hides while a live row with the same path is on screen. Their rows and
    Show-in-Finder controls are named in the accessibility tree.
  - Clicking a completed row or its action reveals the file with
    `NSWorkspace`, and reports `This file is no longer there` when it has been
    moved or deleted instead of opening an empty Finder window.
  - `tests/downloads.sh` now also relaunches the browser on the same profile
    with the shelf open and requires both completed names, both Finder actions
    and exactly two persisted rows with the real destination path.

- **0.7.6 — clearing finished downloads**
  - The shelf header shows **Clear** while the profile holds completed files. It
    drops WebKit's finished rows and the persisted record together, so the next
    frame cannot re-record what was just cleared, and writes the emptied profile
    at once; `NOVA_BROWSER_CLEAR_DOWNLOADS=1` exposes the same path to tests.
  - The control is a named `Clear finished downloads` button in the
    accessibility tree. `tests/downloads.sh` requires the button after a
    restart, then relaunches with the hook and requires `downloads.tsv` to be
    empty.

## Limits

- Tabs are capped at 6; drag/reorder, overflow scrolling and a right-click
  menu (new/duplicate/close/close-others) exist, but there are no tab groups,
  pinning or reopen of a dragged stack across restart beyond `session.tsv`.
  Adversarial stale-callback/focus stress remains WB-03.002.a.
- Address editing moves on grapheme clusters for combining marks, variation
  selectors, keycaps, skin tones, flags and ZWJ sequences, but not Indic
  conjuncts, Arabic/Hebrew vowel marks or Hangul jamo; Escape/reload still have
  no automated coverage (WB-03.001.a).
- Import covers bookmarks only; Chrome history is SQLite and is not imported.
  No Firefox/Safari live-profile import (Netscape HTML export works for all).
- Downloads pause/resume only when the origin supports byte ranges and WebKit
  produced resume data; otherwise the row says it cannot resume. Collisions get
  `-1`, `-2` … automatically — there is no rename prompt — and there is no
  download history or queue across restarts. Session restore reopens URLs but
  not back/forward stacks, scroll offsets or form state.
- Search has no suggestion feed or per-provider settings UI; only the three
  built-in providers, cycled with Cmd+Shift+S.
- No private windows, permissions, find, zoom, inspector, Developer ID
  signing/notarization or release. The chrome accessibility tree covers tabs,
  the address field, toolbar buttons, the engine badge, the open download shelf
  and autocomplete rows; notices, tooltips and the failure panel's body text are
  not exposed, `AXPress` is the only action, and no VoiceOver session was driven
  by hand.
- `loadHTMLString` fixtures do not create back/forward entries. Real http(s)
  history is not smoke-tested.
- WebCore is not implemented. No WebCore path delegates to WebKit.

## Exact next action

0.7.2–0.7.5 exposed the whole chrome to assistive clients, made downloads
pausable/resumable and kept completed transfers across restart (see `TASKS.md`).
Pick the next slice: a hand-driven VoiceOver pass plus notices/tooltips in the
tree (WB-03.004), per-row removal instead of one Clear for all finished files,
or begin WC-01.001 with the trusted fixture harness. WebCore stays unstarted and
must not borrow WebKit evidence.

## Install / verify

After every completed browser phase:

```sh
sh browser/scripts/install-macos.sh
open -a "/Applications/Nova Browser.app"
```

The bundle is ad-hoc signed for local testing, not Developer ID signed or
notarized. It is rebuilt rather than silently updated in place.

```sh
sh browser/tests/address.sh
sh browser/tests/history.sh
sh browser/tests/tabs.sh
sh browser/tests/ime.sh
NOVA_BROWSER_LIFECYCLE_ONLY=1 sh browser/tests/webkit.sh
sh browser/tests/webkit.sh
```

Visual evidence (chrome only; native page subviews are not in the framebuffer):

```sh
NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_UI_SNAPSHOT=/tmp/nova-ui.png \
  browser/target/nova/nova-browser
NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_LAYOUT=vertical \
  NOVA_BROWSER_UI_SNAPSHOT=/tmp/nova-ui-vertical.png \
  browser/target/nova/nova-browser
```

## Resume protocol

On `keep going`, read `AGENTS.md`, `DIRECTIVE.md`, this file, `TASKS.md`,
`DECISIONS.md`, `BLOCKERS.md` and `EVIDENCE.md`; inspect `git status` and the
last session; reproduce the active baseline; implement the next slice. Do not
replan from scratch, do not ask the owner to restate the directive, and do not
claim WebCore progress from WebKit evidence.
