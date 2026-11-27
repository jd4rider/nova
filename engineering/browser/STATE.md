# Nova Browser checkpoint

Updated: 2026-11-27
Branch: `main`
Last known good compiler checkpoint: `cfa00e3` (before browser work)
Browser: **0.9.10 installed at `/Applications/Nova Browser.app`**
Browser working tree: clean after the 0.9.10 commit; see `git status`
Active gate: G0 WebKit; G1 WebCore not started
Active task: **WB-03.014 — page appearance** (0.9.10: pages request dark or
light styles from the chrome theme, not from the macOS system setting)

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

- **0.7.7 — hover and pointer**
  - Motion events now coalesce in the host: a pass keeps only the newest
    position instead of replaying a queue that grew while the loop slept 6 ms
    per event. Idle passes sleep; passes with input waiting do not.
  - The pointer says what a press would do: a hand over toolbar buttons, tabs,
    suggestions, shelf rows and the failure-card buttons, an I-beam over the
    address field, an arrow over disabled back/forward and plain chrome.
    `gui.set_cursor` is re-applied through the view's cursor rects, so AppKit
    resetting to an arrow on entry does not undo it.
  - Hover fade step raised from 90 to 240 per pass, so a target lights up in
    roughly four frames instead of eleven.

- **0.7.8 — search from the address bar**
  - Google is the default engine; plain text becomes a query, and with search
    off it is still rejected rather than guessed. The `G` chip inside the field
    names the engine and cycles Google → Brave → off → DuckDuckGo on a press,
    persisting the choice; Cmd+Shift+S does the same.
  - While typing a query, the first suggestion row reads `Search Google for
    "…"` with the real query URL, drawn with a vector magnifier, so Enter's
    destination is visible before it is pressed. The row is built locally; no
    network request is made.
  - The stray scheme square left of the URL (amber on `http://`) is gone; the
    URL text already carries its scheme.
- **0.7.9 / 0.8.0 — the field and the frame**
  - The address field is a pill (radius 17) and glows orange while editing.
    Hovered toolbar buttons and the active tab carry the same soft halo.
  - The loading bar has seven clock-driven sparks and the toolbar has a six-band
    top-down wash. Both are stateless; the bench stayed at 7 ms per frame.
  - The **+** control in the tab strip already creates a tab (hover 8, AX
    `New tab`); nothing new was needed for it.

- **0.8.1 — address field like Chrome**
  - Hovering the field shows the I-beam even before it is focused (hover
    target 4 maps to the text cursor); the engine chip keeps its hand.
  - A press on the unfocused field selects the whole URL, so typing replaces
    it. Later presses place the caret, a double-click takes the word, and a
    triple-click selects all.

- **0.8.2–0.8.4 — themes and light mode**
  - Gruvbox stays the default. `Cmd+Shift+P` cycles Gruvbox → Midnight →
    Light → Solarized → Nova Dark, persists the choice as `theme` in
    `settings.tsv`, and names the result in a notice; `NOVA_BROWSER_THEME`
    forces one for a run.
  - `theme.tsv` in the profile overrides any of the 15 roles with
    `key<TAB>#rrggbb` (lines whose key starts with `#` are comments), which is
    both "import a theme" and "make your own".
  - Light and Solarized also switch the AppKit appearance, so native menus and
    the page's `prefers-color-scheme` follow the chrome instead of staying dark.
  - The field border is now a one-point rounded ring drawn under the fill;
    `stroke_rect` has no radius, and its square outline was the box around the
    pill. The engine chip uses the same ring.

- **0.9.2 — Chrome profile import: history and the search engine**
  - `--import` and Cmd+Shift+I now take a whole Chromium profile, not just its
    bookmarks: the newest 400 http(s) pages from Chrome's SQLite `History`
    become visits (read through `sqlite3 -json` from an immutable snapshot, so
    a running Chrome is undisturbed), and the default engine in `Preferences`
    is applied when it is Google, DuckDuckGo or Brave.
  - A second import adds nothing: pages Nova already has a visit for are
    skipped, so autocomplete does not fill with duplicates. Frequency is not
    transferred — each imported page counts once inside the existing 400-visit
    bound. Startup pages, extensions, passwords, cookies and open tabs are not
    imported and the docs say so.

