#!/bin/sh
# Build Nova Studio (the Nova rewrite) and install it as a macOS app bundle.
#
# The workbench owns its own window, so the bundle runs the compiler output
# directly — no Terminal wrapper. The icon is rendered from icon.html with
# headless Chrome, and a `nova-studio` launcher is written to ~/.local/bin so
# the workbench can be started from any project folder.
#
#   APP_NAME="Nova Studio" ./install-macos.sh   # install over the Electron app
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
NOVA="$ROOT/target/debug/nova"
DIST="$HERE/dist"
APP_NAME="${APP_NAME:-Nova Studio (Nova)}"
APP="$DIST/$APP_NAME.app"
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

[ -x "$NOVA" ] || { echo "install: build the compiler first (cargo build -p nova-cli)" >&2; exit 1; }
(cd "$HERE" && "$NOVA" build)

GAME="$HERE/target/nova/nova-studio"
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
  iconutil -c icns "$DIST/icon.iconset" -o "$APP/Contents/Resources/AppIcon.icns"
fi

# The GUI binary is the bundle's main executable. macOS only treats a process
# as an app — menu bar, click-to-activate, draggable window — when the binary
# LaunchServices launched is CFBundleExecutable inside Contents/MacOS. A shell
# script there execs a child that macOS considers an outsider.
cp "$GAME" "$APP/Contents/MacOS/nova-studio"
chmod +x "$APP/Contents/MacOS/nova-studio"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>dev.nova.studio.native</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleExecutable</key><string>nova-studio</string>
  <key>LSEnvironment</key>
  <dict>
    <key>NOVA_STUDIO_ROOT</key><string>$ROOT</string>
    <key>NOVA_BIN</key><string>$NOVA</string>
  </dict>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

plutil -lint "$APP/Contents/Info.plist" >/dev/null
printf 'APPL????' > "$APP/Contents/PkgInfo"

# An ad-hoc signature gives the bundle a stable identity, which keeps
# LaunchServices from treating each rebuilt copy as a different app.
codesign --force --sign - "$APP" >/dev/null 2>&1 || true

TARGET="/Applications"
if [ ! -w "$TARGET" ]; then
  TARGET="$HOME/Applications"
  mkdir -p "$TARGET"
fi
rm -rf "$TARGET/$APP_NAME.app"
cp -R "$APP" "$TARGET/$APP_NAME.app"

# A command-line entry point: opens the folder you run it from.
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
cat > "$BIN_DIR/nova-studio" <<CLI
#!/bin/sh
export NOVA_BIN="\${NOVA_BIN:-$NOVA}"
exec "$TARGET/$APP_NAME.app/Contents/MacOS/nova-studio" "\$@"
CLI
chmod +x "$BIN_DIR/nova-studio"

echo "Installed $TARGET/$APP_NAME.app"
echo "Command line: $BIN_DIR/nova-studio"
