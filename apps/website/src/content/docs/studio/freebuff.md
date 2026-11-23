---
title: FreeBuff and AI
description: The verified FreeBuff CLI surface, the embedded TUI, and streaming chat providers.
---

## Investigated FreeBuff interface

At development time the installed FreeBuff CLI is **0.2.36**. Its documented interface exposes `--help`, `--version`, `--continue`, `--cwd` and `--trust-agents`. It is an interactive terminal UI: `--print` and `-p` are rejected, and piping a prompt into stdin only starts the TUI. There is **no headless or JSON mode**, so there is no response stream for Studio to read.

Studio probes `freebuff --help` at startup and only reports headless capability if a future version documents `--print`, `--json` or `--headless`.

## What Studio provides

The AI panel has two tabs.

- **FreeBuff** runs the real CLI in a PTY at the workspace root. It is a genuine terminal, not a parsed or replayed transcript, and requires `node-pty`. Studio does not scrape the screen, invent an endpoint or automate authentication.
- **Chat** streams replies from a provider you configure: Anthropic Claude, or any OpenAI-compatible endpoint such as GitHub Models, OpenAI, OpenRouter, or a local Ollama/LM Studio server.
- Requests are made by the main process. Keys are encrypted with the OS credential store, never exposed to the renderer and never written into the workspace. `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` and `GITHUB_TOKEN` are used when no key is stored.
- The open file and selection are attached only when you press Send with **attach** enabled. Replies can be stopped, inserted at the caret or copied.

## Not implemented

- No headless FreeBuff adapter, because the CLI exposes no protocol to speak.
- No Copilot or other OAuth sign-in. GitHub Models is supported through a personal access token instead.
- No inline ghost text, agent edit review, usage accounting or persisted chat history.
- No automated Electron interaction test yet; the panel is covered by typecheck and production build, not by a GUI suite.

## Security

FreeBuff is a local process with the user's permissions. Studio does not send repository contents on its own. Chat requests go only to the endpoint you configure, from the main process, so the renderer keeps a `connect-src` CSP limited to localhost. Treat repository text and model output as untrusted data, and do not paste secrets into tools.
