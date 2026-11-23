# Homebrew tap

This directory is the source for the public tap:

<https://github.com/jd4rider/homebrew-tap>

## Install Studio and the language together

```sh
brew install --cask jd4rider/tap/nova-studio
```

If Homebrew asks you to trust the tap: `brew trust jd4rider/tap`. The `nova-studio` cask depends on the `nova` formula, so Homebrew installs the compiler/CLI and the desktop app in the same operation. This was verified end to end on macOS arm64: `nova 0.1.0` plus `/Applications/Nova Studio.app`.

## Install the language alone

```sh
brew install jd4rider/tap/nova
nova --version
```

The formula currently installs the prebuilt macOS arm64 CLI archive. Studio’s DMG is macOS arm64 only; Windows and source paths are documented in [INSTALL.md](../../INSTALL.md).

## Update the tap for a release

1. Build and attach the release assets.
2. Replace the version, URL and SHA-256 values in `Formula/nova.rb` and `Casks/nova-studio.rb`.
3. Copy the two files into `jd4rider/homebrew-tap` (`Formula/` and `Casks/`) and push.
4. Validate with:

```sh
brew tap jd4rider/tap
brew info jd4rider/tap/nova
brew info --cask jd4rider/tap/nova-studio
brew install --cask --dry-run jd4rider/tap/nova-studio
```

Homebrew casks do not replace the app’s own code-signing and notarization. The current artifacts are explicitly unsigned.
