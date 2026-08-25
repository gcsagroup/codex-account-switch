use std::fs;
use std::fs::File;

use fs4::fs_std::FileExt;

use crate::error::{AppError, AppResult};
use crate::identity;
use crate::profile::{self, ProfileStore};

pub struct Switcher {
    store: ProfileStore,
    live_auth: std::path::PathBuf,
}

impl Switcher {
    pub fn new(store: ProfileStore, live_auth: std::path::PathBuf) -> Self {
        Self { store, live_auth }
    }

    pub fn store(&self) -> &ProfileStore {
        &self.store
    }

    pub fn save_live_as(&self, alias: &str) -> AppResult<()> {
        let alias = profile::normalize_alias(alias)?;
        let _lock = self.lock()?;
        if !self.live_auth.is_file() {
            return Err(AppError::msg(format!(
                "未找到 live auth: {}",
                self.live_auth.display()
            )));
        }
        let bytes = fs::read(&self.live_auth)?;
        identity::parse_auth_bytes(&bytes)?;
        let existing = self.store.profile_auth_path(&alias).is_file();
        let current = self.store.read_current_alias();
        if existing && current.as_deref() != Some(alias.as_str()) {
            return Err(AppError::msg(format!(
                "别名已存在且不是当前账号，拒绝覆盖: {alias}"
            )));
        }
        if existing {
            let stored = self.store.read_profile_bytes(&alias)?;
            if !identity::same_account(&stored, &bytes)? {
                return Err(AppError::msg(format!(
                    "live 凭据与 {alias} 不属于同一账号，拒绝覆盖；请使用「导入当前」保存为新账号"
                )));
            }
        }
        self.store.save_bytes(&alias, &bytes)?;
        self.store.write_current_alias(&alias)?;
        Ok(())
    }

    /// Adopt credentials that were written to the live auth file by an
    /// external login flow. This keeps the profile and current marker in sync.
    pub fn adopt_live_profile(&self, alias: Option<&str>) -> AppResult<String> {
        let _lock = self.lock()?;
        if !self.live_auth.is_file() {
            return Err(AppError::msg(format!(
                "未找到 live auth: {}",
                self.live_auth.display()
            )));
        }
        let bytes = fs::read(&self.live_auth)?;
        identity::parse_auth_bytes(&bytes)?;
        let alias = self.store.import_bytes(&bytes, alias)?;
        self.store.write_current_alias(&alias)?;
        Ok(alias)
    }

    pub fn use_profile(&self, alias: &str) -> AppResult<()> {
        let alias = profile::normalize_alias(alias)?;
        let _lock = self.lock()?;

        // Validate the target while the lock is held and before touching live.
        let target = self.store.read_profile_bytes(&alias)?;
        identity::parse_auth_bytes(&target)?;
        let current = self.store.read_current_alias();
        if current.as_deref() == Some(alias.as_str()) {
            return Ok(());
        }

        let live_before = if self.live_auth.is_file() {
            Some(fs::read(&self.live_auth)?)
        } else {
            None
        };
        if let Some(bytes) = live_before.as_deref() {
            self.store
                .write_recovery_snapshot(current.as_deref().unwrap_or("untracked-live"), bytes)?;
        }

        // Write back refreshed tokens from live into the previously current profile.
        if let (Some(current), Some(live_bytes)) = (current.as_deref(), live_before.as_deref()) {
            if self.store.profile_auth_path(current).is_file() {
                let stored = self.store.read_profile_bytes(current)?;
                identity::parse_auth_bytes(live_bytes).map_err(|error| {
                    AppError::msg(format!("当前 live auth 无效，已停止切换: {error}"))
                })?;
                if !identity::same_account(&stored, live_bytes)? {
                    return Err(AppError::msg(format!(
                        "live 凭据与 current={current} 不属于同一账号；已保存恢复快照，拒绝覆盖旧 profile"
                    )));
                }
                self.store.save_bytes(current, live_bytes)?;
            }
        }

        if let Some(parent) = self.live_auth.parent() {
            profile::ensure_private_dir(parent)?;
        }
        profile::atomic_write(&self.live_auth, &target)?;
        if let Err(marker_error) = self.store.write_current_alias(&alias) {
            let rollback = match live_before.as_deref() {
                Some(bytes) => profile::atomic_write(&self.live_auth, bytes),
                None => fs::remove_file(&self.live_auth).map_err(AppError::from),
            };
            return match rollback {
                Ok(()) => Err(AppError::msg(format!(
                    "更新 current 失败，已恢复原 live 凭据: {marker_error}"
                ))),
                Err(rollback_error) => Err(AppError::msg(format!(
                    "更新 current 失败且 live 回滚失败: {marker_error}; {rollback_error}"
                ))),
            };
        }
        Ok(())
    }

