# Nova Browser evidence

Date: 2026-11-26
Platform: macOS ARM64, system WebKit, LLVM/Clang

## Compiler and bridge

```sh
(cd examples/nova-tool && ../../bootstrap/nova build)
# built .../examples/nova-tool/target/nova/nova-tool

clang -fobjc-arc -O2 -c runtime/webkit.m -o /tmp/nova-webkit-check.o
clang -fobjc-arc -O2 -c runtime/gui.m -o /tmp/nova-gui-check.o
# both PASS

sh examples/nova-tool/tests/codegen.sh
# passed 248, failed 0
```

Full compiler/toolchain baseline on the final tree:

| Suite | Result |
|---|---|
| codegen | 248 passed, 0 failed |
| diff | 18 matched, 0 differed |
| diff-errors | 27 matched, 0 mismatched |
| parser | 55 passed, 0 failed |
| sema | 4 passed, 0 failed |
| sema-ast | 39 passed, 0 failed |
| cli | 33 passed, 0 failed |
| lsp | diagnostics, completion, hover, definition, symbols, formatting ok |
| boxes | 64 passed, 0 failed |
| formatter | 6 passed, 0 failed |
| toolchain | 128 passed, 0 failed |

## Headless WebKit lifecycle

```sh
(cd browser/tests/lifecycle && ../../../examples/nova-tool/target/nova/nova-tool build)
browser/tests/lifecycle/target/nova/webkit-lifecycle
```

Result: **25 PASS, 0 FAIL**. Covered:

- engine identity `WebKit / WKWebView`, JavaScriptCore, network and declared
  process isolation, plus no false independent-layout claim. Process isolation
  is WKWebView's default architecture; this slice does not independently
  measure it;
- WKWebView creation;
- two real `loadHTMLString` navigations with titles `Fixture One` and
  `Fixture Two`;
- DOM text and page JavaScript (script appends a `js-ran` marker that the
  test reads back from the live DOM);
- absent fixture history queried and back action harmless;
- unsupported `mailto:` scheme fails closed with a clear error;
- navigation generation advances after a new load;
- two independent WKWebView tabs load separate titles, preserve the first tab,
  switch visibility and destroy only the second;
- destruction invalidates the handle;
- native WebKit snapshot written to `/tmp/nova-browser-webkit.png`.

Snapshot: PNG, 2360x1432, RGBA, non-interlaced, 3,188,760 bytes.

The complete browser script also passes:

```sh
NOVA_BROWSER_LIFECYCLE_ONLY=1 sh browser/tests/webkit.sh  # PASS, shell skipped
sh browser/tests/webkit.sh                                # PASS, shell smoke included
```

The CI workflow now runs the address suite and the lifecycle-only mode on
`macos-15`. The hosted run itself has not been observed in this session; only
local execution is evidence.

## Address model

```sh
sh browser/tests/address.sh
# 25 PASS, 0 FAIL
```

Covered: bare hosts become https; localhost/loopback become http; paths and
http(s) URLs are preserved; input is trimmed; `mailto:`, `javascript:`, `file:`,
`data:`, `custom:` and empty `https://` are rejected; `about:blank` is allowed;
insert/backspace/delete and Unicode character counting work. This is a
normalizer, not a full URL parser.

## Live native keyboard and focus

```sh
NOVA_GUI_TRACE=/tmp/nova-browser-trace.log NOVA_BROWSER_TRACE=1 \
  browser/target/nova/nova-browser
# Cmd+L, paste "about:blank", Enter
# browser: navigate about:blank
```

The clean single-cycle run printed exactly one navigation. In a separate typing
run, a keystroke produced `text bytes=1` in the host trace and
`browser: address text length=1` in the shell, confirming the composed-text
event. Cmd+L worked while the page initially owned focus because the narrow host
monitor requested chrome focus; no page text or credentials are logged. This is
macOS GUI-session evidence only; it is not an automated CI test.

## Nova GUI shell smoke

```sh
(cd browser && ../examples/nova-tool/target/nova/nova-tool build)
NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_SNAPSHOT=/tmp/nova-browser-shell.png \
  browser/target/nova/nova-browser
```

Result: exit 0:

```text
browser: state=ready title=Nova Browser — WebKit online snapshot=true
```

`/tmp/nova-browser-shell.png` is a non-empty 2360x1364 RGBA PNG after the tab
chrome was added. The downscaled preview shows the actual WebKit-rendered
fixture, including the JavaScript-enabled status line. The Nova chrome is drawn
separately in the window framebuffer; the native snapshot covers page content
only.

## Tabs

Live window trace while WebKit owned focus:

```text
browser: shortcut code=116 mods=1
browser: tab created count=2
browser: shortcut code=119 mods=1
browser: tab closed count=1
browser: shortcut code=116 mods=3
browser: tab restored count=2
```

One live run first exposed pending shortcuts being stored on an arbitrary
WKWebView entry; after moving focus/shortcut requests to shared window scope,
the create/close/restore sequence passed. Six tabs is a tested cap, not a
scalability claim.

## Installed bundle

```sh
sh browser/scripts/install-macos.sh
# Installed /Applications/Nova Browser.app
# Version 0.3.0 · dev.nova.browser.native · ad-hoc signed

plutil -extract CFBundleShortVersionString raw \
  "/Applications/Nova Browser.app/Contents/Info.plist"
# 0.3.0
codesign --verify --strict "/Applications/Nova Browser.app"
# valid on disk; satisfies its Designated Requirement
```

The bundle contains `Contents/Resources/start.html` and a 1024×1024
`NovaBrowser.icns` (1,115,802 bytes). Running its executable from `/tmp` produced
`state=ready`, proving resource lookup is bundle-relative. `open -a` launched it
through LaunchServices. This is ad-hoc local signing, not Developer ID signing
or notarization.

## 0.4.0 — chrome polish, library and failure surface

```sh
sh browser/tests/history.sh
# library model: every PASS, failures=0
# import: files=1 found=2 new=2 bookmarks=2 path=/tmp/.../bookmarks.tsv
# import: files=1 found=2 new=0 bookmarks=2   (second run: idempotent)
# import: files=1 found=1 new=1 bookmarks=3   (Netscape HTML export)
```

The library suite covers record escaping, visit bounding at 400, bookmark
identity/dedupe, autocomplete ordering (bookmark first, host prefix over title
substring, repeated visit over single), Chrome JSON import with `chrome://`
rejection, Netscape HTML entities and quoted attributes, and failure wording
including `does not bypass` for certificates.

```sh
NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_UI_SNAPSHOT=/tmp/nova-ui-h.png \
  NOVA_BROWSER_SNAPSHOT=/tmp/nova-page.png browser/target/nova/nova-browser
# browser: state=ready title=Nova Browser — WebKit online snapshot=true ui=true tabs=1 layout=horizontal

NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_LAYOUT=vertical \
  NOVA_BROWSER_UI_SNAPSHOT=/tmp/nova-ui-v.png browser/target/nova/nova-browser
# browser: ... ui=true tabs=1 layout=vertical

NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_URL=mailto:test@example.com \
  NOVA_BROWSER_UI_SNAPSHOT=/tmp/nova-ui-fail.png browser/target/nova/nova-browser
# browser: state=error ... ui=true tabs=1 layout=horizontal
```

