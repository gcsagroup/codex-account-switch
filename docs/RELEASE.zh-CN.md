# 构建与发布说明

[English](RELEASE.md) | [简体中文](RELEASE.zh-CN.md) | [繁體中文](RELEASE.zh-TW.md)

本文说明当前源码中实际存在的构建脚本和发布门槛。本地或 CI 构建成功，不等于已经形成可正式对外发布的签名产物。

## 自动发布（1.3.1 起）

日常只需提交并推送到 `main`。工作流运行测试，生成 `v版本-ci.运行ID.重试次数` 标签与独立预发布；不再手工修改 workflow 的标签、安装包 SHA 或发布分支。其他分支／PR 只检查。支持在 Actions 中手动运行 `main`。一次推送多个提交时发布该次推送的最终提交；后续成功推送各自保留独立版本。

构建号由准备任务统一生成（1000 + workflow 运行序号 × 100 + 重试次数），只在 runner 写入 `BUILD_NUMBER`。不会自动提交文件，避免循环触发。完整重跑有新标签；单独重跑失败任务沿用准备任务的输出。

- Windows：x64 与 x86 分别使用 MSVC、静态 CRT，运行测试、Release 构建、PE 架构、内嵌多尺寸图标检查和 10 秒隔离启动检查。发布两个 ZIP，含程序、许可证、第三方声明、版本／提交信息。启动检查不使用真实账号，不代表切换和托盘完整验收。
- macOS：自动构建 arm64；默认仅上传公司签名的新包。管理员在仓库 Actions Secrets 设置 `MACOS_SIGNING_P12_B64`（P12 的 base64）、`MACOS_SIGNING_PASSWORD`、`MACOS_SIGNING_IDENTITY`（Global Cybersecurity Alliance Limited 身份）。证书只导入临时 keychain，任务结束删除；不会自动导出开发机私钥。尚不执行公证。
- 签名配置缺失：发布说明显示 macOS 等待公司签名，仍发布 Windows。管理员显式设置 Actions Variable `MACOS_ALLOW_UNSIGNED=true` 可改为附无公司签名、临时签名的 macOS 测试包。签名配置存在但失败时整个发布失败，不静默降级。
- 汇总任务核对各包来源提交、版本、构建号、SHA256、ZIP 内容，并生成 `BUILD_INFO.json` 与 `SHA256SUMS.txt`。先创建草稿，全部附件上传后公开；失败时不公开不完整新版本。