    fn lock(&self) -> AppResult<File> {
        self.store.ensure_dirs()?;
        let path = self.store.lock_path();
        let file = profile::open_private_lock(&path)?;
        file.lock_exclusive()
            .map_err(|e| AppError::msg(format!("获取文件锁失败: {e}")))?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ProfileStore;
    use tempfile::tempdir;

    #[test]
    fn switches_and_writebacks() {
        let root = tempdir().unwrap();
        let live_dir = root.path().join("codex");
        fs::create_dir_all(&live_dir).unwrap();
        let live = live_dir.join("auth.json");
        fs::write(
            &live,
            br#"{"auth_mode":"chatgpt","tokens":{"account_id":"live","access_token":"token-live"}}"#,
        )
        .unwrap();

        let store = ProfileStore::new(root.path().join("switch"));
        store
            .save_bytes(
                "a",
                br#"{"auth_mode":"chatgpt","tokens":{"account_id":"a","access_token":"token-a"}}"#,
            )
            .unwrap();
        store
            .save_bytes(
                "b",
                br#"{"auth_mode":"chatgpt","tokens":{"account_id":"b","access_token":"token-b"}}"#,
            )
            .unwrap();
        store.write_current_alias("a").unwrap();

        // Simulate live belonging to a with refreshed marker
        fs::write(
            &live,
            br#"{"auth_mode":"chatgpt","tokens":{"account_id":"a","access_token":"token-a-refreshed"}}"#,
        )
        .unwrap();

        let switcher = Switcher::new(store, live.clone());
        switcher.use_profile("b").unwrap();

        let live_val: serde_json::Value =
            serde_json::from_slice(&fs::read(&live).unwrap()).unwrap();
        assert_eq!(live_val["tokens"]["account_id"], "b");

        let a_bytes = switcher.store().read_profile_bytes("a").unwrap();
        let a_val: serde_json::Value = serde_json::from_slice(&a_bytes).unwrap();
        assert_eq!(a_val["tokens"]["access_token"], "token-a-refreshed");
        assert_eq!(switcher.store().read_current_alias().as_deref(), Some("b"));
    }

    #[test]
    fn adopt_live_updates_current_without_overwriting_old_profile() {
        let root = tempdir().unwrap();
        let live = root.path().join("codex/auth.json");
        fs::create_dir_all(live.parent().unwrap()).unwrap();
        let old = br#"{"tokens":{"account_id":"old","access_token":"old-token"}}"#;
        let new = br#"{"tokens":{"account_id":"new","access_token":"new-token"}}"#;
        fs::write(&live, new).unwrap();
        let store = ProfileStore::new(root.path().join("switch"));
        store.save_bytes("old", old).unwrap();
        store.write_current_alias("old").unwrap();
        let switcher = Switcher::new(store, live);

        let alias = switcher.adopt_live_profile(Some("new")).unwrap();
        assert_eq!(alias, "new");
        assert_eq!(
            switcher.store().read_current_alias().as_deref(),
            Some("new")
        );
        assert_eq!(switcher.store().read_profile_bytes("old").unwrap(), old);
    }

    #[test]
    fn refuses_mismatched_live_without_overwriting_old_profile() {
        let root = tempdir().unwrap();
        let live = root.path().join("codex/auth.json");
        fs::create_dir_all(live.parent().unwrap()).unwrap();
        let old = br#"{"tokens":{"account_id":"old","access_token":"old-token"}}"#;
        let external = br#"{"tokens":{"account_id":"external","access_token":"external-token"}}"#;
        let target = br#"{"tokens":{"account_id":"target","access_token":"target-token"}}"#;
        fs::write(&live, external).unwrap();
        let store = ProfileStore::new(root.path().join("switch"));
        store.save_bytes("old", old).unwrap();
        store.save_bytes("target", target).unwrap();
        store.write_current_alias("old").unwrap();
        let switcher = Switcher::new(store, live.clone());

        assert!(switcher.use_profile("target").is_err());
        assert_eq!(fs::read(live).unwrap(), external);
        assert_eq!(switcher.store().read_profile_bytes("old").unwrap(), old);
    }

    #[test]
    fn invalid_target_never_replaces_live() {
        let root = tempdir().unwrap();
        let live = root.path().join("codex/auth.json");
        fs::create_dir_all(live.parent().unwrap()).unwrap();
        let valid = br#"{"tokens":{"account_id":"old","access_token":"old-token"}}"#;
        fs::write(&live, valid).unwrap();
        let store = ProfileStore::new(root.path().join("switch"));
        fs::create_dir_all(store.profile_auth_path("bad").parent().unwrap()).unwrap();
        fs::write(store.profile_auth_path("bad"), b"{}").unwrap();
        let switcher = Switcher::new(store, live.clone());

        assert!(switcher.use_profile("bad").is_err());
        assert_eq!(fs::read(live).unwrap(), valid);
    }

    #[test]
    fn save_current_refuses_to_overwrite_a_different_account() {
        let root = tempdir().unwrap();
        let live = root.path().join("codex/auth.json");
        fs::create_dir_all(live.parent().unwrap()).unwrap();
        let stored = br#"{"tokens":{"account_id":"old","access_token":"old-token"}}"#;
        let external = br#"{"tokens":{"account_id":"new","access_token":"new-token"}}"#;
        fs::write(&live, external).unwrap();
        let store = ProfileStore::new(root.path().join("switch"));
        store.save_bytes("old", stored).unwrap();
        store.write_current_alias("old").unwrap();
        let switcher = Switcher::new(store, live);

        assert!(switcher.save_live_as("old").is_err());
        assert_eq!(switcher.store().read_profile_bytes("old").unwrap(), stored);
    }
}
