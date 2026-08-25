use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(target_os = "macos")]
use std::thread;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

use fs4::fs_std::FileExt;
use rusqlite::{Connection, OpenFlags, MAIN_DB};

use crate::error::{AppError, AppResult};
use crate::identity::AccountIdentity;
use crate::paths;
use crate::profile::{self, Profile};

const COOKIES_FILE: &str = "Cookies";
const MAX_COOKIE_DB_BYTES: u64 = 256 * 1024 * 1024;
static CLAUDE_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct ClaudeProfileStore {
    root: PathBuf,
}

impl ClaudeProfileStore {
    pub fn default_store() -> Self {
        Self::new(paths::app_home().join("claude-desktop"))
    }

    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn ensure_dirs(&self) -> AppResult<()> {
        profile::ensure_private_dir(&self.root)?;
        profile::ensure_private_dir(&self.profiles_dir())?;
        profile::ensure_private_dir(&self.trash_dir())?;
        profile::ensure_private_dir(&self.recovery_dir())?;
        profile::ensure_private_dir(&self.temp_dir())?;
        Ok(())
    }

    fn profiles_dir(&self) -> PathBuf {
        self.root.join("profiles")
    }

    fn current_marker(&self) -> PathBuf {
        self.root.join("current")
    }

    fn lock_path(&self) -> PathBuf {
        self.root.join("cookies.lock")
    }

    fn trash_dir(&self) -> PathBuf {
        self.root.join("trash")
    }

    fn recovery_dir(&self) -> PathBuf {
        self.root.join("recovery")
    }

