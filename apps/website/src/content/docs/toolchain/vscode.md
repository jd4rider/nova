---
title: VS Code extension
description: Install Nova syntax highlighting and the Nova language server in Visual Studio Code.
---

The **Nova Language** extension brings `.nova` editing to Visual Studio Code without Nova Studio. It provides TextMate syntax highlighting and a language client for the same Nova language server used by Studio.

**Status:** v0.2.0 local VSIX. TypeScript compilation, VSIX packaging and `code --install-extension` were verified. A direct `nova lsp` transcript (initialize, open buffer, diagnostic, shutdown) also passed. Automated Extension Host interaction tests are not included.

## Install the VSIX

Download [`nova-language-0.2.0.vsix`](https://github.com/jd4rider/nova/releases/download/v0.2.0/nova-language-0.2.0.vsix), then either:

1. Open **Extensions**, use the `…` menu → **Install from VSIX…**, and select the file.
2. Or install from a terminal:

```sh
code --install-extension nova-language-0.2.0.vsix
```

The extension ID is `jd4rider.nova-language`. It is **not published to the VS Code Marketplace or Open VSX yet**; the release VSIX is the supported binary install.

## Requirements

| Requirement | Notes |
|---|---|
| VS Code 1.85+ | The local install was checked with VS Code 1.143.1. |
| `nova` CLI | Needed for diagnostics, completion, hover, definitions, symbols and formatting. |
| `nova.toml` workspace | Recommended for project work. The server analyzes standalone files too, but has no multi-file index. |

The compiler is not bundled in the extension. Install it with Homebrew or build it from source:

```sh
brew install jd4rider/tap/nova
nova --version
```

If VS Code cannot see your Homebrew path, set **Nova: Server Path** (`nova.serverPath`) to the absolute path of the binary. The extension starts `nova lsp` over stdio and reports failures in **Output → Nova Language Server**.

## What it provides

- Syntax highlighting from the shared [`grammar/nova.tmLanguage.json`](https://github.com/jd4rider/nova/blob/main/grammar/nova.tmLanguage.json): comments, strings and interpolation, keywords, types, functions, numbers, operators and punctuation.
- Language-server integration for the current buffer: diagnostics, completion, hover, go-to-definition, document symbols and whole-document formatting.
- **Nova: Restart Language Server** in the Command Palette.
- Automatic restart when `nova.serverPath` changes.

## Build from source

```sh
cd editors/vscode
npm ci --no-audit --no-fund
npm run typecheck
npm run package
code --install-extension nova-language-0.2.0.vsix
```

The build copies the canonical grammar into `syntaxes/` before compiling. The VSIX is written in `editors/vscode/`.

## Limits

The server currently uses full-document synchronization and single-file indexing. Incremental sync, cancellation, rename, references, semantic tokens, code actions, workspace symbols and multi-file indexing are not implemented. The extension does not include a debugger, task runner, test explorer, AI assistant or Studio UI.

It is an unsigned local VSIX, not a Marketplace-signed extension. The extension only receives documents from VS Code and launches the language server; it does not run project build tasks.

## Related

- [Language server](../lsp/)
- [CLI reference](../cli/)
- [Installation guide](../../getting-started/installation/)
- [Extension README and source](https://github.com/jd4rider/nova/tree/main/editors/vscode)
