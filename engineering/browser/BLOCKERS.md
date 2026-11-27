# Nova Browser blockers

## A real input method is not covered by an automated test

The `NSTextInputClient` pipeline is now tested: `tests/ime.sh` drives
`setMarkedText:selectedRange:replacementRange:` and `insertText:replacementRange:`
— the same two entry points AppKit calls — and asserts that the host reports the
marked text, Nova receives it as a preedit, and the committed string lands in
the focused address field. What is still manual is the converter itself: a live
Japanese/Chinese/Korean input source, its candidate window and its key handling
(`interpretKeyEvents:`) cannot be scripted here. The manual check is to type
with such an input source and confirm the composition appears under the caret
with its rule, then commits.

## The original stack-overflow crash: fixed (2026-11-27)

`~/Library/Logs/DiagnosticReports/nova-browser-2026-11-26-032901.ips` is
`EXC_BAD_ACCESS` / "Thread stack size exceeded" on the main thread, with the
faulting address 80 bytes below the bottom of the 8 MB stack and only
`mach_msg2_internal -> … -> nova_gui_pump -> nova_user_119 -> main` visible. No
single frame was large and the steady-state loop measured 13 KB, which is the
shape of a slow leak rather than recursion.

The cause was in codegen, not the browser: `alloca` was emitted at the current
insertion point, so a `var` declared inside a loop allocated a fresh stack slot
on every iteration. Nova Terminal died of exactly that on 2026-11-27 — 16 bytes
per main-loop frame, 8 MB exhausted in ~37 minutes, fault inside CoreText during
a draw. It is fixed in `libraries/nova-codegen/src/llvm.nova` (all slots now
live in the entry block; see `../DECISIONS.md` DEC-007 and
`../terminal/EVIDENCE.md`). The idle terminal's stack use went from +9.5 KB per
2.7 s to flat at 22 KB.

Fixed and verified in Browser on 2026-11-27. Rebuilt with the repaired tool:
`browser/target/nova/nova-browser.ll` has 0 `alloca` after any function's first
block label (1818 total), `sh browser/scripts/install-macos.sh` installed 0.9.11
at `/Applications/Nova Browser.app`, and the installed binary ran 60 s under
`NOVA_GUI_TRACE` with all 15 stack samples at `used=23KB` of 8176 KB. The
invariant is now a compiler-suite assertion (`codegen-loop-slots.nova` in
`examples/nova-tool/tests/codegen.sh`), so a late `alloca` fails the build
before it can reach either app. A 40-minute live session is still unmeasured:
the leak reads as zero rather than being waited out.

A second report at 03:40 is a **different** crash and is understood:
`-[NovaGuiView validAttributesForMarkedText]: unrecognized selector`, raised
from `-[NSTextInputContext initWithClient:]` inside
`-[NSApplication updateWindows]` on the first run-loop pass — AppKit builds the
input context as soon as the view becomes first responder, before any key
arrives. That was a real defect in the first IME change (the view declared
`NSTextInputClient` without implementing the whole protocol) and is fixed: the
view implements `validAttributesForMarkedText`, `markedRange`, `selectedRange`,
`attributedSubstringForProposedRange:actualRange:`, `characterIndexForPoint:`,
`firstRectForCharacterRange:actualRange:` and the commit/unmark pair. The
installed 0.7.0 binary contains the selector and launches cleanly; no
`nova-browser` crash report has been recorded after 03:40.

## Technical

- **BROWSER-COMP-01 — qualified imported struct signatures.** RESOLVED
  2026-11-26. Codegen now strips the module qualifier at type lookup
  (`cg_unqualified`) and stores declared names in signatures; `scene-gui`
  builds with `gui.Theme`/`gui.Event` parameters and the codegen suite is
  255/0. Browser code still uses primitive accessors; migrating to the typed
  engine contract is now possible and tracked as ordinary work, not a blocker.