The chrome snapshots (2360×1364 RGBA) show the horizontal strip, the vertical
sidebar layout, and the error panel with the typed address, WebKit's detail and
a single action. Native WKWebView content is not part of the Nova framebuffer,
so these prove chrome, not page rendering. The `state=error` line is the
blocked-scheme path failing closed.

`sh browser/tests/address.sh` reports failures=0 and
`NOVA_BROWSER_LIFECYCLE_ONLY=1 sh browser/tests/webkit.sh` reports
failures=0.

## 0.5.0 — downloads, session restore, opt-in search, frame budget

```
$ sh browser/tests/address.sh
... 38 PASS ...
failures=0

$ sh browser/tests/history.sh
... 72 PASS ...
failures=0
import: files=1 found=2 new=2 bookmarks=2 path=/tmp/nova-browser-profile.…/bookmarks.tsv
import: files=1 found=2 new=0 bookmarks=2
import: files=1 found=1 new=1 bookmarks=3

$ sh browser/tests/downloads.sh
--- server ---
--- download ---
download: state=done name=nova-fixture.bin bytes=5120 path=/tmp/nova-browser-download.…/downloads/nova-fixture.bin
downloads: file size 5120 bytes
downloads: PASS

$ NOVA_BROWSER_LIFECYCLE_ONLY=1 sh browser/tests/webkit.sh
... 25 PASS ...
failures=0

$ NOVA_BROWSER_BENCH=120 browser/target/nova/nova-browser
bench: frames=120 avg=8ms worst=203ms fx_step_2000=4ms
```

The download is a real `WKDownload`: a loopback Python server answers
`/payload.bin` with `Content-Disposition: attachment` and 5120 bytes, WebKit
writes the file into `NOVA_BROWSER_DOWNLOAD_DIR`, and the test checks the byte
count on disk. It is not a simulated progress row. The first frame includes
window/WebKit setup, so `worst` is a cold start, not steady state; idle frames
are skipped entirely by the dirty signature.

```
browser: state=ready title=Nova Browser — WebKit online ui=true tabs=1 layout=horizontal
browser: state=ready title=Nova Browser — WebKit online ui=true tabs=1 layout=vertical
```

### 0.5.1 — address selection

`NOVA_BROWSER_FOCUS_ADDRESS` opens with the field focused and selected, so the
chip can be inspected without driving keystrokes:

```sh
NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_FOCUS_ADDRESS=1 \
  NOVA_BROWSER_UI_SNAPSHOT=/tmp/nova-ui-address.png browser/target/nova/nova-browser
NOVA_BROWSER_AUTOCLOSE=1 \
  NOVA_BROWSER_FOCUS_ADDRESS='https://example.com/nova/browser/releases/0.5.0/notes?tab=downloads&from=chrome' \
  NOVA_BROWSER_UI_SNAPSHOT=/tmp/nova-ui-address-long.png browser/target/nova/nova-browser
```

Pixel measurement of the toolbar band (`chip_right` vs the rightmost bright
text pixel) gives margin +6 px for the fixture URL and +4 px for a 142-character
URL that overflows the field: the chip always ends past the last drawn pixel,
including when the ink is clipped mid-glyph.

Zoomed crops (`/tmp/zoom-nova-ui-address*.png`) show the chip beginning on the
`h` of `https` and ending just past the final `/`, inside the field's 16 px
right padding, in the blended blue rather than the border colour. The long-URL
snapshot shows the whole address; when the text exceeds the field, select-all
keeps the start visible and typing scrolls to the caret.

`sh browser/tests/address.sh` stays at 38 PASS / failures=0 (the geometry is
drawing code; the editing model assertions are unchanged).

### 0.5.2 — select-all overflow

Same snapshot commands with a 143-character URL
(`NOVA_BROWSER_FOCUS_ADDRESS='https://example.com/nova/browser/releases/0.5.2/...'`)
now render the end of the URL (`…highlighting-geometry`) with the chip spanning
`x=222..962` against ink `x=227..957`: the field is fully highlighted, the
leading glyph is clipped by the clip rect rather than the chip, and nothing
draws outside the highlight.

`sh browser/tests/address.sh` 38 PASS / `failures=0`,
`sh browser/tests/history.sh` 72 PASS / `failures=0`.

### 0.6.0 — selection, IME, tab drag

```sh
NOVA_BROWSER_AUTOCLOSE=1 NOVA_BROWSER_FOCUS_ADDRESS=1 \
  NOVA_BROWSER_UI_SNAPSHOT=/tmp/nova-ui-focus.png browser/target/nova/nova-browser
NOVA_BROWSER_BENCH=120 browser/target/nova/nova-browser
```

- `tests/address.sh` 38 PASS / 0 FAIL, `tests/history.sh` 72 PASS,
  `tests/downloads.sh` PASS (5120-byte real `WKDownload`), lifecycle 0 failures.
- Bench after the suggestion cache: `frames=120 avg=4ms worst=177ms
  fx_step_2000=3ms` (was `avg=6ms`).
- Horizontal, vertical and focused snapshots render; the address chip still
  hugs the glyphs (`chip=[229..395]`, ink `[233..391]`).
- IME protocol: the view answers `validAttributesForMarkedText`,
  `attributedSubstringForProposedRange:actualRange:`, `characterIndexForPoint:`,
  `hasMarkedText`, `markedRange`, `selectedRange`, `setMarkedText:…`,
  `unmarkText`, `insertText:…` and `firstRectForCharacterRange:actualRange:`.
  Verified that the input context initialises without an exception at launch;
  actual CJK composition has **not** been driven by a test (see BLOCKERS).

### 0.6.1 — launch after a crash

The failure was reproducible only for the installed bundle: direct launch of the
copy in `/Applications` never returned, and `sample` showed

```
nova_gui_pump -> nextEventMatchingMask -> _handleAEOpenEvent
  -> _reopenWindowsAsNecessaryIncludingRestorableState
  -> promptToIgnorePersistentStateWithCrashHistory -> NSAlert runModal
```

i.e. AppKit's crash-restore alert running a modal loop behind the window. After
disabling persistent UI and adding the modal guard:

```sh
"/Applications/Nova Browser.app/Contents/MacOS/nova-browser"   # AUTOCLOSE=1
# launch #1: 1.2s exit=0  browser: state=ready ... WebKit online
# launch #2: 0.9s exit=0  browser: state=ready ... WebKit online
```

The target binary was never affected (1.3 s), which is why earlier checks
passed while the installed app appeared hung.

### 0.7.0 — IME automation, tab menu, stack headroom

```sh
$ sh browser/tests/tabs.sh
PASS menu has the four tab commands
PASS close-others is disabled with one tab
PASS duplicate adds one tab
PASS close-others leaves one tab
PASS session still renders after the commands
tabs: 5 PASS, 0 FAIL

$ sh browser/tests/ime.sh
PASS host marked text is live
PASS Nova receives the composition
PASS host clears marked text on commit
PASS committed text reaches the address field
ime: 4 PASS, 0 FAIL
```

