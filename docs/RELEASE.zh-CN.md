# 构建与发布说明

[English](RELEASE.md) | [简体中文](RELEASE.zh-CN.md) | [繁體中文](RELEASE.zh-TW.md)

本文说明当前源码中实际存在的构建脚本和发布门槛。本地或 CI 构建成功，不等于已经形成可正式对外发布的签名产物。

## 工具链

- 固定 Rust 工具链：`1.97.1`（`rust-toolchain.toml`）
- 最低 Rust 版本：`1.92`（`Cargo.toml`）
- UI 框架：Slint `1.17`
- CI 运行环境：`macos-14` 和 `windows-2025`

请使用仓库固定的工具链，不要使用更旧的系统 Rust。

## 质量检查

打包前运行：

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

当前源码基线包含 32 项测试。测试数量变化时，应同步更新 README 和 CHANGELOG 中的状态说明。

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

脚本会执行 Hardened Runtime 签名、严格签名验证、公证、Staple 和 Gatekeeper 检查。对外分发前每一步都必须通过。

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

## 发布流程

1. 更新 `Cargo.toml` 版本并刷新 `Cargo.lock`。
2. 将三种语言 CHANGELOG 中的 `Unreleased` 替换为发布日期。
3. 重新运行质量检查和平台打包检查。
4. 创建发布提交和标签。
5. 从该标签重新构建，随后完成签名、公证（如适用）并生成校验值。
6. 发布完全对应的二进制、许可证、第三方声明和发布说明。
7. 在 GitHub Release 中写明平台、架构、签名状态和已知限制。

## 当前正式发布阻断项

- macOS App 尚未通过 Developer ID 签名和 Apple 公证。
- Windows 可执行文件尚未完成代码签名和 x64/x86 真机冒烟测试。
- 版本 `0.1.0` 尚未发布正式 GitHub Release。
