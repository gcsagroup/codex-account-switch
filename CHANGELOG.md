# Changelog

[English](CHANGELOG.md) | [简体中文](CHANGELOG.zh-CN.md) | [繁體中文](CHANGELOG.zh-TW.md)

This file records user-visible features, compatibility, security, and delivery changes. The project has no published release yet, so the current implementation is collected under `0.1.0`.

## 0.1.0 - Unreleased

### Added

- Local Codex and Claude Desktop profile save, switch, rename, recoverable delete, and restore.
- Recovery snapshots, transactional rollback, private trash, and Restore Last Deleted.
- Codex five-hour/weekly usage, percentages, update/reset times, credits, available reset credit counts, and per-account status.
- Cumulative and peak tokens, current/longest streaks, longest session, recent activity, and a 52-week insight view.
- Browser authorization, current-profile save, path import, pasted text, and clipboard import.
- System tray actions for showing the window, refreshing usage, switching, and quitting.
- English, Simplified Chinese, Traditional Chinese, Japanese, Korean, French, and Spanish UI.

### Changed

- Renamed the product and deliverables to `Codex Account Switch` / `codex-account-switch`. The legacy `CODEX_SWITCH_HOME` and `~/.codex-switch` names remain for data compatibility.
- Rebuilt the About dialog as “GCSA Internal Utility” with a product summary and Slint attribution.
- Reworked account cards, quota/reset presentation, detail preview, navigation, import panels, and status hierarchy.
- Classified weekly-only Pro responses correctly and stopped showing expired quota windows as current values.
- Added single-flight usage refresh, one-minute active-account refresh, and periodic all-account refresh.
- Preserved the last successful activity snapshot when the activity endpoint fails independently.
- Switched Claude Desktop Cookies capture to the SQLite Backup API with integrity and sidecar checks.
- Added reusable external `CARGO_TARGET_DIR` support to macOS and Windows build scripts.

### Fixed and hardened

- Prevented browser-login or imported accounts from overwriting a different saved profile.
- Added target revalidation, identity checks, token write-back, recovery snapshots, rollback, and private file permissions to Codex switching.
- Added SQLite integrity checks, locks, recovery snapshots, sidecar handling, and rollback to Claude switching.
- Fixed profile-card clipping, missing button text, default-window overflow, language-menu overlap, and macOS close-without-exit behavior.
- Marked invalid profiles explicitly and disabled their Switch/Restart actions.
- Added single-instance locking, usage-refresh serialization, login cancellation, and a 300-second login timeout.
- Completed all seven UI translations and added translation-coverage tests.

### Build and verification

- Pinned Rust `1.97.1`; CI runs formatting, Clippy, tests, and release builds on macOS and Windows.
- Added macOS packaging/signing/notarization scripts.
- Added Windows x64 and x86 builds, SHA-256 files, and an x64 compatibility alias.
- Current automated baseline: 35 tests covering credentials, imports, permissions, recovery, switching, preview, Claude SQLite, usage windows, reset credits, and activity parsing.
