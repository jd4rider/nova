# FreeBuff, Copilot and AI chat

## Verified FreeBuff surface

- Tested FreeBuff CLI version: **0.2.36**.
- Documented flags: `--help`, `--version`, `--continue`, `--cwd`, `--trust-agents`.
- The interface is an interactive terminal UI. `--print` and `-p` are rejected with `unknown option`, and piping a prompt into stdin only starts the TUI. There is **no headless or JSON mode**.
- Studio still probes `freebuff --help` at startup and reports `headless` only when a future version documents `--print`, `--json` or `--headless`.

## What Nova Studio does

The AI panel on the right has two tabs.

### FreeBuff

- **Scraped chat** is the default. Studio runs the real `freebuff` CLI in a PTY at the workspace root, feeds its output into a headless xterm screen, filters the known TUI chrome, and exposes screen snapshots as chat messages over a small structured IPC event stream. This is a **scrape, not a FreeBuff API**. It is labelled experimental in the panel and can be replaced when FreeBuff publishes a supported protocol.
- **Raw TUI** opens the untouched terminal in the bottom panel. Without `node-pty` the panel reports the missing TTY instead of pretending to work.
- **New chat** restarts the FreeBuff process.

### Chat

- **GitHub Copilot** uses the official `@github/copilot-language-server`. Sign-in is GitHub's device flow: Studio shows the one-time code, opens the verification page through the language server, and reads the authenticated user from LSP status. The server stores its own credentials; Studio never receives a Copilot token.
- Copilot **ghost text** is enabled in `.nova` files while signed in. Monaco asks the main process for an inline completion, which forwards the current document position to `textDocument/inlineCompletion`. Copilot's acceptance telemetry command is preserved on the completion item.
- Copilot **chat** uses the language server's `conversation/create` protocol with a work-done token. Streaming text and quota/HTTP errors from the server are shown verbatim.
- Anthropic Claude and any OpenAI-compatible endpoint remain available. Keys are encrypted with the OS credential store (`safeStorage`), never exposed to the renderer, and never written into the workspace.
- The optional `gh copilot` CLI extension is separate from Copilot in the editor. The panel offers **Install gh Copilot CLI** and **Check gh login**, which run real `gh` commands in a terminal.

### Edit mode (keep or rollback)

- Enable **edit mode** in the chat composer. The assistant is asked for one fenced block containing the complete replacement file.
- **Review edit** opens a Monaco side-by-side diff of the current buffer against the proposal. **Keep changes** writes the file only after the diff opens; **Rollback** discards it without touching disk.
- A keep refuses to overwrite a file that changed on disk after the proposal was made. The file and LSP document are updated after a successful keep.

## Limits

- The FreeBuff scrape depends on the TUI's screen layout and prompt text. It is intentionally marked experimental and may miss or include chrome after a FreeBuff change; Raw TUI is the fallback.
- Copilot chat requires an active Copilot plan and available AI credits. Quota errors are surfaced rather than retried or hidden.
- Copilot chat is Ask mode. Agent tools, inline-edit suggestions and Copilot's own multi-file agent approvals are not wired into Studio.
- Chat history is session-only and is not persisted.
- FreeBuff and Copilot run as local processes with the user's permissions. There is no automated GUI interaction test yet.

## Verification

- `npm run typecheck` and `npm run build` pass.
- Copilot's authenticated language server was probed directly: it returned real Nova inline completions for `prin` (`print("The total is: …")`) and reported the signed-in user.
- Copilot chat's request schema was exercised; the account's exhausted-credit error was returned by `conversation/create` and is surfaced verbatim.
- FreeBuff was driven through a real PTY and its screen reconstructed with `@xterm/headless`. The current adapter is heuristic, as described above.

## Security notes

Copilot credentials stay with the official language server, not in Studio's settings or the workspace. FreeBuff runs as a local process with the user's permissions. Chat requests from Anthropic/OpenAI-compatible providers are made by the main process; the renderer makes no AI network requests and the CSP still blocks `connect-src` beyond localhost. Treat repository text and model output as untrusted.
