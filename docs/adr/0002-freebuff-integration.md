# ADR 0002: FreeBuff integration is terminal-only

- **Status:** Accepted
- **Date:** 2026-11-23
- **Verified CLI:** FreeBuff 0.2.36

## Investigation

The installed FreeBuff CLI exposes an interactive terminal interface. Documented/visible options at review time were `--help`, `--version`, `--continue`, `--cwd` and `--trust-agents`.

No documented structured chat protocol, JSON event stream, SDK, streaming API, cancellation endpoint, authentication flow, or inline-completion interface was found. No endpoint, model name or token format was assumed. The CLI's authentication behavior and redistribution terms were not inferred from terminal scraping.

## Decision

Provide a dedicated FreeBuff terminal in Nova Studio, launched with the `freebuff` executable at the workspace root when a PTY is available.

Disable structured chat, agent mode, inline ghost text, context injection and proposed-edit review with an explicit explanation. Do not scrape terminal output or automate authentication. Do not substitute another provider.

Keep a provider boundary for a future documented interface: capability discovery, auth status, streaming events, cancellation, timeouts, usage metadata only when provided, and versioned context. No fake adapter is shipped.

## Consequences

- The directive's AI sidebar is honestly unavailable, not simulated.
- LSP completion remains fully deterministic and independent of AI.
- FreeBuff requires a PTY; without `node-pty` the terminal cannot run the TUI. Studio reports that limitation.
- Repository content is not sent automatically. A user-run terminal program can still do anything with the user's permissions.
- Credentials must never be written to tracked settings, logs or frontend bundles. No keychain integration exists yet because there is no structured auth flow to store.

## Revisit trigger

If FreeBuff publishes a supported structured interface, implement a typed adapter and mock-provider tests for streaming, cancellation, stale responses, malformed events, secret exclusion, patch conflicts and approval gates. A real provider smoke test is allowed only with authorized credentials.
