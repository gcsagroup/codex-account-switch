# Build and release guide

[English](RELEASE.md) | [简体中文](RELEASE.zh-CN.md) | [繁體中文](RELEASE.zh-TW.md)

This guide describes the build scripts and release gates that exist in the current source tree. A successful local or CI build is not, by itself, a signed and supported public release.

## Automatic publishing (from 1.3.1)

Push to `main`; successful checks create an independent `vVERSION-ci.RUN_ID.ATTEMPT` testing prerelease automatically. No hardcoded tag, payload SHA or release branch needs updating. Other branches and PRs only run checks. Manual workflow dispatch on `main` is supported. A push containing multiple commits publishes its final commit.

One preparation job assigns the build number (1000 + workflow run number × 100 + attempt), written to `BUILD_NUMBER` only on runners. No Git write-back loop. Full reruns get new tags; rerunning failed jobs reuses the preparation outputs.

- Windows x64/x86 use MSVC with static CRT, run tests, release builds, PE validation and a 10-second isolated startup check. ZIPs include executable, licenses and provenance. Unsigned; startup checks do not replace interactive account and tray acceptance.
- macOS builds arm64. Company signing requires administrator-configured Actions Secrets: `MACOS_SIGNING_P12_B64`, `MACOS_SIGNING_PASSWORD`, `MACOS_SIGNING_IDENTITY` for Global Cybersecurity Alliance Limited. The temporary keychain is removed after signing. Local private keys are never exported automatically. Notarization is not configured.
- Missing signing configuration withholds only the macOS attachment and discloses that limitation; Windows still publishes. Administrators may explicitly set Actions Variable `MACOS_ALLOW_UNSIGNED=true` for ad-hoc signed macOS packages without company signing. A configured but failing signer fails the release rather than silently downgrading.
- Assembly verifies source commit, version, build, hashes and ZIP contents. `BUILD_INFO.json` and `SHA256SUMS.txt` accompany packages. All draft attachments upload before publication; failures do not expose incomplete new releases.

Use the [Releases list](https://github.com/gcsagroup/codex-account-switch/releases), not the stable-only `/releases/latest` endpoint. Historical company-signed packages remain unchanged. Test scripts with `python3 -m unittest discover -s scripts/tests -v` and lint workflows with `actionlint`; release acceptance also downloads both Windows ZIPs and verifies hashes and provenance.

## Historical v1.3.0 testing prerelease (2026-09-16)

macOS 13+ Apple silicon (arm64), build 19. Signed with Apple Distribution: Global Cybersecurity Alliance Limited. Not Developer ID signed or notarized; Gatekeeper may reject it. No Windows or native Intel Mac binaries are included.

[Download v1.3.0](https://github.com/gcsagroup/codex-account-switch/releases/tag/v1.3.0). Assets: signed App ZIP, `SHA256SUMS.txt` and `SOURCE_SHA256SUMS.txt`. The App includes its license and third-party notices. Prices use the official snapshot verified on 2026-09-16. Model turn-based estimates are not billing data.

The SSH identity GCSA-ToolsTeam pushes the source and tag. The already signed package is held in the dedicated `codex/release-assets-v1.3.0` branch. The tag-triggered CI release job pins its immutable commit, waits for both platform checks, validates source fingerprints, package checksum, version, arm64 architecture and company signature, then publishes the same package using the repository workflow token. No signing private key is uploaded. Run `bash scripts/verify-release-macos.sh dist/releases/v1.3.0` for the same local package checks.

This release adds next reset times, suggested account order and seven-day Token-equivalent value. All 64 tests and native account-card versus analytics checks passed.

## Toolchain

### Publishing a locally company-signed macOS package

When cloud signing credentials are unavailable, first push main and wait for its automatic release. Build locally using the exact version and build number from that release's BUILD_INFO.json, then sign with the existing company identity without exporting its private key. Push the original ZIP and manifest.json, including complete tracked source fingerprints, under `release-macos/` on a separate `codex/signed-macos/<release-tag>` branch via SSH.

The dedicated workflow verifies main ancestry, the tag, successful CI, downloaded Windows packages, macOS version/architecture/company signature and checksums before attaching the original package to the same release and regenerating all three platform manifests. It refuses to replace a different macOS package already published under the same version. Until cloud signing is configured, subsequent company-signed macOS packages still require this local step; Windows releases remain automatic.

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

The application baseline contains 72 Rust tests, plus 16 release-script tests. Windows uses software rendering by default so basic display drivers without OpenGL shader support can launch the app; explicit `SLINT_BACKEND` settings are respected. Keep README and changelog counts aligned when tests change.

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

The script performs hardened-runtime signing, strict signature verification, notarization, stapling, and Gatekeeper assessment. Every step must pass for a stable distribution. Testing prereleases may include an unnotarized package with its signing type and Gatekeeper limitation clearly disclosed.

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

## Manual stable releases (separate from automatic prereleases)

1. Update the version in `Cargo.toml` and refresh `Cargo.lock`.
2. Replace `Unreleased` in all three changelogs with the release date.
3. Re-run the quality checks and platform packaging checks.
4. Create the release commit and tag.
5. Build and sign from the tag, or reuse the validated signed package whose source fingerprints match the tag; complete applicable notarization and checksum checks.
6. Publish matching binaries, license files, third-party notices, and release notes.
7. State the platform, architecture, signing status, and known limitations in the GitHub Release.

## Current public-release blockers

- The macOS package has not yet passed Developer ID signing and Apple notarization.
- Windows executables have not yet passed code signing or real x64/x86 machine smoke tests.
- No formal GitHub Release has been published for version `0.1.0`.
