# Nova Browser

A native macOS browser shell built with the self-hosted Nova compiler and Nova
GUI. WebKit is the default engine today. Nova WebCore is a separate,
Nova-owned pipeline and is **not implemented yet**; there is no hidden fallback
or WebKit-based WebCore.

## Layout

```text
browser/
  app/                 Nova GUI shell and commands
  address/             headless URL normalization and editing primitives
  engine-api/          shared engine capability/status contract
  engines/webkit/      WebKit adapter
  engines/webcore/     WebCore boundary (implementation starts at WC-01)
  fixtures/            trusted, offline test pages
  packaging/           icon source
  scripts/             macOS bundle installer
  tests/               address, lifecycle and native-shell evidence
```

The reusable platform binding is `libraries/nova-webview`; the thin macOS host
is `runtime/webkit.m`. Browser-only code must not call Objective-C directly.

## Build

```sh
# Build the compiler once with the checked-in bootstrap.
(cd examples/nova-tool && ../../bootstrap/nova build)

# Build the browser.
(cd browser && ../examples/nova-tool/target/nova/nova-tool build)
```

The compiler links `WebKit` automatically only when a program references
`nova_webview_`. Programs that do not use the browser binding do not link it.

## Run

```sh
# Trusted offline fixture (default).
browser/target/nova/nova-browser

# Start at a URL.
NOVA_BROWSER_URL=https://jd4rider.github.io/nova/ browser/target/nova/nova-browser

# One-shot evidence: wait for load, snapshot WebKit, exit.
NOVA_BROWSER_AUTOCLOSE=1 \
NOVA_BROWSER_SNAPSHOT=/tmp/nova-browser.png \
  browser/target/nova/nova-browser

# Opt-in diagnostics; no page text is logged.
NOVA_GUI_TRACE=/tmp/nova-browser-trace.log NOVA_BROWSER_TRACE=1 \
  browser/target/nova/nova-browser
```

Click the address field or press Cmd+L, including while WebKit owns page focus.
Type or paste; Cmd+A selects, Cmd+Backspace clears, Enter loads, Escape cancels.
Bare hosts gain `https://`; `localhost` and `127.0.0.1` use `http://`. Plain
text searches Google by default (UTF-8 percent-encoded). The G chip inside the
field names the engine and cycles Google → Brave → off → DuckDuckGo on a press;
the choice is remembered, and the first suggestion row spells out where Enter
will send the text. With search off, plain text is rejected rather than guessed.

Editing moves by grapheme cluster, not by Unicode scalar: Left/Right,
Backspace/Delete and clicks stop only on cluster boundaries, so a combining
accent, a ZWJ emoji family, a flag pair or a skin-tone modifier is deleted and
stepped over as one character. Segmentation covers combining marks, variation
selectors, keycaps, regional-indicator pairs and ZWJ sequences; Indic conjuncts,
Arabic/Hebrew vowel marks and Hangul jamo are not joined yet.

The chrome is painted by Nova into a single view, so it publishes its own
accessibility tree: tabs (the selected one flagged), the address field with the
current URL, the toolbar buttons and the `WEBKIT` badge become native
accessibility elements with screen frames, and activating one performs the same
click a pointer would. The same applies to the open download shelf (a named
close control plus Pause/Resume/Retry rows and Cancel/Show-in-Finder buttons
named with the file) and to autocomplete rows, which are links whose value is
the target URL. The page's tree comes from `WKWebView` itself.
`NOVA_GUI_AX_DUMP=1` prints the tree once for inspection, and
`tests/accessibility.sh` checks it.

The field autocompletes from local history and bookmarks as you type. Up/Down
moves the highlight, Enter opens it, Tab completes it into the field, Escape
dismisses. The list ranks a host-name prefix above a path match, a title prefix
above a title substring, and always shows a bookmark before the same URL in
history. It never reads or sends the typed text anywhere.

Chrome buttons are back, forward, reload/stop, bookmark, Docs and the tab-layout
toggle. Hover and focus fade in over roughly 100 ms, the address bar shows a
blinking caret, and the loading bar follows WebKit's own estimate.

