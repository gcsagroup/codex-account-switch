# 架构与数据流

[English](ARCHITECTURE.md) | [简体中文](ARCHITECTURE.zh-CN.md) | [繁體中文](ARCHITECTURE.zh-TW.md)

## 总览

Codex Account Switch 是单进程原生桌面应用。Slint 负责窗口和系统托盘，Rust 负责本地凭据、SQLite、网络查询、切换事务和后台刷新。项目没有自有后端。

```text
Slint 窗口 / 系统托盘
            |
            v
        src/main.rs
         |       |
         |       +-- Codex Profile / Switcher / Login / Usage
         +---------- Claude Profile / SQLite Switcher
            |
            v
本地文件、ChatGPT 接口、Codex/Claude 桌面进程
```

## 模块

- `ui/app.slint`：窗口、托盘、语言菜单、导入、账号卡片、额度和活动洞察。
- `src/main.rs`：生命周期、UI 回调、共享状态、后台刷新、预览状态和统一退出。
- `src/identity.rs`：`auth.json`、OAuth/JWT claims、API Key 解析和身份比较。
- `src/profile.rs`：Codex Profile、别名校验、原子写入、权限、回收和恢复。
- `src/switcher.rs`：保存/接管/切换、token 写回、快照和回滚。
- `src/login.rs`：启动/取消 `codex login`、检测 live 文件变化和登录超时。
- `src/usage.rs`：额度/活动请求、窗口归类、重置时间、缓存和格式化。
- `src/claude.rs`：Claude Cookies Profile、SQLite 备份/验证、切换、重启、回收和恢复。
- `src/restart.rs`：在 macOS/Windows 退出并重新启动 ChatGPT/Codex 桌面应用。
- `src/paths.rs`：平台 live 路径、`CODEX_HOME` 和 `CODEX_SWITCH_HOME`。
- `src/i18n.rs`：七种界面语言、系统语言检测和翻译覆盖。

## Codex 导入

1. 从 `codex login`、live 文件、用户选择的文件/目录、文本或剪贴板读取。
2. 限制大小，移除可选 Markdown JSON 代码块并解析凭据。
3. 提取身份元数据，生成或校验别名。
4. 以私有权限原子写入 `profiles/<alias>/auth.json`。
5. 浏览器/外部登录接管 live 身份并更新 current，不覆盖其他账号。

## Codex 切换事务

1. 获取 `auth.lock` 并重新验证目标 Profile。
2. 读取 live 文件，在 `recovery/codex/` 创建快照。
3. live 与 current 身份一致时，把刷新 token 写回当前 Profile。
4. 身份不一致时拒绝操作，避免覆盖已保存的当前 Profile。
5. 原子替换 live `auth.json`，再更新 current 标记。
6. current 更新失败时恢复原 live 文件。
7. 只有用户选择“重启”时才重新启动相关桌面进程。

## Claude 切换事务

1. 获取 `cookies.lock` 并验证目标 SQLite Profile。
2. 退出 Claude Desktop。
3. 用 SQLite Backup API 为活动 Cookies 创建快照。
4. 处理 sidecar 并替换 live Cookies。
5. 更新 current 并重新启动 Claude Desktop。
6. 替换失败时恢复原数据库；删除只移动到私有回收区。

## 额度与活动刷新

- `wham/usage` 提供 plan、credits、可用重置次数和额度窗口。
- `wham/profiles/me` 提供 Token、连续天数、会话时长、每日/每周/累计 bucket、推理、技能、聊天和插件/技能排行。
- 按窗口时长归类短窗口和每周窗口；相对重置值换算为本地绝对时间。
- 结果原子缓存到 `cache/usage.json`；错误只保存在内存/UI，不包含 token。
- 额度请求成功但活动请求失败时，不会清除上次活动快照。
- UI 补齐 52×7 每日热力图，并生成每周/累计趋势。
- single-flight 防止重复刷新；当前账号每分钟刷新，完整账号周期或按需刷新。

## 并发与生命周期

- `instance.lock` 防止多个应用实例并发运行。
- `Arc<Mutex<AppShared>>` 保护共享状态；后台结果通过 Slint 事件循环返回。
- 点击卡片只改变预览别名；凭据切换必须显式点击“切换”或“重启”。
- 标题栏关闭、界面退出和托盘退出共用同一关闭路径，停止刷新/登录并结束事件循环。

## 信任边界

- 本地文件系统和当前 OS 用户是主要信任边界。
- ChatGPT/Claude 服务、OAuth/JWT 内容和桌面客户端行为属于第三方依赖。
- 应用检查本地结构和一致性，但不验证服务端签名或撤销状态。
- 未签名产物不提供发布者身份保证。
