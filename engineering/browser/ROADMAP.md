# Nova Browser roadmap

Status: installed WebKit shell 0.3.0 with editable address and tabs; WebCore not
started. No public release or compatibility claim is authorized.

| ID | Deliverable | Status | Acceptance |
|---|---|---|---|
| WB-00 | Audit and baseline | done | `AUDIT.md`, baseline commands, dependency graph |
| WB-01 | Engine interface and mock | partial | primitive contract; mock not shipped |
| WB-02 | macOS WebKit embedding | done for trusted fixture | WKWebView, focus, bundle and lifecycle evidence |
| WB-03 | Native shell and tabs | partial | tabs/address/focus done; errors/accessibility remain |
| WB-04 | Profiles/history/bookmarks/downloads | planned | persistence and privacy tests |
| WB-05 | WebKit usable RC (G0) | planned | real-site smoke, errors, private data behavior |
| WC-01 | Trusted fixture harness/headless runner | planned | deterministic output, network/scripts off |
| WC-02 | HTML tokenizer | planned | state-by-state fixtures |
| WC-03 | Tree builder and DOM | planned | malformed markup, invariants |
| WC-04 | CSS parsing and cascade | planned | selector/computed-style tests |
| WC-05 | Block/inline layout | planned | geometry/reftests |
| WC-06 | Text/images/display lists | planned | deterministic raster evidence |
| WC-07 | Input/scroll/links/forms | planned | local interaction tests |
| WC-08 | Flex/grid/positioning | planned | targeted WPT/reftests |
| WC-09 | WebCore tab integration (G1) | planned | visible engine label; no fallback |
| SEC-01 | Renderer isolation/broker | planned | sandbox/IPC adversarial tests |
| NET-01 | WebCore network/origin policy | planned | local server security tests |
| SEC-02 | Experimental remote gate (G2) | blocked | threat model, sandbox, approval |
| JS-01 | Script interface and budgets | planned | realm/timeout tests |
| JS-02 | Optional maintained engine adapter | blocked | dependency/isolation/security review |
| JS-03 | Nova ECMAScript interpreter | planned | incremental Test262 |
| JS-04 | DOM bindings/event loop/Web APIs | planned | integration and policy tests |
| DEV-01 | Browser-owned inspector | planned | real engine data, local-only |
| REL-01 | App bundle, signing, updates | partial | local ad-hoc bundle installed; Developer ID/notarization absent |
| PORT-01 | Linux/Windows feasibility | planned | platform audit; no false support |

## Priority order

1. Finish WB-03 error/accessibility work and WB-04 persistence without weakening
   WebKit security.
2. Start WC-01/WC-02 in parallel with WebKit work.
3. Do security prerequisites before any WebCore network exposure.
4. Keep JS/advanced rendering optional until foundations are correct.

## Immediate next task

`WB-03.003`: visible loading/offline and certificate-failure states, retry, and a
deterministic local failure fixture. Do not suppress certificate errors. Then
`WB-04.001` history/bookmarks with a temp-profile privacy test.
