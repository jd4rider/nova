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
| Cmd+Shift+I | import Google Chrome bookmarks |
| Cmd+Shift+E | tabs: top → sidebar → rail → top |
| Cmd+Shift+L | read the page aloud, or stop reading |
| Cmd+Shift+V | choose the next reader voice |
| Cmd+Shift+Space | pause or resume reading |
| Cmd+Shift+S | cycle search provider (same order as the field's chip) |
| Cmd+Shift+P | cycle theme (Gruvbox → Midnight → Light → Solarized → Nova Dark) |
| Cmd+T / Cmd+W / Cmd+Shift+T | new / close / restore tab |
| Cmd+1–6 | select a tab |

These chords are forwarded from page focus through the same narrow host queue
as Cmd+L; nothing is executed inside the page.

## Install / update

```sh
sh browser/scripts/install-macos.sh
open -a "/Applications/Nova Browser.app"
```

The script rebuilds, renders the icon, assembles `Nova Browser.app`, ad-hoc
signs it locally and replaces the installed copy. Version 0.8.6 is installed.
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

Cmd+Shift+I imports bookmarks from every detected Chromium-family profile —
Chrome, Chromium, Brave, Edge, Arc and Vivaldi all save the same
`Bookmarks` JSON — and reports how many were added. The same import runs
headlessly:

```sh
browser/target/nova/nova-browser --import                 # detected Chrome profiles
browser/target/nova/nova-browser --import path/to/Bookmarks
browser/target/nova/nova-browser --import path/to/export.html
```

Both Chrome's JSON `Bookmarks` file and a Netscape bookmark export (Google
Takeout, Chrome/Firefox/Safari "Export bookmarks") are understood. Only
`http`/`https` entries are imported; `chrome://` and `javascript:` bookmarks are
skipped. Import is idempotent, so running it twice adds nothing. Chrome's
history lives in a SQLite database and is not imported; visits accumulate from
the first launch of Nova Browser.

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

A finished download is recorded in the profile (`downloads.tsv`: bytes, time,
path, name) once, and the last 20 come back as shelf rows after a restart. Their
rows and Show-in-Finder buttons are in the accessibility tree too. Revealing a
recorded file that was moved or deleted reports that it is gone instead of
opening an empty Finder window. A Clear control in the shelf header forgets the
finished rows and the record together.

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
