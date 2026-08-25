# 安全说明

[English](SECURITY.md) | [简体中文](SECURITY.zh-CN.md) | [繁體中文](SECURITY.zh-TW.md)

## 支持范围

| 版本或产物 | 状态 |
| --- | --- |
| 当前 `0.1.0` 源码 | 接受安全修复 |
| 未签名的本地 `dist/` 构建 | 仅用于本地验证 |
| 更早快照或第三方重新打包 | 不提供支持 |

项目尚未发布经过签名、公证或 Windows 代码签名的正式版本。不要把本地未签名产物视为可信公开发行版。

## 报告安全问题

优先使用 GitHub Private Security Advisory；否则请私下联系维护者。不要在公开 Issue、截图或日志中放入真实凭据。

报告应包含：

- 受影响提交、操作系统和架构。
- 最小复现步骤及预期/实际结果。
- 已脱敏路径、日志或伪造测试凭据。
- 影响，例如凭据覆盖、越权读取、路径穿越或无法退出。

严禁附带真实 `auth.json`、OAuth token、API Key、Claude Cookies、Keychain 数据或完整用户目录路径。

## 需要保护的数据

- Codex `auth.json` 内的 access token、refresh token、account ID 和 API Key。
- Claude Desktop Chromium Cookies 数据库及恢复快照。
- `CODEX_SWITCH_HOME` 下的 Profile、current 标记、恢复区、回收区、额度缓存和日志。

## 已实施保护

### 文件与路径

- Unix 数据目录目标权限为 `0700`，凭据、锁文件和临时文件创建时使用 `0600`。
- 别名仅允许 ASCII 字母、数字、`.`、`_`、`-`，不能以 `.` 开头，最长 64 字符。
- `auth.json` 导入上限为 2 MiB，且必须包含支持的 OAuth 或 API Key 结构。
- 写入使用唯一临时文件、同步和原子替换。
- 单实例锁和各 provider 文件锁避免并发替换。

### Codex 切换

- 在锁内重新验证目标 Profile，并在替换前为 live 文件创建快照。
- 只有 live 与已保存身份一致时才写回刷新后的 token。
- current 标记失败时尝试恢复之前的 live 文件。
- 损坏 Profile 无法切换或触发重启。
- 删除只移动到私有可恢复回收区。

### Claude Desktop 切换

- 使用 SQLite Backup API 获取活动 Cookies。
- 保存或切换前执行 `quick_check` 和 Cookies 表验证。
- 替换前停止 Claude Desktop、保存恢复材料并处理 WAL/journal sidecar。
- 失败时保留或恢复材料；删除可恢复。

### 网络与日志

- 额度请求使用 rustls TLS，向相关 ChatGPT 接口发送当前 OAuth token 和 account ID。
- 项目没有云端、遥测、凭据上传或账号同步服务。
- UI、托盘和应用日志不会主动输出 token、API Key 或完整 Cookies。

## 明确限制

- 本地 OAuth/JWT 解析不验证服务端签名、撤销状态或权限范围。
- OS 用户、凭据存储或管理员账户失陷超出应用保护边界。
- ChatGPT `wham` 接口不是公开稳定 API。
- Claude Cookies 仍依赖 Chromium 与操作系统加密行为。
- 未签名构建不能保证发布者身份。

## 公开发布安全门禁

1. 通过格式检查、Clippy、全部测试和 Release 构建。
2. 在目标 macOS/Windows 上验证启动、切换、回滚、重启和退出。
3. 对 macOS 产物完成签名、公证、staple 和 Gatekeeper 检查。
4. 对 Windows 产物完成代码签名及 x64/x86 SmartScreen、杀毒软件检查。
5. 发布 SHA-256，并明确标注架构。
