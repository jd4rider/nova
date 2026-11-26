# Packaging and release artifacts

Nova publishes unsigned preview artifacts built from this repository.

| Artifact | Platform | Format |
|---|---|---|
| `Nova-Studio-0.1.0-arm64.dmg` | macOS arm64 | DMG with `Nova Studio.app` and `/Applications` link |
| `Nova-Studio-0.1.0-arm64.zip` | macOS arm64 | Zipped `.app` |
| `Nova-Studio-0.1.0-x64-setup.exe` | Windows x64 | NSIS installer, assisted mode |
| `nova-0.1.0-aarch64-apple-darwin.tar.gz` | macOS arm64 | CLI, runtime and README |
| `nova-language-0.1.0.vsix` | VS Code 1.85+ | TextMate grammar and Nova LSP client |
| `nova-0.1.0-source.tar.gz` | Source | Tagged repository source |
| `SHA256SUMS` | All | SHA-256 manifest |

## Build locally

```sh
(cd examples/nova-tool && ../../bootstrap/nova build)

cd editor
npm ci --no-audit --no-fund
CSC_IDENTITY_AUTO_DISCOVERY=false npm run dist:mac
npm run dist:win
```

`dist:win` uses `--config.npmRebuild=false` so the optional macOS-built `node-pty` module is not cross-compiled. Windows Studio reports pipe mode for the terminal; FreeBuff also needs a TTY and is unavailable there.

The VS Code extension is packaged separately:

```sh
cd editors/vscode
npm ci --no-audit --no-fund
npm run package
```

## Signing status

- macOS: unsigned and not notarized. The bundle is ad-hoc signed after packing (`codesign --sign -`) so its resources are sealed and macOS does not report the app as damaged; it is still not Developer ID signed, so Gatekeeper asks for confirmation on first launch.
- Windows: unsigned. SmartScreen may warn.
- VS Code extension: unsigned local VSIX; not published to the Marketplace or Open VSX.
- No release assets are represented as signed until a real Developer ID/Authenticode pipeline exists.

## Homebrew

The public tap lives at <https://github.com/jd4rider/homebrew-tap>, sourced from [`homebrew/`](homebrew/).

```sh
brew install --cask jd4rider/tap/nova-studio
```

This installs both Studio and the `nova` formula. The tap must be updated with new SHA-256 values for every release.
