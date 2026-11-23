# FreeBuff integration and AI chat

## Verified FreeBuff surface

- Tested FreeBuff CLI version: **0.2.36**.
- Documented flags: `--help`, `--version`, `--continue`, `--cwd`, `--trust-agents`.
- The interface is an interactive terminal UI. `--print` and `-p` are rejected with `unknown option`, and piping a prompt into stdin only starts the TUI. There is **no headless or JSON mode**, so there is no response stream for Studio to read.
- Studio probes `freebuff --help` at startup and only reports `headless` when a future version documents `--print`, `--json` or `--headless`.

## What Nova Studio does

The AI panel on the right has two tabs.

- **FreeBuff** runs the real CLI in a PTY at the workspace root. Output is a genuine terminal, not a parsed or replayed transcript. Without `node-pty` the panel reports the missing TTY instead of pretending to work.
- **Chat** streams replies from a provider the user configures:
  - Anthropic Claude (default `claude-sonnet-4-5`).
  - Any OpenAI-compatible endpoint, including GitHub Models (`https://models.github.ai/inference` with a GitHub token), OpenAI, OpenRouter, and a local Ollama or LM Studio server.
- Requests run in the main process. API keys are encrypted with the OS credential store (`safeStorage`) in the app's user-data directory, are never exposed to the renderer, and are never written into the workspace.
- When no key is stored, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` or `GITHUB_TOKEN` is used.
- The current file and selection are attached only when the user presses Send with **attach** enabled. Nothing is uploaded automatically.
- Streaming supports cancellation; **Insert** replaces the selection (or inserts at the caret) and **Copy** copies a reply.

## Not implemented

- No headless FreeBuff adapter, because the CLI exposes no protocol to speak.
- No Copilot or other OAuth login. GitHub Models is reachable with a personal access token, which is a different and documented mechanism.
- No inline ghost text, agent edit review, usage accounting or automatic retries.
- Chat history lives in the panel for the session only; it is not persisted.

## Verification

- `npm run typecheck` and `npm run build` cover the new main-process modules, preload bridge and renderer panel.
- The streaming adapters are exercised against a local SSE server for both wire formats, including split chunks, error responses and aborts.
- There is no automated Electron UI test yet; the panel's interaction is verified by typecheck, production build and a manual launch.

## Security notes

FreeBuff runs as a local process with the user's permissions. Studio does not automate authentication or place credentials in workspace files. Chat requests are made by the main process to the endpoint the user configures; the renderer makes no network requests and the CSP still blocks `connect-src` beyond localhost. Treat repository text and model output as untrusted data.
