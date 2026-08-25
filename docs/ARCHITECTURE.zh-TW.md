# 架構與資料流

[English](ARCHITECTURE.md) | [简体中文](ARCHITECTURE.zh-CN.md) | [繁體中文](ARCHITECTURE.zh-TW.md)

## 總覽

Codex Account Switch 是單一程序原生桌面應用程式。Slint 負責視窗與系統匣，Rust 負責本機憑據、SQLite、網路查詢、切換交易及背景更新。專案沒有自有後端。

```text
Slint 視窗 / 系統匣
            |
            v
        src/main.rs
         |       |
         |       +-- Codex Profile / Switcher / Login / Usage
         +---------- Claude Profile / SQLite Switcher
            |
            v
本機檔案、ChatGPT 端點、Codex/Claude 桌面程序
```

## 模組

- `ui/app.slint`：視窗、系統匣、語言選單、匯入、帳戶卡片、額度與活動洞察。
- `src/main.rs`：生命週期、UI 回呼、共享狀態、背景更新、預覽狀態與統一結束流程。
- `src/identity.rs`：`auth.json`、OAuth/JWT claims、API Key 解析與身分比較。
- `src/profile.rs`：Codex Profile、別名驗證、原子寫入、權限、回收與復原。
- `src/switcher.rs`：儲存/接管/切換、token 回寫、快照與回復。
- `src/login.rs`：啟動/取消 `codex login`、偵測 live 檔案變更與登入逾時。
- `src/usage.rs`：額度/活動請求、視窗分類、重設時間、快取與格式化。
- `src/claude.rs`：Claude Cookies Profile、SQLite 備份/驗證、切換、重新啟動、回收與復原。
- `src/restart.rs`：在 macOS/Windows 結束並重新啟動 ChatGPT/Codex 桌面應用程式。
- `src/paths.rs`：平台 live 路徑、`CODEX_HOME` 與 `CODEX_SWITCH_HOME`。
- `src/i18n.rs`：七種介面語言、系統語言偵測與翻譯覆蓋。

## Codex 匯入

1. 從 `codex login`、live 檔案、使用者選取的檔案/目錄、文字或剪貼簿讀取。
2. 限制大小，移除可選 Markdown JSON 程式碼區塊並解析憑據。
3. 擷取身分中繼資料，產生或驗證別名。
4. 以私人權限原子寫入 `profiles/<alias>/auth.json`。
5. 瀏覽器/外部登入接管 live 身分並更新 current，不覆寫其他帳戶。

## Codex 切換交易

1. 取得 `auth.lock` 並重新驗證目標 Profile。
2. 讀取 live 檔案，在 `recovery/codex/` 建立快照。
3. live 與 current 身分一致時，將更新後 token 回寫目前 Profile。
4. 身分不一致時拒絕操作，避免覆寫已儲存的目前 Profile。
5. 原子替換 live `auth.json`，再更新 current 標記。
6. current 更新失敗時復原原 live 檔案。
7. 只有使用者選擇「重新啟動」時才重新啟動相關桌面程序。

## Claude 切換交易

1. 取得 `cookies.lock` 並驗證目標 SQLite Profile。
2. 結束 Claude Desktop。
3. 使用 SQLite Backup API 為活動 Cookies 建立快照。
4. 處理 sidecar 並替換 live Cookies。
5. 更新 current 並重新啟動 Claude Desktop。
6. 替換失敗時復原原資料庫；刪除只會移至私人回收區。

## 額度與活動更新

- `wham/usage` 提供 plan、credits 與額度視窗。
- `wham/profiles/me` 提供 Token、連續天數、工作階段時長、每日/每週/累計 bucket、推理、技能、聊天與外掛/技能排行。
- 依視窗時長分類短視窗與每週視窗；相對重設值換算為本機絕對時間。
- 結果原子快取至 `cache/usage.json`；錯誤只保留於記憶體/UI，不包含 token。
- 額度請求成功但活動請求失敗時，不會清除上次活動快照。
- UI 補齊 52×7 每日熱圖，並產生每週/累計趨勢。
- single-flight 防止重複更新；目前帳戶每分鐘更新，完整帳戶週期或依需求更新。

## 並行與生命週期

- `instance.lock` 防止多個應用程式執行個體並行。
- `Arc<Mutex<AppShared>>` 保護共享狀態；背景結果透過 Slint 事件迴圈返回。
- 點擊卡片只變更預覽別名；憑據切換必須明確按下「切換」或「重新啟動」。
- 標題列關閉、介面結束與系統匣結束共用同一關閉路徑，停止更新/登入並結束事件迴圈。

## 信任邊界

- 本機檔案系統與目前 OS 使用者是主要信任邊界。
- ChatGPT/Claude 服務、OAuth/JWT 內容與桌面用戶端行為屬於第三方依賴。
- 應用程式檢查本機結構與一致性，但不驗證伺服器簽章或撤銷狀態。
- 未簽署產物不提供發布者身分保證。
