# Security Policy

[English](SECURITY.md) | [简体中文](SECURITY.zh-CN.md) | [繁體中文](SECURITY.zh-TW.md)

## Supported scope

| Version or artifact | Status |
| --- | --- |
| Current `0.1.0` source | Security fixes accepted |
| Unsigned local `dist/` builds | Local validation only |
| Older snapshots or third-party repackaging | Unsupported |

The project does not yet publish signed, notarized, or Windows code-signed releases. Do not treat local unsigned artifacts as trusted public distributions.

## Reporting a vulnerability

Use a GitHub Private Security Advisory when available. Otherwise contact the maintainers privately. Do not place real credentials in public issues, screenshots, or logs.

Include:

- Affected revision, operating system, and architecture.
- Minimal reproduction steps and expected/actual results.
- Redacted paths and logs, or synthetic test credentials.
- The impact, such as credential overwrite, unauthorized reads, path traversal, or failure to terminate.

Never attach a real `auth.json`, OAuth token, API key, Claude Cookies database, Keychain data, or full home-directory path.

## Sensitive local data

- Codex access tokens, refresh tokens, account IDs, and API keys in `auth.json`.
- Claude Desktop Chromium Cookies databases and recovery snapshots.
- Profiles, current markers, recovery data, trash, usage cache, and logs under `CODEX_SWITCH_HOME`.

## Implemented protections

### Files and paths

- Unix data directories target `0700`; credentials, lock files, and temporary files are created with `0600`.
- Aliases allow ASCII letters, digits, `.`, `_`, and `-`, cannot start with `.`, and are limited to 64 characters.
- `auth.json` imports are limited to 2 MiB and must contain a supported OAuth or API-key structure.
- Writes use unique temporary files, synchronization, and atomic replacement.
- Instance and per-provider file locks prevent concurrent replacement.

### Codex switching

- The target profile is revalidated while locked, and the active file is snapshotted before replacement.
- Refreshed tokens are written back only when live and saved identities match.
- A current-marker failure attempts to restore the previous live file.
- Invalid profiles cannot switch or trigger restart.
- Deletion moves data to private recoverable trash.

### Claude Desktop switching

- Active Cookies are captured through the SQLite Backup API.
- `quick_check` and Cookies-table validation run before save or switch.
- Claude Desktop is stopped, recovery data is saved, and WAL/journal sidecars are handled before replacement.
- Failures retain or restore recovery material; deletion is recoverable.

### Network and logs

- Usage requests use rustls TLS and send the current OAuth token and account ID to the relevant ChatGPT endpoints.
- The project has no cloud backend, telemetry, credential upload, or account-sync service.
- UI, tray, and application logs do not intentionally print tokens, API keys, or complete Cookies data.

## Explicit limitations

- Local OAuth/JWT parsing does not verify server signatures, revocation, or scopes.
- Compromise of the OS user, credential store, or administrator account is outside the application boundary.
- ChatGPT `wham` endpoints are not public stable APIs.
- Claude Cookies remain bound to Chromium and operating-system encryption behavior.
- Unsigned builds do not provide publisher-identity assurance.

## Public-release security gates

1. Pass formatting, Clippy, all tests, and release builds.
2. Validate startup, switch, rollback, restart, and quit on target macOS and Windows systems.
3. Sign, notarize, staple, and Gatekeeper-check macOS artifacts.
4. Code-sign Windows artifacts and test x64/x86 with SmartScreen and malware scanners.
5. Publish SHA-256 values and label architectures clearly.
