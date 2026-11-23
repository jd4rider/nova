---
title: FreeBuff integration
description: The verified FreeBuff CLI surface and why structured AI is disabled.
---

## Investigated interface

At development time the installed FreeBuff CLI is **0.2.36**. Its documented/visible interface exposes `--help`, `--version`, `--continue`, `--cwd` and `--trust-agents`. It is an interactive terminal UI. No documented structured chat protocol, JSON event stream, SDK, cancellation API, authentication flow or inline-completion endpoint was found.

## What Studio provides

- A dedicated **FreeBuff terminal** launched at the workspace root when a PTY is available.
- Availability and version in the sidebar.
- An explicit explanation that chat, agent mode and inline ghost text are disabled.
- No terminal-output scraping, no invented API endpoint and no automatic credential handling.
- No substitution of another AI provider.

## Why not fake it

A terminal UI is not a low-latency completion service. Parsing its screen would be fragile, could leak secrets and would not provide cancellation or versioned document context. A chatbot wrapper would violate the directive's requirement for a real integration.

## Future adapter contract

If FreeBuff documents a supported structured interface, an adapter may be implemented behind a typed provider boundary:

- capability discovery and availability
- auth status without logging secrets
- versioned streaming chat events and cancellation
- explicit context selection with secret exclusions
- diff preview, per-file review, apply/reject and rollback
- optional inline completion only when latency and document-version guarantees exist

Until then, deterministic LSP completion remains available and AI status is honestly shown as terminal-only or unavailable.

## Security

FreeBuff is a local process with the user's permissions. Studio does not send repository contents on its own. Do not paste secrets into a terminal tool, and treat repository text and tool output as untrusted data.
