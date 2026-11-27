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
  0.9.2 extends the same profile scan to history and the default search engine
  under WB-04.007.
- **WB-04.007 — Chrome profile import: history and settings.** DONE in 0.9.2.
  `--import` with no path scans every detected Chromium profile; a path may be
  one profile folder, a user-data root or a single bookmarks file. History
  comes from SQLite through `sqlite3 -json` on an immutable snapshot (newest
  400 http(s) pages, one visit each, pages Nova already knows are skipped), and
  the `Preferences` default engine is applied only when it maps to Google,
  DuckDuckGo or Brave. Evidence: `browser/tests/history.sh` imports a fixture
  profile with a real SQLite `History` database and requires
  `history=2 search=duckduckgo` plus two visit lines, then `history=0` on the
  second run. Not imported: startup pages, homepage, passwords, cookies, open
  tabs, extensions, download history, themes.
- **WB-03.009 — Default browser.** DONE in 0.9.3. The bundle declares http and
  https in `CFBundleURLTypes`; `--set-default-browser`, `--default-browser-status`
  and an app-menu item call LaunchServices (`LSRegisterURL`,
  `LSSetDefaultHandlerForURLScheme`) and report what the system recorded. The
  tests never flip the owner's default. Evidence: the installed bundle's
  `--default-browser-status` prints `default: state=0`; a bare binary with no
  bundle identifier reports 0 instead of claiming the position.
- **WB-04.008 — Startup pages and homepage.** DONE in 0.9.3. The import reads
  `session.restore_on_startup` (4 = pages, 1 = restore, 5 = New Tab),
  `session.startup_urls` and `homepage`; `settings.tsv startup=` plus
  `startup.tsv` drive launch, and Cmd+Shift+H opens the homepage or start page.
  Evidence: `history.sh` requires `startup=2 homepage=https://example.com/home`
  from the fixture and a launch probe that reports the two startup tabs.
- **WB-04.009 — Import dry run and profile-sized reads.** DONE in 0.9.3.
  `NOVA_BROWSER_IMPORT_DRY=1` prints `import(dry): ...` and writes nothing.
  Reading the owner's real profile exposed three quadratic paths: the generic
  JSON parser (a new document per node), per-record store insertion (a whole
  list copy per record), and per-character value building with an all-pairs
  dedupe. Bookmarks and history now use linear readers. Measured on the real
  profile: killed at 7.2 GB before, 455 MB after, the same 662 bookmarks and
  372 visits.
- **WB-04.010 — Chrome extension inventory.** DONE in 0.9.5. Every installed
  extension is listed from `Extensions/<id>/<version>/manifest.json`, with
  `__MSG_*__` names resolved through `_locales/en/messages.json` (an unresolved
  name falls back to the id). The import prints `extensions=N` and writes
  `chrome-extensions.tsv`; a dry run writes nothing. Evidence: the fixture
  profile gains one literal and one i18n-named extension and the test requires
  `extensions=2` plus both names in the file.
- **BROWSER-EXT-01 — Chrome extension compatibility.** OPEN, not started. A
  Chrome `.crx` cannot run in `WKWebView`; WebKit's extension model is the
  Safari web-extension bundle, which is a different API surface and packaging.
  The import therefore reports extensions instead of claiming them. Planned
  replacements: `WKContentRuleList` for ad/tracker blocking, userscripts for
  page tweaks, and native features instead of extension for things Nova already
  does (reader, downloads, voice).
- **WB-03.011 — Tab favicons.** DONE in 0.9.6. Tabs draw the site's
  `/favicon.ico`, fetched once per host into `favicons/` under the profile and
  painted with a new `gui.image` host primitive; a letter tile covers sites with
  no icon and `NOVA_BROWSER_NO_FAVICONS=1` turns the fetch off. Evidence:
  `browser/tests/favicons.sh` serves a real icon over loopback and requires the
  cached bytes to equal the served file.
- **WB-04.002.a — Blob downloads.** DONE in 0.9.6. `WKWebView` cannot turn a
  `blob:` URL into a `WKDownload`; a document-start script intercepts
  `<a download>` clicks on `blob:` links, reads the blob as base64 and posts it
  to a message handler that writes the file and registers a finished download.
  `blob:` navigations are no longer treated as a blocked scheme. Evidence:
  `browser/tests/blob.sh` builds a blob in the page, clicks it, and requires the
  bytes on disk and a `state=2` row; `downloads.sh` still passes untouched.
