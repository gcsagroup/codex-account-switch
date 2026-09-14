# Codex Account Switch

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

Codex Account Switch 是本機原生桌面工具，用於儲存、預覽、切換及復原多個 Codex 與 Claude Desktop 帳戶，並顯示 Codex 額度、重設時間及帳戶活動洞察。本專案由 GCSA 內部使用與維護，並非 OpenAI 官方產品。

技術棧為 **Rust 1.97.1**、**Slint 1.17** 與 bundled SQLite。介面不使用 WebView，專案沒有帳戶同步伺服器或遙測服務。

## 專案狀態

- 版本：[`v1.1.0` 預發布版](https://github.com/gcsagroup/codex-account-switch/releases/tag/v1.1.0)，2026-09-15。
- macOS：已完成本機 arm64 App 建置及真實視窗驗收；已使用 Global Cybersecurity Alliance Limited 的 Apple Distribution 憑證簽署；未經 Developer ID 簽章及 Apple 公證；Gatekeeper 可能拒絕開啟，本版為測試預發布。
- Windows：本次不提供下載，尚未完成實機端到端驗收；CI 建置不等同於實際使用驗證。
- Linux：目前不在支援或發布驗收範圍內。

### 帳號卡片與 Token 估值（Ver 1.1）

- 點擊帳號列，在該帳號下方展開或收合詳情；一次展開一個帳號，不跳回頂端、不切換登入。額度、重設次數、Credits、快取及錯誤狀態合併於同一卡片。
- 分析顯示所選 7／30 天已回傳日期的 Token 合計及覆蓋天數。缺失或無效日期不補零；今日可能尚未完整。
- 分析自動依官方價格快照顯示 API 等價價值；預設 GPT-5.6 Sol，可下拉切換模型，單價與核驗日期自動帶入。
- 預設情景假設為輸入 80%、其中快取 50%，可於進階選項調整。介面僅有總 Token，無真實分類明細。採用 [官方標準價格](https://developers.openai.com/api/docs/pricing) 短上下文快照，非帳單或節省金額，不含附加費用。
- 假設依真實帳號儲存在 `~/.codex-switch/valuation.json`（或 `CODEX_SWITCH_HOME`），重新啟動後保留；分析資料仍為依帳號與日期隔離的記憶體快取。
- macOS 統一使用 `dist/Codex Account Switch.app`。打包腳本遞增 `BUILD_NUMBER`，同步顯示及套件版本；直接編譯前也應遞增該檔案。


## 主要功能

### Codex 帳戶

- 從目前的 `$CODEX_HOME/auth.json`、瀏覽器授權、檔案/目錄、文字或剪貼簿匯入。
- 依電子郵件產生別名，也可重新命名、刪除及復原 Profile。
- 透過檔案鎖、復原快照、token 回寫、身分檢查與回復機制，以原子方式切換 `auth.json`。
- 可只切換，也可切換後重新啟動相關 ChatGPT/Codex 桌面程序。
- 支援 OAuth 與 API Key Profile；訂閱額度只適用於相容的 ChatGPT OAuth 帳戶。

### 額度與活動

- 顯示各帳戶的 5 小時/每週額度視窗、百分比、更新/重設時間與可用重設次數。
- 顯示累計/峰值 Token、目前/最長連續天數、最長工作階段與近期活動。
- 活動洞察包含 52 週每日熱圖、每週/累計趨勢、快速模式、推理強度、技能/聊天統計與常用外掛/技能。
- 活動端點暫時失敗時會保留上次成功快照。

### 改版導覽與分析

- 「帳號管理」查看額度；「新增帳號」展開登入與匯入；每列「更多」提供詳情、分析、重啟、重新命名及刪除。
- 「分析」提供預覽帳號的 7／30 天方案用量、產品／模型分組、用戶端輪次、外掛與技能統計；查看分析不會切換使用中的帳號。
- Token 活動保留獨立入口。分析快取僅保留於記憶體，按真實帳號及日期範圍隔離；各面板獨立顯示失敗狀態。
- 「連線狀態」說明系統代理方式。額度更新保留上次成功時間，另記最近嘗試時間，顯示實際成功數量。
- 工具統計彙總介面每日回傳的前 8 個外掛或前 6 個技能，並非所有呼叫的稽核總數。方案用量採用服務端單位，不等同目前每週額度比例。

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

1. 在側欄選擇 **帳號管理**、**分析** 或 **Claude Desktop**。
2. 透過瀏覽器登入、儲存目前帳戶、路徑或文字匯入來新增帳戶。
3. 點擊帳號列即可預覽詳情，在卡片內展開或收合；只有「切換」或「重新啟動」才會啟用帳戶。
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
- Windows：本次不提供下載，尚未完成實機端到端驗收；CI 建置不等同於實際使用驗證。

## 品質檢查

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

目前基準為 50 項測試，涵蓋身分解析、匯入、權限、可復原刪除、交易式切換、詳情預覽、Claude SQLite、額度視窗、重設次數與活動解析。

## 安全與發布邊界

- 憑據與快照只儲存於本機。Unix 資料目錄目標權限為 `0700`，憑據、鎖與暫存檔建立時使用 `0600`。
- 額度查詢只會向相關 ChatGPT 端點傳送目前 OAuth token 與 account ID。
- 專案沒有自有雲端、遙測或憑據同步服務。
- GitHub 測試預發布提供 macOS arm64 ZIP 及 SHA-256 校驗檔，未公證；本次不提供 Windows 下載。
- ChatGPT `wham` 端點不是公開穩定 API，可能發生變更。

## 文件

- [變更記錄](CHANGELOG.zh-TW.md)
- [架構與資料流](docs/ARCHITECTURE.zh-TW.md)
- [建置與發布](docs/RELEASE.zh-TW.md)
- [安全說明](SECURITY.zh-TW.md)
- [第三方軟體說明](THIRD_PARTY_NOTICES.zh-TW.md)

## 授權

專案採用 [MIT License](LICENSE)。「關於」介面包含 Slint 免版稅授權要求的歸屬元件。第三方授權、用途及原始碼 URL 請參閱[第三方軟體說明](THIRD_PARTY_NOTICES.zh-TW.md)，打包時會附帶英文法務主文件 `THIRD_PARTY_NOTICES.md`。
