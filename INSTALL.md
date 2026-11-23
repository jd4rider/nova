# Install Nova and Nova Studio

Nova currently ships **unsigned macOS arm64** and **unsigned Windows x64** preview installers. No Intel Mac, Linux desktop or signed/notarized package is published yet.

- [Homebrew: Studio + language](#homebrew-studio--language)
- [macOS DMG](#macos-dmg)
- [Windows EXE](#windows-exe)
- [Install from source](#install-from-source)
- [Verify a download](#verify-a-download)
- [Uninstall](#uninstall)

## Homebrew: Studio + language

The public tap installs **Nova Studio and the `nova` language together**. The cask declares the language formula as a dependency:

```sh
brew install --cask jd4rider/tap/nova-studio
```

If Homebrew asks you to trust the tap, run `brew trust jd4rider/tap` first. That single command installs:

- `/Applications/Nova Studio.app`
- the `nova` CLI and compiler from the tap

Verify both:

```sh
nova --version
open -a "Nova Studio"
```

If you only want the language:

```sh
brew install jd4rider/tap/nova
nova --version
```

If you already have the tap:

```sh
brew install nova
brew install --cask nova-studio
```

The Homebrew CLI archive is currently macOS arm64. This cask install was verified end to end on macOS arm64: it installed `nova 0.1.0`, and the app launched from `/Applications/Nova Studio.app`. Nova requires `clang` to link native programs; install the Xcode Command Line Tools if needed:

```sh
xcode-select --install
```

## macOS DMG

**[Download Nova Studio 0.1.0 for macOS arm64](https://github.com/jd4rider/nova/releases/download/v0.1.0/Nova-Studio-0.1.0-arm64.dmg)** (115 MB)

1. Open the DMG.
2. Drag **Nova Studio** into **Applications**.
3. Launch it. The app is unsigned, so macOS may require right-click → **Open** the first time, or:

```sh
xattr -dr com.apple.quarantine "/Applications/Nova Studio.app"
```

The DMG installs Studio only. Install the language with Homebrew or from source:

```sh
brew install jd4rider/tap/nova
```

## Windows EXE

**[Download Nova Studio 0.1.0 installer for Windows x64](https://github.com/jd4rider/nova/releases/download/v0.1.0/Nova-Studio-0.1.0-x64-setup.exe)** (94 MB)

Run the NSIS installer and choose an install directory. It creates Start Menu and desktop shortcuts. The installer is unsigned, so Windows SmartScreen may show **More info → Run anyway**.

Windows Studio is built without the optional native `node-pty` module. The terminal therefore runs in pipe mode, and FreeBuff’s TUI cannot run. The language/CLI is **not bundled** in the Windows installer yet; build it from source below.

## Install from source

Requirements:

- Rust 1.96+ and Cargo
- Clang/Apple Clang on `PATH`
- Node 20+ only for Studio

```sh
git clone https://github.com/jd4rider/nova.git
cd nova
cargo build --release -p nova-cli
export PATH="$PWD/target/release:$PATH"
nova --version
nova doctor
```

Compile a project:

```sh
nova new hello
cd hello
nova run
nova test
```

Build Studio from source:

```sh
cd ../editor
npm ci --no-audit --no-fund
npm run typecheck
npm run build
NOVA_BIN="$PWD/../target/release/nova" npm run dev
```

Build installers locally:

```sh
# macOS arm64 DMG + ZIP
CSC_IDENTITY_AUTO_DISCOVERY=false npm run dist:mac

# Windows x64 NSIS EXE (cross-builds on macOS; no native PTY module)
npm run dist:win
```

Artifacts appear under `editor/release/`. The manual packaging workflow (`.github/workflows/release.yml`) builds the same set and uploads it as a workflow artifact; publishing a GitHub release remains an explicit release-owner action so tags and Homebrew checksums stay in sync.

## Verify a download

Every release includes `SHA256SUMS`.

```sh
shasum -a 256 -c SHA256SUMS
# Windows PowerShell:
Get-FileHash .\Nova-Studio-0.1.0-x64-setup.exe -Algorithm SHA256
```

Current preview hashes:

```text
6c0deeabb5b719b2ca51dfad46508959a3b585ce75b50f47729f30fb9b4cb589  Nova-Studio-0.1.0-arm64.dmg
72db666b7db6d41ea1ffae6e6cd91e27d658bfbf4e4d97c3896a76fa01dec0b4  Nova-Studio-0.1.0-x64-setup.exe
03ad1c66644b90837ce5b44fb9fbb375ae7164d4be28851d041bdea0f367c067  nova-0.1.0-aarch64-apple-darwin.tar.gz
```

The Windows executable and macOS app are **not Authenticode/Developer ID signed**. Do not treat checksums as a substitute for code signing.

## Uninstall

```sh
# Homebrew
brew uninstall --cask nova-studio
brew uninstall nova

# Direct DMG
rm -rf "/Applications/Nova Studio.app"
```

Zap Homebrew-managed preferences with `brew uninstall --zap --cask nova-studio`.

## Read next

- [Getting started](https://jd4rider.github.io/nova/getting-started/installation/)
- [Nova Studio guide](https://jd4rider.github.io/nova/studio/overview/)
- [Status and limitations](https://jd4rider.github.io/nova/status/)