`ime.sh` opens a real window (GUI session required) with the address field
focused; the host injects `にほ` through `setMarkedText:` and then `日本` through
`insertText:`, the two exact entry points AppKit uses. The log proves the host
reported `marked=にほ has=1`, Nova printed `browser: preedit=にほ`, and the
commit produced `ime: committed=日本 has=0` and
`browser: address text=…日本`. The converter/candidate window itself is still a
manual check.

```sh
$ NOVA_BROWSER_TAB_MENU_ITEMS=1 NOVA_BROWSER_TAB_ACTION=1 … nova-browser
menu: items=New Tab|Duplicate Tab|Close Tab|Close Other Tabs enabled=1,1,1,0
browser: action duplicate tabs=2
browser: action close-others tabs=1
```

The menu is built by `nova_gui_build_tab_menu()` from the same command set the
right-click handler shows; with one tab Duplicate stays enabled and Close
Others is disabled. The two multi-tab commands run through the tab model
(`duplicate_tab`, `close_other_tabs`). Right-click itself is not injectable, so
opening the menu with a real mouse press remains unverified.

```sh
$ NOVA_BROWSER_BENCH=120 … nova-browser
bench: frames=120 avg=4ms worst=179ms fx_step_2000=3ms stack=13KB/8176KB
$ NOVA_BROWSER_BENCH=2000 … nova-browser
bench: frames=2000 avg=4ms worst=183ms fx_step_2000=4ms stack=13KB/8176KB
```

The stack figure is `pthread_get_stackaddr_np` minus the sampled frame address
inside `nova_gui_pump`. The peak is identical at 120 and 2000 frames, so the
loop neither grows its stack nor recurses. A scan of all 768 generated
functions found no frame reservation larger than 64 KB, which matches the
03:29 report's shallow backtrace: the stack pointer was already exhausted on
entry to `mach_msg`, so the cause is not a big or recursive frame in the pump
path. The original steps remain unknown (BLOCKERS).

```sh
$ sh browser/scripts/install-macos.sh
Installed /Applications/Nova Browser.app
Version 0.7.0 · dev.nova.browser.native · ad-hoc signed
$ plutil -extract CFBundleShortVersionString raw "/Applications/Nova Browser.app/Contents/Info.plist"
0.7.0
$ codesign --verify --deep --strict "/Applications/Nova Browser.app"   # valid
$ "/Applications/Nova Browser.app/Contents/MacOS/nova-browser"          # AUTOCLOSE=1
# launch 1: 1.3s  browser: state=ready title=Nova Browser — WebKit online
# launch 2: 1.0s  browser: state=ready title=Nova Browser — WebKit online
```

All earlier suites were re-run on this build: address 38 PASS / failures=0,
history 72 PASS / failures=0, downloads 5120 bytes PASS, lifecycle 25 PASS /
0 failures. UI snapshot `/tmp/nova-070-focus.png`, 34,473 bytes.

### Crash-report identification (03:40, IME selector)

The 03:40:19 report supplied by the owner is `SIGABRT` /
`-[NovaGuiView validAttributesForMarkedText]: unrecognized selector`, thrown from
`-[NSTextInputContext initWithClient:]` inside `-[NSApplication updateWindows]`
on the first run-loop pass. That is the first IME change declaring
`NSTextInputClient` without the full protocol, and it is a different crash from
the 03:29 stack overflow. The installed 0.7.0 binary contains the selector:

```sh
$ nm -m "/Applications/Nova Browser.app/Contents/MacOS/nova-browser" | grep validAttributesForMarkedText
00000001000444e4 (__TEXT,__text) non-external -[NovaGuiView validAttributesForMarkedText]
$ find ~/Library/Logs/DiagnosticReports -name 'nova-browser*' -newermt '2026-11-26 03:41'
(none)
$ (AUTOCLOSE=1) "/Applications/Nova Browser.app/Contents/MacOS/nova-browser"
browser: state=ready title=Nova Browser — WebKit online
```

`tests/ime.sh` also exercises the same view through the input-client entry
points on every run, so the protocol missing a method fails the build's test
step rather than the launch.

## 0.7.1 — grapheme-aware address editing

The address model no longer steps by Unicode scalar. `browser/address` groups
scalars into extended grapheme clusters (the bundled self-hosted compiler has no
`graphemes()` method; the runtime's splitter exists but is not exposed to Nova
code), and the browser uses those boundaries for Left/Right, Backspace/Delete,
click carets and word snapping.

```sh
$ sh browser/tests/address.sh
PASS accent is two scalars
PASS accent is one cluster
PASS left skips the whole accent
PASS right skips the whole accent
PASS backspace removes the accent
PASS delete removes the accent
PASS family is five scalars
PASS family is one cluster
PASS left keeps the family whole
PASS backspace keeps the family whole
PASS flag is two scalars
PASS left keeps the flag whole
PASS thumb is two scalars
PASS delete keeps the thumb whole
PASS ascii still steps one
PASS snap inside a cluster
PASS snap on a boundary
PASS accent text survives insertion
failures=0                     # 56 PASS total, up from 38
```

The cases are `e` + U+0301, `👨‍👩‍👧` (man ZWJ woman ZWJ girl), `🇺🇸` (two regional
indicators) and `👍🏽` (thumbs-up + skin-tone modifier). Segmentation covers
combining marks (U+0300–036F, U+1AB0–1AFF, U+1DC0–1DFF, U+20D0–20FF,
U+FE20–FE2F), variation selectors, keycaps, emoji skin tones, regional-indicator
pairs and ZWJ sequences. Indic conjuncts, Arabic/Hebrew vowel marks and Hangul
jamo are not joined — the same approximation the runtime documents.

Regression run on the same build:

```sh
$ sh browser/tests/history.sh            # 72 PASS, failures=0
$ sh browser/tests/tabs.sh               # tabs: 5 PASS, 0 FAIL
$ sh browser/tests/ime.sh                # ime: 4 PASS, 0 FAIL
$ sh browser/tests/downloads.sh          # 5120 bytes, PASS
$ NOVA_BROWSER_LIFECYCLE_ONLY=1 sh browser/tests/webkit.sh   # 25 PASS, 0 FAIL
$ NOVA_BROWSER_BENCH=120 browser/target/nova/nova-browser
bench: frames=120 avg=4ms worst=174ms fx_step_2000=4ms stack=13KB/8176KB
$ sh browser/scripts/install-macos.sh
Version 0.7.1 · dev.nova.browser.native · ad-hoc signed
$ PlistBuddy -c 'Print :CFBundleShortVersionString' .../Info.plist   # 0.7.1
$ codesign --verify --deep --strict "/Applications/Nova Browser.app"  # valid
$ (AUTOCLOSE=1) installed binary                                      # 1190 ms, state=ready
```

The address suite covers the model primitives; the browser's key/click wiring of
them is compile-verified and shares those functions, but a real keyboard was not
driven across a combining sequence. That interactive step remains manual.

## 0.7.2 — chrome accessibility tree

The chrome is one rasterized `NSView`, so AppKit had no widget tree to walk. The
app now pushes the controls it just drew while drawing, and the host turns each
entry into an `NSAccessibilityElement` child with a role, label, value, tab
selection and a screen-space frame. The shape (children built on demand, press
forwarded to the control) follows the Mac accessibility layer WebKit uses;
tabs use the `AXTab` role Safari exposes rather than the `AXRadioButton` AppKit's
own tab view would pick, because the strip is Nova-drawn.

