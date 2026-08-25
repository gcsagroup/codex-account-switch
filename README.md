# Codex Account Switch

Codex Account Switch 是一个本地原生桌面工具，用于保存、切换和恢复多份 Codex 与 Claude Desktop 账号。它同时展示 Codex 额度、各账号重置时间和当前账号活动洞察，并提供系统托盘快捷操作。本项目由 GCSA 内部使用和维护，不是 OpenAI 官方产品。

技术栈：**Rust 1.97.1 + Slint 1.17 + bundled SQLite**。界面不使用 WebView，账号数据不经过项目自有服务器。

## 当前状态

- 当前版本：`0.1.0`，尚未正式发布。
- macOS：已完成本机 arm64 App 构建和真实界面验收；当前本地包未签名、未公证。
- Windows：提供 x64 与 x86 构建；已校验 PE 架构、SHA-256 和运行库依赖，尚未完成 Windows 真机验收。
- Linux：源码包含部分通用路径，但没有正式交付和运行验收，不属于当前支持范围。

详细变更见 [CHANGELOG.md](CHANGELOG.md)，实现审查见 [docs/AUDIT_REPORT.md](docs/AUDIT_REPORT.md)。

## 主要功能

### Codex 账号

- 保存当前 `$CODEX_HOME/auth.json`，或从浏览器授权、文件、目录、文本和剪贴板导入。
- 自动从邮箱前缀生成别名，也可以手动命名、改名、删除和恢复上次删除。
- 原子切换 live `auth.json`；切换前保存恢复快照，并写回当前账号刷新后的 token。
- 目标凭据无效、账号身份不一致或 current 标记更新失败时停止切换并回滚。
- 可仅切换账号，也可切换后自动重启 ChatGPT/Codex 桌面端。
- OAuth 和 API Key 凭据都可保存与切换；只有 ChatGPT OAuth 账号可查询订阅额度。

### 额度与活动

- 查询 ChatGPT 5 小时与每周窗口的使用率和重置时间。
- 对只返回单个每周窗口的 Pro 响应进行归类，不误显示为 5 小时窗口。
- 每个账号卡片使用独立信息区显示额度、重置时间和最后更新时间；使用率达到 90% 时高亮提醒。
- 点击账号卡片可在左侧预览该账号的身份、额度和活动数据，不会改变实际使用中的账号；切换仍需点击“切换”或“重启”。
- 左侧账号详情显示累计 Token、峰值日、当前/最长连续天数、最长会话和最近七天用量趋势。
- “活动洞察”提供最近 52 周每日热力图、每周/累计趋势、快速模式占比、常用推理强度、技能与聊天统计，以及最常用的插件/技能。
- 活动接口暂时失败时保留上次成功数据，不因额度接口单独成功而清空洞察。
- 当前账号每分钟后台刷新；完整列表按启动、手动刷新及周期节点更新。
- 缓存过期后显示等待刷新，不继续展示已经重置的旧百分比。

### Claude Desktop

- 保存、切换、改名、删除和恢复多份 Claude Desktop Cookies Profile。
- 使用 SQLite Backup API 创建活动数据库的一致快照，并执行 `quick_check` 和 cookies 表验证。
- 切换前保存恢复快照；失败时恢复原 Cookies 数据库。
- 切换账号时自动退出并重新启动 Claude Desktop。

### 桌面体验

- 系统托盘提供显示窗口、刷新额度、快速切换和退出。
- 支持简体中文、繁体中文、英语、日语、韩语、法语和西班牙语。
- 无效 Profile 会显示为不可用，并禁用切换和重启。
- 应用内“退出”、系统托盘“退出”和 macOS 标题栏关闭按钮都会结束进程。
- 单实例锁防止两个进程同时修改账号文件。

## 使用前提

- macOS 13 或更高版本，或者受支持的 Windows x64/x86 系统。
- 使用“浏览器登录”时需要 `codex` CLI 已安装并可从 `PATH` 运行。
- 使用 Claude Desktop 切换前，需要先在 Claude Desktop 中完成至少一次登录。
- 查询额度需要有效的 ChatGPT OAuth access token 与 account ID；API Key 账号不会显示订阅额度。

## 快速开始

### macOS

本地构建并启动：

```bash
./scripts/package-macos.sh
./scripts/run-macos.sh
```

也可以直接打开：

```text
dist/Codex Account Switch.app
```

当前本地 App 未签名。外部分发前必须完成 Developer ID 签名、公证和 Gatekeeper 验证。

### Windows

交付目录中包含：

- `codex-account-switch-windows-x86_64.exe`：64 位 Windows，推荐使用。
- `codex-account-switch-windows-x86.exe`：32 位 Windows。
- `codex-account-switch.exe`：与 x64 文件完全相同的兼容别名，不是第三个版本。
- 同名 `.sha256` 文件：对应架构可执行文件的 SHA-256 校验值。

## 基本使用

