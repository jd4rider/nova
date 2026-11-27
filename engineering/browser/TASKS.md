# Nova Browser tasks

Statuses: done / in-progress / ready / blocked / planned.
Each task needs a reproducible acceptance check before it is marked done.

## WebKit track

- **WB-00.001 — Audit.** DONE. Evidence: `AUDIT.md`; all 11 compiler suites;
  bridge/GUI compile; address 25/0; lifecycle 25/0.
- **WB-01.001 — Engine contract.** PARTIAL. Primitive accessors are still used,
  but the compiler blocker is fixed (2026-11-26): qualified imported struct
  signatures resolve and `scene-gui` builds. Migrating the contract to typed
  structs is ready work.
- **WB-02.001 — Native view handoff.** DONE.
- **WB-02.002 — WKWebView creation/destruction.** DONE.
- **WB-02.003 — Navigation state.** DONE for local fixtures.
- **WB-02.004 — Trusted fixture rendering.** DONE. Snapshot 2360x1432.
- **WB-02.005 — Scheme policy.** DONE. `mailto:` fails closed.
- **WB-02.006 — Page/chrome focus handoff.** DONE. Cmd+L from page focus and
  composed text input are verified live.
- **WB-02.007 — Installed macOS bundle.** DONE. Version 0.3.0 at
  `/Applications/Nova Browser.app`; 1024px `.icns`, resource fixture, plist,
  ad-hoc signature and installed-binary launch verified.
- **WB-03.001 — Editable address bar.** DONE for typing/loading/focus. Address
  25/0; live Cmd+L/paste/Enter navigated exactly once.
- **WB-03.001.a — Address hardening.** PARTIAL. Search mode, percent encoding,
  anchor selection, word editing, double/triple click, IME marked text and
  grapheme-cluster editing landed: arrows, Backspace/Delete, click carets and
  word snapping stop only on cluster boundaries, so combining accents, ZWJ emoji,
  flags and skin-tone modifiers are never split (address 56 PASS). A real CJK
  converter, Escape/reload automation and accessibility semantics remain.
- **WB-03.002 — Tabs and closed-tab restore.** DONE. Six-tab cap, one WKWebView
  per tab, create/close/select, Cmd+T/W/Shift+T/1–6, closed-tab restore,
  navigation generation and active-navigation guard. Evidence: lifecycle 25/0
  and live trace: create count=2, close count=1, restore count=2.
- **WB-03.002.a — Tab hardening.** PARTIAL. Restart session restore, overflow
  scrolling, drag/reorder and the tab context menu (New Tab, Duplicate Tab,
  Close Tab, Close Other Tabs) landed in 0.6.0/0.7.0. 0.8.5 adds the three tab
  layouts (top, sidebar, rail), New Tab under the vertical list, the hover
  reveal and a chevron that pins the sidebar, with the layout persisted.
  Adversarial stale-callback/focus stress still has no test.
- **WB-03.003 — Error and offline UI.** DONE. Nova-drawn panel, page view
  hidden while failed, category + WebKit detail + attempted URL, Try Again/Back,
  no certificate bypass. Evidence: failure snapshot, library failure-wording
  assertions, `browser/status`.
- **WB-03.005 — Chrome polish and motion.** DONE. `gui.now()` easing, hover on
  tabs/close/buttons/suggestions, blinking caret, WebKit-driven loading bar,
  stop button, persisted tab layout. Evidence: horizontal/vertical snapshots.
  0.8.5 lays the controls on an 8 px grid around the field and replaces font
  glyphs with line-drawn marks so no icon depends on font coverage.
