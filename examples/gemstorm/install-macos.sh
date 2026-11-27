#!/bin/sh
# Build Gemstorm and install it as a native macOS app bundle, so it opens from
# Finder or the Dock instead of a terminal.
#
# Usage:
#   sh examples/gemstorm/install-macos.sh
#   NOVA_GEMSTORM_INSTALL_DIR="$HOME/Applications" sh examples/gemstorm/install-macos.sh
#
# The icon comes from the game itself: `NOVA_GEMSTORM_ICON` draws the same gem
# cluster the menu uses, so the bundle needs no web or image tool. The bundle is
# ad-hoc signed for local use, not Developer ID signed or notarized. Re-run
# after changes to update the installed app.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
APP_NAME="Gemstorm"
APP="$HERE/dist/$APP_NAME.app"
VERSION=$(tr -d '\n' < "$HERE/VERSION")
if [ -n "${NOVA_BIN:-}" ]; then
    NOVA="$NOVA_BIN"
elif [ -x "$ROOT/examples/nova-tool/target/nova/nova-tool" ]; then
    NOVA="$ROOT/examples/nova-tool/target/nova/nova-tool"
else
    NOVA="$ROOT/bootstrap/nova"
fi

[ -x "$NOVA" ] || {
    echo "install: missing compiler: $NOVA" >&2
    echo "install: build it first: (cd examples/nova-tool && ../../bootstrap/nova build)" >&2
    exit 1
}

echo "--- build ---"
(cd "$HERE" && "$NOVA" build)

BIN="$HERE/target/nova/gemstorm"
[ -x "$BIN" ] || { echo "install: build did not produce $BIN" >&2; exit 1; }

echo "--- bundle ---"
rm -rf "$APP" "$HERE/dist/icon.iconset" "$HERE/dist/icon.png"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/gemstorm"
chmod +x "$APP/Contents/MacOS/gemstorm"

# Icon mode opens a 512-point window that renders at 1024 px on Retina, then
# saves the framebuffer; sips and iconutil turn that into the .icns macOS asks
# for.
mkdir -p "$HERE/dist"
if NOVA_GEMSTORM_ICON="$HERE/dist/icon.png" "$BIN" >/dev/null 2>&1; then
    :
else
    echo "install: icon render failed; the app will use the default icon" >&2
fi
if [ -s "$HERE/dist/icon.png" ]; then
    ICONSET="$HERE/dist/icon.iconset"
    mkdir -p "$ICONSET"
    for size in 16 32 64 128 256 512 1024; do
        sips -z "$size" "$size" "$HERE/dist/icon.png" --out "$ICONSET/icon_${size}.png" >/dev/null
    done
    cp "$ICONSET/icon_32.png" "$ICONSET/icon_16x16@2x.png"
    cp "$ICONSET/icon_64.png" "$ICONSET/icon_32x32@2x.png"
    cp "$ICONSET/icon_256.png" "$ICONSET/icon_128x128@2x.png"
    cp "$ICONSET/icon_512.png" "$ICONSET/icon_256x256@2x.png"
    cp "$ICONSET/icon_1024.png" "$ICONSET/icon_512x512@2x.png"
    mv "$ICONSET/icon_16.png" "$ICONSET/icon_16x16.png"
    mv "$ICONSET/icon_32.png" "$ICONSET/icon_32x32.png"
    mv "$ICONSET/icon_128.png" "$ICONSET/icon_128x128.png"
    mv "$ICONSET/icon_256.png" "$ICONSET/icon_256x256.png"
    mv "$ICONSET/icon_512.png" "$ICONSET/icon_512x512.png"
    rm -f "$ICONSET/icon_64.png" "$ICONSET/icon_1024.png"
    iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
fi

echo "--- metadata ---"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Gemstorm</string>
  <key>CFBundleDisplayName</key><string>Gemstorm</string>
  <key>CFBundleIdentifier</key><string>dev.nova.gemstorm</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleExecutable</key><string>gemstorm</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.puzzle-games</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSPrincipalClass</key><string>NSApplication</string>
  <key>NSHumanReadableCopyright</key><string>Gemstorm — built with the self-hosted Nova compiler</string>
</dict>
</plist>
PLIST
plutil -lint "$APP/Contents/Info.plist" >/dev/null
printf 'APPL????' > "$APP/Contents/PkgInfo"

echo "--- sign ---"
codesign --force --sign - "$APP" >/dev/null 2>&1
codesign --verify --strict "$APP"

TARGET=${NOVA_GEMSTORM_INSTALL_DIR:-/Applications}
if [ ! -w "$TARGET" ]; then
    TARGET="$HOME/Applications"
    mkdir -p "$TARGET"
fi
rm -rf "$TARGET/$APP_NAME.app"
cp -R "$APP" "$TARGET/$APP_NAME.app"

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
cat > "$BIN_DIR/nova-gemstorm" <<CLI
#!/bin/sh
exec "$TARGET/$APP_NAME.app/Contents/MacOS/gemstorm" "\$@"
CLI
chmod +x "$BIN_DIR/nova-gemstorm"

echo "Installed $TARGET/$APP_NAME.app"
echo "Version $VERSION · dev.nova.gemstorm · ad-hoc signed"
echo "Command line: $BIN_DIR/nova-gemstorm"
