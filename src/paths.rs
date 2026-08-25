use std::path::PathBuf;

/// Live Codex home: `$CODEX_HOME` or platform default `~/.codex`.
pub fn codex_home() -> PathBuf {
    if let Ok(home) = std::env::var("CODEX_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    dirs_home().join(".codex")
}

pub fn live_auth_path() -> PathBuf {
    codex_home().join("auth.json")
}

pub fn claude_desktop_cookies_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs_home()
            .join("Library")
            .join("Application Support")
            .join("Claude")
            .join("Cookies")
    }
    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs_home().join("AppData").join("Roaming"));
        app_data.join("Claude").join("Cookies")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        dirs_home().join(".config").join("Claude").join("Cookies")
    }
}

/// App data root: `$CODEX_SWITCH_HOME` or `~/.codex-switch`.
pub fn app_home() -> PathBuf {
    if let Ok(home) = std::env::var("CODEX_SWITCH_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    dirs_home().join(".codex-switch")
}

pub fn usage_cache_path() -> PathBuf {
    app_home().join("cache").join("usage.json")
}

fn dirs_home() -> PathBuf {
    if let Some(h) = std::env::var_os("HOME") {
        return PathBuf::from(h);
    }
    if let Some(h) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(h);
    }
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_codex_home_env() {
        let _guard = EnvGuard::set("CODEX_HOME", "/tmp/custom-codex");
        assert_eq!(
            live_auth_path(),
            PathBuf::from("/tmp/custom-codex/auth.json")
        );
    }

    struct EnvGuard {
        key: &'static str,
        prev: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