- **WB-04.011 — Chrome download history and locale names.** DONE in 0.9.6.
  Completed Chrome downloads whose `target_path` still exists become shelf rows
  (`downloads=N`), and extension names resolve through every installed
  `_locales/*` directory instead of English only. Evidence: the fixture profile
  carries a real download row and the import reports `downloads=1`.
- **WB-03.012 — Bookmarks panel.** DONE in 0.9.7. A toolbar button and
  Cmd+Opt+B open a floating panel listing the newest twelve bookmarks with their
  URLs; pressing a row navigates the active tab, Escape or a press outside
  closes it, and the rows and close control are in the accessibility tree.
  Evidence: `browser/tests/bookmarks.sh` opens the panel headlessly and requires
  two rows, newest first, with a real rectangle.
- **BROWSER-EXT-01.a — Content rule list.** SHIPPED in 0.9.7. WebKit compiles a
  `WKContentRuleList` from JSON and applies it to every tab; a small tracker list
  is built in and `NOVA_BROWSER_BLOCKLIST` replaces it. Extensions still cannot
  load `.crx` packages, so BROWSER-EXT-01 stays open for userscripts and
  Safari-style extension bundles. Evidence: `browser/tests/blocklist.sh` blocks
  `/tracker.js` and requires the script never ran.
- **WB-04.012 — Safari import.** DONE in 0.9.7. `Bookmarks.plist` is converted
  with `plutil` and scanned for leaves, `History.db` is queried with sqlite3;
  both the explicit path and the automatic scan use it. macOS may deny the files
  without Full Disk Access; the import then reports nothing rather than a guess.
  Evidence: `browser/tests/safari.sh` imports a binary plist and a history
  database and requires `bookmarks=1 history=1` plus both stored rows.
- **WB-03.013 — Find in page and page zoom.** DONE in 0.9.8. Cmd+F opens a
  find bar over the page; typing searches as you go, Enter/Shift+Enter and
  Cmd+G/Cmd+Shift+G move between matches, Escape closes it and clears the
  highlight. WebKit's `WKFindResult` reports only whether something matched, so
  the bar says Found / No matches instead of a count. Cmd+= / Cmd+- step
  `WKWebView.pageZoom` by 10% and Cmd+0 resets it. Evidence:
  `browser/tests/find.sh` searches a real fixture over loopback and requires
  `zoom=150 reset=100 hit=1 miss=2 cleared=0 panel=320x34`, plus the seeded bar
  geometry. 0.9.9 adds per-host zoom memory (`zoom:<host>` in settings.tsv,
  re-applied on navigation) and publishes the find query and close control to
  the accessibility tree; the same test relaunches the profile and requires
  `zoom: host=127.0.0.1 applied=150 stored=150` plus both AX names (11 PASS).
- **WB-03.017 — Chrome password import.** DONE in 0.9.13. `--import` and
  Cmd+Shift+I read Chrome's `Login Data`, decrypt `v10` rows with PBKDF2 +
  AES-128-CBC in the runtime, and store new hosts in Nova's keychain login
  store. Existing hosts are skipped; a dry run writes nothing; a missing key
  prints `chrome key locked` and imports nothing. Evidence:
  `browser/tests/chrome-logins.sh` 6/6. Not done: Safari passwords (iCloud
  Keychain), multi-account hosts, and any password list UI.
- **WB-03.016 — Saved logins.** DONE in 0.9.12. Submitted password forms on
  normal tabs are captured through WebKit's `novaLogin` bridge and stored in
  the login keychain under `dev.nova.browser.login`, one item per host. A later
  load of that host fills the page's own form; Nova never submits it. Private
  tabs do neither. `--forget-login <host>` removes an entry. 0.9.14 adds the
  panel list (WB-03.018); still open: editing or revealing a password and sync.
  Evidence: `browser/tests/login.sh` 3/3.
- **WB-03.018 — Passwords panel.** DONE in 0.9.14. Cmd+Shift+O lists every
  saved host and username (sorted, read from the keychain once when the panel
  opens) with a per-row **Remove** that deletes the item and refreshes the
  list. `nova_keychain_list` enumerates the login service's accounts. Escape
  or × closes the panel; it is registered above the page. Not done: edit,
  reveal, rename, and showing notes. Evidence: `browser/tests/passwords.sh`
  2/2 — one captured login listed as `host=127.0.0.1 user=ada`, the panel
  `380x100`, `removed=1 remaining=0`, and no keychain item left.
