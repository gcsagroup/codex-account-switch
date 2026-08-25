# 第三方軟體聲明

[English](THIRD_PARTY_NOTICES.md) | [简体中文](THIRD_PARTY_NOTICES.zh-CN.md) | [繁體中文](THIRD_PARTY_NOTICES.zh-TW.md)

Codex Account Switch 使用以下開放原始碼或第三方元件。版本取自目前的 `Cargo.lock`；連結指向對應專案或原始碼儲存庫。

## 應用程式內顯示的歸屬資訊

### Slint 1.17.1

- 用途：原生桌面使用者介面框架。
- 專案與原始碼：[slint.dev](https://slint.dev)、[github.com/slint-ui/slint](https://github.com/slint-ui/slint)
- 本應用程式採用的授權：`LicenseRef-Slint-Royalty-free-2.0`。
- 要求：該授權要求在應用程式頂層可存取的「關於」畫面中顯示 Slint 官方 `AboutSlint` 元件，或在公開下載頁面顯示 Slint 歸屬徽章。本應用程式採用內建 `AboutSlint` 元件。
- 授權原文：[Slint Royalty-free License 2.0](https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md)

## 隨散布物保留的主要相依項目資訊

| 元件 | 目前版本 | 用途 | 授權條款 | 專案或原始碼 URL |
| --- | --- | --- | --- | --- |
| arboard | 3.6.1 | 剪貼簿存取 | MIT OR Apache-2.0 | https://github.com/1Password/arboard |
| base64 | 0.22.1 | 帳戶權杖解析 | MIT OR Apache-2.0 | https://github.com/marshallpierce/rust-base64 |
| chrono | 0.4.45 | 時間與重設時間格式化 | MIT OR Apache-2.0 | https://github.com/chronotope/chrono |
| fs4 | 0.13.1 | 檔案鎖定 | MIT OR Apache-2.0 | https://github.com/al8n/fs4-rs |
| reqwest | 0.12.28 | HTTPS 請求 | MIT OR Apache-2.0 | https://github.com/seanmonstar/reqwest |
| rustls | 0.23.43 | TLS 實作 | Apache-2.0 OR ISC OR MIT | https://github.com/rustls/rustls |
| rfd | 0.15.4 | 原生檔案選擇器 | MIT | https://github.com/PolyMeilex/rfd |
| rusqlite | 0.40.2 | Claude Desktop SQLite 資料存取 | MIT | https://github.com/rusqlite/rusqlite |
| libsqlite3-sys | 0.38.2 | 隨應用程式編譯的 SQLite 繫結與原始碼 | MIT；SQLite 核心為 Public Domain | https://github.com/rusqlite/rusqlite |
| serde / serde_json | 1.0.229 / 1.0.151 | 資料序列化 | MIT OR Apache-2.0 | https://serde.rs / https://github.com/serde-rs/json |
| thiserror | 2.0.19 | 錯誤類型 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| windows-sys | 0.61.2 | Windows 系統 API | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |

Cargo 會為不同平台解析額外的間接相依項目。精確版本以 `Cargo.lock` 為準；各元件的完整授權原文以其原始碼套件內的 `LICENSE`、`LICENSE-*`、`COPYING` 或 `NOTICE` 檔案為準。本文記錄歸屬與來源，不變更任何第三方授權條款。