| Shortcut | Action |
| --- | --- |
| Cmd+L | focus the address field |
| Cmd+R | reload |
| Cmd+D | bookmark the current page |
| Cmd+Opt+B | open or close the bookmarks panel |
| Cmd+F | find in the page |
| Cmd+G / Cmd+Shift+G | next / previous match |
| Cmd+= / Cmd+- | zoom the page in / out |
| Cmd+0 | reset the page zoom |
| Cmd+Shift+I | import Chrome bookmarks, history and saved passwords |
| Cmd+Shift+O | open or close the passwords panel |
| Cmd+Shift+E | tabs: top → sidebar → rail → top |
| Cmd+Shift+L | read the page aloud, or stop reading |
| Cmd+Shift+V | choose the next reader voice |
| Cmd+Shift+Space | pause or resume reading |
| Cmd+Shift+S | cycle search provider (same order as the field's chip) |
| Cmd+Shift+P | cycle theme (Gruvbox → Midnight → Light → Solarized → Nova Dark) |
| Cmd+T / Cmd+W / Cmd+Shift+T | new / close / restore tab |
| Cmd+Shift+N | open a private tab |
| Cmd+1–6 | select a tab |

These chords are forwarded from page focus through the same narrow host queue
as Cmd+L; nothing is executed inside the page.

Pages are also told which appearance to prefer. A dark theme (Gruvbox,
Midnight, Nova Dark) makes `prefers-color-scheme: dark` true, while Light and
Solarized make it false, so a site's own dark styles come up with the chrome
theme rather than with the macOS system setting.

## Install / update

```sh
sh browser/scripts/install-macos.sh
open -a "/Applications/Nova Browser.app"
```

The script rebuilds, renders the icon, assembles `Nova Browser.app`, ad-hoc
signs it locally and replaces the installed copy. Version 0.9.15 is installed.
Re-run it after every completed phase. This is not Developer ID signing or
notarization.

## Tabs

Each tab owns a separate WKWebView. Click a tab to select it, its `x` to close
it, and use Cmd+T for a new one: the horizontal strip has no plus, so the tabs
take the full width. Cmd+T creates, Cmd+W closes, Cmd+Shift+T restores the last
closed tab, Cmd+1–6 selects a tab, and these shortcuts work while WebKit owns
page focus through a narrow host-level request queue. The cap is six tabs. Drag
a tab sideways to reorder it, and right-click a tab for New Tab, Duplicate Tab,
Close Tab and Close Other Tabs. When the horizontal strip no longer fits it
scrolls to keep the active tab in view. On quit the navigable tabs and the active index
are written to `session.tsv`; the next launch reopens them unless
`NOVA_BROWSER_NO_RESTORE=1`. Restored tabs reload their URLs, not scroll offsets
or form state.

Tabs are a horizontal strip, a left sidebar, or a collapsed rail. The toolbar
button or Cmd+Shift+E cycles top → sidebar → rail → top, and `settings.tsv`
keeps the choice (`layout=horizontal|vertical|rail`). In the vertical layouts
the New Tab row sits under the last tab, not above it; in the rail it is a
`+` under the numbered tab marks. Hovering the rail opens its list of titles
over the page and clicking the chevron pins it back open. The page rectangle,
tab hit boxes and tooltips all follow the active layout.

Toolbar controls are one row of 30 px buttons with an 8 px gap on each side of
the address field, vertically centred on it. Back, forward, reload/stop, star,
downloads, help and the sidebar toggle are drawn from lines rather than font
glyphs, so no control depends on font coverage, and a button only paints a
wash or glow while the pointer is on it.

## Reading aloud

The speaker button next to the bookmark reads the page, and Cmd+Shift+L does
the same from the keyboard. The text is the page's own `innerText` from WebKit,
so it reads what a reader mode would: article text without the navigation
around it. Click again to stop.

The speaker button opens the reader panel: read / pause / resume, stop a step
slower or faster, and pick a voice from the installed English ones. The panel
hangs over the page, which is why it is painted above the WebView rather than
under it. Escape or a click elsewhere dismisses it; Cmd+Shift+L starts or stops
reading without the panel, and Cmd+Shift+Space pauses and resumes.

The voice is a system voice, so it works offline and needs no download. Fresh
profiles pick the best installed English voice, and the choice is saved as
`voice` in `settings.tsv`; speed is saved as `voice_rate` (25–200% of the system
rate, default 50%). Speed and voice changes apply to the next reading rather
than restarting the current one mid-sentence. Each voice row also has a play
triangle that previews that voice at the current speed without changing the
saved pick.

A neural voice (Kokoro) needs a model and an inference runtime. Start with
`NOVA_BROWSER_VOICE=kokoro` (or `NOVA_BROWSER_FAT=1`) and the panel gains a
"Download voice model" row: it fetches the quantized model into
`<profile>/kokoro/model.onnx` through curl, checks the size, and installs it
only if the download is complete. `NOVA_KOKORO_URL` overrides the source for a
local mirror or a test; `NOVA_KOKORO_MIN_BYTES` overrides the size floor.

The model alone cannot speak. Kokoro runs through a synthesizer command: the
browser looks for `kokoro-tts` or `kokoro-cli` on the usual paths, or uses
`NOVA_KOKORO_CMD`. The contract is `<command> <text-file> <out.wav>`, with the
model path in `NOVA_KOKORO_MODEL`, so a short wrapper around whatever you
installed is enough. Synthesis is slow, so it runs as a child process: the
panel shows a wheel, a moving bar and the elapsed seconds while it works, and
the speaker button shows the same wheel when the panel is closed. The phases
come from the process itself, not a timer, and pause suspends the synthesizer
or the player rather than pretending to stop it.

Built-in voices stay the default. Kokoro joins the voice list once its model is
installed, marked `needs a synthesizer` until a command is found; choosing it
when it cannot run says why and reads with the system voice instead of failing
silently. `backend` reports `kokoro` only while its job is actually producing
audio, and a failed command shows its own error text.

The window's close, minimize and fullscreen lights are macOS's own buttons,
moved onto the tab strip when tabs are horizontal and onto the toolbar when they
are vertical, so they sit on the same row as the chrome instead of floating at
the window's top edge.

## History, bookmarks and Google import

Visits and bookmarks are stored as plain, escaped UTF-8 lines in the profile
directory (`~/Library/Application Support/Nova Browser`, or `$NOVA_BROWSER_PROFILE`):

```text
history.tsv     one line per visit, bounded at 400
bookmarks.tsv   star toggles, one line per bookmark
session.tsv     tabs to reopen plus the active index
settings.tsv    tab layout (horizontal/vertical/rail), search (google), theme
theme.tsv       optional role overrides, one key<TAB>#rrggbb per line
```

Set `NOVA_BROWSER_NO_HISTORY=1` to keep a session out of history entirely; the
file is then neither read nor written. Nothing is synced off the machine.

Cmd+Shift+I imports from every detected Chromium-family profile — Chrome,
Chromium, Brave, Edge, Arc and Vivaldi all save the same `Bookmarks` JSON and
the same `History`/`Preferences` files — and reports what was added. The same
import runs headlessly:

```sh
browser/target/nova/nova-browser --import                 # detected Chrome profiles
browser/target/nova/nova-browser --import path/to/Default  # one profile folder
browser/target/nova/nova-browser --import path/to/Bookmarks
browser/target/nova/nova-browser --import path/to/export.html
```

Both Chrome's JSON `Bookmarks` file and a Netscape bookmark export (Google
Takeout, Chrome/Firefox/Safari "Export bookmarks") are understood. Only
`http`/`https` entries are imported; `chrome://` and `javascript:` bookmarks are
skipped. Import is idempotent, so running it twice adds nothing.

