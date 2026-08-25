# 安全说明

## 支持范围

| 版本 | 状态 |
|---|---|
| `0.1.0` 当前源码 | 接受安全修复 |
| 未签名的 `dist/` 本机构建 | 仅用于本地验证 |
| 更早快照或第三方重新打包 | 不提供支持 |

项目尚未发布经过签名、公证或 Windows 代码签名的公开安装包。不要把仓库内未签名的本地构建当作可信公开发行版。

## 报告安全问题

如果项目托管平台支持 Private Security Advisory，请优先通过该私密渠道报告。否则请私下联系维护者，不要在公开 Issue、截图或日志中粘贴真实凭据。

报告建议包含：

- 受影响版本、操作系统和架构。
- 最小化复现步骤与预期/实际结果。
- 已脱敏的路径、日志或伪造测试凭据。
- 影响范围，例如凭据覆盖、越权读取、路径穿越或退出失败。

严禁附带真实 `auth.json`、OAuth token、API Key、Claude Cookies、Keychain 数据或完整用户目录路径。

## 需要保护的数据

- Codex `auth.json` 中的 access token、refresh token、account ID 和 API Key。
- Claude Desktop Chromium Cookies 数据库及其恢复快照。
- `CODEX_SWITCH_HOME` 下的 Profile、current 标记、恢复区、回收区和额度缓存。
- 可能包含账号别名、邮箱或本地路径的运行日志。

## 已实施保护

### 文件与路径

- Unix 数据目录目标权限为 `0700`，凭据、锁文件和临时文件从创建时使用 `0600`。
- 别名只允许 ASCII 字母、数字、`.`、`_`、`-`，禁止以 `.` 开头，最长 64 字符。
- `auth.json` 导入上限为 2 MiB，并要求有效 OAuth token 或 API Key 结构。
- 写入使用唯一临时文件、同步和原子替换，失败时清理临时文件。
- 单实例文件锁和账号切换文件锁避免并发覆盖。

### Codex 切换

- 切换前重新验证目标 Profile，并为当前 live 凭据创建恢复快照。
- 写回刷新 token 前比较账号身份，live 与 current 不一致时拒绝覆盖旧 Profile。
- current 标记更新失败时尝试恢复原 live 文件。
- 损坏 Profile 可以显示，但不能切换或触发重启。
- 删除先移入应用私有回收区，最近一次删除可以恢复。

### Claude Desktop

- 使用 SQLite Backup API 获取活动 Cookies 数据库的一致快照。
- 保存和切换前执行 SQLite `quick_check`，并验证 cookies 表存在。
- 切换前停止 Claude Desktop、保存恢复快照并处理 WAL/journal sidecar。
- 替换或重启失败时保留恢复材料；删除同样进入私有回收区。

### 网络与日志

- 额度请求使用 rustls TLS，向 ChatGPT 域名发送当前 OAuth token 和 account ID。
- 项目没有自有云端、遥测、账号同步或凭据上传服务。
- UI、系统托盘和应用日志不主动输出 token、API Key 或完整 Cookies。
- 额度刷新失败不会绕过凭据校验，也不会阻止本地切换流程。

## 明确不保证的边界

- 本项目只解析第三方 OAuth/JWT 的本地结构和账号字段，不验证服务端签名、撤销状态或权限范围。
- 如果本机用户账户、Keychain/凭据管理器或管理员权限已经失陷，本项目无法保护同一用户可读取的数据。
- ChatGPT `wham` 接口不是公开稳定 API，字段和访问规则可能变化。
- Claude Cookies 仍依赖操作系统的 Chromium 加密密钥；复制 Profile 不等于绕过系统密钥保护。
- 未签名构建不提供发布者身份保证；外部分发前必须完成平台代码签名和来源验证。

## 发布安全门禁

对外分发前至少需要：

1. 通过格式检查、Clippy、全部测试和 Release 构建。
2. 在目标 macOS/Windows 机器上完成真实启动、切换、回滚和退出验收。
3. macOS 完成 Developer ID 签名、公证、staple 和 Gatekeeper 检查。
4. Windows 完成代码签名、SmartScreen/杀毒误报检查及 x64/x86 真机测试。
5. 发布 SHA-256 校验值，并确保下载页明确区分架构。
