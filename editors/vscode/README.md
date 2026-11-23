# Nova Language for Visual Studio Code

Syntax highlighting and the real Rust Nova language server for `.nova` files, without requiring Nova Studio.

## Install

### Download the VSIX

Download `nova-language-0.1.0.vsix` from the [v0.1.0 release](https://github.com/jd4rider/nova/releases/download/v0.1.0/nova-language-0.1.0.vsix), then either:

1. Open **Extensions** in VS Code, use the `…` menu → **Install from VSIX…**, and select the file.
2. Or install from a terminal:

```sh
code --install-extension nova-language-0.1.0.vsix
```

Reload VS Code if it asks. The extension ID is `jd4rider.nova-language`. It is **not published to the VS Code Marketplace yet**; the VSIX is the install path.

### Build from source

```sh
cd editors/vscode
npm ci --no-audit --no-fund
npm run package
code --install-extension nova-language-0.1.0.vsix
```

`npm run package` runs the TypeScript compiler and `@vscode/vsce`. The VSIX is written in this directory.

## Requirements

- VS Code 1.85 or newer.
- The `nova` CLI for language-server features. Install it with `brew install jd4rider/tap/nova` or [build it from source](../../INSTALL.md#install-from-source).
- Syntax highlighting works without the compiler; diagnostics, completion, hover, definitions, symbols and formatting need `nova lsp`.

If VS Code cannot find `nova` from a GUI launch, set **Nova: Server Path** (`nova.serverPath`) to an absolute path such as `/opt/homebrew/bin/nova`. You can also open a terminal in VS Code and run `which nova` to find it.

## What it provides

- TextMate grammar for comments, strings and interpolation, keywords, types, functions, numbers, operators and punctuation, copied from the shared [`grammar/nova.tmLanguage.json`](../../grammar/nova.tmLanguage.json) at build time.
- Language-server integration over stdio using `nova lsp`:
  - diagnostics for the current buffer
  - completion
  - hover
  - go-to-definition
  - document symbols
  - whole-document formatting
- **Nova: Restart Language Server** in the Command Palette.
- Automatic server restart when `nova.serverPath` changes.

Open a folder containing `nova.toml` for normal project work. The server can still analyze a standalone `.nova` file, but the current LSP has full-document sync and single-file indexing only.

## Settings

| Setting | Default | Purpose |
|---|---|---|
| `nova.serverPath` | `nova` | Executable used for `nova lsp`. Use an absolute path when `PATH` is not inherited by VS Code. |

The extension's output is in **Output → Nova Language Server**. If startup fails, the notification links to settings and the installation guide.

## Limitations

- The server uses full-document synchronization. Incremental sync, cancellation, rename, references, semantic tokens, code actions, workspace symbols and multi-file indexing are not implemented.
- No debugger, test explorer, task runner, AI assistant or Nova Studio UI is included.
- No VS Code Marketplace or Open VSX publication exists yet. The release VSIX is the supported binary distribution.
- The extension is not a sandbox around the compiler. The language server only reads buffers sent by VS Code and does not run project build tasks.
- The extension has been compiled and packaged in this repository and installed via the VS Code CLI. Automated Extension Host interaction tests are not included yet.

See the [Nova installation guide](../../INSTALL.md) and [language-server documentation](https://jd4rider.github.io/nova/toolchain/lsp/).