- **WB-03.015 — Private tabs.** DONE in 0.9.11. `--private` and Cmd+Shift+N
  create a view on `WKWebsiteDataStore.nonPersistentDataStore`, so cookies and
  cache end with the tab. `record_visit` refuses private tabs, `save_session`
  skips them and writes nothing for a private-only session, and favicons are
  not cached. Tabs are labelled **Private ·**. Not a separate window; there is
  no permission prompt yet. Evidence: `browser/tests/private.sh` 5/5 —
  `private: tabs=1 flag=1 persistent=0 visits=0`, no session/history line for
  the private URL, and the normal-tab control run saves both.
- **WB-03.014 — Page appearance.** DONE in 0.9.10. `nova_webview_set_appearance_all`
  sets every tab's `WKWebView.appearance` from the chrome theme, before the
  first load and on every theme change, so pages answer
  `prefers-color-scheme: dark` for the dark themes and `light` for Light and
  Solarized instead of following the macOS system setting. Evidence:
  `browser/tests/dark.sh` loads a loopback page and requires
  `media=1` under Gruvbox and `media=0` under Light.
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
- **WB-05.005 — Voice previews.** DONE in 0.9.0. A play triangle on each row
  speaks a sample at the current speed through the system synthesizer, restores
  the chosen engine, and writes no setting. A running Kokoro job is stopped
  before the sample so two engines never talk over each other.
- **WB-03.006 — Chrome above the page view.** DONE in 0.8.6. `gui.overlay`
  registers rectangles the host paints above the WebView and makes clickable;
  the address suggestion list uses it. The rail name panel still resizes the
  page instead of floating, which is fine but not the same mechanism.

- **WB-03.007 — Window buttons on the layout row.** DONE in 0.8.9, corrected
  in 0.9.1. The three AppKit traffic lights are moved onto the tab strip's
  centre (horizontal tabs) or the toolbar's centre (vertical tabs and the rail)
  every pass, because a full-size content view pins them to the window edge.
  0.9.1 places them inside AppKit's 32 pt titlebar view instead of by the
  window height, which had put all three off the strip; a row the bar cannot
  hold is pulled up (vertical 29 → 25) so none hangs off it. `traffic.sh`
  reads the row centre and asserts the close frame stays inside the titlebar.
- **WB-03.008 — Chrome drag handle.** DONE in 0.9.1. `chrome_background`
  decides from the layout geometry whether a press is empty chrome, and only
  those presses start a window move (`gui.set_manual_drag` +
  `gui.begin_window_drag`). The old host strip grabbed the top 35 px, which was
  both small and in the way of the tabs. `NOVA_BROWSER_DRAG_CHECK` reports the
  decision for eight probe points per layout in `traffic.sh`.
- **WB-03.010 — Tooltips above the page.** DONE in 0.9.4. The page is a native
  subview above the chrome framebuffer, so tips near the page edge were painted
  under it; the controls at the right of the toolbar showed none. `draw_tooltip`
  registers its box with the overlay already used by the suggestion list and
  the reader panel. Evidence: `browser/tests/tooltip.sh` forces a tip that
  crosses the page edge and requires `overlay=1`, `crosses=1`, an in-window box
  and the host's `overlay rects` / `top=1` trace.

## Compiler / library blockers

- **BROWSER-COMP-01 — Imported qualified struct signatures.** READY.
- **BROWSER-COMP-02 — Native callbacks.** OPEN. Polling is used; the narrow
  AppKit monitor only queues Cmd+L and tab commands for the active window.
- **BROWSER-STD-01 — URL parser and browser-owned HTTP/storage.** READY.
- **BROWSER-STD-02 — String append and JSON copies.** OPEN. Appending to a
  `String` in a loop copies the whole string, so building a 300 KB value one
  character at a time costs gigabytes; `json.js_parse` returns a new document
  per node. Profile import now avoids both, but there is no O(1) append or
  range-slice builtin. Next: `String.slice(start, end)` (or a growable buffer).
- **BROWSER-BOX-01 — Reusable webview/engine packages.** PLANNED.