- **0.9.3 — default browser, startup pages, real-profile import**
  - The bundle declares http/https in `CFBundleURLTypes` and asks LaunchServices
    to become the handler (`LSRegisterURL`, `LSSetDefaultHandlerForURLScheme`);
    `--set-default-browser`, `--default-browser-status` and the App menu report
    the result. The tests never change the system default (`state=0`).
  - Chrome `restore_on_startup` 4/1/5 imported: startup pages in `startup.tsv`,
    mode in `settings.tsv startup=`, homepage honoured on launch and Cmd+Shift+H.
  - `NOVA_BROWSER_IMPORT_DRY=1` prints `import(dry): ...` and writes nothing.
  - The real Chrome profile now imports instead of dying: bookmarks scan
    line-by-line over quote-split pieces, history is a tab-separated `sqlite3`
    query, and imports merge in one pass. 7.2 GB/killed → 455 MB, 662 bookmarks,
    372 visits, 400 on auto-discovery.
  - Language snag filed as `BROWSER-STD-02`: loop string append copies the whole
    string and `json.js_parse` copies per node; both are avoided, not fixed.
- **0.9.4 — tooltips above the page**
  - A tip that crosses the page edge is registered with `gui.overlay`, so the
    host copies it above the WebView instead of painting it underneath (which is
    why the right-hand toolbar controls showed no tips). `tests/tooltip.sh`
    forces a tip at the right edge and checks the box and the host's
    `overlay rects` trace.
- **0.9.5 — Chrome extension inventory**
  - Every profile's `Extensions/<id>/<version>/manifest.json` is read; i18n
    names resolve through `_locales/en/messages.json`, unresolved ones fall back
    to the id. The import prints `extensions=N` and writes
    `chrome-extensions.tsv`. The real profile: 85 installed, none runnable.
  - WebKit has no Chrome extension runtime (`WKWebView` loads Safari web
    extensions). Replacements are tracked as `BROWSER-EXT-01`: content rules,
    userscripts, native features; none implemented yet.
  - Nova is the system handler for http/https on this machine: after
    `lsregister -f`, `--set-default-browser` reports `set=1 state=1` and
    `NSWorkspace` resolves a web URL to `Nova Browser.app`. Revert in System
    Settings → Desktop & Dock → Default web browser.
- **0.9.10 — pages follow the chrome appearance**
  - Every WKWebView gets the appearance the chrome theme implies, before the
    first load and again on every theme change, so `prefers-color-scheme`
    answers dark for Gruvbox/Midnight/Nova Dark and light for Light/Solarized
    even when the macOS system setting is the other one.
  - Evidence: `browser/tests/dark.sh` loads a real page over loopback and
    requires `dark: theme=gruvbox light=0 media=1` and
    `dark: theme=light light=1 media=0`.

- **0.9.9 — per-host zoom memory, find bar in the accessibility tree**
  - Zoom now persists per host in `settings.tsv` as `zoom:<host>` and is
    re-applied on every navigation to that host; Cmd+0 stores 100%.
  - The find bar publishes its query as an `AXTextField` and its × as a named
    `Close find bar` button, so the bar is usable without the mouse.
  - Evidence: `browser/tests/find.sh` is 11/11, including a second launch of the
    same profile restoring `zoom: host=127.0.0.1 applied=150 stored=150`, and
    the AX dump naming both find controls.

- **0.9.8 — find in page, page zoom**
  - Cmd+F opens a find bar over the top right of the page. Typing searches as
    you go, Enter/Shift+Enter step forward/back, Cmd+G repeats either, and
    Escape closes the bar and clears the highlight. WebKit's WKFindResult has
    no match count, so the bar says Found or No matches instead of a number.
  - Cmd+= / Cmd+- step `WKWebView.pageZoom` by 10% and Cmd+0 resets it. The
    page lays out at the new scale; zoom is per tab and starts at 100%.
  - Both commands work while the page owns focus through the same host key
    queue as Cmd+L. Evidence: `browser/tests/find.sh` finds a real word, reports
    no match for a missing one, clears the state, and reads back zoom 150/100.

- **0.9.7 — bookmarks panel, content blocking, Safari import**
  - A Bookmarks panel (toolbar list button, Cmd+Opt+B) lists saved pages newest
    first with their URLs; a press opens one, Escape or a press outside closes
    it. Cmd+D still saves or removes the page.
  - `WKContentRuleList` now blocks a small built-in tracker list, and
    `NOVA_BROWSER_BLOCKLIST=<file>` replaces it with any JSON rule list.
  - Tabs prefer the icon a page declares with `<link rel="icon">` and fall back
    to `/favicon.ico`; a sidecar records which URL produced the cached bytes.
  - Safari joins Chrome in the import: `Bookmarks.plist` through `plutil`,
    `History.db` through sqlite3, for an explicit path or the automatic scan.

