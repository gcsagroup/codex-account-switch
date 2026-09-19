# Codex Account Switch

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

Codex Account Switch 是本機原生桌面工具，用於儲存、預覽、切換及復原多個 Codex 與 Claude Desktop 帳戶，並顯示 Codex 額度、重設時間及帳戶活動洞察。本專案由 GCSA 內部使用與維護，並非 OpenAI 官方產品。

技術棧為 **Rust 1.97.1**、**Slint 1.17** 與 bundled SQLite。介面不使用 WebView，專案沒有帳戶同步伺服器或遙測服務。

## 專案狀態

- 自動更新：每次推送 `main`，檢查通過後在 [Releases](https://github.com/gcsagroup/codex-account-switch/releases) 發布獨立測試版，不必手動建立標籤。基線版本 1.3.1；標籤為 `v版本-ci.執行ID.重試次數`，應用程式建置號自動遞增。其他分支和 PR 只檢查，不發布。
- Windows：提供 x64／x86 ZIP，包含程式、授權與建置資訊；執行對應架構測試、建置與隔離啟動檢查。未簽名，實際帳戶切換與系統匣互動仍需人工驗收。
- macOS：雲端自動建置 arm64，預設須設定公司簽名才發布新包；未設定時說明原因並繼續發布 Windows。管理員可明確設定 `MACOS_ALLOW_UNSIGNED=true` 發布無公司簽名測試包。[歷史 v1.3.0 公司簽名包](https://github.com/gcsagroup/codex-account-switch/releases/tag/v1.3.0) 保留，尚未公證。
- Linux：目前不在支援或發布驗收範圍內。

### 重置時間與推薦次序（Ver 1.3 (19)）

- 帳戶展開區新增近 7 天 Token 合計與自動按模型計算的 API 等價估算。統計包含今日的 7 個 UTC 日期，標示日期範圍、覆蓋天數、官方分類／分攤天數與未計價量；不等同於按小時重設的額度週期。共用分析快取，背景載入，失敗保留成功資料，帳戶隔離。
- Codex 帳戶無論收合或展開，都顯示下次每週／5 小時視窗重置時間。時間已到時顯示「待更新」，不直接當成額度已恢復。
- 可用帳戶依每週重置時間由早到晚推薦；沒有週視窗時採用短視窗。同時間以剩餘額度多者優先，最後依帳戶別名穩定排序。耗盡帳戶另列恢復次序；若多個視窗都耗盡，採用其中較晚的恢復時間。
- 重置時間未知者排後；無效帳戶、更新失敗、視窗到期或超過 15 分鐘的快取不推薦。建議隨額度更新，卡片位置不跳動；不自動切換帳戶或消耗重置次數。

### 自動模型計價與重置歷史（Ver 1.2）

- 點擊帳號列，在該帳號下方展開或收合詳情；一次展開一個帳號，不跳回頂端、不切換登入。額度、重設次數、Credits、快取及錯誤狀態合併於同一卡片。
- macOS 統一使用 `dist/Codex Account Switch.app`。打包腳本遞增 `BUILD_NUMBER`，同步顯示及套件版本；直接編譯前也應遞增該檔案。
- 預設依帳號實際出現的模型計價。官方介面提供模型的一般輸入、快取輸入及輸出 Token 時直接分項計算；否則依當天模型輪次分攤總 Token，明確標為估算。輪次比例不等於 Token 比例，誤差可能較大。
- 逐模型列出 Token 分類、金額與來源，另列未計價 Token。未知模型及無法分攤的部分不套用其他模型價格；原下拉選單保留在「單一模型對照」。
- 使用 2026-09-16 核驗的[官方標準短上下文價格](https://developers.openai.com/api/docs/pricing)。預設輸入 80%、快取占輸入 50%，僅適用於估算部分。不是帳單，不含長上下文、快速模式、快取寫入及工具附加費。
- 帳號詳情中的「重置歷史記錄」讀取官方過去 30 天獲得／使用記錄，自動遍歷分頁、去重並以本地時間倒序顯示。失敗保留完整快取；私有 `cache/reset-history.json` 依真實帳號隔離，重新啟動後仍可讀取。僅讀歷史，不消耗次數。
- 本地模型日誌缺乏可靠帳號歸屬，不跨帳號混入估值。個人帳號可能無法取得工作區 Token 明細，介面保留限制與涵蓋天數。


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
- Windows：`%APPDATA%\Claude\Cookies`

## 品質檢查

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

目前基準為 64 項測試，涵蓋身分解析、匯入、權限、可復原刪除、交易式切換、詳情預覽、Claude SQLite、額度視窗、重設次數與活動解析。

## 安全與發布邊界

- 憑據與快照只儲存於本機。Unix 資料目錄目標權限為 `0700`，憑據、鎖與暫存檔建立時使用 `0600`。
- 額度查詢只會向相關 ChatGPT 端點傳送目前 OAuth token 與 account ID。
- 專案沒有自有雲端、遙測或憑據同步服務。
- 自動預發布提供 Windows x64／x86 ZIP、`SHA256SUMS.txt` 與 `BUILD_INFO.json`。macOS 新包與簽名狀態以當次發布說明為準；缺少簽名設定時不取代歷史公司簽名包。
- ChatGPT `wham` 端點不是公開穩定 API，可能發生變更。

## 文件

- [變更記錄](CHANGELOG.zh-TW.md)
- [架構與資料流](docs/ARCHITECTURE.zh-TW.md)
- [建置與發布](docs/RELEASE.zh-TW.md)
- [安全說明](SECURITY.zh-TW.md)
- [第三方軟體說明](THIRD_PARTY_NOTICES.zh-TW.md)

## 授權

專案採用 [MIT License](LICENSE)。「關於」介面包含 Slint 免版稅授權要求的歸屬元件。第三方授權、用途及原始碼 URL 請參閱[第三方軟體說明](THIRD_PARTY_NOTICES.zh-TW.md)，打包時會附帶英文法務主文件 `THIRD_PARTY_NOTICES.md`。