    fn temp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }

    fn last_deleted_marker(&self) -> PathBuf {
        self.root.join("last-deleted")
    }

    fn profile_path(&self, alias: &str) -> PathBuf {
        self.profiles_dir().join(alias).join(COOKIES_FILE)
    }

    pub fn read_current_alias(&self) -> Option<String> {
        fs::read_to_string(self.current_marker())
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn write_current_alias(&self, alias: &str) -> AppResult<()> {
        self.ensure_dirs()?;
        profile::atomic_write(&self.current_marker(), alias.as_bytes())
    }

    fn clear_current_alias(&self) -> AppResult<()> {
        let path = self.current_marker();
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn list(&self) -> AppResult<Vec<Profile>> {
        self.ensure_dirs()?;
        let current = self.read_current_alias();
        let mut profiles = Vec::new();
        for entry in fs::read_dir(self.profiles_dir())? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let alias = entry.file_name().to_string_lossy().to_string();
            if profile::normalize_alias(&alias).is_err() {
                continue;
            }
            let path = entry.path().join(COOKIES_FILE);
            if !path.is_file() {
                continue;
            }
            let valid = validate_cookie_db_path(&path).is_ok();
            profiles.push(Profile {
                is_current: current.as_deref() == Some(alias.as_str()),
                alias,
                is_valid: valid,
                identity: AccountIdentity {
                    email: if valid {
                        String::new()
                    } else {
                        "(无效 Claude Desktop Cookies)".into()
                    },
                    plan: "Claude Desktop".into(),
                    ..AccountIdentity::default()
                },
            });
        }
        profiles.sort_by(|a, b| a.alias.cmp(&b.alias));
        Ok(profiles)
    }

    fn save_database(&self, alias: &str, source: &Path) -> AppResult<()> {
        let alias = profile::normalize_alias(alias)?;
        validate_cookie_db_path(source)?;
        self.ensure_dirs()?;
        let dir = self.profiles_dir().join(alias);
        profile::ensure_private_dir(&dir)?;
        let bytes = sqlite_backup_bytes(source, &self.temp_dir())?;
        profile::atomic_write(&dir.join(COOKIES_FILE), &bytes)?;
        validate_cookie_db_path(&dir.join(COOKIES_FILE))
    }

    fn read_bytes(&self, alias: &str) -> AppResult<Vec<u8>> {
        let alias = profile::normalize_alias(alias)?;
        let path = self.profile_path(&alias);
        if !path.is_file() {
            return Err(AppError::msg(format!("Claude Desktop 账号不存在: {alias}")));
        }
        validate_cookie_db_path(&path).map_err(|error| {
            AppError::msg(format!("Claude Desktop 账号数据已损坏: {alias}: {error}"))
        })?;
        let bytes = fs::read(path)?;
        Ok(bytes)
    }

    pub fn import_file(&self, path: &Path, alias: Option<&str>) -> AppResult<String> {
        let alias = alias
            .filter(|a| !a.trim().is_empty())
            .ok_or_else(|| AppError::msg("导入 Claude Desktop Cookies 时必须填写别名"))?;
        let alias = profile::normalize_alias(alias)?;
        let alias = self.unique_alias_from(&alias)?;
        self.save_database(&alias, path)?;
        Ok(alias)
    }

    pub fn import_bytes(&self, _bytes: &[u8], _alias: Option<&str>) -> AppResult<String> {
        Err(AppError::msg(
            "Claude Desktop 不支持粘贴导入；请使用「保存当前」",
        ))
    }

    pub fn rename(&self, from: &str, to: &str) -> AppResult<()> {
        let from = profile::normalize_alias(from)?;
        let to = profile::normalize_alias(to)?;
        if from == to {
            return Ok(());
        }
        let from_dir = self.profiles_dir().join(&from);
        let to_dir = self.profiles_dir().join(&to);
        if !from_dir.join(COOKIES_FILE).is_file() {
            return Err(AppError::msg(format!("Claude Desktop 账号不存在: {from}")));
        }
        if to_dir.exists() {
            return Err(AppError::msg(format!("目标别名已存在: {to}")));
        }
        fs::rename(from_dir, to_dir)?;
        if self.read_current_alias().as_deref() == Some(from.as_str()) {
            if let Err(error) = self.write_current_alias(&to) {
                let _ = fs::rename(
                    self.profiles_dir().join(&to),
                    self.profiles_dir().join(&from),
                );
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn remove(&self, alias: &str) -> AppResult<()> {
        let alias = profile::normalize_alias(alias)?;
        let dir = self.profiles_dir().join(&alias);
        if !dir.exists() {
            return Err(AppError::msg(format!("Claude Desktop 账号不存在: {alias}")));
        }
        self.ensure_dirs()?;
        let trashed_name = format!("{}-{alias}", unique_file_stamp());
        let trashed_path = self.trash_dir().join(&trashed_name);
        fs::rename(&dir, &trashed_path)?;
        let marker = format!("{alias}\n{trashed_name}\n");
        if let Err(error) = profile::atomic_write(&self.last_deleted_marker(), marker.as_bytes()) {
            let _ = fs::rename(&trashed_path, &dir);
            return Err(error);
        }
        if self.read_current_alias().as_deref() == Some(alias.as_str()) {
            if let Err(error) = self.clear_current_alias() {
                let _ = fs::rename(&trashed_path, &dir);
                let _ = fs::remove_file(self.last_deleted_marker());
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn restore_last_removed(&self) -> AppResult<String> {
        let marker = fs::read_to_string(self.last_deleted_marker())?;
        let mut lines = marker.lines();
        let alias = profile::normalize_alias(lines.next().unwrap_or_default())?;
        let trashed_name = lines.next().unwrap_or_default();
        if trashed_name.is_empty()
            || !trashed_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return Err(AppError::msg("Claude 删除恢复记录已损坏"));
        }
        let source = self.trash_dir().join(trashed_name);
        let target = self.profiles_dir().join(&alias);
        if !source.is_dir() {
            return Err(AppError::msg("没有可恢复的 Claude Desktop 账号"));
        }
        if target.exists() {
            return Err(AppError::msg(format!("无法恢复，别名已存在: {alias}")));
        }
        fs::rename(source, target)?;
        fs::remove_file(self.last_deleted_marker())?;
        Ok(alias)
    }

    pub fn can_restore_last_removed(&self) -> bool {
        let Ok(marker) = fs::read_to_string(self.last_deleted_marker()) else {
            return false;
        };
        let mut lines = marker.lines();
        profile::normalize_alias(lines.next().unwrap_or_default()).is_ok()
            && lines
                .next()
                .filter(|name| {
                    !name.is_empty()
                        && name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
                })
                .map(|name| self.trash_dir().join(name).is_dir())
                .unwrap_or(false)
    }

    fn write_recovery_snapshot(&self, label: &str, bytes: &[u8]) -> AppResult<PathBuf> {
        self.ensure_dirs()?;
        let label = profile::normalize_alias(label).unwrap_or_else(|_| "live".to_string());
        let path = self
            .recovery_dir()
            .join(format!("{}-{label}.sqlite", unique_file_stamp()));
        profile::atomic_write(&path, bytes)?;
        Ok(path)
    }

    fn unique_alias(&self) -> AppResult<String> {
        self.unique_alias_from("claude")
    }

    fn unique_alias_from(&self, base: &str) -> AppResult<String> {
        let base = profile::normalize_alias(base)?;
        let mut candidate = base.to_string();
        for index in 2..=999 {
            if !self.profile_path(&candidate).is_file() {
                return Ok(candidate);
            }
            let suffix = format!("-{index}");
            let prefix_len = 64usize.saturating_sub(suffix.len());
            candidate = format!("{}{}", &base[..base.len().min(prefix_len)], suffix);
        }
        Err(AppError::msg("无法生成唯一 Claude Desktop 账号别名"))
    }
}

pub struct ClaudeSwitcher {
    store: ClaudeProfileStore,
    live_cookies: PathBuf,
}

impl ClaudeSwitcher {
    pub fn new(store: ClaudeProfileStore) -> Self {
        Self {
            store,
            live_cookies: paths::claude_desktop_cookies_path(),
        }
    }

    #[cfg(test)]
    fn with_live(store: ClaudeProfileStore, live_cookies: PathBuf) -> Self {
        Self {
            store,
            live_cookies,
        }
    }

    pub fn store(&self) -> &ClaudeProfileStore {
        &self.store
    }

    pub fn save_live_as(&self, alias: &str) -> AppResult<()> {
        let alias = profile::normalize_alias(alias)?;
        let _lock = self.lock()?;
        if self.store.profile_path(&alias).is_file() {
            return Err(AppError::msg(format!(
                "别名已存在，拒绝直接覆盖 Claude 会话: {alias}"
            )));
        }
        let was_running = claude_desktop_running();
        if was_running {
            quit_claude_desktop()?;
        }
        let result = self.save_live_stopped(&alias);
        if was_running {
            if let Err(open_error) = open_claude_desktop() {
                return match result {
                    Ok(()) => Err(AppError::msg(format!(
                        "会话已保存，但重新打开 Claude Desktop 失败: {open_error}"
                    ))),
                    Err(save_error) => Err(AppError::msg(format!(
                        "保存失败且重新打开 Claude Desktop 失败: {save_error}; {open_error}"
                    ))),
                };
            }
        }
        result
    }

    pub fn save_live_auto(&self, alias: Option<&str>) -> AppResult<String> {
        let alias = match alias.filter(|a| !a.trim().is_empty()) {
            Some(alias) => profile::normalize_alias(alias)?,
            None => self.store.unique_alias()?,
        };
        self.save_live_as(&alias)?;
        Ok(alias)
    }

    pub fn use_profile(&self, alias: &str) -> AppResult<()> {
        let alias = profile::normalize_alias(alias)?;
        let _lock = self.lock()?;
        // Validate the target before stopping Claude.
        let target = self.store.read_bytes(&alias)?;
        let was_running = claude_desktop_running();
        quit_claude_desktop()?;
        let result = self.use_profile_stopped(&alias, &target);
        let reopen = if was_running || result.is_ok() {
            open_claude_desktop()
        } else {
            Ok(())
        };
        match (result, reopen) {
            (Ok(()), Ok(())) => Ok(()),
            (Ok(()), Err(error)) => Err(AppError::msg(format!(
                "Claude 会话已切换，但应用重新打开失败: {error}"
            ))),
            (Err(error), Ok(())) => Err(error),
            (Err(error), Err(open_error)) => Err(AppError::msg(format!(
                "切换失败且 Claude Desktop 无法重新打开: {error}; {open_error}"
            ))),
        }
    }

    pub fn open_desktop(&self) -> AppResult<()> {
        open_claude_desktop()
    }

    fn save_live_stopped(&self, alias: &str) -> AppResult<()> {
        if !self.live_cookies.is_file() {
            return Err(AppError::msg(format!(
                "未找到 Claude Desktop Cookies {}",
                self.live_cookies.display()
            )));
        }
        recover_and_validate_live_database(&self.live_cookies).map_err(|e| {
            AppError::msg(format!(
                "Claude Desktop Cookies 无法安全备份 {}: {e}",
                self.live_cookies.display()
            ))
        })?;
        self.store.save_database(alias, &self.live_cookies)?;
        self.store.write_current_alias(alias)
    }

    fn use_profile_stopped(&self, alias: &str, target: &[u8]) -> AppResult<()> {
        let current = self.store.read_current_alias();
        if current.as_deref() == Some(alias) {
            return Ok(());
        }

        let live_before = if self.live_cookies.is_file() {
            recover_and_validate_live_database(&self.live_cookies)?;
            Some(sqlite_backup_bytes(
                &self.live_cookies,
                &self.store.temp_dir(),
            )?)
        } else {
            None
        };
        if let Some(bytes) = live_before.as_deref() {
            self.store
                .write_recovery_snapshot(current.as_deref().unwrap_or("untracked-live"), bytes)?;
        }
        if let (Some(current), Some(_)) = (current.as_deref(), live_before.as_deref()) {
            if self.store.profile_path(current).is_file() {
                self.store.save_database(current, &self.live_cookies)?;
            }
        }

        remove_cookie_sidecars(&self.live_cookies)?;
        profile::atomic_write(&self.live_cookies, target)?;
        if let Err(marker_error) = self.store.write_current_alias(alias) {
            rollback_live_database(&self.live_cookies, live_before.as_deref())?;
            return Err(AppError::msg(format!(
                "更新 Claude current 失败，已恢复原 Cookies: {marker_error}"
            )));
        }
        validate_cookie_db_path(&self.live_cookies)
    }

    fn lock(&self) -> AppResult<File> {
        self.store.ensure_dirs()?;
        let file = profile::open_private_lock(&self.store.lock_path())?;
        file.lock_exclusive()
            .map_err(|e| AppError::msg(format!("获取 Claude Desktop 凭据锁失败: {e}")))?;
        Ok(file)
    }
}

fn validate_cookie_db_path(path: &Path) -> AppResult<()> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_COOKIE_DB_BYTES {
        return Err(AppError::msg(
            "Claude Cookies 数据库为空或超过 256 MiB 限制",
        ));
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    verify_cookie_connection(&connection)
}

fn recover_and_validate_live_database(path: &Path) -> AppResult<()> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    verify_cookie_connection(&connection)
}

fn verify_cookie_connection(connection: &Connection) -> AppResult<()> {
    let quick_check: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if quick_check != "ok" {
        return Err(AppError::msg(format!(
            "Claude Cookies SQLite quick_check 失败: {quick_check}"
        )));
    }
    let has_cookies: i64 = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='cookies')",
        [],
        |row| row.get(0),
    )?;
    if has_cookies != 1 {
        return Err(AppError::msg("SQLite 文件不包含 cookies 表"));
    }
    Ok(())
}

fn sqlite_backup_bytes(source: &Path, temp_dir: &Path) -> AppResult<Vec<u8>> {
    profile::ensure_private_dir(temp_dir)?;
    validate_cookie_db_path(source)?;
    let temp_path = temp_dir.join(format!("cookies-backup-{}.sqlite", unique_file_stamp()));
    let result = (|| -> AppResult<Vec<u8>> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        drop(options.open(&temp_path)?);
        let connection = Connection::open_with_flags(
            source,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.backup(MAIN_DB, &temp_path, None)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600))?;
        }
        validate_cookie_db_path(&temp_path)?;
        Ok(fs::read(&temp_path)?)
    })();
    let _ = fs::remove_file(&temp_path);
    result
}

