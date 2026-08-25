# 构建与发布说明

## 工具链

- Rust：固定 `1.97.1`。
- 最低 Rust：`1.92`，由 `Cargo.toml` 声明。
- UI：Slint `1.17`。
- CI：macOS 14 与 Windows 2025。

Homebrew Rust 1.91.1 与当前 Slint 不兼容。构建时应使用仓库的 `rust-toolchain.toml` 或明确调用 Rust 1.97.1。

## 发布前检查

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

当前基线为 32 项测试。变更测试数量后，应同步更新 README、CHANGELOG 和审查报告。

## macOS

### 打包

```bash
./scripts/package-macos.sh
./scripts/run-macos.sh
```

输出：

```text
dist/Codex Account Switch.app
```

App 的 `Contents/Resources` 同时包含项目 `LICENSE` 和 `THIRD_PARTY_NOTICES.md`。

当前脚本生成 arm64 本地 App，最低系统版本为 macOS 13。`package-macos.sh` 只打包，不签名。

脚本尊重外部 `CARGO_TARGET_DIR`；未设置时仍使用仓库内的 `target/`。

### 签名与公证

先在 Keychain 中配置 Developer ID 和 `notarytool` profile：

```bash
export APPLE_SIGN_IDENTITY='Developer ID Application: ...'
export APPLE_NOTARY_PROFILE='codex-account-switch-notary'
./scripts/sign-notarize-macos.sh
```

脚本会执行 hardened runtime 签名、严格验证、公证、staple 和 Gatekeeper 检查。任何一步失败都不能发布。

## Windows

### macOS 交叉构建依赖

```bash
brew install mingw-w64 lld
cargo install cargo-xwin --locked
```

### 构建

```bash
./scripts/build-windows.sh
./scripts/build-windows.sh x64
./scripts/build-windows.sh x86
```

Windows 脚本同样尊重外部 `CARGO_TARGET_DIR`。

目标映射：

- x64：`x86_64-pc-windows-gnu`
- x86：`i686-pc-windows-msvc`，通过 `cargo-xwin` 构建并静态链接 CRT

x86 链接时可能出现微软静态库缺少 PDB 的 `LNK4099` 警告。Release 构建成功、PE 架构正确且没有动态 VC Runtime 导入时，该警告不影响可执行文件运行。

### 输出文件

- `dist/windows/codex-account-switch-windows-x86_64.exe`：64 位正式文件。
- `dist/windows/codex-account-switch-windows-x86.exe`：32 位正式文件。
- `dist/windows/codex-account-switch.exe`：x64 兼容别名，与 x64 文件哈希相同。
- `*.sha256`：两个架构正式文件的校验值。
- `LICENSE`、`THIRD_PARTY_NOTICES.md`：项目许可证与第三方归属、许可及源码 URL，发布时必须与可执行文件一同提供。

### 静态校验

```bash
file dist/windows/*.exe

cd dist/windows
shasum -a 256 -c codex-account-switch-windows-x86_64.exe.sha256
shasum -a 256 -c codex-account-switch-windows-x86.exe.sha256
```

静态校验不能替代 Windows 真机验收。发布前必须在 x64 和 x86 环境验证启动、Codex 导入/切换、Claude SQLite、重启、托盘和退出。

## 版本与变更记录

1. 更新 `Cargo.toml` 版本并刷新 `Cargo.lock`。
2. 将 `CHANGELOG.md` 对应版本从“未发布”改为发布日期。
3. 确保 README、SECURITY、审查报告和产物名称一致。
4. 创建可信提交和发布标签。
5. 从该标签重新构建、签名并生成 SHA-256。
6. 发布说明中明确平台、架构、签名状态和已知限制。

## 当前发布阻断项

- macOS 尚未完成 Developer ID 签名与公证。
- Windows 尚未完成代码签名和 x64/x86 真机验收。
- 当前仓库尚未建立正式 Release 和可追溯的发布提交基线。
