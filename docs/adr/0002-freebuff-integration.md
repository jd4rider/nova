# ADR 0002: FreeBuff integration and AI chat

- **Status:** Accepted, amended 2026-11-23
- **Date:** 2026-11-23
- **Verified CLI:** FreeBuff 0.2.36

## Investigation

The installed FreeBuff CLI exposes an interactive terminal interface. Documented/visible options at review time were `--help`, `--version`, `--continue`, `--cwd` and `--trust-agents`. Probing confirmed that `--print` and `-p` are rejected with `unknown option`, and piping a prompt into stdin only starts the TUI. There is no headless or JSON mode, so there is no response stream to read.

No structured chat protocol, SDK, streaming API, cancellation endpoint, authentication flow or inline-completion interface was found. No endpoint, model name or token format was inferred or guessed.

## Decision

1. **Embed and scrape the real CLI (amended).** The AI panel's FreeBuff tab runs `freebuff` in a real PTY at the workspace root. The original decision was to leave the TUI untouched. At the user's request Studio now reconstructs the screen with `@xterm/headless`, filters known chrome, and exposes snapshots over a small IPC protocol as an **experimental scrape**. It is explicitly not described as a FreeBuff API, and Raw TUI remains available. Without `node-pty` the panel reports the missing TTY.
2. **Offer an opt-in chat path over documented HTTP APIs.** Anthropic and OpenAI-compatible endpoints are supported, including GitHub Models, OpenAI, OpenRouter and local Ollama/LM Studio servers. Adapters live in the main process, stream deltas over narrow IPC, support cancellation, and surface provider errors verbatim.
3. **Never substitute silently.** The panel names the configured provider and model and shows whether a key came from storage, the environment or nowhere. Unconfigured chat refuses to send instead of falling back to another service.
4. **Keep the capability probe.** Studio reads `freebuff --help` at startup and only reports headless capability if a future version documents `--print`, `--json` or `--headless`. If that appears, implement a typed adapter with mock-provider tests rather than screen scraping.

## Consequences

- Conversation text and the attached file leave the machine only when the user sends a message to the endpoint they configured, with attachment enabled.
- Keys are encrypted with `safeStorage` per provider in the app's user-data directory with `0600` permissions. They never enter the renderer, the workspace, logs or the git tree.
- `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` and `GITHUB_TOKEN` are honoured when no key is stored; the panel reports the source.
- Chat history is session-only and no usage accounting exists. GitHub Copilot uses the official language server's device flow for chat and inline completions, so no Copilot token is stored or handled by Studio. Anthropic/OpenAI-compatible keys remain encrypted with `safeStorage`.
- Edit mode proposes one complete file, then opens a Monaco diff. Keep writes only after review; rollback never touches disk. External changes detected at keep time abort the write.
- LSP completion stays fully deterministic and independent of AI.
- Verification: typecheck and production build pass; adapters are tested against a local SSE server for both wire formats, split chunks, HTTP errors, aborts and endpoint mapping; Copilot's language server returned real inline completions for a Nova file while signed in; a launch smoke test confirms the main process boots. No automated GUI interaction test exists yet.

## Revisit trigger

If FreeBuff publishes a supported structured interface, implement a typed adapter and mock-provider tests for streaming, cancellation, stale responses, malformed events, secret exclusion, patch conflicts and approval gates. A real provider smoke test is allowed only with authorized credentials.