```sh
$ NOVA_GUI_AX_DUMP=1 <installed binary>
ax: children=11 built=11
ax: role=AXTab label=New Tab value= selected=1 frame=1214,1219,200,30
ax: role=AXButton label=Back value= selected=0 frame=1218,1181,30,30
ax: role=AXButton label=Forward value= selected=0 frame=1258,1181,30,30
ax: role=AXButton label=Reload value= selected=0 frame=1298,1181,30,30
ax: role=AXTextField label=Address value=https://fixture.nova.invalid/ selected=0 frame=1344,1179,761,34
ax: role=AXButton label=Bookmark value= selected=0 frame=2115,1181,30,30
ax: role=AXButton label=Downloads value= selected=0 frame=2151,1181,30,30
ax: role=AXButton label=Nova docs value= selected=0 frame=2187,1181,30,30
ax: role=AXButton label=Tab layout value= selected=0 frame=2223,1181,30,30
ax: role=AXStaticText label=Rendering engine: WebKit value= selected=0 frame=2261,1189,37,14
ax: role=AXButton label=New tab value= selected=0 frame=2264,1219,30,30
```

`built=11` is AppKit's own count from `accessibilityChildren`, so these are the
children a client receives, not a Nova-side claim. Frames are screen-space
converted from the top-left drawing coordinates. `AXPress` emits the ordinary
click at the element centre, reusing the chrome hit-testing.

```sh
$ sh browser/tests/accessibility.sh
PASS every pushed element becomes an accessibility child
PASS the tab is exposed as a tab
PASS the active tab reports selection
PASS the address field is a text field
PASS back is a pressable button
PASS reload is a pressable button
PASS the new-tab control is exposed
PASS the engine badge is readable
PASS the session still renders with accessibility on
PASS all element frames are non-empty and in the drawn geometry
PASS dump stays opt-in
accessibility: 11 PASS, 0 FAIL
```

Regression and install on the same build:

```sh
$ sh browser/tests/address.sh            # 56 PASS, failures=0
$ sh browser/tests/history.sh            # 72 PASS, failures=0
$ sh browser/tests/tabs.sh               # 5 PASS, 0 FAIL
$ sh browser/tests/ime.sh                # 4 PASS, 0 FAIL
$ sh browser/tests/downloads.sh          # 5120 bytes, PASS
$ NOVA_BROWSER_LIFECYCLE_ONLY=1 sh browser/tests/webkit.sh   # 25 PASS, 0 FAIL
$ NOVA_BROWSER_BENCH=120 browser/target/nova/nova-browser
bench: frames=120 avg=5ms worst=179ms fx_step_2000=4ms stack=13KB/8176KB
$ sh browser/scripts/install-macos.sh
Version 0.7.2 · dev.nova.browser.native · ad-hoc signed
$ codesign --verify --deep --strict "/Applications/Nova Browser.app"   # exit 0
$ PlistBuddy -c 'Print :CFBundleShortVersionString' .../Info.plist     # 0.7.2
$ installed binary with NOVA_BROWSER_AUTOCLOSE=1                       # 1315 ms, state=ready
$ installed binary with NOVA_GUI_AX_DUMP=1                             # 12 ax lines
```

Building the tree exposed a compiler bug: an extern call taking two strings and
a trailing integer miscompiles — the second string and the integer arrive as
register garbage (`pushed=6094079072` on every element). Value and selection are
therefore attached by separate one-argument calls, which the dump verifies.

Not covered: no VoiceOver session was driven by hand, `AXPress` is the only
action, and the download shelf, suggestion rows, notices and tooltips are not in
the tree. The page's accessibility belongs to `WKWebView` and is untouched.

## 0.7.3 — download pause/resume

0.5.0 could only cancel or reveal a download. A running row now pauses: the
partial file stays and WebKit's `cancel:` completion supplies resume data.
Clicking the paused row resumes through the public
`WKWebView.resumeDownloadFromResumeData:completionHandler:`, which re-enters
`decideDestinationUsingResponse:` — the entry keeps its existing path so the
transfer continues into the same file instead of allocating `name-1`. The same
path retries a failed transfer that carries resume data. A row whose origin
gave no resume data says "cannot resume" rather than silently restarting.

Evidence from a real loopback transfer (51,200 bytes, Range-capable server with
`ETag`/`Last-Modified`, body trickled in 2 KB chunks):

```sh
$ NOVA_BROWSER_DOWNLOAD_PAUSE=1 browser/target/nova/nova-browser --download http://127.0.0.1:PORT/payload.bin
download: paused at 10240
download: paused state received=10240 resumable=1
download: resumed
download: state=done name=nova-fixture.bin bytes=51200 path=.../downloads/nova-fixture.bin
downloads: resumed file size 51200 bytes
$ same binary, same download directory, second run
download: state=done name=nova-fixture-1.bin bytes=51200 path=.../downloads/nova-fixture-1.bin
downloads: PASS
```

Regressions and install on the same build:

```sh
$ sh browser/tests/accessibility.sh   # 11 PASS, 0 FAIL
$ sh browser/tests/address.sh         # failures=0
$ sh browser/tests/history.sh         # failures=0, imports new=1
$ sh browser/tests/tabs.sh            # 5 PASS, 0 FAIL
$ sh browser/tests/ime.sh             # 4 PASS, 0 FAIL
$ NOVA_BROWSER_LIFECYCLE_ONLY=1 sh browser/tests/webkit.sh   # 25 PASS, 0 FAIL
$ NOVA_BROWSER_BENCH=120 browser/target/nova/nova-browser
bench: frames=120 avg=6ms worst=176ms fx_step_2000=4ms stack=13KB/8176KB
$ sh browser/scripts/install-macos.sh
Version 0.7.3 · dev.nova.browser.native · ad-hoc signed
$ codesign --verify --deep --strict "/Applications/Nova Browser.app"   # exit 0
$ PlistBuddy -c 'Print :CFBundleShortVersionString' .../Info.plist     # 0.7.3
$ installed binary with NOVA_BROWSER_AUTOCLOSE=1                       # 1244 ms, state=ready
```

Two things had to change beyond the shelf:

- The first attempt produced `resumable=0` at 10,240 bytes. The origin sent no
  `ETag`/`Last-Modified`, so WebKit refused to hand out resume data; the test
  server now sends both and honours `Range` with a 206. This is a real
  limitation of pause, not a test artefact, and the shelf states it.
- `WKDownload.webView` is nil for a download that began as a main-frame
  navigation, so the entry keeps the `WKWebView` the delegate was called with
  (strongly, released on finish) — otherwise resume has no view to call.

The frame budget moved from 5 ms to 6 ms average because each shelf row polls
one more accessor (`download_resumable`) per frame; the shelf is closed in the
bench, so this is the snapshot cost, not a regression in the idle path.

Not covered at the time: no download history across restarts, no rename prompt
(collisions suffix automatically), no speed/ETA, and the shelf was not yet in the
accessibility tree. History landed in 0.7.5 and the shelf tree in 0.7.4.