fn remove_cookie_sidecars(path: &Path) -> AppResult<()> {
    for suffix in ["journal", "wal", "shm"] {
        let sidecar = PathBuf::from(format!("{}-{suffix}", path.display()));
        if sidecar.exists() {
            let len = fs::metadata(&sidecar)?.len();
            if len > 0 {
                return Err(AppError::msg(format!(
                    "拒绝删除非空 SQLite sidecar: {} ({len} bytes)",
                    sidecar.display()
                )));
            }
            fs::remove_file(sidecar)?;
        }
    }
    Ok(())
}

fn rollback_live_database(path: &Path, previous: Option<&[u8]>) -> AppResult<()> {
    match previous {
        Some(bytes) => profile::atomic_write(path, bytes),
        None if path.exists() => {
            fs::remove_file(path)?;
            Ok(())
        }
        None => Ok(()),
    }
}

fn unique_file_stamp() -> String {
    format!(
        "{}-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        CLAUDE_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(target_os = "macos")]
fn claude_desktop_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "Claude"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn claude_desktop_running() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn quit_claude_desktop() -> AppResult<()> {
    if !claude_desktop_running() {
        return Ok(());
    }
    let _ = Command::new("osascript")
        .args(["-e", "tell application \"Claude\" to quit"])
        .status();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(15) {
        if !claude_desktop_running() {
            thread::sleep(Duration::from_millis(500));
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    let _ = Command::new("killall").arg("Claude").status();
    thread::sleep(Duration::from_secs(1));
    if claude_desktop_running() {
        Err(AppError::msg("无法退出 Claude Desktop，请手动退出后重试"))
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
fn quit_claude_desktop() -> AppResult<()> {
    Err(AppError::msg("当前系统暂不支持自动切换 Claude Desktop"))
}

#[cfg(target_os = "macos")]
fn open_claude_desktop() -> AppResult<()> {
    let status = Command::new("open")
        .args(["-a", "Claude"])
        .status()
        .map_err(|e| AppError::msg(format!("无法打开 Claude Desktop: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::msg("无法打开 Claude Desktop"))
    }
}

#[cfg(not(target_os = "macos"))]
fn open_claude_desktop() -> AppResult<()> {
    Err(AppError::msg("当前系统暂不支持打开 Claude Desktop"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_cookie_db(path: &Path, marker: i64) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch("CREATE TABLE cookies (id INTEGER PRIMARY KEY, value INTEGER NOT NULL);")
            .unwrap();
        connection
            .execute("INSERT INTO cookies(value) VALUES (?1)", [marker])
            .unwrap();
    }

    fn cookie_marker(path: &Path) -> i64 {
        let connection = Connection::open(path).unwrap();
        connection
            .query_row("SELECT value FROM cookies LIMIT 1", [], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn stores_lists_renames_and_removes_profiles() {
        let dir = tempdir().unwrap();
        let store = ClaudeProfileStore::new(dir.path().to_path_buf());
        let source = dir.path().join("source.sqlite");
        create_cookie_db(&source, 1);
        store.save_database("work", &source).unwrap();
        store.write_current_alias("work").unwrap();

        let profiles = store.list().unwrap();
        assert_eq!(profiles.len(), 1);
        assert!(profiles[0].is_current);
        assert_eq!(profiles[0].identity.plan, "Claude Desktop");

        store.rename("work", "home").unwrap();
        assert_eq!(store.read_current_alias().as_deref(), Some("home"));
        assert!(!store.can_restore_last_removed());
        store.remove("home").unwrap();
        assert!(store.can_restore_last_removed());
        assert!(store.list().unwrap().is_empty());
        assert_eq!(store.restore_last_removed().unwrap(), "home");
        assert!(!store.can_restore_last_removed());
        assert_eq!(cookie_marker(&store.profile_path("home")), 1);
    }

    #[test]
    fn switches_and_writes_back_cookie_database() {
        let dir = tempdir().unwrap();
        let live = dir.path().join("Cookies");
        create_cookie_db(&live, 3);

        let store = ClaudeProfileStore::new(dir.path().join("store"));
        let source_a = dir.path().join("a.sqlite");
        let source_b = dir.path().join("b.sqlite");
        create_cookie_db(&source_a, 1);
        create_cookie_db(&source_b, 2);
        store.save_database("a", &source_a).unwrap();
        store.save_database("b", &source_b).unwrap();
        store.write_current_alias("a").unwrap();

        let switcher = ClaudeSwitcher::with_live(store, live.clone());
        let target = switcher.store.read_bytes("b").unwrap();
        switcher.use_profile_stopped("b", &target).unwrap();

        assert_eq!(cookie_marker(&live), 2);
        assert_eq!(cookie_marker(&switcher.store.profile_path("a")), 3);
        assert_eq!(switcher.store.read_current_alias().as_deref(), Some("b"));
    }

    #[test]
    fn rejects_non_sqlite_import() {
        let dir = tempdir().unwrap();
        let store = ClaudeProfileStore::new(dir.path().to_path_buf());
        let bad = dir.path().join("bad.sqlite");
        fs::write(&bad, b"not sqlite").unwrap();
        assert!(store.save_database("bad", &bad).is_err());
    }

    #[test]
    fn refuses_to_overwrite_existing_cookie_profile() {
        let dir = tempdir().unwrap();
        let live = dir.path().join("live/Cookies");
        create_cookie_db(&live, 2);
        let store = ClaudeProfileStore::new(dir.path().join("store"));
        let existing = dir.path().join("existing.sqlite");
        create_cookie_db(&existing, 1);
        store.save_database("work", &existing).unwrap();
        store.write_current_alias("work").unwrap();
        let switcher = ClaudeSwitcher::with_live(store, live);

        assert!(switcher.save_live_as("work").is_err());
        assert_eq!(cookie_marker(&switcher.store.profile_path("work")), 1);
    }
}
