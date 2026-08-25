#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
VERSION="$(awk -F ' *= *' '/^version = / { gsub(/\"/, "", $2); print $2; exit }' "$ROOT/Cargo.toml")"
RUSTC_BIN="$(rustup which --toolchain 1.97.1 rustc)"
(cd "$ROOT" && CARGO_TARGET_DIR="$TARGET_DIR" RUSTC="$RUSTC_BIN" rustup run 1.97.1 cargo build --locked --release)

APP="$ROOT/dist/Codex Account Switch.app"
if [[ -e "$APP" ]]; then
  find "$APP" -depth -delete
fi
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# Info.plist
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>zh_CN</string>
  <key>CFBundleExecutable</key><string>codex-account-switch</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundleIdentifier</key><string>local.gcsa.codex-account-switch</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>Codex Account Switch</string>
  <key>CFBundleDisplayName</key><string>Codex Account Switch</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSAppleEventsUsageDescription</key><string>Codex Account Switch uses Apple Events only when you request an app restart after switching accounts.</string>
  <key>NSAppDataUsageDescription</key><string>Codex Account Switch reads local account session files selected by you and stores local profiles.</string>
  <key>LSUIElement</key><false/>
</dict>
</plist>
PLIST

# Launcher wrapper
cat > "$APP/Contents/MacOS/codex-account-switch" <<'WRAP'
#!/bin/bash
DIR="$(cd "$(dirname "$0")" && pwd)"
REAL="$DIR/codex-account-switch-bin"
LOG_DIR="${CODEX_SWITCH_HOME:-$HOME/.codex-switch}"
umask 077
mkdir -p "$LOG_DIR"
chmod 700 "$LOG_DIR"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] launchd/open wrapper start" >>"$LOG_DIR/app.log"
exec "$REAL" >>"$LOG_DIR/app.log" 2>&1
WRAP
chmod +x "$APP/Contents/MacOS/codex-account-switch"

cp "$TARGET_DIR/release/codex-account-switch" "$APP/Contents/MacOS/codex-account-switch-bin"
chmod +x "$APP/Contents/MacOS/codex-account-switch-bin"

if [[ -f "$ROOT/assets/AppIcon.icns" ]]; then
  cp "$ROOT/assets/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
fi

cp "$ROOT/LICENSE" "$APP/Contents/Resources/LICENSE"
cp "$ROOT/THIRD_PARTY_NOTICES.md" "$APP/Contents/Resources/THIRD_PARTY_NOTICES.md"

# Refresh LaunchServices icon cache for this app
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$APP" 2>/dev/null || true

echo "Packaged: $APP"
echo "Unsigned package. Run scripts/sign-notarize-macos.sh before external distribution."
