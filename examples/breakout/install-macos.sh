#!/bin/sh
# Build Breakout and install it as /Applications/Breakout.app.
#
# The bundle's executable opens Terminal.app running the game, because the
# game is a terminal TUI. The icon is rendered from icon.html with Chrome.
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
NOVA="$ROOT/target/debug/nova"
DIST="$HERE/dist"
APP="$DIST/Breakout.app"
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

(cd "$HERE" && "$NOVA" build)
GAME="$HERE/target/nova/breakout"
[ -x "$GAME" ] || { echo "install: build did not produce $GAME" >&2; exit 1; }

rm -rf "$APP" "$DIST/icon.iconset"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$DIST/icon.iconset"

if [ -x "$CHROME" ]; then
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars \
    --screenshot="$DIST/icon.png" --window-size=1024,1024 \
    "file://$HERE/icon.html" >/dev/null 2>&1
else
  echo "install: Chrome not found; the app will use the default icon" >&2
fi

if [ -f "$DIST/icon.png" ]; then
  for size in 16 32 64 128 256 512 1024; do
    sips -z "$size" "$size" "$DIST/icon.png" --out "$DIST/icon.iconset/icon_${size}.png" >/dev/null
  done
  cp "$DIST/icon.iconset/icon_32.png" "$DIST/icon.iconset/icon_16x16@2x.png"
  cp "$DIST/icon.iconset/icon_64.png" "$DIST/icon.iconset/icon_32x32@2x.png"
  cp "$DIST/icon.iconset/icon_256.png" "$DIST/icon.iconset/icon_128x128@2x.png"
  cp "$DIST/icon.iconset/icon_512.png" "$DIST/icon.iconset/icon_256x256@2x.png"
  cp "$DIST/icon.iconset/icon_1024.png" "$DIST/icon.iconset/icon_512x512@2x.png"
  mv "$DIST/icon.iconset/icon_16.png" "$DIST/icon.iconset/icon_16x16.png"
  mv "$DIST/icon.iconset/icon_32.png" "$DIST/icon.iconset/icon_32x32.png"
  mv "$DIST/icon.iconset/icon_128.png" "$DIST/icon.iconset/icon_128x128.png"
  mv "$DIST/icon.iconset/icon_256.png" "$DIST/icon.iconset/icon_256x256.png"
  mv "$DIST/icon.iconset/icon_512.png" "$DIST/icon.iconset/icon_512x512.png"
  rm -f "$DIST/icon.iconset/icon_64.png" "$DIST/icon.iconset/icon_1024.png"
  iconutil -c icns "$DIST/icon.iconset" -o "$APP/Contents/Resources/breakout.icns"
fi

cp "$GAME" "$APP/Contents/Resources/breakout"
chmod +x "$APP/Contents/Resources/breakout"

cat > "$APP/Contents/MacOS/Breakout" <<'LAUNCH'
#!/bin/sh
HERE="$(cd "$(dirname "$0")/../Resources" && pwd)"
exec open -a Terminal "$HERE/breakout"
LAUNCH
chmod +x "$APP/Contents/MacOS/Breakout"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Breakout</string>
  <key>CFBundleDisplayName</key><string>Nova Breakout</string>
  <key>CFBundleIdentifier</key><string>io.nova.breakout</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleExecutable</key><string>Breakout</string>
  <key>CFBundleIconFile</key><string>breakout</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

plutil -lint "$APP/Contents/Info.plist" >/dev/null
TARGET="/Applications"
if [ ! -w "$TARGET" ]; then
  TARGET="$HOME/Applications"
  mkdir -p "$TARGET"
fi
rm -rf "$TARGET/Breakout.app"
cp -R "$APP" "$TARGET/Breakout.app"
echo "Installed $TARGET/Breakout.app"
