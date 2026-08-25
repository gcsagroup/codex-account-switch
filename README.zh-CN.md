# Codex Account Switch

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

Codex Account Switch 是一个本地原生桌面工具，用于保存、预览、切换和恢复多份 Codex 与 Claude Desktop 账号，并展示 Codex 额度、重置时间和账户活动洞察。本项目由 GCSA 内部使用和维护，不是 OpenAI 官方产品。

技术栈为 **Rust 1.97.1**、**Slint 1.17** 和 bundled SQLite。界面不使用 WebView，项目没有账号同步服务器或遥测服务。

## 项目状态

- 版本：`0.1.0`，尚未发布。
- macOS：已完成本机 arm64 App 构建和真实窗口验收；尚未签名或公证。
- Windows：x64/x86 构建及 PE、SHA-256、运行库导入检查通过；仍需 Windows 真机验收。
- Linux：当前不属于支持或发布验收范围。

## 主要功能

### Codex 账号

- 从当前 `$CODEX_HOME/auth.json`、浏览器授权、文件/目录、文本或剪贴板导入。
- 按邮箱生成别名，也可改名、删除和恢复 Profile。
- 通过文件锁、恢复快照、token 写回、身份校验和回滚原子切换 `auth.json`。
- 可仅切换，也可切换后重启相关 ChatGPT/Codex 桌面进程。
- 支持 OAuth 与 API Key Profile；订阅额度仅适用于兼容的 ChatGPT OAuth 账号。

### 额度与活动

- 显示各账号的 5 小时/每周额度窗口、百分比、更新时间和重置时间。
- 点击账号卡片只预览左侧详情，不会改变实际使用中的账号。
- 显示累计/峰值 Token、当前/最长连续天数、最长会话和近期活动。
- 活动洞察包含 52 周每日热力图、每周/累计趋势、快速模式、推理强度、技能/聊天统计和常用插件/技能。
- 活动接口暂时失败时保留上次成功快照。

### Claude Desktop 账号

- 保存、切换、改名、删除和恢复多份加密 Chromium Cookies Profile。
- 使用 SQLite Backup API、`quick_check`、表验证、文件锁、恢复快照和失败回滚。
- 成功切换时自动退出并重新启动 Claude Desktop。

### 桌面体验

- 托盘支持显示窗口、刷新额度、切换账号和退出。
- 支持英语、简体中文、繁体中文、日语、韩语、法语和西班牙语。
- 损坏 Profile 会保留显示，但无法切换或重启。
- 单实例保护数据目录；标题栏关闭、界面退出和托盘退出都会结束应用。

## 使用条件

- macOS 13 或更高版本，或受支持的 Windows x64/x86 系统。
- 浏览器登录需要 `codex` CLI 位于 `PATH`。
- 保存 Claude Cookies 前，Claude Desktop 至少完成过一次登录。
- 查询订阅额度需要有效的 ChatGPT OAuth access token 与 account ID。

## 构建与运行

仓库固定 Rust `1.97.1`，`Cargo.toml` 声明的最低版本为 Rust `1.92`。

```bash
cargo build --locked --release
```

macOS：

```bash
./scripts/package-macos.sh
./scripts/run-macos.sh
```

输出：`dist/Codex Account Switch.app`

在 macOS 上交叉构建 Windows：

```bash
brew install mingw-w64 lld
cargo install cargo-xwin --locked

./scripts/build-windows.sh       # x64 和 x86
./scripts/build-windows.sh x64   # 仅 x64
./scripts/build-windows.sh x86   # 仅 x86
```

输出：

- `dist/windows/codex-account-switch-windows-x86_64.exe`
- `dist/windows/codex-account-switch-windows-x86.exe`
- `dist/windows/codex-account-switch.exe`（x64 兼容别名）

## 基本流程

1. 在顶部选择 **Codex** 或 **Claude Desktop**。
2. 通过浏览器登录、保存当前、路径或文本导入添加账号。
3. 点击卡片预览；只有点击“切换”或“重启”才会真正启用账号。
4. 删除会移入应用私有回收区，可用“恢复上次删除”撤销最近一次删除。
5. Codex“切换”后需要手动重启客户端；“重启”会自动处理。Claude Desktop 切换会自动重启客户端。

## 本地数据与兼容性

默认数据目录继续使用 `~/.codex-switch`，也可通过 `CODEX_SWITCH_HOME` 覆盖。旧目录和环境变量名称被刻意保留，确保产品改名后已有 Profile、缓存和恢复数据不会隐藏或被自动迁移。

| 路径 | 用途 |
| --- | --- |
| `$CODEX_HOME/auth.json` | Codex 当前凭据，默认 `~/.codex/auth.json` |
| `~/.codex-switch/profiles/<alias>/auth.json` | Codex Profile |
| `~/.codex-switch/current` | 当前 Codex 别名标记 |
| `~/.codex-switch/cache/usage.json` | 额度与活动缓存 |
| `~/.codex-switch/recovery/codex/` | Codex 切换前快照 |
| `~/.codex-switch/trash/codex/` | 可恢复的 Codex 删除项 |
| `~/.codex-switch/claude-desktop/` | Claude Profile、current、恢复区与回收区 |
| `~/.codex-switch/app.log` | 本地日志，不记录 token |

Claude Desktop 当前 Cookies 路径：

- macOS：`~/Library/Application Support/Claude/Cookies`
- Windows：`%APPDATA%\Claude\Cookies`

## 质量检查

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

当前基线为 32 项测试，覆盖身份解析、导入、权限、可恢复删除、事务切换、详情预览、Claude SQLite、额度窗口和活动解析。

## 安全与发布边界

- 凭据和快照只存于本地。Unix 数据目录目标权限为 `0700`，凭据、锁和临时文件创建时使用 `0600`。
- 额度查询只向相关 ChatGPT 接口发送当前 OAuth token 与 account ID。
- 项目没有自有云端、遥测或凭据同步服务。
- 当前 macOS/Windows 产物是本地验收版本，不是已签名的公开发行版。
- ChatGPT `wham` 接口不是公开稳定 API，可能发生变化。

## 文档

- [变更记录](CHANGELOG.zh-CN.md)
- [架构与数据流](docs/ARCHITECTURE.zh-CN.md)
- [构建与发布](docs/RELEASE.zh-CN.md)
- [安全说明](SECURITY.zh-CN.md)
- [第三方软件说明](THIRD_PARTY_NOTICES.zh-CN.md)

## 许可

项目采用 [MIT License](LICENSE)。“关于”界面包含 Slint 免版税许可要求的归属组件。第三方许可、用途和源码 URL 见[第三方软件说明](THIRD_PARTY_NOTICES.zh-CN.md)，打包时会附带英文法务主文件 `THIRD_PARTY_NOTICES.md`。