A profile folder also brings over the parts of Chrome that map onto Nova:

- **History.** The newest 400 `http(s)` pages from Chrome's SQLite `History`
  database become visits, so autocomplete starts with the sites you actually
  use. The database is read through `sqlite3 -json` from an immutable snapshot,
  so a running Chrome is neither blocked nor disturbed. A URL Nova already has
  a visit for is skipped, which keeps a second import from duplicating rows.
  Frequency is not transferred: each imported page counts once, and the store's
  existing 400-visit bound still applies.
- **Search engine.** The default provider in `Preferences` is applied when it is
  Google, DuckDuckGo or Brave; any other engine leaves Nova's current choice
  alone and is not reported as imported.
- **Startup pages.** Chrome's "Open a specific page or set of pages" becomes
  Nova's own startup set (`startup.tsv` + `startup=pages`), so the next launch
  opens those pages instead of the last session. "Continue where you left off"
  imports as `startup=restore` and "Open the New Tab page" as `startup=newtab`.
- **Homepage.** A real Chrome homepage (not the New Tab page) is saved as
  `homepage` and opened with Cmd+Shift+H; without one, that shortcut opens
  Nova's start page in a new tab.
- **Bookmarks.** Bookmark bar and Other bookmarks, as above.
- **Saved passwords.** The `logins` table of `Login Data`, decrypted with the
  `Chrome Safe Storage` key (see Saved logins above). Hosts Nova already has a
  login for are skipped, so the import never overwrites a credential changed in
  Nova. On a dry run the count is reported and no keychain entry is written.

