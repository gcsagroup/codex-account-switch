#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/dist/Codex Account Switch.app"
LOG="${CODEX_SWITCH_HOME:-$HOME/.codex-switch}/app.log"

if [[ ! -x "$APP/Contents/MacOS/codex-account-switch-bin" ]]; then
  echo "缺少 app 包，请先: cargo build --release && ./scripts/package-macos.sh"
  exit 1
fi

# Prefer LaunchServices so process is NOT tied to this shell/Cursor
open "$APP"
sleep 1
if pgrep -f 'Codex Account Switch.app/Contents/MacOS/codex-account-switch-bin' >/dev/null; then
  echo "Codex Account Switch 已通过 LaunchServices 启动"
  echo "日志: $LOG"
else
  echo "启动失败，日志："
  tail -n 40 "$LOG" || true
  exit 1
fi
