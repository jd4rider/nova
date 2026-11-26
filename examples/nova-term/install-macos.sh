#!/bin/sh
# Build Nova Terminal and install it as a macOS app bundle.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
NOVA="${NOVA_BIN:-$ROOT/bootstrap/nova}"
DIST="$HERE/dist"
APP_NAME="Nova Terminal"
APP="$DIST/$APP_NAME.app"

[ -x "$NOVA" ] || { echo "install: missing $NOVA (build the Nova compiler first)" >&2; exit 1; }
(cd "$HERE" && "$NOVA" build)

BIN="$HERE/target/nova/nova-term"
[ -x "$BIN" ] || { echo "install: build did not produce $BIN" >&2; exit 1; }

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/nova-term"
chmod +x "$APP/Contents/MacOS/nova-term"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>dev.nova.terminal</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleExecutable</key><string>nova-term</string>
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
cat > "$BIN_DIR/nova-term" <<CLI
#!/bin/sh
export NOVA_TERM_ROOT="\${NOVA_TERM_ROOT:-\$PWD}"
exec "$TARGET/$APP_NAME.app/Contents/MacOS/nova-term" "\$@"
CLI
chmod +x "$BIN_DIR/nova-term"

echo "Installed $TARGET/$APP_NAME.app"
echo "Command line: $BIN_DIR/nova-term"
