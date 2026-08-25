# 第三方软件声明

[English](THIRD_PARTY_NOTICES.md) | [简体中文](THIRD_PARTY_NOTICES.zh-CN.md) | [繁體中文](THIRD_PARTY_NOTICES.zh-TW.md)

Codex Account Switch 使用以下开源或第三方组件。版本来自当前 `Cargo.lock`；链接指向对应项目或源码仓库。

## 应用内显示的归属信息

### Slint 1.17.1

- 用途：原生桌面用户界面框架。
- 项目与源码：[slint.dev](https://slint.dev)、[github.com/slint-ui/slint](https://github.com/slint-ui/slint)
- 本应用采用的许可：`LicenseRef-Slint-Royalty-free-2.0`。
- 要求：该许可要求在应用顶层可访问的“关于”界面中显示 Slint 官方 `AboutSlint` 组件，或在公开下载页面显示 Slint 归属徽标。本应用采用内置 `AboutSlint` 组件。
- 许可原文：[Slint Royalty-free License 2.0](https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md)

## 随分发物保留的主要依赖信息

| 组件 | 当前版本 | 用途 | 许可证 | 项目或源码 URL |
| --- | --- | --- | --- | --- |
| arboard | 3.6.1 | 剪贴板访问 | MIT OR Apache-2.0 | https://github.com/1Password/arboard |
| base64 | 0.22.1 | 账号令牌解析 | MIT OR Apache-2.0 | https://github.com/marshallpierce/rust-base64 |
| chrono | 0.4.45 | 时间与重置时间格式化 | MIT OR Apache-2.0 | https://github.com/chronotope/chrono |
| fs4 | 0.13.1 | 文件锁 | MIT OR Apache-2.0 | https://github.com/al8n/fs4-rs |
| reqwest | 0.12.28 | HTTPS 请求 | MIT OR Apache-2.0 | https://github.com/seanmonstar/reqwest |
| rustls | 0.23.43 | TLS 实现 | Apache-2.0 OR ISC OR MIT | https://github.com/rustls/rustls |
| rfd | 0.15.4 | 原生文件选择框 | MIT | https://github.com/PolyMeilex/rfd |
| rusqlite | 0.40.2 | Claude Desktop SQLite 数据访问 | MIT | https://github.com/rusqlite/rusqlite |
| libsqlite3-sys | 0.38.2 | 随应用编译的 SQLite 接口与源码 | MIT；SQLite 核心为 Public Domain | https://github.com/rusqlite/rusqlite |
| serde / serde_json | 1.0.229 / 1.0.151 | 数据序列化 | MIT OR Apache-2.0 | https://serde.rs / https://github.com/serde-rs/json |
| thiserror | 2.0.19 | 错误类型 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| windows-sys | 0.61.2 | Windows 系统接口 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |

Cargo 会为不同平台解析额外的传递依赖。精确版本以 `Cargo.lock` 为准；各组件的完整许可原文以其源码包内的 `LICENSE`、`LICENSE-*`、`COPYING` 或 `NOTICE` 文件为准。本文记录归属与来源，不改变任何第三方许可条款。