Extensions are listed, not run: `extensions.tsv` and `chrome-extensions.tsv`
record what Chrome has (name from whichever `_locales` locale is installed,
version, enabled state), but WebKit loads Safari web extensions and cannot load
a Chrome `.crx` package.

Safari is read too: `--import ~/Library/Safari` (and the no-argument scan when
the folder is readable) converts `Bookmarks.plist` through `plutil` and queries
`History.db` with sqlite3. macOS may deny those files without Full Disk Access;
an unreadable file imports nothing rather than reporting a guess.

Not imported: cookies, site permissions, open tabs (the last session is not
read), themes, or Safari's passwords (they live in iCloud Keychain, not in a
file Nova can read). Completed downloads come over when the file still exists on
disk. Chrome's passwords are imported into Nova's own keychain store; Chrome's
copies stay where they are.

A dry run applies everything in memory and writes nothing, so you can see what
would change first:

```sh
NOVA_BROWSER_IMPORT_DRY=1 browser/target/nova/nova-browser --import
```

## Default browser

The bundle declares `http` and `https`, and two commands move the LaunchServices
handler:

```sh
"/Applications/Nova Browser.app/Contents/MacOS/nova-browser" --default-browser-status
"/Applications/Nova Browser.app/Contents/MacOS/nova-browser" --set-default-browser
```

The app menu also has **Make Nova Browser the Default Browser**. Both go through
`LSSetDefaultHandlerForURLScheme` for both schemes, so macOS and every other app
that opens a link follow the choice. A bare binary has no bundle identifier and
reports `state=0` rather than pretending; the commands only mean something for
the installed app. This ad-hoc signed build is not notarized, so macOS may still
show a confirmation for a link opened from another app.

## Site permissions

When a page asks for the camera or microphone, Nova draws a card naming the
site and the device and holds WebKit's request until you press **Allow** or
**Block**. The answer is remembered per host and device in `settings.tsv` as
`permission:<host>:<kind>`, so the card appears once. Allowing here only lets
WebKit start the capture; macOS still controls the device itself in System
Settings → Privacy & Security, with the usage strings the bundle declares.
Private tabs deny capture requests without showing the card.

Web notifications are not offered. WebKit exposes no public `WKUIDelegate` hook
for `Notification.requestPermission()` — only private SPI — so Nova does not
pretend to grant a permission it cannot deliver.

## Private tabs

Cmd+Shift+N opens a private tab, and `nova-browser --private` starts the window
with one. A private tab uses WebKit's `WKWebsiteDataStore.nonPersistentDataStore`,
so its cookies, cache and local storage live only as long as the tab. Nova
writes no history visit for it, saves no session entry for it, and caches no
favicon for it; the tab is labelled **Private ·** so it is never mistaken for a
normal one. Bookmarks still work, and downloads still go to the Downloads folder.

This is a private *tab*, not a separate private window: it sits in the same tab
strip as normal tabs, and closing it is how you leave. A private-only session
never overwrites the saved session, so quitting it does not lose the tabs you
had before.

## Saved logins

When you sign in on a normal tab, Nova stores that username and password for the
host in the login keychain, under its own `dev.nova.browser.login` service, and
says **Saved login for example.com**. The next time the host shows a password
form, Nova fills it; it never submits the form for you. Private tabs neither
save nor fill a login.

