#!/usr/bin/env bash
# 核对已签名原包与当前源码；不重新编译、不改动正在使用的 App。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PAYLOAD="$(cd "${1:?请提供安装包与校验文件目录}" && pwd)"
VERSION="$(awk -F ' *= *' '/^version = / { gsub(/\"/, "", $2); print $2; exit }' "$ROOT/Cargo.toml")"
BUILD="$(tr -d '\n' < "$ROOT/BUILD_NUMBER")"
ZIP="Codex-Account-Switch-v${VERSION}-macos-arm64.zip"
(cd "$ROOT" && shasum -a 256 -c "$PAYLOAD/SOURCE_SHA256SUMS.txt")
(cd "$PAYLOAD" && shasum -a 256 -c SHA256SUMS.txt)
CHECK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-release-check.XXXXXX")"
trap 'rm -rf "$CHECK_DIR"' EXIT
ditto -x -k "$PAYLOAD/$ZIP" "$CHECK_DIR"
APP="$CHECK_DIR/Codex Account Switch.app"
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")" == "$VERSION" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$APP/Contents/Info.plist")" == "$BUILD" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP/Contents/Info.plist")" == 'local.gcsa.codex-account-switch' ]]
codesign --verify --deep --strict --verbose=2 "$APP"
for item in "$APP" "$APP/Contents/MacOS/codex-account-switch-bin"; do
  SIGNATURE="$(codesign -d --verbose=2 "$item" 2>&1)"
  [[ "$SIGNATURE" == *'Authority=Apple Distribution: Global Cybersecurity Alliance Limited (R3JK7R29AC)'* ]]
  [[ "$SIGNATURE" == *'TeamIdentifier=R3JK7R29AC'* ]]
done
[[ "$(lipo -archs "$APP/Contents/MacOS/codex-account-switch-bin")" == 'arm64' ]]
echo "发布包核验通过：v${VERSION}，构建 ${BUILD}，arm64，公司签名。"