## 0.7.4 — shelf and suggestion accessibility

0.7.2 published tabs, the address field and the toolbar; the shelf and the
autocomplete rows were Nova-drawn surfaces with no names. Both now push their
own elements: the shelf close control, its empty-state text, and one
Pause/Resume/Retry control per running row plus Cancel/Show-in-Finder named with
the file; autocomplete rows are `AXLink` children whose label is the title (or
URL) and whose value is the target URL.

`NOVA_BROWSER_DOWNLOADS_OPEN=1` opens the shelf before the first frame, so the
empty shelf's controls can be checked without a download:

```sh
$ NOVA_BROWSER_DOWNLOADS_OPEN=1 NOVA_GUI_AX_DUMP=1 nova-browser
ax: children=13 built=13
ax: role=AXButton label=Close downloads value= selected=0 frame=2262,539,22,22
ax: role=AXStaticText label=No downloads yet value= selected=0 frame=1958,509,140,14
```

`tests/accessibility.sh` grew from 11 to 16 checks: the base chrome (11
elements), the open shelf (13 children), a seeded bookmark that must appear as
exactly one link with `value=https://nova.example/docs`, and the opt-in guard.

```sh
$ sh browser/tests/accessibility.sh
... 16 checks ...
accessibility: 16 PASS, 0 FAIL
```

Regressions and install on the same build:

```sh
$ sh browser/tests/address.sh      # failures=0
$ sh browser/tests/history.sh      # failures=0, imports new=1
$ sh browser/tests/tabs.sh         # 5 PASS, 0 FAIL
$ sh browser/tests/ime.sh          # 4 PASS, 0 FAIL
$ sh browser/tests/downloads.sh    # downloads: PASS (pause/resume + collision)
$ NOVA_BROWSER_LIFECYCLE_ONLY=1 sh browser/tests/webkit.sh   # 25 PASS, 0 FAIL
$ NOVA_BROWSER_BENCH=120 browser/target/nova/nova-browser
bench: frames=120 avg=6ms worst=175ms fx_step_2000=4ms stack=13KB/8176KB
$ sh browser/scripts/install-macos.sh
Version 0.7.4 · dev.nova.browser.native · ad-hoc signed
$ codesign --verify --deep --strict "/Applications/Nova Browser.app"   # exit 0
$ PlistBuddy -c 'Print :CFBundleShortVersionString' .../Info.plist     # 0.7.4
$ installed binary with NOVA_BROWSER_AUTOCLOSE=1                       # 1252 ms, state=ready
```

Not covered: notices, tooltips and the failure panel's body text are still not
in the tree, `AXPress` is the only action (no focus/scroll/value setters), and
no VoiceOver session was driven by hand.

## 0.7.5 — download history across restart

A finished transfer used to vanish with the process. Completion now writes one
line per file to `<profile>/downloads.tsv` (`bytes`, `when`, escaped path,
escaped name) from both the frame loop and the `--download` path, which returns
before the loop. On launch the last 20 records return as shelf rows with
negative idents, so they cannot collide with live WebKit ids; a history row is
suppressed while a live row with the same destination path is on screen.
Revealing goes through `NSWorkspace` and fails honestly when the file was moved:

```sh
$ NOVA_BROWSER_PROFILE=$TMP/profile ... nova-browser --download http://127.0.0.1:$PORT/payload.bin
download: state=done name=nova-fixture.bin bytes=51200 path=.../downloads/nova-fixture.bin
$ cat $TMP/profile/downloads.tsv
51200	281391913	/tmp/.../downloads/nova-fixture.bin	nova-fixture.bin
```

`tests/downloads.sh` now ends with a restart: the same profile is opened with
`NOVA_BROWSER_DOWNLOADS_OPEN=1 NOVA_GUI_AX_DUMP=1`, and the dump must contain
two history rows and their actions, with exactly two persisted records:

```sh
ax: role=AXStaticText label=nova-fixture.bin value= selected=0 frame=1952,533,332,42
ax: role=AXButton label=Show nova-fixture.bin in Finder value= selected=0 frame=2250,541,26,26
ax: role=AXStaticText label=nova-fixture-1.bin value= selected=0 frame=1952,487,332,42
ax: role=AXButton label=Show nova-fixture-1.bin in Finder value= selected=0 frame=2250,495,26,26
downloads: 2 completed rows survived restart
downloads: PASS
```

Regressions on the same build: accessibility 16/16, address failures=0, history
failures=0, tabs 5/5, IME 4/4, lifecycle 25/0, bench
`frames=120 avg=6ms worst=180ms fx_step_2000=4ms stack=15KB/8176KB`. Installed
`Version 0.7.5 · dev.nova.browser.native · ad-hoc signed`, plist 0.7.5,
`codesign --verify --deep --strict` exit 0, installed launch 1242 ms to
`state=ready`.

Not covered: there is no way to clear a history row from the shelf, cancelled
and failed transfers are not recorded, and a file deleted outside the browser
only surfaces when its row is clicked.

## 0.7.6 — clearing finished downloads

History could be added but not removed. The shelf header now shows **Clear**
while the profile holds completed files; it drops WebKit's finished rows and the
persisted record in one action, so the next frame cannot re-record the cleared
paths, and writes the emptied profile immediately. The first version cleared
memory but left `downloads.tsv` intact: the profile is written by the frame
loop, and the autoclose path returns before that save. Persisting inside the
clear path fixed it.

The control is a named button, and `tests/downloads.sh` requires it after the
restart before exercising the same path through `NOVA_BROWSER_CLEAR_DOWNLOADS=1`:

```sh
$ ... NOVA_BROWSER_CLEAR_DOWNLOADS=1 NOVA_BROWSER_AUTOCLOSE=1 NOVA_GUI_AX_DUMP=1 nova-browser
ax: role=AXStaticText label=No downloads yet value= selected=0 frame=1958,509,140,14
downloads: clear emptied the profile record
downloads: PASS
```

Regressions on the same build: accessibility 16/16, address failures=0, history
failures=0, tabs 5/5, IME 4/4, lifecycle 25/0, bench
`frames=120 avg=5ms worst=165ms fx_step_2000=4ms stack=15KB/8176KB`. Installed
`Version 0.7.6 · dev.nova.browser.native · ad-hoc signed`, plist 0.7.6,
`codesign --verify --deep --strict` exit 0, installed launch 1251 ms to
`state=ready`. The website gains a Nova Browser page and the site check passes
(52 pages, 2294 internal references).

Not covered: Clear forgets every completed record at once — there is no per-row
removal or confirmation — and records for files these rows never held (cancelled
and failed transfers) still do not exist.

## 0.7.7 — hover and pointer shape

Hover felt slow and landed on the wrong control because motion was queued, not
sampled. `next_event` returned one event per pass and the loop slept 6 ms after
every event, so moves arrived faster than the loop consumed them and the pointer
position used for hit testing trailed the cursor. Two changes: the host now drops
queued moves and drags when a newer one is already waiting (a move never swallows
the click or drag that follows), and the browser sleeps only on passes with no
event. The hover fade step went from 90 to 240 per pass.

