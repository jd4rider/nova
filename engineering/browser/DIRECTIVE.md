# Nova Browser dual-engine directive

Source: owner-supplied directive, 2026-11-26. The requested
`nova-browser-dual-engine-master-directive.md` file was not present at audit
time; the owner message is authoritative. This file records the binding
engineering interpretation for future sessions.

## Mission

Build a native desktop browser in Nova with two honest engines:

1. **WebKit** — macOS system `WKWebView`, useful immediately, with native
   sandboxing, TLS and modern web compatibility left intact.
2. **Nova WebCore** — an independent Nova-owned document, style, layout and
   paint pipeline. It must not delegate HTML/CSS/layout/rendering to WebKit or
   Chromium.

Nova application and engine logic must be first-party Nova. Thin Objective-C
platform adapters are allowed at the ABI boundary and must be documented.
Electron, Chromium, CEF, a mandatory Node runtime and hidden WebCore delegation
are prohibited.

## Non-negotiables

- Audit before adding components; reuse existing Nova libraries.
- Never claim compatibility, security, performance or self-hosting without
  reproducible evidence.
- Preserve the compiler, LSP, Nova Studio, Nova GUI, Boxes, website and working
  tests.
- WebKit is default. WebCore is experimental and visibly labelled.
- Never silently fall back to WebKit while claiming WebCore rendered a page.
- WebCore starts with trusted local fixtures, network and scripts disabled.
- No arbitrary local file access or privileged bridge from page content.
- External audited libraries are acceptable for TLS/fonts/images/OS primitives;
  do not invent cryptography.
- Missing language capabilities get a minimal repro and a tracked compiler or
  stdlib task.

## Security gates

- **G0** — WebKit shell usable with platform security intact.
- **G1** — WebCore local trusted fixtures only; no scripts/network by default.
- **G2** — isolated experimental WebCore networking only after threat model,
  sandbox/IPC/origin controls and adversarial tests; explicit opt-in.
- **G3** — script-enabled remote browsing only after realm isolation, execution
  limits, policy enforcement and security review.
- **G4** — broader readiness requires measured compatibility, patching process,
  security review, crash recovery and documented limits.

Never bypass a gate to demonstrate a site. Preserve restricted mode and work on
another slice when a prerequisite is blocked.

## Current scope

The first slice is WB-00 through WB-02: audit, engine contract and a real
WKWebView embedded in a Nova GUI window, with a trusted offline fixture and
headless lifecycle evidence. WebCore begins at WC-01 and is not implemented.
Public naming, signing, release and remote WebCore browsing require owner
approval and additional gates.