自动版是预发布，入口为 [Releases 列表](https://github.com/gcsagroup/codex-account-switch/releases)，不是只返回稳定版的 `/releases/latest`。历史 v1.3.0 公司签名包不被覆盖。Windows 尚无代码签名；macOS 无公司签名或未公证状态必须以每次发布说明为准。

本地验证发布脚本：`python3 -m unittest discover -s scripts/tests -v`；workflow 可用 `actionlint` 检查。实际发布验收必须确认 Actions 成功、下载两个 Windows ZIP 并核对 `SHA256SUMS.txt` 与 `BUILD_INFO.json` 中的提交号。

## 本机公司签名包补充发布

云端证书尚未配置时，可以用本机已有公司证书签名，不导出私钥。先推送源码到 `main`，等待该提交的自动发布成功；按该 Release 的 `BUILD_INFO.json` 使用同一版本和构建号重新打包、签名。

将原包及 `manifest.json` 放到独立分支 `codex/signed-macos/<发布标签>` 的 `release-macos/` 目录，再通过 SSH 推送。清单包含平台、签名状态、提交、版本、构建号、发布标签、包名、SHA256 和完整 tracked 源码指纹（构建号由 App 内 plist 单独核验）。此分支只触发公司签名包发布工作流，不重复触发主 CI。

工作流验证来源属于 main、标签指向该提交且 CI 已成功；下载已发布的 Windows 包，验证 macOS 原包、完整构建输入、App 版本、arm64 架构及公司签名，再上传到同一个 Release，重建三平台校验清单和发布说明。已有同版本不同内容的 macOS 包会被拒绝覆盖。未配置云端证书前，后续提交的 macOS 公司签名包仍需此本机步骤，Windows 继续自动发布。

## 历史 v1.3.0 测试预发布（2026-09-16）

macOS 13+、Apple 芯片（arm64），构建号 19。包使用 Apple Distribution: Global Cybersecurity Alliance Limited 签名；未使用 Developer ID、未公证，Gatekeeper 可能拒绝打开。本版不附 Windows 或 Intel Mac 原生二进制。

[下载 v1.3.0](https://github.com/gcsagroup/codex-account-switch/releases/tag/v1.3.0)。附件包含签名 App ZIP、`SHA256SUMS.txt` 和 `SOURCE_SHA256SUMS.txt`，App 内含许可证与第三方声明。价格使用 2026-09-16 核验的官方快照；按轮次分摊的估值不是账单。

源码与标签由 SSH 身份 GCSA-ToolsTeam 推送。已签名原包放在独立的 `codex/release-assets-v1.3.0` 发布分支；标签触发的 CI 发布任务固定该分支的提交 SHA，等待两个平台检查通过，再验证源码指纹、安装包校验和、版本、arm64 架构和公司签名，使用仓库工作流令牌发布同一原包。签名私钥不上传。可用 `bash scripts/verify-release-macos.sh dist/releases/v1.3.0` 在本地执行相同包校验。

本次新增账户下次重置时间、推荐使用次序与近 7 天 Token 等价价值；64 项测试及原生账户卡片与分析页对照验证通过。

## 工具链

- 固定 Rust 工具链：`1.97.1`（`rust-toolchain.toml`）
- 最低 Rust 版本：`1.92`（`Cargo.toml`）
- UI 框架：Slint `1.17`
- CI 运行环境：`macos-15` 和 `windows-2025`

请使用仓库固定的工具链，不要使用更旧的系统 Rust。

## 质量检查

打包前运行：

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

当前应用基线包含 73 项 Rust 测试，另有 18 项发布脚本测试。Windows 默认软件渲染，避免基础显示驱动缺少 OpenGL shader 支持而无法启动；保留显式 `SLINT_BACKEND` 设置。测试数量变化时，应同步更新 README 和 CHANGELOG 中的状态说明。

## macOS

### 本地打包与运行

```bash
./scripts/package-macos.sh
./scripts/run-macos.sh
```

输出：

```text
dist/Codex Account Switch.app
```

打包脚本会：

- 使用 Rust `1.97.1` 和 `Cargo.lock` 构建；
- 尊重外部设置的 `CARGO_TARGET_DIR`；
- 将 macOS 13 设为最低系统版本；
- 在 `Contents/Resources` 中包含 `LICENSE` 和英文主版本 `THIRD_PARTY_NOTICES.md`；
- 生成用于本地验证的未签名 App。

当前脚本按宿主机架构构建；现有本地验证基线为 Apple 芯片（`arm64`）。

### 签名与公证

配置 Developer ID 身份和 `notarytool` Keychain Profile 后运行：

```bash
export APPLE_SIGN_IDENTITY='Developer ID Application: ...'
export APPLE_NOTARY_PROFILE='codex-account-switch-notary'
./scripts/sign-notarize-macos.sh
```

脚本会执行 Hardened Runtime 签名、严格签名验证、公证、Staple 和 Gatekeeper 检查。正式稳定版对外分发前每一步都必须通过。明确标注为测试预发布的版本可附未公证包，但必须公开其签名类型和 Gatekeeper 限制。

## Windows

### 在 macOS 上交叉构建的依赖

```bash
brew install mingw-w64 lld
cargo install cargo-xwin --locked
```

### 构建

```bash
./scripts/build-windows.sh       # x64 和 x86
./scripts/build-windows.sh x64
./scripts/build-windows.sh x86
```

脚本尊重外部设置的 `CARGO_TARGET_DIR`，并使用以下目标：

- x64：`x86_64-pc-windows-gnu`
- x86：`i686-pc-windows-msvc`，通过 `cargo-xwin` 构建并静态链接 CRT

x86 链接时可能因微软静态库缺少 PDB 而出现 `LNK4099` 警告。如果 Release 构建成功、PE 架构正确，且可执行文件没有动态导入 `VCRUNTIME` 或 `MSVCP`，该警告不代表运行时依赖缺失。

### 输出

- `dist/windows/codex-account-switch-windows-x86_64.exe`：x64 正式文件
- `dist/windows/codex-account-switch-windows-x86.exe`：x86 正式文件
- `dist/windows/codex-account-switch.exe`：x64 可执行文件的兼容副本
- `*.sha256`：两个架构正式文件的校验值
- `LICENSE` 和 `THIRD_PARTY_NOTICES.md`：再分发时必须随产物提供的文件

### 静态验证

```bash
file dist/windows/*.exe

cd dist/windows
shasum -a 256 -c codex-account-switch-windows-x86_64.exe.sha256
shasum -a 256 -c codex-account-switch-windows-x86.exe.sha256
```

静态验证不能代替 Windows 真机测试。正式发布前，必须在两个支持的架构上验证启动、Codex 导入与切换、Claude Desktop 切换、重启动作、托盘行为和明确退出。

## 手动稳定版发布流程（与自动预发布分开）

1. 更新 `Cargo.toml` 版本并刷新 `Cargo.lock`。
2. 将三种语言 CHANGELOG 中的 `Unreleased` 替换为发布日期。
3. 重新运行质量检查和平台打包检查。
4. 创建发布提交和标签。
5. 从标签构建并签名，或复用已验收且源码指纹与标签一致的签名原包；完成适用的公证与校验。
6. 发布完全对应的二进制、许可证、第三方声明和发布说明。
7. 在 GitHub Release 中写明平台、架构、签名状态和已知限制。

## 当前正式发布阻断项

- macOS App 尚未通过 Developer ID 签名和 Apple 公证。
- Windows 可执行文件尚未完成代码签名和 x64/x86 真机冒烟测试。
- `v1.3.0` 为 GitHub 测试预发布，尚不是通过上述门槛的稳定版。
