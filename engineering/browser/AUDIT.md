# Nova Browser and Nova WebCore audit

Date: 2026-11-26
Platform: macOS ARM64, LLVM + Clang
Repository: `jd4rider/nova`
Status: WebKit fixture shell with editable address/focus handoff implemented;
WebCore not started

## Directive availability

The owner requested `nova-browser-dual-engine-master-directive.md`. No file with
that name existed in the repository or project tree at audit time. The complete
directive text supplied in the owner message is the authority for this work and
is recorded as binding scope in `DIRECTIVE.md`, `ROADMAP.md` and `TASKS.md`.
Future sessions must treat those files plus the continuation protocol as the
resume source.

## Baseline commands

| Command | Result |
|---|---|
| `(cd examples/nova-tool && ../../bootstrap/nova build)` | PASS; Nova-built compiler refreshed |
| `sh examples/nova-tool/tests/codegen.sh` | PASS, 248/0 after WebKit linker changes |
| Full 11-suite compiler baseline | PASS; results in `EVIDENCE.md` |
| `sh browser/tests/webkit.sh` | PASS; lifecycle 25/0, shell smoke ran locally |
| `sh browser/tests/address.sh` | PASS, 25/0 normalization/editing assertions |
| `sh browser/tests/webkit.sh` | PASS, lifecycle 25/0, shell smoke, tabs |
| `clang -fobjc-arc -O2 -c runtime/webkit.m` | PASS |
| `clang -fobjc-arc -O2 -c runtime/gui.m` | PASS |

The full 11-suite compiler baseline is recorded in `EVIDENCE.md` after the
checkpoint run. No claim is made for untested platforms or hosted-runner GUI
behavior.

## Component audit

| Component | Status | Evidence / source | Gap or task |
|---|---|---|---|
| Nova compiler and LLVM/Clang pipeline | verified | `bootstrap/nova`, `examples/nova-tool`; codegen 248/0 | Preserve; browser adds no new language runtime |
| Nova GUI window and rasterizer | verified | `runtime/gui.m`, `libraries/nova-gui`; real window smoke; accessibility 11/11 | One window, no multi-window; chrome tree covers tabs/toolbar only |
| Native-view handoff | verified | `nova_gui_native_view()` in `runtime/gui.m` | Opaque handle only; main-thread contract documented |
| Page/chrome focus handoff | verified | `runtime/webkit.m` focus/shortcut queue; live Cmd+L/T/W/Shift+T | IME/marked-text focus not handled |
| GUI text input | verified | `NOVA_GUI_TEXT`, `typed_text()`, IME 4/4 | No accessibility text-range API; composition is protocol-tested only |
| Tabs | verified | one WKWebView per tab; live create/close/restore; lifecycle two-view test | Six-tab cap, no drag/reorder or restart restore |
| C ABI / FFI | partial | `extern "C"` supports scalar, String and Unit values; no callbacks | No structs/pointers/callbacks; polling + integer handles |
| Qualified imported struct types | broken for signatures | `contract.PageStatus` return/parameter rejected by codegen | `BROWSER-COMP-01`; use flattened accessors for now |
| WebKit embedding | verified for trusted fixture | `runtime/webkit.m`, `libraries/nova-webview`, `browser/engines/webkit` | One view per tab; permissions, downloads, private profiles, find, zoom, inspector absent |
| Native shell | partial | `browser/app/src/main.nova`; address 56/0; accessibility 11/11; lifecycle 25/0 | No manual VoiceOver pass; Escape/reload automation; shelf rows in the tree |
| Installed app bundle | verified | `/Applications/Nova Browser.app` 0.3.0; icon, plist, ad-hoc signature | Developer ID/notarization absent; local testing only |
| Engine contract | partial | `browser/engine-api/src/engine.nova` | Primitive accessors due `BROWSER-COMP-01`; no dynamic trait objects |
| WebCore document pipeline | missing | no source | WC-01 onward |
| HTML tokenizer/tree builder/DOM | missing | no source | WC-02/WC-03 |
| CSS parser/cascade | missing | no source | WC-04 |
| Layout/paint/display list | missing | no source | WC-05/WC-06 |
| JavaScript engine inside WebCore | not started | WebKit uses system JavaScriptCore | JS-01/02/03; no hidden delegation allowed |
| Networking for WebCore | missing | `nova_http_get` shells to `curl`; WebKit owns its network | URL/HTTP/cookie/origin work in NET-01 |
| Browser history/bookmarks/downloads storage | missing | no browser storage schema | WB-04 |
| Nova Boxes packaging | partial | registry read/pack exists; publishing absent | Browser libraries are source imports, not published packages |
| LSP / Nova Studio | verified, not browser-integrated | existing tests | Browser inspector is DEV-01 |
| Accessibility / IME / screen reader semantics | partial | chrome tree from a pushed element list; `tests/accessibility.sh` 11/11; IME 4/4 | Page side is WKWebView's; no manual VoiceOver run; WB-03.004 |
| Naming and public branding | unknown | no trademark/search clearance performed | Do not publish browser branding |
| Licensing | partial | repository license; system WebKit framework | Record WebKit/OS dependency notices before release |
| Packaging, signing, updates | missing | no app bundle | REL-01; owner credentials required |
| CI | partial | compiler CI on `macos-15`; lifecycle step added | Hosted GUI shell smoke remains local evidence |

## What already exists and must be reused

- `runtime/gui.m` and `libraries/nova-gui` for the window, framebuffer, theme,
  input polling, filesystem helpers and PNG output.
- `libraries/nova-boxes` and the compiler manifest reader for future packages.
- `libraries/nova-lsp` and Nova Studio for the eventual inspector surface.
- `libraries/nova-web` only for trusted HTML template generation; it is not an
  HTML parser and must not be treated as WebCore.
- `examples/nova-tool` build and linking logic, extended to link WebKit only
  when `nova_webview_` symbols are present.

## Deliberately absent / not implemented

- No Electron, Chromium, CEF, Node runtime or second application SDK.
- No WebCore HTML/CSS/layout/paint implementation yet.
- No hidden WebKit fallback behind a WebCore claim.
- No arbitrary local file or external-scheme navigation in the WebKit bridge.
- No address search provider, IME/marked-text handling, tab drag/reorder,
  restart session restore, credentials, cookies, history, bookmarks, downloads,
  private mode or permission UI yet.
- No network-enabled WebCore, no script-enabled remote WebCore, no release.

## Dependency and risk notes

- WebKit is the system framework; updates follow the OS. No bundled WebKit.
- The bridge uses supported `WKWebView` APIs only, default process isolation
  and default website data store. JavaScript remains enabled for WebKit pages;
  WebCore begins with scripts and network disabled.
- `nova_webview_snapshot` writes a native WebKit snapshot because the Nova
  framebuffer PNG cannot include native subviews.
- `loadHTMLString` fixtures do not populate back/forward history; real http(s)
  navigations use WebKit history. This is recorded rather than hidden.
- Cmd+L is consumed by WKWebView when page content owns focus. The host monitor
  only requests chrome focus; Nova owns the address state and URL policy.
