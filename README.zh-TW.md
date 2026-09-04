# Codex Account Switch

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

Codex Account Switch 是本機原生桌面工具，用於儲存、預覽、切換及復原多個 Codex 與 Claude Desktop 帳戶，並顯示 Codex 額度、重設時間及帳戶活動洞察。本專案由 GCSA 內部使用與維護，並非 OpenAI 官方產品。

技術棧為 **Rust 1.97.1**、**Slint 1.17** 與 bundled SQLite。介面不使用 WebView，專案沒有帳戶同步伺服器或遙測服務。

## 專案狀態

- 版本：`0.1.0`，尚未發布。
- macOS：已完成本機 arm64 App 建置及真實視窗驗收；尚未簽署或公證。
- Windows：x64/x86 建置及 PE、SHA-256、執行階段匯入檢查通過；仍需 Windows 實機驗收。
- Linux：目前不在支援或發布驗收範圍內。

## 主要功能

### Codex 帳戶

- 從目前的 `$CODEX_HOME/auth.json`、瀏覽器授權、檔案/目錄、文字或剪貼簿匯入。
- 依電子郵件產生別名，也可重新命名、刪除及復原 Profile。
- 透過檔案鎖、復原快照、token 回寫、身分檢查與回復機制，以原子方式切換 `auth.json`。
- 可只切換，也可切換後重新啟動相關 ChatGPT/Codex 桌面程序。
- 支援 OAuth 與 API Key Profile；訂閱額度只適用於相容的 ChatGPT OAuth 帳戶。

### 額度與活動

- 顯示各帳戶的 5 小時/每週額度視窗、百分比、更新/重設時間與可用重設次數。
- 點擊帳戶卡片只會預覽左側詳情，不會變更實際使用中的帳戶。
- 顯示累計/峰值 Token、目前/最長連續天數、最長工作階段與近期活動。
- 活動洞察包含 52 週每日熱圖、每週/累計趨勢、快速模式、推理強度、技能/聊天統計與常用外掛/技能。
- 活動端點暫時失敗時會保留上次成功快照。

### Claude Desktop 帳戶

- 儲存、切換、重新命名、刪除及復原多份加密 Chromium Cookies Profile。
- 使用 SQLite Backup API、`quick_check`、資料表驗證、檔案鎖、復原快照與失敗回復。
- 成功切換時自動結束並重新啟動 Claude Desktop。

### 桌面體驗

- 系統匣支援顯示視窗、重新整理額度、切換帳戶與結束程式。
- 支援英文、簡體中文、繁體中文、日文、韓文、法文與西班牙文。
- 損壞的 Profile 仍會顯示，但無法切換或重新啟動。
- 單一執行個體保護資料目錄；標題列關閉、介面結束與系統匣結束都會終止應用程式。

## 使用條件

- macOS 13 或更新版本，或受支援的 Windows x64/x86 系統。
- 瀏覽器登入需要 `codex` CLI 位於 `PATH`。
- 儲存 Claude Cookies 前，Claude Desktop 至少完成過一次登入。
- 查詢訂閱額度需要有效的 ChatGPT OAuth access token 與 account ID。

## 建置與執行

儲存庫固定 Rust `1.97.1`，`Cargo.toml` 宣告的最低版本為 Rust `1.92`。

```bash
cargo build --locked --release
```

macOS：

```bash
./scripts/package-macos.sh
./scripts/run-macos.sh
```

輸出：`dist/Codex Account Switch.app`

在 macOS 交叉建置 Windows：

```bash
brew install mingw-w64 lld
cargo install cargo-xwin --locked

./scripts/build-windows.sh       # x64 與 x86
./scripts/build-windows.sh x64   # 僅 x64
./scripts/build-windows.sh x86   # 僅 x86
```

輸出：

- `dist/windows/codex-account-switch-windows-x86_64.exe`
- `dist/windows/codex-account-switch-windows-x86.exe`
- `dist/windows/codex-account-switch.exe`（x64 相容別名）

## 基本流程

1. 在頂端選擇 **Codex** 或 **Claude Desktop**。
2. 透過瀏覽器登入、儲存目前帳戶、路徑或文字匯入來新增帳戶。
3. 點擊卡片預覽；只有按下「切換」或「重新啟動」才會真正啟用帳戶。
4. 刪除項目會移至應用程式私人回收區，可用「復原上次刪除」撤銷最近一次刪除。
5. Codex「切換」後需手動重新啟動用戶端；「重新啟動」會自動處理。Claude Desktop 切換會自動重新啟動用戶端。

## 本機資料與相容性

預設資料目錄繼續使用 `~/.codex-switch`，也可由 `CODEX_SWITCH_HOME` 覆寫。舊目錄與環境變數名稱刻意保留，確保產品改名後既有 Profile、快取與復原資料不會被隱藏或自動移轉。

| 路徑 | 用途 |
| --- | --- |
| `$CODEX_HOME/auth.json` | Codex 目前憑據，預設為 `~/.codex/auth.json` |
| `~/.codex-switch/profiles/<alias>/auth.json` | Codex Profile |
| `~/.codex-switch/current` | 目前 Codex 別名標記 |
| `~/.codex-switch/cache/usage.json` | 額度與活動快取 |
| `~/.codex-switch/recovery/codex/` | Codex 切換前快照 |
| `~/.codex-switch/trash/codex/` | 可復原的 Codex 刪除項目 |
| `~/.codex-switch/claude-desktop/` | Claude Profile、current、復原區與回收區 |
| `~/.codex-switch/app.log` | 本機日誌，不記錄 token |

Claude Desktop 目前 Cookies 路徑：

- macOS：`~/Library/Application Support/Claude/Cookies`
- Windows：`%APPDATA%\Claude\Cookies`

## 品質檢查

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

目前基準為 35 項測試，涵蓋身分解析、匯入、權限、可復原刪除、交易式切換、詳情預覽、Claude SQLite、額度視窗、重設次數與活動解析。

## 安全與發布邊界

- 憑據與快照只儲存於本機。Unix 資料目錄目標權限為 `0700`，憑據、鎖與暫存檔建立時使用 `0600`。
- 額度查詢只會向相關 ChatGPT 端點傳送目前 OAuth token 與 account ID。
- 專案沒有自有雲端、遙測或憑據同步服務。
- 目前 macOS/Windows 產物為本機驗收版本，不是已簽署的公開發行版。
- ChatGPT `wham` 端點不是公開穩定 API，可能發生變更。

## 文件

- [變更記錄](CHANGELOG.zh-TW.md)
- [架構與資料流](docs/ARCHITECTURE.zh-TW.md)
- [建置與發布](docs/RELEASE.zh-TW.md)
- [安全說明](SECURITY.zh-TW.md)
- [第三方軟體說明](THIRD_PARTY_NOTICES.zh-TW.md)

## 授權

專案採用 [MIT License](LICENSE)。「關於」介面包含 Slint 免版稅授權要求的歸屬元件。第三方授權、用途及原始碼 URL 請參閱[第三方軟體說明](THIRD_PARTY_NOTICES.zh-TW.md)，打包時會附帶英文法務主文件 `THIRD_PARTY_NOTICES.md`。
