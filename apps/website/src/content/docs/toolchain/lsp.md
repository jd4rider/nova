---
title: Language server
description: The implemented stdio LSP surface and its advertised capabilities.
---

Nova ships a language server implemented in Nova over stdio JSON-RPC. It is part
of the self-hosted toolchain, not a separate front end. Start it with:

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

The server reuses the Nova lexer, parser and semantic checker. It publishes only
capabilities it implements. Positions are converted from UTF-8 byte spans to the
negotiated UTF-16 encoding. Document formatting calls the Nova formatter in
`libraries/nova-fmt`.

## Not implemented

Incremental document synchronization, cancellation, rename, references, semantic
tokens, code actions, signature help, inlay hints, workspace symbols and
multi-file indexing. These are not advertised to clients.

## Limits

`examples/nova-tool/tests/lsp.sh` drives initialize, open, diagnostics,
completion, hover, definition, symbols and formatting as an automated transcript.
Registry boxes are not resolved by the server: it indexes the open buffer and
path imports only.

Nova Studio uses the same server. The [Nova Language VS Code extension](../vscode/)
launches the same `nova lsp` process and is the supported non-Studio editor
integration. Other editors can connect to the stdio process independently of the
IDE.
