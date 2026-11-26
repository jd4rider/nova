#!/bin/sh
# Build Nova Breakout and install it as a macOS app bundle (a real window, not
# Terminal). Set APP_NAME to install over an older bundle.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
NOVA="${NOVA_BIN:-$ROOT/bootstrap/nova}"
DIST="$HERE/dist"
APP_NAME="${APP_NAME:-Nova Breakout}"
APP="$DIST/$APP_NAME.app"
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

[ -x "$NOVA" ] || { echo "install: missing $NOVA (build the Nova compiler first)" >&2; exit 1; }
(cd "$HERE" && "$NOVA" build)

BIN="$HERE/target/nova/breakout-gui"
[ -x "$BIN" ] || { echo "install: build did not produce $BIN" >&2; exit 1; }

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/breakout-gui"
chmod +x "$APP/Contents/MacOS/breakout-gui"

# The icon is a web page: Chrome renders it once at 1024px, then iconutil packs
# the sizes macOS asks for.
ICONSET="$DIST/icon.iconset"
rm -rf "$ICONSET" "$DIST/icon.png"
mkdir -p "$ICONSET"
if [ -x "$CHROME" ]; then
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars \
    --screenshot="$DIST/icon.png" --window-size=1024,1024 \
    "file://$HERE/icon.html" >/dev/null 2>&1 || true
else
  echo "install: Chrome not found; the app will use the default icon" >&2
fi
if [ -f "$DIST/icon.png" ]; then
  for size in 16 32 64 128 256 512 1024; do
    sips -z "$size" "$size" "$DIST/icon.png" --out "$ICONSET/icon_${size}.png" >/dev/null
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

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>dev.nova.breakout</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleExecutable</key><string>breakout-gui</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
plutil -lint "$APP/Contents/Info.plist" >/dev/null
printf 'APPL????' > "$APP/Contents/PkgInfo"
codesign --force --sign - "$APP" >/dev/null 2>&1 || true

TARGET="/Applications"
if [ ! -w "$TARGET" ]; then
  TARGET="$HOME/Applications"
  mkdir -p "$TARGET"
fi
rm -rf "$TARGET/$APP_NAME.app"
cp -R "$APP" "$TARGET/$APP_NAME.app"

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
cat > "$BIN_DIR/nova-breakout" <<CLI
#!/bin/sh
exec "$TARGET/$APP_NAME.app/Contents/MacOS/breakout-gui" "\$@"
CLI
chmod +x "$BIN_DIR/nova-breakout"

echo "Installed $TARGET/$APP_NAME.app"
echo "Command line: $BIN_DIR/nova-breakout"