- **0.9.6 — favicons, blob downloads, more Chrome import**
  - Tabs draw the site's `/favicon.ico` from `favicons/` in the profile, or a
    letter tile when a site has none. `gui.image` paints a decoded image file
    into the framebuffer; `NOVA_BROWSER_NO_FAVICONS=1` disables the fetch.
  - Blob downloads work: a page-side bridge reads `blob:` links (`<a download>`)
    and hands the bytes to the host, which writes the file and lists it as a
    finished download. That is the chatgpt.com export path.
  - Chrome's completed downloads whose file still exists are imported into the
    shelf; extension i18n names resolve from whichever locale is installed.
  - `NOVA_BROWSER_URL` now loads in the headless `AUTOCLOSE` path too, which is
    what lets the favicon test drive a real HTTP page.
- **0.9.1 — window buttons back on the strip, whole chrome drags**
  - The three traffic lights had been placed from the window's height while
    AppKit keeps them in its own 32 pt titlebar view, so they landed hundreds of
    points below the strip and disappeared. The host now places the row from
    that view's own bounds and pulls a row it cannot hold up until the buttons
    are whole: centre 23 with horizontal tabs, 25 (requested 29) with vertical
    tabs or the rail.
  - The browser hit-tests its own drag handle instead of the host's fixed 35 px
    strip. Empty tab strip, toolbar and sidebar background move the window;
    a tab, the address field and every toolbar control keep their own press,
    which the fixed strip used to steal and hand to the window drag.

- **0.9.0 — voice previews**
  - Every voice row carries a play triangle that speaks a short sample at the
    current speed. Previews never change the saved voice or engine, and a Kokoro
    pick is restored after the sample starts, so comparing system voices is safe.

- **0.8.9 — window buttons follow the layout**
  - The close, minimize and fullscreen lights are still AppKit's standard
    buttons, moved onto the tab strip's centre (horizontal tabs) or the
    toolbar's centre (vertical tabs and the rail). AppKit resets them on resize
    and full-screen exit, so the placement is re-applied every pass.

- **0.8.8 — Kokoro is a choice, with a real wheel**
  - The reader stays on system voices by default. Kokoro joins the voice list
    once its model is installed and a synthesizer command is found; the pick is
    saved as `engine` in `settings.tsv`.
  - The host runs Kokoro as a child process (`<cmd> <text> <wav>`, model path in
    the environment) and reports synthesizing / playing / failed from the
    process itself. The panel and the speaker button show a wheel, a moving bar
    and the elapsed seconds; a failure shows the command's own error text.
  - Pause suspends the running child (SIGSTOP/SIGCONT), so it is a real pause.
    Choosing Kokoro without a command says why and reads with the system voice.

- **0.8.7 — reader controls, floating rail, voice-model fetch**
  - The speaker opens a reader panel above the page: read/pause/resume, stop,
    speed in 25% steps (persisted as `voice_rate`, default 50), and the
    installed English voices with quality tags. Escape or an outside click
    closes it; Cmd+Shift+Space pauses and resumes.
  - The rail's name panel no longer takes width from the page. It is an
    overlay, so crossing the rail does not reflow the document, and presses in
    the panel column come back through the event queue.
  - The panel's download row fetches the Kokoro model with curl in a pty slot,
    samples the `.part` file for progress, verifies the size, and only then
    promotes it to `<profile>/kokoro/model.onnx`. A short or failed fetch leaves
    nothing behind.
  - `backend` stays `system` even with the model installed: the neural runtime
    is not linked, and the notice says so. Voice enumeration is cached and
    threaded through a native English count after the uncached re-sort made the
    panel take seconds per frame.

