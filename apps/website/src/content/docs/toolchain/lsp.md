---
title: Language server
description: The implemented stdio LSP surface and its advertised capabilities.
---

Nova ships a Rust language server using stdio JSON-RPC. Start it with:

```sh
nova lsp
```

The server always uses stdio; `nova lsp` takes no extra flag.

## Implemented

- Initialization and capability negotiation
- Full-document open/change/save/close synchronization
- Diagnostics for the current unsaved buffer
- Completion for keywords, symbols and imports
- Hover with source text
- Go-to-definition
- Document symbols
- Document formatting through the formatter

The server reuses the lexer, parser and semantic checker. It publishes only capabilities it implements. Positions are converted from UTF-8 byte spans to the negotiated UTF-16 encoding.

## Not implemented

Incremental document synchronization, cancellation, rename, references, semantic tokens, code actions, signature help, inlay hints, workspace symbols and multi-file indexing. These are not advertised to clients.

## Limits

The server is tested through manual CLI checks and unit coverage of its position mapping, not an automated JSON-RPC transcript suite. A server restart or malformed input should not crash the IDE; report a reproducible transcript if it does.

Nova Studio uses the same server. The [Nova Language VS Code extension](../vscode/) launches the same `nova lsp` process and is the supported non-Studio editor integration. Other editors can connect to the stdio process independently of the IDE.
