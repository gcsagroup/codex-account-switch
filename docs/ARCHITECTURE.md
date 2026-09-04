# Architecture and Data Flow

[English](ARCHITECTURE.md) | [简体中文](ARCHITECTURE.zh-CN.md) | [繁體中文](ARCHITECTURE.zh-TW.md)

## Overview

Codex Account Switch is a single-process native desktop application. Slint owns the window and system tray; Rust owns local credentials, SQLite operations, network queries, switching transactions, and background refresh. There is no project-operated backend.

```text
Slint window / system tray
            |
            v
        src/main.rs
         |       |
         |       +-- Codex profile / switcher / login / usage
         +---------- Claude profile / SQLite switcher
            |
            v
Local files, ChatGPT endpoints, Codex/Claude desktop processes
```

## Modules

- `ui/app.slint`: window, tray, language menu, imports, account cards, usage, and activity insights.
- `src/main.rs`: lifecycle, UI callbacks, shared state, background refresh, preview state, and coordinated exit.
- `src/identity.rs`: `auth.json`, OAuth/JWT claims, API-key parsing, and identity comparison.
- `src/profile.rs`: Codex profile storage, alias validation, atomic writes, permissions, trash, and recovery.
- `src/switcher.rs`: save/adopt/switch flows, token write-back, snapshots, and rollback.
- `src/login.rs`: start/cancel `codex login`, detect live-file changes, and enforce login timeout.
- `src/usage.rs`: usage/activity requests, window classification, reset times, cache, and formatting.
- `src/claude.rs`: Claude Cookies profiles, SQLite backup/validation, switching, restart, trash, and recovery.
- `src/restart.rs`: stop and relaunch ChatGPT/Codex desktop applications on macOS and Windows.
- `src/paths.rs`: platform live paths plus `CODEX_HOME` and `CODEX_SWITCH_HOME`.
- `src/i18n.rs`: seven UI languages, system-language detection, and translation coverage.

## Codex import

1. Read from `codex login`, the live file, a selected file/directory, text, or clipboard.
2. Enforce the size limit, remove an optional Markdown JSON fence, and parse credentials.
3. Extract identity metadata and generate or validate the alias.
4. Atomically write `profiles/<alias>/auth.json` with private permissions.
5. Browser/external login adopts the live identity and updates the current marker without overwriting another account.

## Codex switch transaction

1. Acquire `auth.lock` and revalidate the target profile.
2. Read the live file and create a snapshot under `recovery/codex/`.
3. If live and current identities match, write refreshed tokens back to the current profile.
4. Reject an identity mismatch instead of overwriting the saved current profile.
5. Atomically replace live `auth.json`, then update the current marker.
6. Restore the previous live file if the current-marker update fails.
7. Relaunch the relevant desktop process only when the user selected Restart.

## Claude switch transaction

1. Acquire `cookies.lock` and validate the target SQLite profile.
2. Stop Claude Desktop.
3. Use the SQLite Backup API to snapshot active Cookies.
4. Handle sidecars and replace live Cookies.
5. Update the current marker and relaunch Claude Desktop.
6. Restore the prior database on replacement failure; deletions move to private trash.

## Usage and activity refresh

- `wham/usage` supplies plan, credits, available reset counts, and limit windows.
- `wham/profiles/me` supplies token totals, streaks, session duration, daily/weekly/cumulative buckets, reasoning, skills, chats, and plugin/skill rankings.
- Window duration classifies short and weekly limits; relative reset values become local absolute times.
- Results are atomically cached in `cache/usage.json`; errors stay in memory/UI and do not contain tokens.
- A successful usage request does not erase the last activity snapshot when the activity request fails.
- The UI fills the daily series to a 52-by-7 heatmap and derives weekly/cumulative trend points.
- Single-flight refresh prevents duplicate work. The active account refreshes every minute; all accounts refresh periodically and on demand.

## Concurrency and lifecycle

- `instance.lock` prevents concurrent application instances.
- `Arc<Mutex<AppShared>>` protects shared state; background results return through the Slint event loop.
- Clicking a card changes only the preview alias. Switching credentials requires an explicit Switch or Restart action.
- Close, in-app Quit, and tray Quit share one shutdown path that stops refresh/login work and exits the event loop.

## Trust boundaries

- The local filesystem and current OS user are the primary trust boundary.
- ChatGPT/Claude services, OAuth/JWT content, and desktop-client behavior are third-party dependencies.
- The application validates local structure and consistency but does not verify server signatures or revocation.
- Unsigned artifacts do not provide publisher identity.