- **0.8.6 — reading aloud, chrome above the page**
  - A speaker button and Cmd+Shift+L read the page with a system voice;
    Cmd+Shift+V cycles the installed English voices and `settings.tsv voice`
    keeps the choice. Text comes from WebKit's `document.body.innerText`.
  - `runtime/speech.m` is a polled AVSpeechSynthesizer adapter (speak, stop,
    pause, resume, voice list ordered by quality). nova-tool compiles it only
    when the IR calls `nova_speech_*` and links AVFoundation.
  - Kokoro is a semantic seam, not a shipped voice: `NOVA_BROWSER_FAT=1` or
    `NOVA_BROWSER_VOICE=kokoro` asks for it, and the app reports the missing
    model instead of pretending. No ONNX runtime is bundled.
  - Chrome can now paint above a page. `gui.overlay` registers rectangles; the
    host copies those pixels from the frame into a transparent view above the
    WebView and re-enters clicks there through the normal event queue. The
    address suggestion list is the first user, so it is visible and clickable
    below the toolbar instead of hidden behind the page.

- **0.8.5 — tab layouts, toolbar row, glyphs, imports, page agent**
  - Tabs have three layouts: top strip, left sidebar, collapsed rail. The
    toolbar button and Cmd+Shift+E cycle them; `settings.tsv layout` stores
    `horizontal`, `vertical` or `rail`.
  - The horizontal strip no longer has a plus, so tabs use the full width.
    In both vertical layouts the New Tab row sits under the last tab; the rail
    keeps a `+` there too.
  - Hovering the rail opens a 210 px list of tab titles over the page; clicking
    the chevron pins the sidebar back open. The page rect gives the panel its
    width while open, because the chrome is painted under the WebView and a
    floating panel would be hidden.
  - Toolbar controls are 30 px buttons with an 8 px gap on each side of the
    field, centred on its 34 px height. Back, forward, reload/stop, star,
    download, help and the sidebar toggle are line-drawn glyphs, not font
    characters, and a button paints a rounded wash and glow only on hover.
  - Bookmark import now discovers Chromium, Brave, Edge, Arc and Vivaldi
    profiles as well as Chrome; all share the same JSON file.
  - The page user agent is a Safari/WebKit string so Google's embedded-WebView
    OAuth refusal is not triggered; `NOVA_BROWSER_UA` overrides it.

## Limits

- Tabs are capped at 6; drag/reorder, overflow scrolling and a right-click
  menu (new/duplicate/close/close-others) exist, but there are no tab groups,
  pinning or reopen of a dragged stack across restart beyond `session.tsv`.
  Adversarial stale-callback/focus stress remains WB-03.002.a.
- Address editing moves on grapheme clusters for combining marks, variation
  selectors, keycaps, skin tones, flags and ZWJ sequences, but not Indic
  conjuncts, Arabic/Hebrew vowel marks or Hangul jamo; Escape/reload still have
  no automated coverage (WB-03.001.a).
- Pages follow the chrome appearance through `prefers-color-scheme`; there is
  no separate per-page or per-site appearance override, and a page that ignores
  the query is not restyled.
- Import covers bookmarks only; Chrome history is SQLite and is not imported.
  No Firefox/Safari live-profile import (Netscape HTML export works for all).
- Downloads pause/resume only when the origin supports byte ranges and WebKit
  produced resume data; otherwise the row says it cannot resume. Collisions get
  `-1`, `-2` … automatically — there is no rename prompt — and there is no
  download history or queue across restarts. Session restore reopens URLs but
  not back/forward stacks, scroll offsets or form state.
- Search has no suggestion feed or per-provider settings UI; only the three
  built-in providers, cycled with Cmd+Shift+S.
- No private windows, permissions, inspector, Developer ID signing/notarization
  or release. Find reports Found / No matches, not a match count; the bar's field
  and close control are in the accessibility tree, its status text is not. Zoom
  is stored per host, so every tab showing that host shares one zoom level. The
  chrome accessibility tree covers tabs,
  the address field, toolbar buttons, the engine badge, the open download shelf
  and autocomplete rows; notices, tooltips and the failure panel's body text are
  not exposed, `AXPress` is the only action, and no VoiceOver session was driven
  by hand.
- `loadHTMLString` fixtures do not create back/forward entries. Real http(s)
  history is not smoke-tested.
- WebCore is not implemented. No WebCore path delegates to WebKit.

## Exact next action

Find and zoom landed in 0.9.8 (see `TASKS.md`). Pick the next daily-use slice:
private windows and permission prompts (site features are still unhandled),
notices/tooltips/find bar in the accessibility tree plus a hand-driven
VoiceOver pass (WB-03.004), per-host zoom persistence, or begin WC-01.001 with
the trusted fixture harness. WebCore stays unstarted and must not borrow WebKit
evidence.

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
