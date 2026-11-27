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
