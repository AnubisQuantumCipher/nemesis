#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="NEMESIS Desktop"
PROCESS_NAME="nemesis-desktop"
BUNDLE_ID="dev.nemesis.desktop.local"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP_DIR="$ROOT_DIR/desktop"
DIST_DIR="$ROOT_DIR/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_RESOURCES="$APP_CONTENTS/Resources"
APP_BINARY="$APP_MACOS/$PROCESS_NAME"
INFO_PLIST="$APP_CONTENTS/Info.plist"

pkill -x "$PROCESS_NAME" >/dev/null 2>&1 || true

npm --prefix "$DESKTOP_DIR" test
npm --prefix "$DESKTOP_DIR" run build
npm --prefix "$DESKTOP_DIR" run tauri -- build --debug --no-bundle

rm -rf "$APP_BUNDLE"
mkdir -p "$APP_MACOS" "$APP_RESOURCES"
cp "$DESKTOP_DIR/src-tauri/target/debug/$PROCESS_NAME" "$APP_BINARY"
cp "$DESKTOP_DIR/src-tauri/icons/icon.png" "$APP_RESOURCES/icon.png"
chmod +x "$APP_BINARY"

cat >"$INFO_PLIST" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>$PROCESS_NAME</string>
  <key>CFBundleIdentifier</key>
  <string>$BUNDLE_ID</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleIconFile</key>
  <string>icon.png</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
PLIST

open_app() {
  /usr/bin/open -n "$APP_BUNDLE"
}

case "$MODE" in
  run)
    open_app
    ;;
  --debug|debug)
    lldb -- "$APP_BINARY"
    ;;
  --logs|logs)
    open_app
    /usr/bin/log stream --info --style compact --predicate "process == \"$PROCESS_NAME\""
    ;;
  --telemetry|telemetry)
    open_app
    /usr/bin/log stream --info --style compact --predicate "subsystem == \"$BUNDLE_ID\""
    ;;
  --verify|verify)
    open_app
    for _ in $(seq 1 50); do
      if pgrep -x "$PROCESS_NAME" >/dev/null; then
        printf '%s\n' 'PASS_NEMESIS_DESKTOP_PROCESS'
        exit 0
      fi
      sleep 0.1
    done
    printf '%s\n' 'FAIL_NEMESIS_DESKTOP_PROCESS' >&2
    exit 1
    ;;
  *)
    echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
