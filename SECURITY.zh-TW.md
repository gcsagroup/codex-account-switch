# 安全說明

[English](SECURITY.md) | [简体中文](SECURITY.zh-CN.md) | [繁體中文](SECURITY.zh-TW.md)

## 支援範圍

| 版本或產物 | 狀態 |
| --- | --- |
| 目前 `0.1.0` 原始碼 | 接受安全修正 |
| 未簽署的本機 `dist/` 建置 | 僅供本機驗證 |
| 更早快照或第三方重新打包 | 不提供支援 |

專案尚未發布經簽署、公證或 Windows 程式碼簽署的正式版本。請勿將本機未簽署產物視為可信公開發行版。

## 回報安全問題

優先使用 GitHub Private Security Advisory；否則請私下聯絡維護者。請勿在公開 Issue、截圖或日誌中放入真實憑據。

回報應包含：

- 受影響提交、作業系統與架構。
- 最小重現步驟及預期/實際結果。
- 已去識別化的路徑、日誌或模擬測試憑據。
- 影響，例如憑據覆寫、未授權讀取、路徑穿越或無法結束程式。

嚴禁附帶真實 `auth.json`、OAuth token、API Key、Claude Cookies、Keychain 資料或完整使用者目錄路徑。

## 需要保護的資料

- Codex `auth.json` 內的 access token、refresh token、account ID 與 API Key。
- Claude Desktop Chromium Cookies 資料庫及復原快照。
- `CODEX_SWITCH_HOME` 下的 Profile、current 標記、復原區、回收區、額度快取與日誌。

## 已實作保護

### 檔案與路徑

- Unix 資料目錄目標權限為 `0700`，憑據、鎖檔與暫存檔建立時使用 `0600`。
- 別名只允許 ASCII 字母、數字、`.`、`_`、`-`，不能以 `.` 開頭，最長 64 字元。
- `auth.json` 匯入上限為 2 MiB，且必須包含支援的 OAuth 或 API Key 結構。
- 寫入使用唯一暫存檔、同步與原子替換。
- 單一執行個體鎖與各 provider 檔案鎖避免並行替換。

### Codex 切換

- 在鎖內重新驗證目標 Profile，並在替換前為 live 檔案建立快照。
- 只有 live 與已儲存身分一致時才回寫更新後的 token。
- current 標記失敗時嘗試復原先前的 live 檔案。
- 損壞的 Profile 無法切換或觸發重新啟動。
- 刪除只會移至私人可復原回收區。

### Claude Desktop 切換

- 使用 SQLite Backup API 取得活動 Cookies。
- 儲存或切換前執行 `quick_check` 與 Cookies 資料表驗證。
- 替換前停止 Claude Desktop、儲存復原材料並處理 WAL/journal sidecar。
- 失敗時保留或復原材料；刪除可復原。

### 網路與日誌

- 額度請求使用 rustls TLS，向相關 ChatGPT 端點傳送目前 OAuth token 與 account ID。
- 專案沒有雲端、遙測、憑據上傳或帳戶同步服務。
- UI、系統匣與應用程式日誌不會主動輸出 token、API Key 或完整 Cookies。

## 明確限制

- 本機 OAuth/JWT 解析不驗證伺服器簽章、撤銷狀態或權限範圍。
- OS 使用者、憑據儲存或管理員帳戶遭入侵超出應用程式保護邊界。
- ChatGPT `wham` 端點不是公開穩定 API。
- Claude Cookies 仍依賴 Chromium 與作業系統加密行為。
- 未簽署建置無法保證發布者身分。

## 公開發布安全門檻

1. 通過格式檢查、Clippy、全部測試與 Release 建置。
2. 在目標 macOS/Windows 驗證啟動、切換、回復、重新啟動與結束。
3. 對 macOS 產物完成簽署、公證、staple 與 Gatekeeper 檢查。
4. 對 Windows 產物完成程式碼簽署及 x64/x86 SmartScreen、防毒軟體檢查。
5. 發布 SHA-256，並清楚標示架構。