Chrome's saved passwords come over with Cmd+Shift+I or `--import`. Nova reads
the profile's `Login Data`, decrypts each `v10` row with the key from Chrome's
`Chrome Safe Storage` keychain item (macOS may ask once before handing it over),
and stores hosts Nova does not already know in its own keychain store. An
existing Nova login is never replaced, and without the key the import prints
`chrome key locked` and imports none rather than writing blank passwords.

One login is kept per host. Cmd+Shift+O opens the passwords panel: every saved
host with its username, a **Show** button that reads the password from the
keychain only when you press it (and turns into **Hide**), and **Remove**.
Escape or the × closes it. Removing one without opening the window is still a
command:

```sh
browser/target/nova/nova-browser --forget-login example.com
```

This is Nova's own store. Passwords saved in Chrome or Safari are not imported,
and logins are not synced anywhere. While the app is ad-hoc signed, a rebuilt
binary can look like a different app to the keychain, so macOS may ask once
before Nova reads an entry it wrote before the rebuild.

## Bookmarks

The list button in the toolbar opens the bookmarks panel, and Cmd+Opt+B does the
same. Newest first, twelve rows with each URL under its name; a press opens the
page in the active tab, Escape or a press outside closes the panel. Cmd+D still
saves or removes the current page.

## Find and page zoom

Cmd+F opens a find bar over the top right of the page. Typing searches as you
go, Enter takes the next match, Shift+Enter the previous one, Cmd+G and
Cmd+Shift+G repeat either, and Escape closes the bar and clears the highlight.
WebKit's `WKFindResult` reports whether something matched but not how many, so
the bar says **Found** or **No matches** rather than inventing a count. The
query field and close button are in the accessibility tree.

Cmd+= and Cmd+- step the page zoom by 10% through WebKit's own `pageZoom`, so
the document lays out at the new scale instead of the pixels being stretched;
Cmd+0 returns to 100%. The setting is remembered per host in `settings.tsv` as
`zoom:<host>`, so coming back to a site restores the zoom you left it at.

## Downloads

WebKit hands attachment responses to a `WKDownloadDelegate`; Nova polls that
registry and draws the shelf. The toolbar download button opens it, each row
shows the file name, received/expected bytes, a progress line, cancel while
running or reveal when finished. Click a running row to pause it: the partial
file and WebKit's resume data are kept, and clicking the paused row resumes from
the bytes the server acknowledged. If the origin sent no byte-range support
there is nothing to resume from and the row says so. A colliding name gets `-1`,
`-2` and so on; there is no rename prompt. Files go to the system Downloads
folder unless `NOVA_BROWSER_DOWNLOAD_DIR` says otherwise.
`browser/tests/downloads.sh` proves the path with a real loopback download that
pauses mid-transfer, resumes to 51,200 bytes, then downloads again into the same
directory as `nova-fixture-1.bin`.

Blob downloads — chatgpt.com's export button, for example — never go through
`WKDownload`, because WebKit cannot download a `blob:` URL. A document-start
script intercepts `<a download>` clicks on blob links, reads the blob as base64
and posts it to the host, which writes the file into the same directory and
records a finished row. `browser/tests/blob.sh` proves that path end to end.

A finished download is recorded in the profile (`downloads.tsv`: bytes, time,
path, name) once, and the last 20 come back as shelf rows after a restart. Their
rows and Show-in-Finder buttons are in the accessibility tree too. Revealing a
recorded file that was moved or deleted reports that it is gone instead of
opening an empty Finder window. A Clear control in the shelf header forgets the
finished rows and the record together.

## Content blocking

WebKit compiles the rules itself with `WKContentRuleList`; there is no extension
and no background process. A small list ships built in (DoubleClick, Google Tag
Manager and Analytics, Facebook, ScorecardResearch), and
`NOVA_BROWSER_BLOCKLIST=<path>` replaces it with any JSON rule list.
`browser/tests/blocklist.sh` serves a page whose `/tracker.js` is blocked and
checks the script never ran.

## Favicons

Each tab draws the icon the page declares with `<link rel="icon">` (or an
apple-touch-icon), falling back to `/favicon.ico` when it declares none. The
bytes are fetched once per host with `curl` into `favicons/` under the profile and
painted with a new `gui.image` host primitive; a sidecar records which URL was
used so a declared icon replaces an earlier guess.
Sites without one get a letter tile. The fetch is skipped with
`NOVA_BROWSER_NO_FAVICONS=1`. `browser/tests/favicons.sh` serves a real PNG over
loopback and requires the cached bytes to match it.