1. 启动应用，在顶部选择 **Codex** 或 **Claude Desktop**。
2. 首次使用可选择“浏览器登录”“保存当前”“从路径”或“粘贴文本”。
3. 点击账号卡片可在左侧查看详情；需要真正启用时，再点击“切换”或“重启”。
4. 删除操作会移入应用私有回收区；使用“恢复上次删除”撤销最近一次删除。
5. Codex 只执行“切换”时，需要手动重启相关 CLI/App；“重启”按钮会自动处理。
6. Claude Desktop 切换成功后会自动重启。
7. 标题栏关闭、界面“退出”或托盘“退出”都会结束 Codex Account Switch。

## 本地数据路径

默认数据根目录为 `~/.codex-switch`，可用 `CODEX_SWITCH_HOME` 覆盖。
该路径保留旧产品名是为了兼容已有 Profile、缓存和恢复数据，产品改名后不会自动迁移或清空。

| 路径 | 用途 |
|---|---|
| `$CODEX_HOME/auth.json` | Codex 当前 live 凭据，默认 `~/.codex/auth.json` |
| `~/.codex-switch/profiles/<alias>/auth.json` | Codex 已保存 Profile |
| `~/.codex-switch/current` | 当前 Codex 别名 |
| `~/.codex-switch/cache/usage.json` | 本地额度与活动缓存 |
| `~/.codex-switch/recovery/codex/` | Codex 切换前恢复快照 |
| `~/.codex-switch/trash/codex/` | Codex 私有回收区 |
| `~/.codex-switch/claude-desktop/profiles/<alias>/Cookies` | Claude 已保存 Profile |
| `~/.codex-switch/claude-desktop/current` | 当前 Claude 别名 |
| `~/.codex-switch/claude-desktop/recovery/` | Claude 切换前恢复快照 |
| `~/.codex-switch/claude-desktop/trash/` | Claude 私有回收区 |
| `~/.codex-switch/app.log` | 本地运行日志，不记录 token |

Claude Desktop live Cookies 路径：

- macOS：`~/Library/Application Support/Claude/Cookies`
- Windows：`%APPDATA%\Claude\Cookies`

支持的环境变量：

- `CODEX_HOME`：覆盖 Codex live 目录。
- `CODEX_SWITCH_HOME`：覆盖 Codex Account Switch 数据目录。

## 从源码构建

项目通过 `rust-toolchain.toml` 固定 Rust `1.97.1`，`Cargo.toml` 声明的最低 Rust 版本为 `1.92`。

```bash
cargo build --locked --release
```

macOS App：

```bash
./scripts/package-macos.sh
```

在 macOS 上交叉构建 Windows x64/x86：

```bash
brew install mingw-w64 lld
cargo install cargo-xwin --locked

./scripts/build-windows.sh       # 两种架构
./scripts/build-windows.sh x64   # 仅 64 位
./scripts/build-windows.sh x86   # 仅 32 位
```

构建与交付细节见 [docs/RELEASE.md](docs/RELEASE.md)。

## 测试

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

当前测试基线为 32 项，覆盖身份解析、导入、权限、回收恢复、事务切换、详情预览、Claude SQLite、额度窗口和活动洞察解析。

## 安全边界

- Unix 上应用数据目录目标权限为 `0700`，凭据、锁和临时文件从创建时使用 `0600`。
- 导入和切换前检查格式、大小、账号身份与别名边界。
- Claude Profile 保存的是 Chromium 加密后的 Cookies；解密密钥仍由操作系统 Keychain/凭据系统管理。
- UI、托盘和日志不输出 access token、refresh token、API Key 或完整 Cookies。
- 项目不会验证第三方 OAuth/JWT 的服务端签名，也不提供云同步或远程凭据服务。

完整说明见 [SECURITY.md](SECURITY.md)。

## 已知限制

- `wham/usage` 与 `wham/profiles/me` 是 ChatGPT 非公开稳定接口，字段或访问规则可能变化。
- 额度查询失败不会阻止本地账号切换；界面会保留错误状态供用户手动刷新。
- Windows 构建已完成静态验证，但在正式分发前仍需要 Windows x64/x86 真机测试和代码签名。
- macOS 本地 App 在配置签名身份前不是可公开分发的正式安装包。
- 当前仓库尚无正式 Release；版本历史统一记录在 `0.1.0 - 未发布`。

## 相关文档

- [完整变更记录](CHANGELOG.md)
- [架构与数据流](docs/ARCHITECTURE.md)
- [项目审查与实施报告](docs/AUDIT_REPORT.md)
- [构建与发布说明](docs/RELEASE.md)
- [安全说明](SECURITY.md)

## 许可

本项目采用 [MIT License](LICENSE)。“关于”入口包含工具简介，并按 Slint 免版税许可的要求显示官方归属组件、版本与 URL；其余主要依赖的许可证、用途和源码地址见 [第三方软件说明](THIRD_PARTY_NOTICES.md)。macOS 与 Windows 打包脚本会把项目许可证和第三方说明一并放入分发物。
