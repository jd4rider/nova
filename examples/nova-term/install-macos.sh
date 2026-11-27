#!/bin/sh
# Build Nova Terminal and install it as a macOS app bundle.
#
# Usage:
#   sh examples/nova-term/install-macos.sh
#   NOVA_TERM_INSTALL_DIR="$HOME/Applications" sh examples/nova-term/install-macos.sh
#
# The bundle is ad-hoc signed for local testing, not Developer ID signed or
# notarized. Re-run this after each terminal slice to update the installed app.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
APP_NAME="Nova Terminal"
APP="$HERE/dist/$APP_NAME.app"
VERSION=$(tr -d '\n' < "$HERE/VERSION")
NOVA=${NOVA_BIN:-"$ROOT/examples/nova-tool/target/nova/nova-tool"}
CHROME=${CHROME_BIN:-"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"}

[ -x "$NOVA" ] || {
    echo "install: missing compiler: $NOVA" >&2
    echo "install: build it first: (cd examples/nova-tool && ../../bootstrap/nova build)" >&2
    exit 1
}

echo "--- build ---"
(cd "$HERE" && "$NOVA" build)

BIN="$HERE/target/nova/nova-term"
[ -x "$BIN" ] || { echo "install: build did not produce $BIN" >&2; exit 1; }

echo "--- bundle ---"
rm -rf "$APP" "$HERE/dist/icon.iconset" "$HERE/dist/icon.png"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$HERE/dist/icon.iconset"
cp "$BIN" "$APP/Contents/MacOS/nova-term"
chmod +x "$APP/Contents/MacOS/nova-term"

echo "--- icon ---"
[ -x "$CHROME" ] || {
    echo "install: Chrome is required to render packaging/icon.html" >&2
    echo "install: set CHROME_BIN to a Chromium-compatible browser" >&2
    exit 1
}
# A private profile keeps this render independent of any other headless Chrome
# (another app installer, a preview server) that is already running.
PROFILE=$(mktemp -d "${TMPDIR:-/tmp}/nova-term-icon.XXXXXX")
trap 'rm -rf "$PROFILE"' EXIT
"$CHROME" --headless=new --disable-gpu --hide-scrollbars \
    --default-background-color=00000000 --force-device-scale-factor=1 \
    --virtual-time-budget=1000 --user-data-dir="$PROFILE" \
    --screenshot="$HERE/dist/icon.png" --window-size=1024,1024 \
    "file://$HERE/packaging/icon.html" >/dev/null 2>&1
[ -s "$HERE/dist/icon.png" ] || { echo "install: icon render failed" >&2; exit 1; }

for size in 16 32 64 128 256 512 1024; do
    sips -z "$size" "$size" "$HERE/dist/icon.png" \
        --out "$HERE/dist/icon.iconset/icon_${size}.png" >/dev/null
done
cp "$HERE/dist/icon.iconset/icon_32.png" "$HERE/dist/icon.iconset/icon_16x16@2x.png"
cp "$HERE/dist/icon.iconset/icon_64.png" "$HERE/dist/icon.iconset/icon_32x32@2x.png"
cp "$HERE/dist/icon.iconset/icon_256.png" "$HERE/dist/icon.iconset/icon_128x128@2x.png"
cp "$HERE/dist/icon.iconset/icon_512.png" "$HERE/dist/icon.iconset/icon_256x256@2x.png"
cp "$HERE/dist/icon.iconset/icon_1024.png" "$HERE/dist/icon.iconset/icon_512x512@2x.png"
mv "$HERE/dist/icon.iconset/icon_16.png" "$HERE/dist/icon.iconset/icon_16x16.png"
mv "$HERE/dist/icon.iconset/icon_32.png" "$HERE/dist/icon.iconset/icon_32x32.png"
mv "$HERE/dist/icon.iconset/icon_128.png" "$HERE/dist/icon.iconset/icon_128x128.png"
mv "$HERE/dist/icon.iconset/icon_256.png" "$HERE/dist/icon.iconset/icon_256x256.png"
mv "$HERE/dist/icon.iconset/icon_512.png" "$HERE/dist/icon.iconset/icon_512x512.png"
rm -f "$HERE/dist/icon.iconset/icon_64.png" "$HERE/dist/icon.iconset/icon_1024.png"
iconutil -c icns "$HERE/dist/icon.iconset" \
    -o "$APP/Contents/Resources/NovaTerminal.icns"

echo "--- metadata ---"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>dev.nova.terminal</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleExecutable</key><string>nova-term</string>
  <key>CFBundleIconFile</key><string>NovaTerminal</string>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSPrincipalClass</key><string>NSApplication</string>
  <key>NSHumanReadableCopyright</key><string>Nova Terminal — built with the self-hosted Nova compiler</string>
</dict>
</plist>
PLIST
plutil -lint "$APP/Contents/Info.plist" >/dev/null
printf 'APPL????' > "$APP/Contents/PkgInfo"

echo "--- sign ---"
codesign --force --sign - "$APP"
codesign --verify --strict "$APP"

TARGET=${NOVA_TERM_INSTALL_DIR:-/Applications}
if [ ! -w "$TARGET" ]; then
    TARGET="$HOME/Applications"
    mkdir -p "$TARGET"
fi
rm -rf "$TARGET/$APP_NAME.app"
cp -R "$APP" "$TARGET/$APP_NAME.app"

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
cat > "$BIN_DIR/nova-term" <<CLI
#!/bin/sh
export NOVA_TERM_ROOT="\${NOVA_TERM_ROOT:-\$PWD}"
exec "$TARGET/$APP_NAME.app/Contents/MacOS/nova-term" "\$@"
CLI
chmod +x "$BIN_DIR/nova-term"

echo "Installed $TARGET/$APP_NAME.app"
echo "Version $VERSION · dev.nova.terminal · ad-hoc signed"
echo "Command line: $BIN_DIR/nova-term"
