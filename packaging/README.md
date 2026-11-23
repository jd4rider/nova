# Packaging and release artifacts

Nova publishes unsigned preview artifacts built from this repository.

| Artifact | Platform | Format |
|---|---|---|
| `Nova-Studio-0.1.0-arm64.dmg` | macOS arm64 | DMG with `Nova Studio.app` and `/Applications` link |
| `Nova-Studio-0.1.0-arm64.zip` | macOS arm64 | Zipped `.app` |
| `Nova-Studio-0.1.0-x64-setup.exe` | Windows x64 | NSIS installer, assisted mode |
| `nova-0.1.0-aarch64-apple-darwin.tar.gz` | macOS arm64 | CLI and README |
| `nova-0.1.0-source.tar.gz` | Source | Tagged repository source |
| `SHA256SUMS` | All | SHA-256 manifest |

## Build locally

```sh
cargo build --release -p nova-cli

cd editor
npm ci --no-audit --no-fund
CSC_IDENTITY_AUTO_DISCOVERY=false npm run dist:mac
npm run dist:win
```

`dist:win` uses `--config.npmRebuild=false` so the optional macOS-built `node-pty` module is not cross-compiled. Windows Studio reports pipe mode for the terminal; FreeBuff also needs a TTY and is unavailable there.

## Signing status

- macOS: unsigned and not notarized. The DMG is an ad-hoc/linker-signed Electron build.
- Windows: unsigned. SmartScreen may warn.
- No release assets are represented as signed until a real Developer ID/Authenticode pipeline exists.

## Homebrew

The public tap lives at <https://github.com/jd4rider/homebrew-tap>, sourced from [`homebrew/`](homebrew/).

```sh
brew install --cask jd4rider/tap/nova-studio
```

This installs both Studio and the `nova` formula. The tap must be updated with new SHA-256 values for every release.
