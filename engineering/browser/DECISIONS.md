# Browser decisions

## ADR-0001 — Native WebKit embedding through a thin Objective-C host

Status: accepted for WB-02.

### Context

Nova GUI owns a single `NSWindow` and a CoreGraphics framebuffer. Nova cannot
call Objective-C methods or receive C-to-Nova callbacks. The browser needs a
real WKWebView, native page focus, WebKit process isolation and supported
system updates without moving application logic out of Nova.

### Decision

- Add `runtime/webkit.m` as a thin Objective-C/AppKit/WebKit adapter.
- Expose only C functions with scalar/String arguments and opaque `Int` handles.
- Add `nova_gui_native_view()` to hand the AppKit content view to trusted native
  adapters without exposing raw Objective-C pointers to ordinary Nova code.
- Add `libraries/nova-webview` as the Nova binding and
  `browser/engines/webkit` as the engine adapter.
- Point WebKit callbacks at an entry object and expose polled `state`, `wait`,
  `title`, `url`, `error`, `body_text` and `snapshot` functions.
- Keep WebKit navigation to `http`, `https` and `about`; reject unsupported
  schemes and never invoke arbitrary external handlers.
- Link WebKit only when the resolved IR mentions `nova_webview_`.
- Put browser-only UI and future services in `browser/`; keep the system host
  reusable and free of browser chrome.

### Alternatives rejected

- **Electron/Chromium/CEF** — prohibited hidden substitute, wrong process and
  dependency model, no Nova-owned engine path.
- **A Swift/Objective-C browser app** — violates first-party Nova application
  and engine logic.
- **Calling WebKit HTML/CSS/layout as WebCore** — false independence and a
  security/compatibility claim with no evidence.
- **A raw pointer ABI for arbitrary Obj-C objects** — Nova has no ownership or
  lifetime model for foreign objects; integer handles keep the boundary small.

### Consequences

- Native views are composited above the Nova framebuffer. Chrome must stay in a
  non-overlapping region; WebKit owns page input once focused.
- Only one window and one view are proven; tabs and multiple windows are future
  work.
- The bridge is a documented platform adapter, not WebCore. Its approximate
  size is 416 lines including comments, and it will be replaced only by another
  explicitly supported platform host.
- Qualified imported struct signatures are rejected by the current compiler, so
  the engine contract exposes primitive accessors until `BROWSER-COMP-01` is
  fixed.
- `loadHTMLString` fixtures do not create back/forward entries. Real http(s)
  navigation uses WebKit history; the fixture lifecycle test asserts only that
  an unavailable back action is harmless.

## ADR-0002 — WebCore is a separate pipeline, not a WebKit wrapper

Status: accepted; implementation not started.

WebCore will own bytes -> encoding -> HTML tokenizer/tree builder/DOM -> CSS
tokenizer/parser/selectors/cascade -> layout tree/fragments -> display list ->
paint/composite. It begins as trusted local fixtures with network and scripts
disabled (G1). Any script engine adapter must be labelled; long-term ECMAScript
work is `JS-03`. No WebCore code may call `runtime/webkit.m`.

## ADR-0003 — Address text events and explicit chrome focus

Status: accepted for WB-03.001.

### Context

Nova GUI exposed key codes but not the composed text, so an address field could
not accept punctuation, shifted characters or non-ASCII input correctly. Once
WKWebView owns focus it consumes Cmd+L, so Nova chrome cannot receive the
shortcut through the normal responder chain.

### Decision

- Emit a separate `NOVA_GUI_TEXT` event with composed UTF-8, capped at 255 bytes
  on a UTF-8 boundary. Keep the existing key event for shortcuts and games.
- Keep address editing state and URL policy in Nova. `browser/address` is a
  headless-tested normalizer, not a browser networking layer.
- Add native `focus_chrome`, `focus_page` and `page_focused` operations behind
  opaque handles.
- Install one narrow local AppKit key monitor for Cmd+L in the WebKit host. It
  only sets a polled `chrome_focus_request` and moves first responder to the
  Nova content view; it does not execute page content or call into Nova.
- Never put a native text field or browser policy in Objective-C.

### Alternatives rejected

- **Reconstruct text from key codes** — breaks punctuation, dead keys,
  composition and non-ASCII input.
- **A browser-specific native menu item** — leaks browser UI into the reusable
  Nova GUI host and affects other apps.
- **A general native callback bridge** — no designed thread/lifetime contract;
  polled focus requests are smaller and safer.
- **A native address field** — violates Nova-owned chrome and gives the platform
  host browser policy.

### Consequences

- Cmd+L works from page focus, but only because the host requests focus; any
  future command interception must be explicit and reviewed.
- IME composition, grapheme-aware movement, selection ranges, search mode and
  automated Escape/reload coverage remain follow-up work.
- Text events are per-key and capped; clipboard paste is handled by Nova's
  paste path rather than a giant text event.

## ADR-0004 — One WKWebView per tab; one installed bundle per phase

Status: accepted for WB-02.007/WB-03.002.

### Context

A single native view cannot provide independent page histories or state. The
browser also needs a launchable bundle for continued owner testing, including
its fixture outside the repository working directory.

### Decision

- Each tab owns one WKWebView handle. Only the active view is visible; inactive
  views stay alive for independent history/state.
- Tab create/close/select and closed-tab restore live in Nova. A six-tab cap
  bounds native views until overflow UI exists.
- Navigation generations are exposed by the bridge, and the bridge ignores
  completion/failure callbacks from a non-current `WKNavigation`.
- Cmd+L/T/W/Shift+T/1–6 are queued on the window host when WebKit owns focus;
  Nova polls them and owns all tab/address logic.
- `browser/scripts/install-macos.sh` rebuilds and replaces
  `/Applications/Nova Browser.app` with a bundled fixture and generated icon.
  The bundle is ad-hoc signed locally, never notarized or silently updated.

### Consequences

- Each tab costs a WebKit process/view; six is an explicit tested bound, not a
  scalability claim.
- Tab drag/reorder, overflow scrolling and process-restart restore remain
  follow-up work.
- The installed bundle contains no WebCore and clearly remains a WebKit build.
- Re-running the installer after each phase is part of the continuation protocol
  in `AGENTS.md` and `STATE.md`.
