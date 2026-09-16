# Changelog

[English](CHANGELOG.md) | [简体中文](CHANGELOG.zh-CN.md) | [繁體中文](CHANGELOG.zh-TW.md)

This log records source changes and local verification; a Git tag does not imply a published GitHub Release.

## 1.2.0 - 2026-09-16 (testing prerelease)

- Automatic mode prices the models actually reported for the account. When the official API supplies uncached input, cached input and output tokens by model, these are priced separately. Otherwise, each day’s total is apportioned by model turn counts and explicitly labeled as an estimate. Turn shares can differ substantially from token shares.
- Each model shows token categories, equivalent value and source. Unknown models and unallocated tokens remain unpriced. The former dropdown is available under Single-model comparison.
- Uses the [official standard short-context price snapshot](https://developers.openai.com/api/docs/pricing) verified on 2026-09-16. The default 80% input / 50% cached-input assumptions apply only to estimated portions. Values are not bills and exclude long context, fast mode, cache writes and tool surcharges.
- Expand an account and open Reset history to read official received/used events for the past 30 days. All pages are read, deduplicated and sorted newest first in local time. Failed refreshes preserve the complete cache. The private `cache/reset-history.json` persists across restarts and separates real account IDs. This is read-only and consumes no reset credits.
- Local session logs lack reliable account attribution and are not pooled into valuations. Personal accounts may not expose workspace token details; the UI states this limitation and coverage.

- macOS arm64 build 14, signed by Global Cybersecurity Alliance Limited; ZIP, package checksums and source fingerprints are published after macOS/Windows CI and signed-package validation. 57 local tests pass. Not notarized; no Windows binaries.

## v1.1.0 - 2026-09-15 (tagged local prerelease; no GitHub Release)

- Inline account details consolidate limits, timestamps, cache and error status.
- Automatic token API-equivalent estimates from an official price snapshot, model selection, optional advanced ratios, date coverage and daily trends.
- macOS packaging increments the build number shown in the app and bundle.
- 50 automated tests; macOS arm64 ZIP with Global Cybersecurity Alliance Limited Apple Distribution signature and SHA-256 checksum. Not notarized; no Windows binaries in this release.

## 0.1.0 - Unreleased

### Added

- Added account/analytics navigation, collapsed import and More actions, and live 7/30-day usage, product, plugin, and skill analytics.
- Enabled system proxy support for the reproduced 403 issue; failed refreshes preserve success timestamps and report per-panel errors.

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
- Historical automated baseline: 35 tests covering credentials, imports, permissions, recovery, switching, preview, Claude SQLite, usage windows, reset credits, and activity parsing.