## Performance

`NOVA_BROWSER_BENCH=120 browser/target/nova/nova-browser` draws 120 frames and
prints `bench: frames=120 avg=4ms worst=179ms fx_step_2000=3ms
stack=13KB/8176KB` on this machine. The stack figure is the peak main-thread
stack in use while pumping, measured against the thread's real stack size; the
same 13 KB peak at 2000 frames means the loop neither recurses nor leaks stack. Hover/focus easing runs in one allocation-free pass, tab titles are
cached for 500 ms, and a frame is only drawn when the chrome state, notice,
tooltip or pointer signature changes — an idle window just polls events. The
`worst` figure is the first frame (window and WebKit setup).

## Errors, offline and certificates

A failed navigation hides the page view and draws a Nova-owned panel with the
failure category, WebKit's own description and the attempted address. Retry
reloads; Back appears when WebKit has somewhere to go. An unsupported address
offers only the instruction to edit it, because retrying cannot succeed.

Categories are `offline`, `certificate`, `dns`, `unreachable`, `blocked` and a
generic `failed`. A certificate failure is never bypassable: there is no
"proceed anyway" path, and Nova does not disable TLS validation to make a page
load. The panel states that explicitly.

## Test

```sh
sh browser/tests/address.sh
sh browser/tests/history.sh
sh browser/tests/downloads.sh
sh browser/tests/tabs.sh
sh browser/tests/ime.sh
sh browser/tests/webkit.sh
```

`history.sh` covers record encoding, visit bounding, bookmark identity,
autocomplete ranking, Chrome JSON and Netscape HTML import, and the failure
wording, then runs a real `--import` against a temp profile twice to prove the
second run adds nothing. `address.sh` covers normalization, scheme rejection
and editing primitives. `tabs.sh` checks the tab context menu's item set and
the Duplicate / Close Others commands; `ime.sh` drives the `NSTextInputClient`
marked-text and commit entry points and checks that the composition reaches the
address field. Both need a GUI session (the IME one opens a window for a few
seconds).
`webkit.sh` builds the lifecycle project, loads trusted fixtures through real
WKWebView, checks titles/DOM/JavaScript/state/snapshot/teardown, then runs a
native window smoke if a GUI session is available. In CI where only the
headless lifecycle is safe:

```sh
NOVA_BROWSER_LIFECYCLE_ONLY=1 sh browser/tests/webkit.sh
```

## Security boundary

- WebKit navigation is limited to `http`, `https` and `about`; unsupported
  schemes fail closed with an error.
- Chrome drawn below the page rectangle is covered by the page view, so
  overlays (the address suggestion list, the rail's name panel) are registered
  with `gui.overlay` and the host paints them in a transparent view above the
  page, routing clicks inside them back to the browser.
- The page's user agent names the WebKit version the way Safari does, because
  Google refuses OAuth in a WebView whose agent it cannot identify; the
  renderer is still WebKit. `NOVA_BROWSER_UA` overrides it.
- The address normalizer rejects `mailto:`, `javascript:`, `file:`, `data:` and
  unknown schemes instead of silently upgrading them.
- No arbitrary external scheme launch, page-to-native bridge, file access from
  page content, private API, disabled TLS or disabled WebKit sandbox.
- Cmd+L interception only requests that Nova chrome take keyboard focus; it does
  not execute page content or expose a native callback bridge.
- The trusted fixture has no remote resources. Page JavaScript is enabled for
  WebKit only; WebCore starts with scripts and network disabled.
- History, bookmarks and settings stay in the local profile directory. No
  autocomplete query leaves the process, and `NOVA_BROWSER_NO_HISTORY=1`
  disables history persistence.
- Certificate errors are surfaced, never suppressed; there is no bypass action.
- WebCore must not call `runtime/webkit.m` or use WebKit for HTML/CSS/layout.

## Status

See `engineering/browser/AUDIT.md`, `STATE.md`, `TASKS.md`, `ROADMAP.md`,
`BLOCKERS.md` and `EVIDENCE.md`. No public release, compatibility claim or
WebCore-completion claim is authorized.
