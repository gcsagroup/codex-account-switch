# Codex Account Switch

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

Codex Account Switch is a native desktop utility for storing, previewing, switching, and recovering multiple local Codex and Claude Desktop accounts. It also shows Codex usage limits, reset times, and account activity insights. The project is maintained for internal GCSA use and is not an official OpenAI product.

Built with **Rust 1.97.1**, **Slint 1.17**, and bundled SQLite. The UI does not use a WebView, and the project has no account-sync server or telemetry service.

## Project status

- Automatic updates: every `main` push publishes an independent testing prerelease on [Releases](https://github.com/gcsagroup/codex-account-switch/releases) after checks pass. No manual tag is needed. Base version: 3.0.0; tags use `vVERSION-ci.RUN_ID.ATTEMPT`, with increasing application build numbers. Other branches and PRs only run checks.
- Windows: x64/x86 ZIPs include the executable, licenses and build metadata. Each architecture runs tests, builds and an isolated startup check. Packages are unsigned; real-account switching and tray interactions still need manual acceptance.
- macOS: CI builds arm64; new packages require company signing by default. Missing configuration is disclosed without blocking Windows releases. Administrators may explicitly set `MACOS_ALLOW_UNSIGNED=true` for packages without company signing. The [historical v1.3.0 company-signed package](https://github.com/gcsagroup/codex-account-switch/releases/tag/v1.3.0) remains available and is not notarized.
- Linux: not currently supported or release-tested.

### Account workspace and full backup (Ver 1.4)

The bottom sidebar links to [GCSA products](https://gcsa.org/products) and [GitHub repositories](https://github.com/orgs/gcsagroup/repositories), opening in your default browser.

- Search by name, email or plan. Compact cards keep usage, resets and recommendations visible, with separate Details, Analytics and Switch actions.
- Export all saves Codex and Claude Desktop profiles into one `.casbackup` file. Import previews additions, identical-data skips and renamed conflicts before confirmation.
- Backup totals include both services. Results show separate Codex / Claude Desktop counts and account names; use the sidebar to view each service. For example, 3 Codex + 1 Claude Desktop = 4 backed-up accounts. Skipped identical entries remain visible in import previews.
- Backups contain credentials and are **not encrypted**. Keep them in a trusted local location. Identical data is skipped; conflicts use `-import-2` and similar suffixes. Existing accounts and active logins are unchanged. Export never overwrites existing files.
- Includes saved `auth.json` and Claude Cookies; excludes unsaved live sessions, analytics caches, system keys and deleted accounts. Claude may need sign-in again on another computer.
- Limits: 500 accounts, 48 MiB raw data, 64 MiB backup file. All entries are validated before writing; write failures roll back new accounts. Refresh after an abnormal interruption. macOS backups are readable/writable only by the current user.

### Account reset times and suggested order (Ver 1.3 (19))

- Expanded accounts show Token totals and automatic per-model API-equivalent estimates for the last 7 UTC dates, including today. Dates, coverage, exact/estimated days and unpriced tokens remain visible; this is not the hourly quota-reset cycle. Loading shares the analysis cache, runs in the background, preserves successful data on failure and stays account-scoped.
- Collapsed and expanded Codex account cards both show the next weekly and five-hour reset times. Passed deadlines are labeled as awaiting refresh, not as a confirmed fresh quota.
- Available accounts are suggested by earliest weekly reset (short-window reset when no weekly limit exists), then greater remaining quota, then alias. Exhausted accounts have a separate recovery order: all exhausted windows must reset before an account can recover.
- Unknown reset times come last. Invalid profiles, failed requests, expired windows and snapshots older than 15 minutes are not recommended. Recommendations update with usage refreshes; card positions remain stable and the app does not switch accounts or consume reset credits automatically.

### Automatic model valuation and reset history (Ver 1.2)

- Click an account row to expand or collapse details in place. Only one account is expanded; this neither scrolls to the top nor switches the active login. Limits, reset credits, balance, cache timestamps and errors stay inside that account card.
- Use `dist/Codex Account Switch.app` for macOS builds and acceptance. Packaging increments `BUILD_NUMBER`, shared by the visible and bundle versions; increment it before direct compilation too.
- Automatic mode prices the models actually reported for the account. When the official API supplies uncached input, cached input and output tokens by model, these are priced separately. Otherwise, each day’s total is apportioned by model turn counts and explicitly labeled as an estimate. Turn shares can differ substantially from token shares.
- Each model shows token categories, equivalent value and source. Unknown models and unallocated tokens remain unpriced. The former dropdown is available under Single-model comparison.
- Uses the [official standard short-context price snapshot](https://developers.openai.com/api/docs/pricing) verified on 2026-09-16. The default 80% input / 50% cached-input assumptions apply only to estimated portions. Values are not bills and exclude long context, fast mode, cache writes and tool surcharges.
- Expand an account and open Reset history to read official received/used events for the past 30 days. All pages are read, deduplicated and sorted newest first in local time. Failed refreshes preserve the complete cache. The private `cache/reset-history.json` persists across restarts and separates real account IDs. This is read-only and consumes no reset credits.
- Local session logs lack reliable account attribution and are not pooled into valuations. Personal accounts may not expose workspace token details; the UI states this limitation and coverage.


Account cards now show weekly remaining quota and rounded credit balances. Details separate owned resets from currently usable resets and show expiry dates, estimated messages and model/limit status; reset history loads using its event schema. Missing percentages stay unknown. See [acceptance evidence](docs/2026-11-21-quota-release-acceptance.zh-CN.md).

### Analysis workspace (Ver 3.0)

- Selected 7/30-day metrics show separate Token, turn and consumption coverage. Missing values, explicit zero, unsupported dimensions and stale responses are distinguished; lifetime and 52-week activity are collapsed separately.
- Daily trends, category rankings, daily detail and CSV exports retain scope, units, coverage and pricing assumptions. Model/source filters affect consumption only and persist per account.
- Prices were verified on 2026-11-14, with the historical 2026-09-16 snapshot retained for gpt-5.3-codex. Standard/Fast/Ultrafast and short/long context are manually selected scenarios, not detected billing tiers. Unverified combinations remain unpriced; cache-write prices are reference-only without classified usage.
- Compare complete 7/30-day UTC calendar periods excluding today. Missing data, changed units, failed refreshes and zero baselines suppress misleading percentages.
- Local project/chat analysis reads only the local SQLite index, never chat bodies. It filters by update date and shows cumulative chat tokens separately from cloud account totals. Unknown creators remain filterable; a creator match does not prove usage ownership.
- Purpose tags are local and manual, not official workspace classifications. At most the latest 10,000 records are read with an explicit truncation notice. Record counts are not completed-task counts. New analysis controls currently use Chinese labels.


## Features

### Codex accounts

- Import from the current `$CODEX_HOME/auth.json`, browser authorization, a file or directory, pasted text, or the clipboard.
- Generate aliases from email addresses, or rename, delete, and recover profiles manually.
- Switch `auth.json` atomically with file locking, recovery snapshots, token write-back, identity checks, and rollback.
- Switch only, or switch and restart the relevant ChatGPT/Codex desktop process.
- Store OAuth or API-key profiles. Subscription usage is available only for compatible ChatGPT OAuth accounts.

### Usage and activity

- Show five-hour and weekly usage windows, percentages, update/reset times, and the available reset credit count for each account.
- Click an account row to preview its details identity and usage without changing the active account.
- Show cumulative and peak tokens, current and longest streaks, longest session, and recent activity.
- Provide a 52-week activity view with daily heatmap, weekly/cumulative trends, fast-mode and reasoning statistics, skill/chat counts, and frequently used plugins or skills.
- Keep the last successful activity snapshot when the activity endpoint temporarily fails.

### Navigation and analytics

- Accounts holds quota details; Add account expands login/import, and More contains restart, rename, and delete actions. Details and Analytics have dedicated buttons.
- Analytics shows 7/30-day plan usage by product/model, turns by model/client, and plugin/skill activity for the previewed account. Viewing analytics does not switch the active account.
- Choose the viewed account directly at the top of Analytics without changing the active login. Token summaries, daily/weekly/cumulative activity, equivalent value and tool insights share one page. The activity chart covers 52 weeks; the 7/30-day filter applies to usage and valuation. Analytics caches are memory-only and isolated by account identity and date range, with independent error states.
- Connection explains system proxy support. Failed quota refreshes retain the last success time and show the actual success count.
- Tool totals aggregate the daily top 8 plugins or top 6 skills returned by the service, rather than a complete audit. Plan usage follows server units and is separate from current weekly quota.

### Claude Desktop accounts

- Save, switch, rename, delete, and recover multiple encrypted Chromium Cookies profiles.
- Use the SQLite Backup API, `quick_check`, table validation, file locking, recovery snapshots, and rollback.
- Stop and relaunch Claude Desktop as part of a successful switch.

### Desktop behavior

- System tray actions for showing the window, refreshing usage, switching accounts, and quitting.
- UI languages: English, Simplified Chinese, Traditional Chinese, Japanese, Korean, French, and Spanish.
- Invalid profiles remain visible but cannot be switched or restarted.
- One process owns the data directory; title-bar close, in-app Quit, and tray Quit terminate the application.

## Requirements

- macOS 13 or later, or a supported Windows x64/x86 system.
- `codex` CLI available on `PATH` for browser login.
- Claude Desktop must have completed at least one login before its Cookies database can be saved.
- A valid ChatGPT OAuth access token and account ID are required for subscription usage queries.

## Build and run

The repository pins Rust `1.97.1`; `Cargo.toml` declares Rust `1.92` as the minimum supported version.

```bash
cargo build --locked --release
```

macOS App:

```bash
./scripts/package-macos.sh
./scripts/run-macos.sh
```

Output: `dist/Codex Account Switch.app`

Cross-build Windows artifacts from macOS:

```bash
brew install mingw-w64 lld
cargo install cargo-xwin --locked

./scripts/build-windows.sh       # x64 and x86
./scripts/build-windows.sh x64   # x64 only
./scripts/build-windows.sh x86   # x86 only
```

Outputs:

- `dist/windows/codex-account-switch-windows-x86_64.exe`
- `dist/windows/codex-account-switch-windows-x86.exe`
- `dist/windows/codex-account-switch.exe` — x64 compatibility alias

## Basic workflow

1. Select **Accounts**, **Analysis**, or **Claude Desktop** in the sidebar.
2. Add an account through browser login, Save Current, path import, or pasted text.
3. Click an account row to preview its details. Use **Switch** or **Restart** only to activate an account.
4. Deleted profiles move to the private application trash; use Restore Last Deleted to undo the latest deletion.
5. A Codex **Switch** requires a manual client restart; **Restart** handles it automatically. Claude Desktop switches restart the client automatically.

## Local data and compatibility

The default application data root remains `~/.codex-switch`, or `CODEX_SWITCH_HOME` when set. The legacy directory and environment-variable names are intentionally preserved so the product rename does not hide or migrate existing profiles, caches, or recovery data.

| Path | Purpose |
| --- | --- |
| `$CODEX_HOME/auth.json` | Active Codex credentials; defaults to `~/.codex/auth.json` |
| `~/.codex-switch/profiles/<alias>/auth.json` | Saved Codex profile |
| `~/.codex-switch/current` | Active Codex alias marker |
| `~/.codex-switch/cache/usage.json` | Local usage and activity cache |
| `~/.codex-switch/recovery/codex/` | Pre-switch Codex snapshots |
| `~/.codex-switch/trash/codex/` | Recoverable Codex deletions |
| `~/.codex-switch/claude-desktop/` | Claude profiles, current marker, recovery, and trash |
| `~/.codex-switch/app.log` | Local application log; tokens are not logged |

Claude Desktop live Cookies paths:

- macOS: `~/Library/Application Support/Claude/Cookies`
- Windows: `%APPDATA%\Claude\Cookies`

## Quality checks

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

The isolated analysis and quota changes pass 92 tests covering identity parsing, imports, permissions, recoverable deletion, transactional switching, detail preview, Claude SQLite handling, usage windows, reset credits, and activity parsing.

## Security and release boundary

- Credentials and snapshots are local files. Unix data directories target `0700`; credentials, locks, and temporary files are created with `0600`.
- The application sends the current OAuth token and account ID only to the relevant ChatGPT endpoints for usage queries.
- There is no project-operated cloud backend, telemetry, or credential synchronization.
- Automatic prereleases include Windows x64/x86 ZIPs, `SHA256SUMS.txt` and `BUILD_INFO.json`. Each release states macOS package availability and signing status; missing signing configuration does not replace the historical company-signed package.
- ChatGPT `wham` endpoints are not public stable APIs and may change.

## Documentation

- [Changelog](CHANGELOG.md)
- [Architecture and data flow](docs/ARCHITECTURE.md)
- [Build and release](docs/RELEASE.md)
- [Security policy](SECURITY.md)
- [Third-party notices](THIRD_PARTY_NOTICES.md)

## License

The project is licensed under the [MIT License](LICENSE). The About dialog includes the Slint attribution required by its royalty-free license. Third-party licenses, purposes, and source URLs are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and are copied into macOS and Windows packages.
