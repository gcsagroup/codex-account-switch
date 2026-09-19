#!/usr/bin/env bash
# 只使用 CI 显式配置的证书；不导出开发机私钥。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/dist/Codex Account Switch.app"
mkdir -p "$ROOT/dist/ci"
SIGNING=unsigned
if [[ -n "${MACOS_SIGNING_P12_B64:-}" ]]; then
  : "${MACOS_SIGNING_PASSWORD:?缺少证书密码}"
  : "${MACOS_SIGNING_IDENTITY:?缺少签名身份}"
  [[ "$MACOS_SIGNING_IDENTITY" == *'Global Cybersecurity Alliance Limited'* ]] || { echo '签名身份必须属于公司' >&2; exit 1; }
  SIGN_DIR="$(mktemp -d "${RUNNER_TEMP:?}/codex-sign.XXXXXX")"
  KEYCHAIN="$SIGN_DIR/signing.keychain-db"
  KEY_PASSWORD="$(openssl rand -hex 32)"
  trap 'security delete-keychain "$KEYCHAIN" 2>/dev/null || true; rm -f "$SIGN_DIR/certificate.p12"; rmdir "$SIGN_DIR" 2>/dev/null || true' EXIT
  printf '%s' "$MACOS_SIGNING_P12_B64" | base64 --decode > "$SIGN_DIR/certificate.p12"
  security create-keychain -p "$KEY_PASSWORD" "$KEYCHAIN"
  security set-keychain-settings -lut 21600 "$KEYCHAIN"
  security unlock-keychain -p "$KEY_PASSWORD" "$KEYCHAIN"
  security import "$SIGN_DIR/certificate.p12" -P "$MACOS_SIGNING_PASSWORD" -A -t cert -f pkcs12 -k "$KEYCHAIN" >/dev/null
  security set-key-partition-list -S apple-tool:,apple: -k "$KEY_PASSWORD" "$KEYCHAIN" >/dev/null
  security list-keychains -d user -s "$KEYCHAIN"
  codesign --force --options runtime --timestamp --keychain "$KEYCHAIN" --sign "$MACOS_SIGNING_IDENTITY" "$APP/Contents/MacOS/codex-account-switch-bin"
  codesign --force --options runtime --timestamp --keychain "$KEYCHAIN" --sign "$MACOS_SIGNING_IDENTITY" "$APP"
  codesign --verify --deep --strict "$APP"
  SIGNATURE="$(codesign -dv --verbose=2 "$APP" 2>&1)"
  [[ "$SIGNATURE" == *'Global Cybersecurity Alliance Limited'* && "$SIGNATURE" == *'TeamIdentifier=R3JK7R29AC'* ]]
  SIGNING=company-signed
elif [[ "${MACOS_ALLOW_UNSIGNED:-false}" != true ]]; then
  printf '%s\n' 'macOS 构建已通过，自动下载包等待公司签名配置。' > "$ROOT/dist/ci/macos-signing-pending.txt"
  exit 0
else
  # Apple 芯片需要临时签名；这不提供公司发布者身份。
  codesign --force --sign - "$APP/Contents/MacOS/codex-account-switch-bin"
  codesign --force --sign - "$APP"
  codesign --verify --deep --strict "$APP"
fi
[[ "$(lipo -archs "$APP/Contents/MacOS/codex-account-switch-bin")" == arm64 ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$APP/Contents/Info.plist")" == "$CI_BUILD_NUMBER" ]]
ZIP="$ROOT/dist/ci/Codex-Account-Switch-${RELEASE_TAG:?}-macos-arm64.zip"
ditto -c -k --keepParent "$APP" "$ZIP"
python scripts/ci-release.py macos-manifest --package "$ZIP" --signing "$SIGNING"