Hit testing and drawing were compared box by box for the toolbar: back, forward
and reload are hit at `toolbar_y + 2, 30x30` and drawn at the same rectangle; the
address field is 34 high in both; star/downloads/docs/layout use the same metrics
for hit and paint. The offset was temporal, not geometric.

The pointer shape now follows the hover target through a new
`nova_gui_set_cursor` host call (0 arrow, 1 hand, 2 I-beam), applied from the
view's cursor rects so an AppKit reset to arrow does not stick: hand over
toolbar buttons, tabs, suggestions, shelf rows and failure buttons; I-beam over
the address field; arrow over disabled back/forward and plain chrome.

Checks on the installed 0.7.7: build clean, downloads PASS (including clear),
accessibility, address (failures=0), history (failures=0), tabs, IME, lifecycle
25/0, bench `frames=120 avg=7ms worst=182ms fx_step_2000=4ms
stack=15KB/8176KB`. `Version 0.7.7 · dev.nova.browser.native · ad-hoc signed`,
plist 0.7.7, signature valid, launch 1263 ms.

Not covered: no automated test drives real pointer motion, so the lag and the
cursor shape were reasoned from the queue and hit-box code, not measured under a
moving hand. Frames are ~2 ms slower than 0.7.6 in this bench run, within the
run-to-run spread but not proven to be noise.

## 0.8.0 — search engines, pill field, glow

Search existed but was opt-in and invisible: the default was `off`, the only
control was Cmd+Shift+S, and the field had a small amber square for `http://`
pages that read as a stray glyph. Now `load_store` defaults to Google, a chip
inside the field shows the engine initial and cycles it on a press (persisted in
`settings.tsv`), and `with_search_row` prepends `Search Google for "…"` with the
real query URL whenever the text is not a URL. The suggestion row is exposed to
AX as a link; the dump shows
`role=AXLink label=Search Google for "nov" value=https://www.google.com/search?q=nov`.

The scheme square is gone. The field is a pill (radius 17) with a glow while
editing, hovered icon buttons and the active tab carry the same halo, the
toolbar has a six-band wash, and the loading bar has seven sparks. Glow uses a
new host primitive, `nova_gui_round_rect_alpha` (rounded path + opacity), so the
halo is three cheap fills instead of an offscreen shadow pass.

One real bug surfaced while testing: the autoclose path used by the AX test and
the downloads test draws its own frame and rebuilt suggestions separately, so
the engine row was missing there and the AX count changed by one. Both paths now
call `with_search_row`.

Checks on the installed 0.8.0: build clean, accessibility 17/17, address 0
failures, downloads PASS (including clear), history failures=0, tabs 5/5, IME
4/4, lifecycle 25/0, bench `frames=120 avg=7ms worst=179ms fx_step_2000=4ms
stack=15KB/8176KB`, `Version 0.8.0 · dev.nova.browser.native · ad-hoc signed`,
signature valid, launch 1226 ms.

Not covered: the search row and Enter path are verified structurally and in the
model tests, not by a live typing run; no real page was searched. Firefox
`places.sqlite` is not read — only Netscape `bookmarks.html` exports through
`--import`. The glow is not measured for visual quality, only for frame cost.

## 0.8.1 — click-to-select-all in the address field

Hover already produced an I-beam over the field (target 4 → `cursor_text`), but
a press from outside edited at the clicked caret, so typing inserted into the
middle of a URL instead of replacing it. The click path now treats
`!current.editing` like a triple click: `address_select_all` over the whole live
URL, with the same AX field value and the same selection chip. A second press in
the focused field places the caret; a double-click selects the word.

Checks on the installed 0.8.1: build clean, accessibility 17/17, address 0
failures, downloads PASS, history failures=0, tabs 5/5, IME 4/4, lifecycle 25/0,
bench `frames=120 avg=7ms worst=172ms`, signature valid, launch 1209 ms.

Not covered: no automated mouse press drives this path — the rule is verified by
inspection and by the unchanged AX field, not by a simulated click.

## 0.8.4 — themes, light mode, no square around the pill

The palette was hardcoded Gruvbox. Theme resolution is now a pipeline:
`settings.tsv theme` (default `gruvbox`) or `NOVA_BROWSER_THEME`, through
`theme_named` (gruvbox, midnight, light, solarized, nova), then `theme.tsv`
overrides parsed with a new six-digit hex reader. `Cmd+Shift+P` cycles and
persists; the notice names the result. Resolving each name headlessly prints
`browser: theme=<name>` under `NOVA_BROWSER_TRACE`, which is how the five names
were checked.

One bug caught that way: the first Solarized insertion matched an ambiguous
anchor and was skipped, so `theme=solarized` resolved to Gruvbox colours while
reporting the Solarized name. The palette was inserted against the unique
`gui.dark()` anchor and re-checked.

Light and Solarized call `nova_gui_set_appearance(1)`, which sets
`NSAppearanceNameAqua` on the app, window and view, so menus and the page's
`prefers-color-scheme` are light too; dark themes set `DarkAqua`.

The square around the rounded field was the border: `nova_gui_stroke_rect` draws
a plain rectangle, so a pill fill was outlined by a box. The border is now a
rounded ring one point outside the fill, and the engine chip does the same.

Checks on the installed 0.8.4: build clean, accessibility 17/17, address 0
failures, downloads PASS, history failures=0, tabs 5/5, IME 4/4, lifecycle 25/0,
bench `frames=120 avg=7ms worst=184ms fx_step_2000=5ms`, signature valid, launch
1238 ms. `theme.tsv` overrides are parsed but only unit-checked by inspection:
no test asserts the resulting pixel colour, and no screenshot was taken.

## 0.8.5 — tab layouts, one toolbar row, drawn glyphs

`Session` gained `rail`. Layout geometry moved behind `sidebar_span`, so the
page rect, tab boxes, hit tests and accessibility frames all follow the active
layout instead of assuming a fixed sidebar. Widths that used to reserve space
for the horizontal plus now use `gui.width() - 96`, and the plus itself is gone
from both the drawing and the hit test.

New Tab moved to `new_tab_box`, below `tab_box_y(count)`. That removed the
fixed `(8, 64)` control which also carried the only New tab accessibility
element in a top strip; `tests/accessibility.sh` now expects 10 chrome children
and checks the Tab layout button instead, 12 with the shelf open and 13 with a
suggestion.

The rail's hover panel first rendered nothing. The cause is structural: the
chrome is the window's content view and each WKWebView is its subview, so
anything drawn below `page_top` is covered. The panel now takes page width
while open (`left = rail_panel_width() + 1`) and a `NOVA_BROWSER_RAIL_OPEN=1`
hook snapshots it; `/tmp/nova-rail-open2.png` shows titles and New Tab. The same
ordering explains why the autocomplete list cannot float over the page.

Icon buttons used to be a rounded fill plus a stroke plus a font glyph. They are
now transparent at rest, with a wash and glow from the eased hover energy, and
the marks are line-drawn (chevrons, octagon reload, star, tray, sidebar) so no
control depends on font coverage. `draw_icon_button` takes an int kind.

