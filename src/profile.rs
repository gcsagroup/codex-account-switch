use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AppError, AppResult};
use crate::identity::{self, AccountIdentity, AuthCredentials};
use crate::paths;

const MAX_AUTH_BYTES: usize = 2 * 1024 * 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct Profile {
    pub alias: String,
    pub identity: AccountIdentity,
    pub is_current: bool,
    pub is_valid: bool,
}

pub struct ProfileStore {
    root: PathBuf,
}

impl ProfileStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn default_store() -> Self {
        Self::new(paths::app_home())
    }

    pub fn ensure_dirs(&self) -> AppResult<()> {
        ensure_private_dir(&self.root)?;
        ensure_private_dir(&self.profiles_dir())?;
        ensure_private_dir(&self.root.join("cache"))?;
        ensure_private_dir(&self.recovery_dir())?;
        ensure_private_dir(&self.trash_dir())?;
        Ok(())
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.root.join("profiles")
    }

    pub fn current_marker(&self) -> PathBuf {
        self.root.join("current")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.root.join("auth.lock")
    }

    fn recovery_dir(&self) -> PathBuf {
        self.root.join("recovery").join("codex")
    }

    fn trash_dir(&self) -> PathBuf {
        self.root.join("trash").join("codex")
    }

    fn last_deleted_marker(&self) -> PathBuf {
        self.root.join("last-deleted-codex")
    }

    pub fn profile_auth_path(&self, alias: &str) -> PathBuf {
        self.profiles_dir().join(alias).join("auth.json")
    }

    pub fn read_current_alias(&self) -> Option<String> {
        fs::read_to_string(self.current_marker())
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn write_current_alias(&self, alias: &str) -> AppResult<()> {
        self.ensure_dirs()?;
        atomic_write(self.current_marker().as_path(), alias.as_bytes())?;
        Ok(())
    }

    pub fn clear_current_alias(&self) -> AppResult<()> {
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
        let dir = self.profiles_dir();
        if !dir.exists() {
            return Ok(profiles);
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let alias = entry.file_name().to_string_lossy().to_string();
            if !is_safe_alias(&alias) {
                continue;
            }
            let auth_path = entry.path().join("auth.json");
            if !auth_path.is_file() {
                continue;
            }
            let bytes = fs::read(&auth_path)?;
            let parsed = identity::parse_auth_bytes(&bytes);
            let is_valid = parsed.is_ok();
            let creds = parsed.unwrap_or_else(|_| AuthCredentials {
                identity: AccountIdentity {
                    email: "(无效 auth.json)".into(),
                    ..AccountIdentity::default()
                },
                access_token: None,
                account_id: None,
            });
            profiles.push(Profile {
                is_current: current.as_deref() == Some(alias.as_str()),
                alias,
                identity: creds.identity,
                is_valid,
            });
        }
        profiles.sort_by(|a, b| a.alias.cmp(&b.alias));
        Ok(profiles)
    }

    pub fn save_bytes(&self, alias: &str, bytes: &[u8]) -> AppResult<()> {
        let alias = normalize_alias(alias)?;
        validate_auth_json(bytes)?;
        self.ensure_dirs()?;
        let dir = self.profiles_dir().join(&alias);
        ensure_private_dir(&dir)?;
        atomic_write(&dir.join("auth.json"), bytes)?;
        Ok(())
    }

    pub fn import_file(&self, path: &Path, alias: Option<&str>) -> AppResult<String> {
        let bytes = fs::read(path)?;
        self.import_bytes(&bytes, alias)
    }

    /// Import auth.json content from raw bytes / pasted text.
    pub fn import_bytes(&self, bytes: &[u8], alias: Option<&str>) -> AppResult<String> {
        if bytes.len() > MAX_AUTH_BYTES {
            return Err(AppError::msg("auth.json 超过 2 MiB 限制"));
        }
        let text =
            std::str::from_utf8(bytes).map_err(|_| AppError::msg("粘贴内容不是合法 UTF-8 文本"))?;
        let cleaned = strip_json_fences(text);
        let bytes = cleaned.as_bytes();
        let creds = identity::parse_auth_bytes(bytes)?;
        let base = match alias {
            Some(a) if !a.trim().is_empty() => normalize_alias(a)?,
            _ => alias_base_from_email(&creds.identity.email),
        };
        let alias = unique_alias_from_base(self, &base)?;
        self.save_bytes(&alias, bytes)?;
        Ok(alias)
    }

    pub fn rename(&self, from: &str, to: &str) -> AppResult<()> {
        let from = normalize_alias(from)?;
        let to = normalize_alias(to)?;
        if from == to {
            return Ok(());
        }
        let from_dir = self.profiles_dir().join(&from);
        let to_dir = self.profiles_dir().join(&to);
        if !from_dir.join("auth.json").is_file() {
            return Err(AppError::msg(format!("账号不存在: {from}")));
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
        let alias = normalize_alias(alias)?;
        let dir = self.profiles_dir().join(&alias);
        if !dir.exists() {
            return Err(AppError::msg(format!("账号不存在: {alias}")));
        }
        self.ensure_dirs()?;
        let trashed_name = format!("{}-{}-{alias}", now_nanos(), next_counter());
        let trashed_path = self.trash_dir().join(&trashed_name);
        fs::rename(&dir, &trashed_path)?;
        let marker = format!("{alias}\n{trashed_name}\n");
        if let Err(error) = atomic_write(&self.last_deleted_marker(), marker.as_bytes()) {
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
        let alias = normalize_alias(lines.next().unwrap_or_default())?;
        let trashed_name = lines.next().unwrap_or_default();
        if !is_safe_leaf_name(trashed_name) {
            return Err(AppError::msg("删除恢复记录已损坏"));
        }
        let source = self.trash_dir().join(trashed_name);
        let target = self.profiles_dir().join(&alias);
        if !source.is_dir() {
            return Err(AppError::msg("没有可恢复的 Codex 账号"));
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
        normalize_alias(lines.next().unwrap_or_default()).is_ok()
            && lines
                .next()
                .filter(|name| is_safe_leaf_name(name))
                .map(|name| self.trash_dir().join(name).is_dir())
                .unwrap_or(false)
    }

    pub fn write_recovery_snapshot(&self, label: &str, bytes: &[u8]) -> AppResult<PathBuf> {
        self.ensure_dirs()?;
        let label = normalize_alias(label).unwrap_or_else(|_| "live".to_string());
        let path =
            self.recovery_dir()
                .join(format!("{}-{}-{label}.json", now_nanos(), next_counter()));
        atomic_write(&path, bytes)?;
        Ok(path)
    }

    pub fn read_profile_bytes(&self, alias: &str) -> AppResult<Vec<u8>> {
        let alias = normalize_alias(alias)?;
        let path = self.profile_auth_path(&alias);
        if !path.is_file() {
            return Err(AppError::msg(format!("账号不存在: {alias}")));
        }
        let bytes = fs::read(path)?;
        validate_auth_json(&bytes)?;
        Ok(bytes)
    }

    pub fn read_profile_creds(&self, alias: &str) -> AppResult<AuthCredentials> {
        let bytes = self.read_profile_bytes(alias)?;
        identity::parse_auth_bytes(&bytes)
    }
}

pub fn normalize_alias(alias: &str) -> AppResult<String> {
    let alias = alias.trim();
    if alias.is_empty() {
        return Err(AppError::msg("别名不能为空"));
    }
    if !is_safe_alias(alias) {
        return Err(AppError::msg("别名仅允许字母、数字、._-，且不能以 . 开头"));
    }
    Ok(alias.to_string())
}

fn is_safe_alias(alias: &str) -> bool {
    if alias.is_empty() || alias.starts_with('.') || alias.len() > 64 {
        return false;
    }
    alias
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn alias_base_from_email(email: &str) -> String {
    let mut base = email
        .split('@')
        .next()
        .unwrap_or("account")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(56)
        .collect::<String>();
    if base.starts_with('.') {
        base.insert(0, '_');
    }
    if base.is_empty() {
        "account".to_string()
    } else {
        base
    }
}

fn unique_alias_from_base(store: &ProfileStore, base: &str) -> AppResult<String> {
    let base = normalize_alias(base)?;
    let mut candidate = base.clone();
    let mut i = 2;
    while store.profiles_dir().join(&candidate).exists() {
        let suffix = format!("-{i}");
        let prefix_len = 64usize.saturating_sub(suffix.len());
        candidate = format!("{}{}", &base[..base.len().min(prefix_len)], suffix);
        i += 1;
        if i > 999 {
            return Err(AppError::msg("无法生成唯一别名"));
        }
    }
    Ok(candidate)
}

fn validate_auth_json(bytes: &[u8]) -> AppResult<()> {
    if bytes.len() > MAX_AUTH_BYTES {
        return Err(AppError::msg("auth.json 超过 2 MiB 限制"));
    }
    identity::parse_auth_bytes(bytes)?;
    Ok(())
}

fn strip_json_fences(input: &str) -> String {
    let trimmed = input.trim().trim_start_matches('\u{feff}');
    let trimmed = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix("```").unwrap_or(trimmed);
    trimmed.trim().to_string()
}

pub fn atomic_write(path: &Path, data: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    let tmp = unique_temp_path(path)?;
    let result = (|| -> AppResult<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut f = options.open(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        drop(f);
        replace_file(&tmp, path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
            if let Some(parent) = path.parent() {
                fs::File::open(parent)?.sync_all()?;
            }
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

pub fn ensure_private_dir(path: &Path) -> AppResult<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

pub fn open_private_lock(path: &Path) -> AppResult<fs::File> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(file)
}

fn unique_temp_path(path: &Path) -> AppResult<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::msg("目标文件缺少父目录"))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::msg("目标文件名不是有效 UTF-8"))?;
    Ok(parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        next_counter()
    )))
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> AppResult<()> {
    fs::rename(from, to)?;
    Ok(())
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> AppResult<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let from_wide: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to_wide: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    let result = unsafe {
        MoveFileExW(
            from_wide.as_ptr(),
            to_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

fn next_counter() -> u64 {
    TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn is_safe_leaf_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_list_rename_remove() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().to_path_buf());
        let auth =
            br#"{"auth_mode":"chatgpt","tokens":{"account_id":"a1","access_token":"token-a"}}"#;
        store.save_bytes("work", auth).unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].alias, "work");
        store.rename("work", "home").unwrap();
        store.write_current_alias("home").unwrap();
        assert_eq!(store.read_current_alias().as_deref(), Some("home"));
        store.remove("home").unwrap();
        assert!(store.list().unwrap().is_empty());
        assert!(store.read_current_alias().is_none());
    }

    #[test]
    fn import_bytes_strips_markdown_fence() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().to_path_buf());
        let pasted = r#"```json
{"auth_mode":"chatgpt","tokens":{"account_id":"paste1","access_token":"token-paste"}}
```"#;
        let alias = store
            .import_bytes(pasted.as_bytes(), Some("from-paste"))
            .unwrap();
        assert_eq!(alias, "from-paste");
        assert!(store.profile_auth_path("from-paste").is_file());
    }

    #[test]
    fn rejects_invalid_auth_and_avoids_alias_overwrite() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().to_path_buf());
        assert!(store.import_bytes(br#"{}"#, Some("bad")).is_err());
        let auth = br#"{"tokens":{"account_id":"a1","access_token":"token-a"}}"#;
        assert_eq!(store.import_bytes(auth, Some("work")).unwrap(), "work");
        assert_eq!(store.import_bytes(auth, Some("work")).unwrap(), "work-2");
    }

    #[test]
    fn remove_is_recoverable() {
        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().to_path_buf());
        let auth = br#"{"tokens":{"account_id":"a1","access_token":"token-a"}}"#;
        store.save_bytes("work", auth).unwrap();
        assert!(!store.can_restore_last_removed());
        store.remove("work").unwrap();
        assert!(store.can_restore_last_removed());
        assert!(!store.profile_auth_path("work").exists());
        assert_eq!(store.restore_last_removed().unwrap(), "work");
        assert!(!store.can_restore_last_removed());
        assert!(store.profile_auth_path("work").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn private_files_and_directories_have_strict_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let store = ProfileStore::new(dir.path().join("store"));
        let auth = br#"{"tokens":{"account_id":"a1","access_token":"token-a"}}"#;
        store.save_bytes("work", auth).unwrap();
        let file_mode = fs::metadata(store.profile_auth_path("work"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let dir_mode = fs::metadata(store.profiles_dir())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(file_mode, 0o600);
        assert_eq!(dir_mode, 0o700);
    }
}
