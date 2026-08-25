# Codex Account Switch

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

Codex Account Switch is a native desktop utility for storing, previewing, switching, and recovering multiple local Codex and Claude Desktop accounts. It also shows Codex usage limits, reset times, and account activity insights. The project is maintained for internal GCSA use and is not an official OpenAI product.

Built with **Rust 1.97.1**, **Slint 1.17**, and bundled SQLite. The UI does not use a WebView, and the project has no account-sync server or telemetry service.

## Project status

- Version: `0.1.0`, unreleased.
- macOS: local arm64 App build and real-window validation completed; unsigned and not notarized.
- Windows: x64 and x86 artifacts build successfully and pass PE, SHA-256, and runtime-import checks; real Windows smoke testing is still required.
- Linux: not currently supported or release-tested.

## Features

### Codex accounts

- Import from the current `$CODEX_HOME/auth.json`, browser authorization, a file or directory, pasted text, or the clipboard.
- Generate aliases from email addresses, or rename, delete, and recover profiles manually.
- Switch `auth.json` atomically with file locking, recovery snapshots, token write-back, identity checks, and rollback.
- Switch only, or switch and restart the relevant ChatGPT/Codex desktop process.
- Store OAuth or API-key profiles. Subscription usage is available only for compatible ChatGPT OAuth accounts.

### Usage and activity

- Show five-hour and weekly usage windows, percentages, update times, and reset times for each account.
- Click an account card to preview its identity, usage, and activity on the left without changing the active account.
- Show cumulative and peak tokens, current and longest streaks, longest session, and recent activity.
- Provide a 52-week activity view with daily heatmap, weekly/cumulative trends, fast-mode and reasoning statistics, skill/chat counts, and frequently used plugins or skills.
- Keep the last successful activity snapshot when the activity endpoint temporarily fails.

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

1. Select **Codex** or **Claude Desktop** in the header.
2. Add an account through browser login, Save Current, path import, or pasted text.
3. Click a card to preview it. Use **Switch** or **Restart** only when you want to activate it.
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

The current baseline is 32 tests covering identity parsing, imports, permissions, recoverable deletion, transactional switching, detail preview, Claude SQLite handling, usage windows, and activity parsing.

## Security and release boundary

- Credentials and snapshots are local files. Unix data directories target `0700`; credentials, locks, and temporary files are created with `0600`.
- The application sends the current OAuth token and account ID only to the relevant ChatGPT endpoints for usage queries.
- There is no project-operated cloud backend, telemetry, or credential synchronization.
- Current macOS and Windows artifacts are local validation builds, not signed public releases.
- ChatGPT `wham` endpoints are not public stable APIs and may change.

## Documentation

- [Changelog](CHANGELOG.md)
- [Architecture and data flow](docs/ARCHITECTURE.md)
- [Build and release](docs/RELEASE.md)
- [Security policy](SECURITY.md)
- [Third-party notices](THIRD_PARTY_NOTICES.md)

## License

The project is licensed under the [MIT License](LICENSE). The About dialog includes the Slint attribution required by its royalty-free license. Third-party licenses, purposes, and source URLs are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and are copied into macOS and Windows packages.