The page agent was verified end to end: a loopback page printing
`navigator.userAgent` renders exactly
`Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML,
like Gecko) Version/18.4 Safari/605.1.15` (`/tmp/nova-ua.png`). That satisfies
Google's documented requirement for its OAuth page; the Google flow itself was
not exercised, and no account sign-in was verified.

Checks: build clean, accessibility 17/17, address 0 failures, downloads PASS,
history 0 failures, tabs 5/5, IME 4/4, lifecycle 25/0, bench `frames=120 avg=7ms
worst=177ms fx_step_2000=4ms`, installed 0.8.5 ad-hoc signed. Snapshots:
horizontal, vertical and rail layouts all report `layout=` in the autoclose line.

## 0.8.6 — reading aloud and chrome above the page

Reader: a temp-profile run of the fixture page with `NOVA_BROWSER_READER=1`
printed `reader: chars=948 backend=system voice= speaking=1 notice=Reading with
Albert`. That proves the text came out of WebKit, a system voice was selected
(the empty stored voice fell back to the default English voice), and the
synthesizer entered its speaking state. Audio itself is not verifiable here and
was muted for the test with `NOVA_SPEECH_SILENT=1`; the UI snapshot shows the
speaker lit while speaking.

Kokoro was not exercised because nothing can synthesize it yet: no model and no
ONNX runtime are in the bundle. `backend` reports `system` and the note says the
model is missing; that is the honest slim-build result.

Overlay: with four bookmarks seeded and `NOVA_BROWSER_FOCUS_ADDRESS=nov`, the
suggestion list is taller than the 118 px chrome band. The GUI trace recorded
`overlay rects=1 top=1`, so the host registered the list and ordered the overlay
above the page view; the frame snapshot shows all five rows including the search
row. Previously everything below 118 px was painted under the WebView.

Two implementation notes: the topmost check is `subviews.lastObject`, because
WebKit page views are created after the chrome and would otherwise bury the
overlay; and `hitTest:` converts the superview point once, returning nil outside
the registered rectangles so the page keeps its own clicks.

Checks: build clean, accessibility 17/17 (one more control, expectations moved
to 11/13/14), address 0 failures, tabs, IME, history, downloads PASS, bench
`frames=120 avg=7ms worst=176ms fx_step_2000=4ms` after the change, and the app
installed as 0.8.6 ad-hoc signed.

## 0.8.7 — reader controls, floating rail, model fetch

Reader: `browser/tests/reader.sh` starts the fixture with `NOVA_BROWSER_READER=1`
and `NOVA_SPEECH_SILENT=1`, and reads `chars=948 backend=system voice=
rate=50 speaking=1`. That is WebKit's text, the configured default rate, and the
host in its speaking state; audio is muted and unverified.

Panel: the same run with `NOVA_BROWSER_READER_PANEL=1` saves a frame and the GUI
trace reports `overlay rects=1 top=1`, so the panel is registered above the page
view rather than under it. `ax_reader` pushes its rows as elements only while it
is open, which keeps the toolbar's accessibility count at 11 when closed.

Rail: `NOVA_BROWSER_LAYOUT=rail NOVA_BROWSER_RAIL_OPEN=1` still reports
`overlay rects=1 top=1`, and the page view now keeps `page_left` width instead
of shrinking to `rail_panel_width()` while the pointer is on the rail.

Voice model: a `file://` stub with `NOVA_KOKORO_MIN_BYTES=16` installed to
`<profile>/kokoro/model.onnx` (`bytes=38 ready=1`), and a second run with the
model present still printed `backend=system` with the note `Kokoro model
present, neural runtime not bundled`. A 4-byte stub was rejected, printed
`Voice model download failed`, and left no `model.onnx`. None of this is neural
synthesis: no ONNX runtime is linked.

Performance note: enumerating voices re-sorted every installed voice per lookup,
which made the panel take seconds per frame (a 20 s run produced no frame). The
host now caches the sorted list and exposes `nova_speech_english_count`, so the
reader, panel and install runs each finish in about 1.1–1.3 s. `backend` was
changed to always report `system` because a downloaded model is not an engine.

## 0.8.8 — Kokoro as a choice, with its own clock

Engine: the reader defaults to the system voices; `settings.tsv engine=kokoro`
(or `NOVA_BROWSER_ENGINE=kokoro` in tests) selects the neural one. The panel row
appears only once the model is installed, and says `needs a synthesizer` until
the host finds a command.

Job: `runtime/speech.m` stages the page text beside the model, runs
`<command> <text-file> <out.wav>` as an `NSTask`, and reports phases from the
process itself: 1 synthesizing, 3 playing (`afplay`), 4 failed with the
command's last output line, 5 synthesized but muted. `backend` is `kokoro` only
during 1–3, and pause sends SIGSTOP/SIGCONT to the synthesizer or the player.

Verified headlessly with stub commands in `browser/tests/reader.sh`. With a model
but no command, the fallback prints `Kokoro needs a synthesizer command
(install kokoro-tts or set NOVA_KOKORO_CMD) · reading with Albert`. A slow stub
reports `phase=1` and `backend=kokoro`, then `phase=5` and `synth: end
backend=system`. A stub that exits 3 reports `phase=4` with its own
`no such voice` text. The panel's mid-synthesis frame is
`/tmp/nova-kokoro-panel.png`, registered as `overlay rects=1 top=1`.

This does not evidence real Kokoro audio: no `kokoro-tts`/`kokoro-cli` is
installed (Python has an `onnxruntime`, but no Kokoro runner uses it here), and
the stub writes silence.

## 0.8.9 — window buttons on the layout's row

