# FreeBuff integration

## Verified state

- Installed and tested FreeBuff CLI version: **0.2.36**.
- Available documented flags: `--help`, `--version`, `--continue`, `--cwd`, `--trust-agents`.
- Interface is an interactive terminal UI. No structured headless protocol, JSON stream, SDK, completion API, cancellation endpoint, or documented authentication flow was found.

## What Nova Studio does

- Shows FreeBuff availability and version in the sidebar.
- Offers **Open FreeBuff terminal**, which starts the CLI in a real terminal rooted at the workspace.
- Clearly disables chat, agent mode, inline ghost text, context selection, and proposed-edit review.
- Sends no repository contents and does not read or scrape FreeBuff output.
- Never substitutes another provider.

## What remains possible

A documented structured interface could be added behind a typed adapter with capabilities, streaming events, cancellation, timeouts, usage metadata, and mock tests. Until then, fabricating a chat experience would violate the project's honesty requirements.

## Security notes

FreeBuff runs as a local process with the user's permissions. Nova Studio does not automate authentication or place credentials in workspace files. Review what you paste or grant to any terminal tool. The terminal is not an approval gate for agent edits; none exists yet.
