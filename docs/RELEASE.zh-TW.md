# 建置與發布說明

[English](RELEASE.md) | [简体中文](RELEASE.zh-CN.md) | [繁體中文](RELEASE.zh-TW.md)

本文說明目前原始碼中實際存在的建置腳本和發布門檻。本機或 CI 建置成功，不等於已經形成可正式對外發布的簽章產物。

## 工具鏈

- 固定 Rust 工具鏈：`1.97.1`（`rust-toolchain.toml`）
- 最低 Rust 版本：`1.92`（`Cargo.toml`）
- UI 框架：Slint `1.17`
- CI 執行環境：`macos-14` 和 `windows-2025`

請使用儲存庫固定的工具鏈，不要使用更舊的系統 Rust。

## 品質檢查

打包前執行：

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

目前原始碼基線包含 32 項測試。測試數量變更時，應同步更新 README 和 CHANGELOG 中的狀態說明。

## macOS

### 本機打包與執行

```bash
./scripts/package-macos.sh
./scripts/run-macos.sh
```

輸出：

```text
dist/Codex Account Switch.app
```

打包腳本會：

- 使用 Rust `1.97.1` 和 `Cargo.lock` 建置；
- 尊重外部設定的 `CARGO_TARGET_DIR`；
- 將 macOS 13 設為最低系統版本；
- 在 `Contents/Resources` 中包含 `LICENSE` 和英文主版本 `THIRD_PARTY_NOTICES.md`；
- 產生供本機驗證使用的未簽章 App。

目前腳本依主機架構建置；現有本機驗證基線為 Apple 晶片（`arm64`）。

### 簽章與公證

設定 Developer ID 身分和 `notarytool` Keychain Profile 後執行：

```bash
export APPLE_SIGN_IDENTITY='Developer ID Application: ...'
export APPLE_NOTARY_PROFILE='codex-account-switch-notary'
./scripts/sign-notarize-macos.sh
```

腳本會執行 Hardened Runtime 簽章、嚴格簽章驗證、公證、Staple 和 Gatekeeper 檢查。對外散布前每一步都必須通過。

## Windows

### 在 macOS 上交叉建置的相依工具

```bash
brew install mingw-w64 lld
cargo install cargo-xwin --locked
```

### 建置

```bash
./scripts/build-windows.sh       # x64 和 x86
./scripts/build-windows.sh x64
./scripts/build-windows.sh x86
```

腳本尊重外部設定的 `CARGO_TARGET_DIR`，並使用以下目標：

- x64：`x86_64-pc-windows-gnu`
- x86：`i686-pc-windows-msvc`，透過 `cargo-xwin` 建置並靜態連結 CRT

x86 連結時可能因微軟靜態程式庫缺少 PDB 而出現 `LNK4099` 警告。如果 Release 建置成功、PE 架構正確，且執行檔沒有動態匯入 `VCRUNTIME` 或 `MSVCP`，該警告不代表執行階段相依項目缺失。

### 輸出

- `dist/windows/codex-account-switch-windows-x86_64.exe`：x64 正式檔案
- `dist/windows/codex-account-switch-windows-x86.exe`：x86 正式檔案
- `dist/windows/codex-account-switch.exe`：x64 執行檔的相容副本
- `*.sha256`：兩個架構正式檔案的校驗值
- `LICENSE` 和 `THIRD_PARTY_NOTICES.md`：再散布時必須隨產物提供的檔案

### 靜態驗證

```bash
file dist/windows/*.exe

cd dist/windows
shasum -a 256 -c codex-account-switch-windows-x86_64.exe.sha256
shasum -a 256 -c codex-account-switch-windows-x86.exe.sha256
```

靜態驗證不能取代 Windows 實機測試。正式發布前，必須在兩個支援的架構上驗證啟動、Codex 匯入與切換、Claude Desktop 切換、重新啟動動作、系統匣行為和明確結束。

## 發布流程

1. 更新 `Cargo.toml` 版本並重新產生 `Cargo.lock`。
2. 將三種語言 CHANGELOG 中的 `Unreleased` 替換為發布日期。
3. 重新執行品質檢查和平台打包檢查。
4. 建立發布提交和標籤。
5. 從該標籤重新建置，接著完成簽章、公證（如適用）並產生校驗值。
6. 發布完全對應的二進位檔、授權條款、第三方聲明和發布說明。
7. 在 GitHub Release 中寫明平台、架構、簽章狀態和已知限制。

## 目前正式發布阻斷項目

- macOS App 尚未通過 Developer ID 簽章和 Apple 公證。
- Windows 執行檔尚未完成程式碼簽章和 x64/x86 實機冒煙測試。
- 版本 `0.1.0` 尚未發布正式 GitHub Release。
