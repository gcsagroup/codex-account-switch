#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/dist/Codex Account Switch.app"
ZIP="$ROOT/dist/Codex-Account-Switch-notarization.zip"

: "${APPLE_SIGN_IDENTITY:?Set APPLE_SIGN_IDENTITY to a Developer ID Application identity}"
: "${APPLE_NOTARY_PROFILE:?Set APPLE_NOTARY_PROFILE to an xcrun notarytool keychain profile}"

if [[ ! -x "$APP/Contents/MacOS/codex-account-switch-bin" ]]; then
  echo "Missing package: $APP" >&2
  exit 1
fi

codesign --force --deep --options runtime --timestamp \
  --sign "$APPLE_SIGN_IDENTITY" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

ditto -c -k --keepParent "$APP" "$ZIP"
xcrun notarytool submit "$ZIP" --keychain-profile "$APPLE_NOTARY_PROFILE" --wait
xcrun stapler staple "$APP"
xcrun stapler validate "$APP"
spctl --assess --type execute --verbose=2 "$APP"

echo "Signed, notarized, and stapled: $APP"