- **WB-03.004 — Accessibility/IME/keyboard.** PARTIAL. `NovaGuiView` is a full
  `NSTextInputClient`; unmodified keys go through `interpretKeyEvents:`, editing
  selectors map back to Nova key codes, marked text is drawn under the caret,
  and the marked/commit pair is exercised by `tests/ime.sh`. 0.7.2 adds the
  chrome accessibility tree: the app pushes the controls it just drew and the
  host exposes them as `NSAccessibilityElement` children with roles, labels,
  values, per-tab selection and screen frames; pressing an element emits the
  ordinary click at its centre. The open download shelf publishes its close
  control, empty-state message and per-row Pause/Resume/Cancel/Show controls,
  and autocomplete rows are links carrying their URL as the value;
  `tests/accessibility.sh` walks the same children AppKit gets (16 checks across
  the base chrome, an open shelf and a seeded suggestion). Still open: no
  VoiceOver session driven by hand and no automated Escape/reload.
- **WB-04.001 — History and bookmarks.** DONE. Escaped line store, 400-visit
  bound, bookmark identity/dedupe, local autocomplete ranking, star toggle,
  `NOVA_BROWSER_NO_HISTORY=1` separation. Evidence: library suite 0 failures.
- **WB-04.003 — Google bookmark import.** DONE. Chrome JSON and Netscape HTML,
  per-profile discovery, Cmd+Shift+I, `--import`, idempotent temp-profile test.
  0.8.5 discovers Chromium, Brave, Edge, Arc and Vivaldi profiles too, since all
  save the same `Bookmarks` JSON; Firefox/Safari come through the HTML export.
  No history import from any browser (SQLite/WebKit stores).
- **WB-04.002 — Downloads.** DONE. `WKDownloadDelegate` registry, attachment/
  MIME response policy, Nova shelf with cancel/reveal, `--download`, loopback
  test writing a real file. 0.7.3 adds pause/resume: clicking a running row
  keeps the partial file and WebKit's resume data, and clicking the paused (or
  failed-with-resume-data) row resumes through
  `WKWebView.resumeDownloadFromResumeData:`. Rows whose origin cannot resume say
  so; collisions suffix `-1`, `-2`. Evidence: `browser/tests/downloads.sh`
  pauses a 51,200-byte loopback transfer at 10,240 bytes, resumes it to the full
  length, and downloads again into the same directory as `nova-fixture-1.bin`.
  0.7.5 adds history: a completed file is written once to `downloads.tsv` with
  its destination path, the last 20 come back as shelf rows after restart, and
  clicking a row reveals it in Finder or reports that it is gone. The test
  relaunches on the same profile and requires both rows and their actions.
  0.7.6 adds a **Clear** control to the shelf header: it drops WebKit's finished
  rows and the persisted record together and writes the emptied profile at once.
  The test requires the named `Clear finished downloads` button after a restart,
  then clears through `NOVA_BROWSER_CLEAR_DOWNLOADS=1` and requires an empty file.
  0.7.7 fixes pointer lag: queued motion coalesces to the newest position, input
  no longer waits behind the idle sleep, and the cursor becomes a hand over any
  actionable chrome and an I-beam over the address field.
  0.7.8 makes Google the default search engine and puts a chip in the field to
  switch it; the first suggestion is a real `Search Google for …` row. 0.7.9 and
  0.8.0 round the field and add glow, a toolbar wash and loading sparks through
  a new `round_rect_alpha` host primitive.
- **WB-04.004 — Session restore.** DONE. `session.tsv` (navigable tabs + active
  index), restart reopen, opt-out env. Evidence: library session assertions and
  restored-tab snapshot.
- **WB-04.005 — Opt-in search.** PARTIAL. Off → DuckDuckGo → Google → Brave via
  Cmd+Shift+S, UTF-8 percent encoding; plain text still rejected while off. No
  suggestion feed.
- **WB-04.006 — Frame budget.** DONE for measured work. Allocation-free
  `fx_step`, title cache, idle-frame skipping, `NOVA_BROWSER_BENCH`. Evidence:
  bench 120 frames avg 8 ms, `fx_step_2000` 4 ms.
