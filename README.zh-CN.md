# Codex Account Switch

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

Codex Account Switch 是一个本地原生桌面工具，用于保存、预览、切换和恢复多份 Codex 与 Claude Desktop 账号，并展示 Codex 额度、重置时间和账户活动洞察。本项目由 GCSA 内部使用和维护，不是 OpenAI 官方产品。

技术栈为 **Rust 1.97.1**、**Slint 1.17** 和 bundled SQLite。界面不使用 WebView，项目没有账号同步服务器或遥测服务。

## 项目状态

- 版本：[v1.2.0 测试预发布版](https://github.com/gcsagroup/codex-account-switch/releases/tag/v1.2.0)，Ver 1.2 (14)，2026-09-16。
- macOS：已完成本机 arm64 App 构建和真实窗口验收；已用 Global Cybersecurity Alliance Limited 的 Apple Distribution 证书签名；未经过 Developer ID 签名及 Apple 公证，Gatekeeper 可能拒绝打开；本版为测试预发布。
- Windows：本次不提供下载，尚未完成真机端到端验收；CI 构建不等同于实际使用验证。
- Linux：当前不属于支持或发布验收范围。

### 自动模型计价与重置历史（Ver 1.2）

- 点击账号行，在该账号下方展开或收起详情；只展开一个账号，不跳回顶部，也不切换登录。额度、重置次数、Credits、缓存及错误状态合并在同一卡片。
- macOS 统一在 `dist/Codex Account Switch.app` 运行与验收。打包脚本递增 `BUILD_NUMBER`，版本同时显示于窗口与应用包；直接编译前也应递增该文件。
- 默认自动按账号实际出现的模型计价。官方接口提供模型的普通输入、缓存输入和输出 Token 时，直接分项计算；否则将当天总 Token 按当天模型轮次分摊，明确标为估算。轮次占比不等于 Token 占比，误差可能较大。
- 逐模型展示分类 Token、等价金额和数据来源，列出未计价 Token。未知模型、缺少活动明细或无法分摊的部分不套用其他模型价格。原模型下拉保留在「单一模型对照」中。
- 采用 2026-09-16 核验的[官方标准短上下文价格](https://developers.openai.com/api/docs/pricing)。默认分摊假设仍为输入 80%、缓存占输入 50%，只对估算部分生效；不代表账单，不含长上下文、快速模式、缓存写入与工具附加费用。
- 在账号展开详情中点击「重置历史记录」，读取官方过去 30 天的获得／使用记录，自动遍历分页、去重、按时间倒序显示本地时区。刷新失败保留完整缓存；历史保存在私有的 `cache/reset-history.json`，按真实账号隔离。此入口只读历史，不消耗重置次数。
- 本地模型日志缺少可靠账号归属，不跨账号混入估值。个人账号的工作区 Token 明细接口可能不可用；界面保留该限制与覆盖天数。


## 主要功能

### Codex 账号

- 从当前 `$CODEX_HOME/auth.json`、浏览器授权、文件/目录、文本或剪贴板导入。
- 按邮箱生成别名，也可改名、删除和恢复 Profile。
- 通过文件锁、恢复快照、token 写回、身份校验和回滚原子切换 `auth.json`。
- 可仅切换，也可切换后重启相关 ChatGPT/Codex 桌面进程。
- 支持 OAuth 与 API Key Profile；订阅额度仅适用于兼容的 ChatGPT OAuth 账号。

### 额度与活动

- 显示各账号的 5 小时/每周额度窗口、百分比、更新/重置时间和可用重置次数。
- 显示累计/峰值 Token、当前/最长连续天数、最长会话和近期活动。
- 活动洞察包含 52 周每日热力图、每周/累计趋势、快速模式、推理强度、技能/聊天统计和常用插件/技能。
- 活动接口暂时失败时保留上次成功快照。

### 改版导航与分析

- 左侧「账号管理」查看额度；「添加账号」展开登录和导入；各行「更多」提供详情、分析、重启、改名与删除。
- 「分析」展示当前预览账号的 7／30 天套餐用量、模型／产品分组、使用端轮次、插件与技能统计；查看分析不会切换实际账号。
- Token 活动保留独立入口。分析缓存仅在内存中保留，并按真实账号、日期范围隔离；失败面板保留旧数据并明确提示。
- 「连接状态」说明系统代理方式。额度刷新分别记录上次成功和最近尝试时间，显示本次成功账号数，失败不会再显示全部更新成功。
- 工具统计汇总接口每日返回的前 8 个插件或前 6 个技能；不是全部调用的审计总数。套餐用量遵循服务端单位，不等同于当前周额度比例。

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

1. 在侧栏选择 **账号管理**、**分析** 或 **Claude Desktop**。
2. 通过浏览器登录、保存当前、路径或文本导入添加账号。
3. 点击账号行即可查看详情，在卡片内展开或收起；“更多”中仍保留详情和分析入口；只有点击“切换”或“重启”才会真正启用账号。
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
- Windows：本次不提供下载，尚未完成真机端到端验收；CI 构建不等同于实际使用验证。

## 质量检查

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

当前基线为 57 项测试，覆盖身份解析、导入、权限、可恢复删除、事务切换、详情预览、Claude SQLite、额度窗口、重置次数和活动解析。

## 安全与发布边界

- 凭据和快照只存于本地。Unix 数据目录目标权限为 `0700`，凭据、锁和临时文件创建时使用 `0600`。
- 额度查询只向相关 ChatGPT 接口发送当前 OAuth token 与 account ID。
- 项目没有自有云端、遥测或凭据同步服务。
- GitHub 预发布提供 macOS arm64 ZIP 和 SHA-256 校验文件；未公证。Windows 不提供本版本下载。
- ChatGPT `wham` 接口不是公开稳定 API，可能发生变化。

## 文档

- [变更记录](CHANGELOG.zh-CN.md)
- [架构与数据流](docs/ARCHITECTURE.zh-CN.md)
- [构建与发布](docs/RELEASE.zh-CN.md)
- [安全说明](SECURITY.zh-CN.md)
- [第三方软件说明](THIRD_PARTY_NOTICES.zh-CN.md)

## 许可

项目采用 [MIT License](LICENSE)。“关于”界面包含 Slint 免版税许可要求的归属组件。第三方许可、用途和源码 URL 见[第三方软件说明](THIRD_PARTY_NOTICES.zh-CN.md)，打包时会附带英文法务主文件 `THIRD_PARTY_NOTICES.md`。