The three traffic lights are still AppKit's standard buttons, so close,
minimize, zoom and full screen keep their native behaviour; the host only sets
their frames through `nova_gui_place_traffic_lights`. `browser/tests/traffic.sh`
runs each layout with `NOVA_GUI_TRACE` and reads the host's report:
`traffic center_y=23` for horizontal tabs (the tab strip's centre) and
`traffic center_y=29` for vertical tabs and the rail (the toolbar controls'
centre). Each run also asserts the close button exists and the page still
renders.

A framebuffer snapshot cannot show these buttons: AppKit draws them above the
Nova view. The trace's frame numbers are the evidence for placement, not a
screenshot.

## 0.9.0 — voice previews

Each voice row draws a play triangle with its own hit rectangle (`190 + row`)
and accessibility button; pressing it calls `preview_voice`, which speaks
"This is how this voice reads a page." through the system synthesizer at
`voice_rate`, then restores the engine that was chosen.

`browser/tests/reader.sh` drives the same helper with `NOVA_BROWSER_PREVIEW=Albert`
and `NOVA_BROWSER_ENGINE=kokoro`, and reads
`preview: voice=Albert found=1 speaking=1 backend=system engine=kokoro
notice=Preview: Albert`. That covers the four claims: the named voice speaks,
the system engine does the speaking, the Kokoro pick survives, and the notice
names the voice. The profile's settings file is checked to contain no `voice` or
`engine` line afterwards, so a preview cannot silently change the default.

Audio is muted in this run; that the sample is audible, and that it uses the
voice's real timbre, is not evidenced.

## 0.9.1 — window buttons visible, whole chrome drags

The 0.8.9 placement read the window height (800) while AppKit keeps the buttons
in its own 32 pt `NSTitlebarView`, so the close button sat at y=770 — outside
that view and off the window's top. That is why all three buttons vanished.
The host now places the row from the titlebar view's own bounds, in whichever
direction that view counts y, and pulls a row it cannot hold up so no button
hangs off its edge.

`browser/tests/traffic.sh` (15/15) reads:

- horizontal: `traffic center_y=23 placed=23 close=13,2 host=NSTitlebarView
  flipped=0 hostH=32 buttonH=14`;
- vertical and rail: `center_y=29 placed=25 close=13,0`, the requested toolbar
  centre pulled up because the titlebar view is 32 pt tall;
- for each layout, the close frame satisfies `0 <= y` and `y + buttonH <=
  hostH`, so the row is whole inside the titlebar;
- the page still renders (`browser: state=ready`).

The drag handle moved from the host to the app. The host keeps its automatic
35 px strip for apps that do not opt in; the browser calls
`gui.set_manual_drag(true)` and starts the move with `gui.begin_window_drag()`
only when `chrome_background` says the press missed every control.
`NOVA_BROWSER_DRAG_CHECK=1` prints one probe line per layout:

- horizontal: `drag strip=1 tab=0 toolbar=1 button=0 address=0 page=0 header=1
  empty=0`;
- vertical: same with `empty=1` (the sidebar below its rows);
- rail: `header=0` (that point is the collapse chevron) and `empty=1`.

No mouse injection exists here, so what is evidenced is the hit test: which
presses ask for a window move and which keep their click. A human dragging the
window, and the buttons in real pixels, are not evidenced — AppKit draws them
above the Nova view, so framebuffer snapshots still cannot show them.

## 0.9.2 — Chrome profile import: history and search

The import now reads a whole profile folder instead of only its `Bookmarks`
file. The format is not assumed: this machine's Chrome store was inspected
read-only (key presence and table schema only, no page contents) and
`Preferences` contains `default_search_provider_data`, while `History` has
`urls(url, title, visit_count, typed_count, last_visit_time, hidden)` filled by
Chrome itself. The fixture in `browser/tests/history.sh` uses that schema.

History is read with `sqlite3 -json 'file:<profile>/History?immutable=1'`, so a
running Chrome is not blocked and its locks are not a failure, and the newest
400 `http(s)` rows become one visit each. The search engine is applied only when
`short_name`/`keyword` maps to Google, DuckDuckGo or Brave.

Evidence from `browser/tests/history.sh` (exit 0):

- the library suite prints `failures=0`, including `history import count`,
  `history skips chrome scheme`, `history import limit`, `chrome search
  duckduckgo`, `chrome search older shape` and `chrome search unknown engine`;
- importing the fixture profile prints `import: files=1 found=2 new=2
  bookmarks=2 history=2 search=duckduckgo`;
- `history.tsv` holds exactly two `visit` lines and `settings.tsv` holds
  `search=duckduckgo`;
- the second run prints `new=0 bookmarks=2 history=0 search=duckduckgo`, so the
  import is idempotent for history as well as bookmarks;
- auto-discovery is covered too: with `HOME` pointed at a fixture home holding
  `Library/Application Support/Google/Chrome/Default`, `--import` with no path
  prints the same `files=1 found=2 new=2 bookmarks=2 history=2
  search=duckduckgo`. The real Chrome profile is never read by the test.

Not evidenced: an import from this user's real Chrome profile (only the format
was checked above; no page data was read), Chrome running while the import
happens on a real profile, and behaviour when `sqlite3` is absent (the code
reports no history and still imports bookmarks and the engine). Visit frequency
is intentionally not transferred, so a Chrome-frequent page does not outrank a
Nova-frequent one.

## 0.9.3 — default browser, startup pages, real-profile import

Importing the owner's own Chrome profile is now a routine check instead of an
out-of-memory kill. The profile is read with `NOVA_BROWSER_IMPORT_DRY=1`, so
nothing is written and no page URL or title is printed or recorded here; only
counts are reported.

```sh
CH="$HOME/Library/Application Support/Google/Chrome/Default"
NOVA_BROWSER_PROFILE=$(mktemp -d) NOVA_BROWSER_IMPORT_DRY=1 \
  /usr/bin/time -l browser/target/nova/nova-browser --import "$CH"
```

- `import(dry): files=1 found=662 new=662 bookmarks=662 history=372 startup=0
  homepage= search=`, exit 0, **peak RSS 455 MB** (7,239,426,048 bytes before,
  which ended in `Killed: 9`, exit 137);
- `--import` with no path (Chrome, Chromium, Edge, Brave and Vivaldi were
  present) prints `files=3 found=662 new=662 bookmarks=662 history=400`,
  **peak RSS 474 MB**, exit 0;
- `sh browser/tests/history.sh` exits 0 with `failures=0`; the fixture profile
  prints `import: files=1 found=2 new=2 bookmarks=2 history=2 startup=2
  homepage=https://example.com/home search=duckduckgo`, then `new=0
  history=0` on the repeat run, plus an `import(dry)` line and zero files
  written;
- `traffic.sh` 15/15, `address.sh` 0 failures, bundle installed as 0.9.3 with a
  valid ad-hoc signature and `--default-browser-status` printing
  `default: state=0`.

The memory came from three separate quadratics, each measured by isolating one
file in a fixture `HOME`:

| Path | Before | Cause | After |
|---|---|---|---|
| `Bookmarks` (629 KB) | 7.9 GB | character-by-character decoding of every value, including a 300 KB `sync_metadata` string, plus an all-pairs dedupe | linear split scan, one-pass dedupe |
| `History` (23 MB) | 6.3 GB | `sqlite3 -json` dump parsed by the generic JSON parser, which copies its document per node | `sqlite3 -separator` rows read as tab-separated lines |
| full profile | 7.2 GB, killed | per-record `add_bookmark`/`add_visit` copying the whole store | `merge_bookmarks`/`merge_visits` in one pass |

The underlying language costs are recorded as `BROWSER-STD-02`: appending to a
`String` in a loop copies it, and `json.js_parse` copies the document per node.
The import avoids both rather than hiding them.

Default browser: the bundle carries `CFBundleURLTypes` for http/https and the
app calls `LSRegisterURL` + `LSSetDefaultHandlerForURLScheme`. The *system*
default is not changed by any test or by this session; the owner has not asked
for that switch, so only the reporting path is evidenced (`state=0`).

## Not yet evidenced

- hosted CI run of the browser address/lifecycle steps;
- network navigation to the Nova docs or modern sites;
- private windows and Chrome history import (bookmarks and local visit
  persistence are evidenced above);
- download pause/resume, rename prompts and download history across restarts;
- tab drag/reorder and right-click performed by a human (the model, the menu's
  item set and its commands are tested; no mouse injection exists here);
- a real CJK input source composing through the IME (the protocol entry points
  are automated, the converter is not);
- automated Escape/reload behavior;
- WebKit failure pages, certificate errors, permission prompts, find, zoom,
  inspector;
- any WebCore HTML/CSS/layout/paint output;
- Developer ID signing, notarization, silent updates or cross-platform support.

No compatibility, security, performance or WebCore-completion claim is made
beyond the commands and outputs above.