- **WB-05.001 — G0 usable WebKit candidate.** DONE for the 0.7.0 review.
  Verdict: usable as a local WebKit shell within the documented limits — real
  `WKWebView` pages, tabs, downloads, session restore, opt-in search, history
  and bookmarks; **not** a general-purpose browser (no Nova networking layer,
  no import of Chrome history, no signed distribution) and no WebCore. Evidence:
  address 38/0, library 72/0, lifecycle 0 failures, downloads 5120 bytes, tabs
  5/5, IME 4/4, bench 4 ms avg with 13 KB peak stack, installed 0.7.0 launch
  ~1 s, valid ad-hoc signature.

## WebCore track

- **WC-01.001 — Trusted fixture harness.** READY.
- **WC-02.001 — HTML tokenizer states.** PLANNED.
- **WC-03.001 — Tree builder and DOM invariants.** PLANNED.
- **WC-04.001 — CSS tokenizer/parser/cascade.** PLANNED.
- **WC-05.001 — Block/inline layout.** PLANNED.
- **WC-06.001 — Text/images/display list/raster.** PLANNED.
- **WC-07.001 — Input/scroll/links/forms.** PLANNED.
- **WC-08.001 — Flex/grid/positioning.** PLANNED.
- **WC-09.001 — Engine switching with visible identity.** PLANNED.

## Reading aloud

- **WB-05.001 — Page reading with a system voice.** DONE. The browser extracts
  `document.body.innerText` and `runtime/speech.m` speaks it through
  AVSpeechSynthesizer. The speaker opens a panel with read/pause/resume, stop,
  speed and voice rows; Cmd+Shift+L toggles, Cmd+Shift+Space pauses, and
  Cmd+Shift+V cycles voices. `settings.tsv` keeps `voice` and `voice_rate`.
  Verified by state (`speaking=1`, chars, rate), not by listening.
- **WB-05.002 — Kokoro neural voice.** DONE as a command engine. The model is
  downloaded and size-verified into the profile, and the host runs
  `<NOVA_KOKORO_CMD> <text-file> <out.wav>`, plays the result with afplay, and
  reports real phases. No Kokoro command is installed on this machine, so the
  tests use a stub; a real Kokoro install still has to be heard to be believed.
- **WB-05.003 — Reader panel above the page.** DONE in 0.8.7. Panel geometry is
  computed from a native English-voice count (the row list is cached in the
  host) and registered with `gui.overlay`; a downloaded model does not change
  the reported backend.
- **WB-05.004 — Engine choice and synthesis feedback.** DONE in 0.8.8. Built-in
  voices are the default, Kokoro is a saved choice (`engine`), and the panel
  shows a wheel, a moving bar and elapsed seconds while the job runs. Without a
  synthesizer command, choosing Kokoro names the problem and keeps the system
  voice.
- **WB-03.006 — Chrome above the page view.** DONE in 0.8.6. `gui.overlay`
  registers rectangles the host paints above the WebView and makes clickable;
  the address suggestion list uses it. The rail name panel still resizes the
  page instead of floating, which is fine but not the same mechanism.

- **WB-03.007 — Window buttons on the layout row.** DONE in 0.8.9. The three
  AppKit traffic lights are moved onto the tab strip's centre (horizontal tabs)
  or the toolbar's centre (vertical tabs and the rail) every pass, because a
  full-size content view pins them to the window edge. `browser/tests/traffic.sh`
  reads both placements from the GUI trace.

## Compiler / library blockers

- **BROWSER-COMP-01 — Imported qualified struct signatures.** READY.
- **BROWSER-COMP-02 — Native callbacks.** OPEN. Polling is used; the narrow
  AppKit monitor only queues Cmd+L and tab commands for the active window.
- **BROWSER-STD-01 — URL parser and browser-owned HTTP/storage.** READY.
- **BROWSER-BOX-01 — Reusable webview/engine packages.** PLANNED.
