# Build and release guide

[English](RELEASE.md) | [简体中文](RELEASE.zh-CN.md) | [繁體中文](RELEASE.zh-TW.md)

This guide describes the build scripts and release gates that exist in the current source tree. A successful local or CI build is not, by itself, a signed and supported public release.

## Toolchain

- Pinned Rust toolchain: `1.97.1` (`rust-toolchain.toml`)
- Minimum Rust version: `1.92` (`Cargo.toml`)
- UI framework: Slint `1.17`
- CI runners: `macos-14` and `windows-2025`

Use the repository toolchain rather than an older system Rust installation.

## Quality checks

Run these checks before packaging:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

The current source baseline contains 35 tests. If that number changes, update the status statements in the README and changelog.

## macOS

### Package and run locally

```bash
./scripts/package-macos.sh
./scripts/run-macos.sh
```

Output:

```text
dist/Codex Account Switch.app
```

The package script:

- builds with Rust `1.97.1` and `Cargo.lock`;
- respects an externally supplied `CARGO_TARGET_DIR`;
- sets macOS 13 as the minimum system version;
- includes `LICENSE` and the English `THIRD_PARTY_NOTICES.md` in `Contents/Resources`;
- creates an unsigned package for local validation.

The current script builds for the host architecture. The existing local validation baseline is Apple silicon (`arm64`).

### Sign and notarize

Configure a Developer ID identity and a `notarytool` keychain profile, then run:

```bash
export APPLE_SIGN_IDENTITY='Developer ID Application: ...'
export APPLE_NOTARY_PROFILE='codex-account-switch-notary'
./scripts/sign-notarize-macos.sh
```

The script performs hardened-runtime signing, strict signature verification, notarization, stapling, and Gatekeeper assessment. Every step must pass before external distribution.

## Windows

### Cross-build prerequisites on macOS

```bash
brew install mingw-w64 lld
cargo install cargo-xwin --locked
```

### Build

```bash
./scripts/build-windows.sh       # x64 and x86
./scripts/build-windows.sh x64
./scripts/build-windows.sh x86
```

The script respects an externally supplied `CARGO_TARGET_DIR` and uses these targets:

- x64: `x86_64-pc-windows-gnu`
- x86: `i686-pc-windows-msvc`, built with `cargo-xwin` and a statically linked CRT

The x86 linker can emit `LNK4099` warnings for missing PDB files in Microsoft static libraries. If the release build succeeds, the PE architecture is correct, and the executable does not dynamically import `VCRUNTIME` or `MSVCP`, that warning is not a runtime dependency failure.

### Outputs

- `dist/windows/codex-account-switch-windows-x86_64.exe`: x64 release executable
- `dist/windows/codex-account-switch-windows-x86.exe`: x86 release executable
- `dist/windows/codex-account-switch.exe`: compatibility copy of the x64 executable
- `*.sha256`: checksum files for the architecture-specific executables
- `LICENSE` and `THIRD_PARTY_NOTICES.md`: files that must accompany redistribution

### Static verification

```bash
file dist/windows/*.exe

cd dist/windows
shasum -a 256 -c codex-account-switch-windows-x86_64.exe.sha256
shasum -a 256 -c codex-account-switch-windows-x86.exe.sha256
```

Static verification does not replace real Windows testing. Before public release, test both supported architectures on Windows, including launch, Codex import and switching, Claude Desktop switching, restart actions, tray behavior, and explicit exit.

## Release flow

1. Update the version in `Cargo.toml` and refresh `Cargo.lock`.
2. Replace `Unreleased` in all three changelogs with the release date.
3. Re-run the quality checks and platform packaging checks.
4. Create the release commit and tag.
5. Rebuild from that tag, then sign, notarize where applicable, and generate checksums.
6. Publish matching binaries, license files, third-party notices, and release notes.
7. State the platform, architecture, signing status, and known limitations in the GitHub Release.

## Current public-release blockers

- The macOS package has not yet passed Developer ID signing and Apple notarization.
- Windows executables have not yet passed code signing or real x64/x86 machine smoke tests.
- No formal GitHub Release has been published for version `0.1.0`.
