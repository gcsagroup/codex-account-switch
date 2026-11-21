# Changelog

## 3.0.0 — 2026-11-21

- Show weekly remaining quota and rounded credit balance directly in account cards. Distinguish owned resets from resets usable now; include expiry dates, estimated messages and model/limit status.
- Fix reset-history response validation while retaining pagination and error handling. Missing usage percentages no longer appear as 100% remaining.

- Lead with selected-period metrics and separate Token, turn and consumption coverage; distinguish missing values, explicit zero, unsupported dimensions and stale results.
- Add daily trends, category rankings, daily detail, complete-period comparisons and CSV exports with scope, units and pricing assumptions. Persist consumption filters per account.
- Update verified model prices and add speed/context scenarios; distinguish classified usage, turn-based allocation and unpriced usage. Estimates are not bills.
- Add read-only local project/chat analysis, unknown-creator filtering and manual purpose tags. Cumulative chat tokens are neither period consumption nor cloud account totals.
- Keep the existing macOS installation path and local signing. This candidate is not notarized, publicly released or verified on native Windows.

[English](CHANGELOG.md) | [简体中文](CHANGELOG.zh-CN.md) | [繁體中文](CHANGELOG.zh-TW.md)

This log records source changes and local verification; a Git tag does not imply a published GitHub Release.

## 1.4.1 - 2026-09-22

- Embed multi-resolution Windows EXE icons and set the application window icon. Release packaging rejects missing icon resources or missing required sizes.
- Export results, import previews and import results show Codex / Claude Desktop counts and account names. The sidebar separates service lists: 3 Codex plus 1 Claude account correctly totals 4.
- Keep identical-data skips visible in import previews and results; preserve existing conflict handling and active logins.
- Add a 3+1 account round-trip regression and Windows icon-resource checks for both PE architectures.

## 1.4.0 - 2026-09-21

- Add SSH delivery of locally company-signed macOS packages, verifying the source inputs and matching Windows release before combining all platforms. Signing keys stay local.
- Add sidebar links to the GCSA product catalog and GitHub organization repositories, opening in the default browser.
- Select accounts directly in Analytics. Token summaries, activity charts and insights now share the same page, removing the nested activity screen; heatmaps adapt to the width beside the sidebar.
- Searchable, compact account cards with dedicated Details, Analytics and Switch actions; preserve reset times, recommendations and weekly Token estimates.
- Full saved-account backup for Codex and Claude Desktop with import preview, identical-data skips and conflict renaming. Existing profiles and active logins remain unchanged.
- Explicit unencrypted-credential and cross-computer recovery notices; validate format, size and aliases, preserve private permissions and roll back failed writes.
- 72 application tests and 16 release-script tests, including both-service backup round trips, conflicts, corrupt input, rollback and search.

## 1.3.1 - 2026-09-20 (automatic testing prereleases)

- Fix Windows startup failure on basic display drivers without `glCreateShader`: use software rendering by default while respecting an explicit `SLINT_BACKEND`. CI startup checks require both a live process and evidence of the UI event loop, preserving failure logs.
- Every `main` push tests, packages and publishes an independent prerelease, without hardcoded tags or a local package commit. Run IDs and attempts distinguish tags; shared build numbers do not write back to Git and cannot trigger a release loop.
- Windows x64/x86 run native CI tests, builds, PE architecture verification and isolated startup checks. ZIPs include licenses, build provenance and SHA256 checksums. Unsigned; startup checks are not full interactive acceptance.
- macOS builds in CI and supports company certificates from GitHub Secrets. By default, missing signing configuration only withholds the macOS attachment; Windows still publishes. Packages without company signing require explicit administrator opt-in. Historical signed releases remain separate.
- Validate commit, version, build number and ZIP hashes before publishing. Upload all draft attachments before making a release public. Add release-script regression tests and trilingual documentation.

## 1.3.0 - 2026-09-16 (testing prerelease)

- Expanded accounts show Token totals and automatic per-model API-equivalent estimates for the last 7 UTC dates, including today. Dates, coverage, exact/estimated days and unpriced tokens remain visible; this is not the hourly quota-reset cycle. Loading shares the analysis cache, runs in the background, preserves successful data on failure and stays account-scoped.
- Collapsed and expanded Codex account cards both show the next weekly and five-hour reset times. Passed deadlines are labeled as awaiting refresh, not as a confirmed fresh quota.
- Available accounts are suggested by earliest weekly reset (short-window reset when no weekly limit exists), then greater remaining quota, then alias. Exhausted accounts have a separate recovery order: all exhausted windows must reset before an account can recover.
- Unknown reset times come last. Invalid profiles, failed requests, expired windows and snapshots older than 15 minutes are not recommended. Recommendations update with usage refreshes; card positions remain stable and the app does not switch accounts or consume reset credits automatically.

- macOS arm64 build 19, signed by Global Cybersecurity Alliance Limited. All 64 local tests and native-window acceptance passed. The release workflow publishes the ZIP, checksum and source fingerprints after both platform CI jobs and package validation pass. Not notarized; no Windows binaries.

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
