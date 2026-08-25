#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
ARCH="${1:-all}"
DIST="$ROOT/dist/windows"

case "$ARCH" in
  all) TARGETS=(x86_64-pc-windows-gnu i686-pc-windows-msvc) ;;
  x64|x86_64) TARGETS=(x86_64-pc-windows-gnu) ;;
  x86|i686) TARGETS=(i686-pc-windows-msvc) ;;
  *)
    echo "用法: $0 [all|x64|x86]" >&2
    exit 2
    ;;
esac

cd "$ROOT"
mkdir -p "$DIST"
cp "$ROOT/LICENSE" "$DIST/LICENSE"
cp "$ROOT/THIRD_PARTY_NOTICES.md" "$DIST/THIRD_PARTY_NOTICES.md"

for target in "${TARGETS[@]}"; do
  rustup target add --toolchain 1.97.1 "$target" >/dev/null
  if [[ "$target" == "i686-pc-windows-msvc" ]]; then
    if [[ -n "${CARGO_HOME:-}" ]]; then
      cargo_xwin="$CARGO_HOME/bin/cargo-xwin"
    else
      cargo_xwin="$(dirname "$(rustup show home)")/.cargo/bin/cargo-xwin"
    fi
    if [[ ! -x "$cargo_xwin" ]]; then
      echo "缺少 cargo-xwin，请先执行: cargo install cargo-xwin --locked" >&2
      exit 1
    fi
    lld_bin="$(brew --prefix lld)/bin"
    XWIN_ARCH=x86 \
      PATH="$lld_bin:$PATH" \
      CARGO_TARGET_DIR="$TARGET_DIR" \
      RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C target-feature=+crt-static" \
      RUSTC="$(rustup which --toolchain 1.97.1 rustc)" \
      "$cargo_xwin" xwin build --locked --release --target "$target"
  else
    RUSTC="$(rustup which --toolchain 1.97.1 rustc)" \
      CARGO_TARGET_DIR="$TARGET_DIR" rustup run 1.97.1 cargo build --locked --release --target "$target"
  fi

  if [[ "$target" == "i686-pc-windows-msvc" ]]; then
    output="$DIST/codex-account-switch-windows-x86.exe"
  else
    output="$DIST/codex-account-switch-windows-x86_64.exe"
  fi
  cp "$TARGET_DIR/$target/release/codex-account-switch.exe" "$output"
  (
    cd "$DIST"
    shasum -a 256 "$(basename "$output")" > "$(basename "$output").sha256"
  )
  ls -lh "$output" "$output.sha256"
done

# Preserve the original output path as the 64-bit compatibility alias.
if [[ -f "$DIST/codex-account-switch-windows-x86_64.exe" ]]; then
  cp "$DIST/codex-account-switch-windows-x86_64.exe" "$DIST/codex-account-switch.exe"
fi

echo "Windows 可执行文件已生成在: $DIST"
echo "许可证文件已生成在: $DIST/LICENSE 和 $DIST/THIRD_PARTY_NOTICES.md"
