#!/bin/sh
# Build Nova Breakout and install it as a macOS app bundle (a real window, not
# Terminal). Set APP_NAME to install over an older bundle.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
NOVA="$ROOT/target/debug/nova"
DIST="$HERE/dist"
APP_NAME="${APP_NAME:-Nova Breakout}"
APP="$DIST/$APP_NAME.app"

[ -x "$NOVA" ] || { echo "install: build the compiler first (cargo build -p nova-cli)" >&2; exit 1; }
(cd "$HERE" && "$NOVA" build)

BIN="$HERE/target/nova/breakout-gui"
[ -x "$BIN" ] || { echo "install: build did not produce $BIN" >&2; exit 1; }

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/breakout-gui"
chmod +x "$APP/Contents/MacOS/breakout-gui"

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
