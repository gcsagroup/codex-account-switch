# Third-party software notices

[English](THIRD_PARTY_NOTICES.md) | [简体中文](THIRD_PARTY_NOTICES.zh-CN.md) | [繁體中文](THIRD_PARTY_NOTICES.zh-TW.md)

Codex Account Switch uses the open-source and third-party components listed below. Versions are taken from the current `Cargo.lock`. Links point to the relevant project or source repository.

## Attribution shown inside the application

### Slint 1.17.1

- Purpose: native desktop user-interface framework.
- Project and source: [slint.dev](https://slint.dev), [github.com/slint-ui/slint](https://github.com/slint-ui/slint)
- License used by this application: `LicenseRef-Slint-Royalty-free-2.0`.
- Requirement: this license requires either the official Slint `AboutSlint` component in a top-level, accessible About screen or the Slint attribution badge on the public download page. This application uses the in-app `AboutSlint` component.
- License text: [Slint Royalty-free License 2.0](https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md)

## Main dependency notices retained with distributions

| Component | Current version | Purpose | License | Project or source URL |
| --- | --- | --- | --- | --- |
| arboard | 3.6.1 | Clipboard access | MIT OR Apache-2.0 | https://github.com/1Password/arboard |
| base64 | 0.22.1 | Account token parsing | MIT OR Apache-2.0 | https://github.com/marshallpierce/rust-base64 |
| chrono | 0.4.45 | Time and reset-time formatting | MIT OR Apache-2.0 | https://github.com/chronotope/chrono |
| fs4 | 0.13.1 | File locking | MIT OR Apache-2.0 | https://github.com/al8n/fs4-rs |
| reqwest | 0.12.28 | HTTPS requests | MIT OR Apache-2.0 | https://github.com/seanmonstar/reqwest |
| rustls | 0.23.43 | TLS implementation | Apache-2.0 OR ISC OR MIT | https://github.com/rustls/rustls |
| rfd | 0.15.4 | Native file picker | MIT | https://github.com/PolyMeilex/rfd |
| rusqlite | 0.40.2 | Claude Desktop SQLite data access | MIT | https://github.com/rusqlite/rusqlite |
| libsqlite3-sys | 0.38.2 | Bundled SQLite bindings and source | MIT; SQLite core is Public Domain | https://github.com/rusqlite/rusqlite |
| serde / serde_json | 1.0.229 / 1.0.151 | Data serialization | MIT OR Apache-2.0 | https://serde.rs / https://github.com/serde-rs/json |
| thiserror | 2.0.19 | Error types | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| windows-sys | 0.61.2 | Windows system APIs | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |

Cargo resolves additional transitive dependencies for different platforms. Refer to `Cargo.lock` for the exact resolved versions and to each source package's `LICENSE`, `LICENSE-*`, `COPYING`, or `NOTICE` files for the full license text. This document records attribution and provenance; it does not modify any third-party license terms.