- **BROWSER-COMP-03 — one module-level namespace.** Every top-level function
  from every imported module shares one namespace: `history.slice_chars`
  collided with `address.slice_chars`, and a module function named `join` or a
  qualified `gui.min(...)` call broke name resolution with a misleading
  "unknown local <alias>" error. New modules now use unique names
  (`join_records`, `parse_records`, a local `smaller`). A qualified-name binding
  fix is still needed.
- **BROWSER-COMP-04 — list parameters are rejected on some helpers.**
  `unsupported parameter type on items` appears for `List<String>` parameters
  in the browser app even though the same type works in libraries. Downloads
  therefore keep rows in a local `Downloads` struct and pass that instead.
- **BROWSER-COMP-02 — no native callbacks.** Polling works, but host callbacks,
  cancellation and cross-thread events need a designed lifetime/thread contract.
  Cmd+L uses a narrow local AppKit monitor that sets a polled focus request; it
  is not a general callback system.
- **Tab mechanics are bounded.** Six tabs maximum; horizontal/vertical layout
  is switchable and persisted and restart session restore is implemented, but
  there is no drag/reorder or overflow scrolling. The bridge guards callbacks
  against its current `WKNavigation`, but no adversarial stale-callback stress
  test exists yet.
- **No browser networking layer.** `nova_http_get` shells out to curl; WebCore
  needs a URL/HTTP/cookie/cache/origin implementation before NET-01.
- **Address is intentionally narrow.** `browser/address` normalizes known hosts
  and http(s)/about URLs; it is not a full URL parser, IDN/homograph handler or
  search-provider implementation. Plain search text is rejected.
- **Import is bookmarks only.** Chrome history is a SQLite database and is not
  read; no live Firefox/Safari profile import exists, though Netscape HTML
  exports from any of them import correctly.
- **Download pause depends on the origin.** Pausing keeps WebKit's resume data
  only when the server supports byte ranges and sent a validator; otherwise the
  row reports "cannot resume" and the partial file is left behind. Completed
  files survive restart as history rows, but there is no queue across restarts,
  Clear forgets every completed record at once (no per-row removal), there is no
  rename prompt (collisions suffix automatically) and no per-download speed or
  ETA.
- **CJK composition is protocol-only; accessibility is structural only.**
  Marked text, cluster-aware cursor/selection and the `NSTextInputClient` entry
  points are implemented and tested, and the chrome now exposes tabs, the
  address field, toolbar buttons and the engine badge as accessibility
  children (tabs, toolbar, open shelf, autocomplete). A real CJK converter, a
  hand-driven VoiceOver run and automated Escape/reload remain (WB-03.001.a /
  WB-03.004).
- **No WebCore.** Trusted fixture harness and document pipeline are not started.
- **Fixture history.** `loadHTMLString` does not populate back/forward history.
  This is a WebKit fixture limitation, not a claim that real navigation lacks
  history.
- **Hosted GUI evidence.** The lifecycle and address tests are headless and can
  run in CI; the native window smoke and live Cmd+L/tab flows require a GUI
  session. Hosted success is not claimed.
- **Local install is ad-hoc signed.** `/Applications/Nova Browser.app` is for
  local testing; no Developer ID, notarization or silent updater exists.
- **Chrome accessibility tree is partial.** It covers the tab strip, address
  field, toolbar, open download shelf and autocomplete rows; notices, tooltips
  and the failure panel's body text are not exposed, and `AXPress` is the only
  action. The page side belongs to WKWebView.

## Security / approval

- WebCore remote networking (G2) requires threat model, renderer isolation,
  broker/origin controls, adversarial tests and explicit owner approval.
- Enabling scripts in remote WebCore (G3) requires realm isolation, execution
  limits, policy enforcement and security review.
- Public browser branding/name clearance and signed/notarized distribution need
  owner action and credentials. Do not bypass Gatekeeper or publish unsigned
  updates.
- Do not expose raw page-to-native bridges or arbitrary local file access.

## Not blockers

- Electron/Chromium are prohibited, not a fallback.
- Lack of a finished WebCore does not block WebKit shell work.
- A session limit is not a blocker; resume from `STATE.md`.
