# 架构与数据流

## 总览

Codex Account Switch 是单进程原生桌面应用。Slint 负责窗口和系统托盘，Rust 负责本地凭据、SQLite、网络查询、切换事务与后台刷新。应用没有自有服务端。

```text
Slint 窗口 / 系统托盘
          │
          ▼
      src/main.rs
       │      │
       │      ├── Codex Profile / Switcher / Login / Usage
       │      └── Claude Profile / SQLite Switcher
       ▼
本地文件、ChatGPT 接口、Codex/Claude 桌面进程
```

## 模块职责

- `ui/app.slint`：窗口、托盘、语言菜单、导入面板、账号列表、额度与活动洞察视图。
- `src/main.rs`：应用生命周期、UI 回调、共享状态、后台刷新、退出路径和模块编排。
- `src/identity.rs`：解析 `auth.json`、OAuth/JWT claims、API Key 和账号身份比较。
- `src/profile.rs`：Codex Profile 存储、别名校验、原子写入、权限、回收与恢复。
- `src/switcher.rs`：Codex 保存当前、接管外部登录、切换、token 写回、快照和回滚事务。
- `src/login.rs`：启动和取消 `codex login`，检测 live `auth.json` 变化及登录超时。
- `src/usage.rs`：额度与 Token 活动请求、窗口归类、重置时间、缓存和格式化。
- `src/claude.rs`：Claude Cookies Profile、SQLite 一致性备份、切换、重启、回收与恢复。
- `src/restart.rs`：macOS/Windows 上退出并重新启动 ChatGPT/Codex 桌面端。
- `src/paths.rs`：平台 live 路径、`CODEX_HOME` 与 `CODEX_SWITCH_HOME`。
- `src/i18n.rs`：七种语言、系统语言检测和翻译完整性测试。

## Codex 导入流程

1. 来源可以是 `codex login`、当前 live 文件、用户选择的文件、文本或剪贴板。
2. 限制输入大小，清理可选 Markdown JSON 代码块并解析凭据。
3. 提取邮箱、plan 和 account ID；生成或校验别名。
4. 以私有权限原子写入 `profiles/<alias>/auth.json`。
5. 外部登录采用 adopt-live 流程，同步 current 标记但不覆盖旧账号。

## Codex 切换事务

1. 获取 `auth.lock` 独占锁并重新验证目标 Profile。
2. 读取 live `auth.json`，在 `recovery/codex/` 创建恢复快照。
3. 如果 live 与 current 属于同一账号，将刷新后的 token 写回当前 Profile。
4. 如果身份不一致，停止切换，避免把外部账号覆盖到旧 Profile。
5. 原子替换 live `auth.json`，再更新 current 标记。
6. current 更新失败时恢复切换前 live 文件。
7. 用户选择“重启”时，等待 ChatGPT/Codex 退出后重新启动。

## Claude 切换事务

1. 获取 `cookies.lock` 并验证目标 SQLite Profile。
2. 请求 Claude Desktop 退出。
3. 使用 SQLite Backup API 为 live Cookies 创建一致恢复快照。
4. 清理可能冲突的 sidecar，原子替换 live Cookies。
5. 更新 current 标记并重新启动 Claude Desktop。
6. 替换失败时恢复原数据库；删除操作只移动到私有回收区。

## 额度刷新

- `wham/usage` 提供 plan、credits 和限额窗口。
- `wham/profiles/me` 提供累计/峰值 Token、连续天数、最长会话、每日/每周/累计 bucket、推理强度、技能/聊天统计和常用插件/技能。
- API primary/secondary 按 `limit_window_seconds` 归类为短窗口或每周窗口。
- `reset_after_seconds` 会根据获取时间换算为本地绝对重置时间。
- 结果原子写入 `cache/usage.json`；错误保存在内存/UI 状态，不包含 token。额度请求成功但活动请求临时失败时保留上次成功的活动快照。
- UI 将每日 bucket 补齐为 52×7 热力图；每周和累计 bucket 分别形成 52 点趋势，累计值按可见区间起点归一化。
- single-flight 防止重复刷新；后台线程每分钟刷新当前账号，并周期性刷新全部账号。

## 并发与生命周期

- `instance.lock` 防止多实例同时运行。
- UI 状态由 `Arc<Mutex<AppShared>>` 保护；后台结果通过 Slint 事件循环回到 UI。
- 应用退出会停止刷新循环、取消浏览器登录、隐藏托盘和窗口，然后结束事件循环。
- 标题栏关闭、界面退出和托盘退出使用同一退出函数。

## 信任边界

- 本地文件系统和当前 OS 用户是主要信任边界。
- ChatGPT/Claude、OAuth/JWT 和桌面客户端行为均属于第三方依赖。
- 应用只做本地结构与一致性检查，不验证服务端签名或 token 撤销状态。
- 未签名的构建产物不提供发布者身份保证。
