//! 已保存账户的可移植备份。只合并新目录，不改当前账户或实时登录文件。
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine};
use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::{claude, identity, profile};

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_ACCOUNTS: usize = 500;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Service {
    Codex,
    ClaudeDesktop,
}

impl Service {
    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::ClaudeDesktop => "Claude Desktop",
        }
    }
    fn folder(self, root: &Path) -> PathBuf {
        match self {
            Self::Codex => root.join("profiles"),
            Self::ClaudeDesktop => root.join("claude-desktop/profiles"),
        }
    }
    fn file(self) -> &'static str {
        match self {
            Self::Codex => "auth.json",
            Self::ClaudeDesktop => "Cookies",
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive {
    format: String,
    version: u32,
    created_at: String,
    accounts: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    service: Service,
    alias: String,
    data: String,
}

pub struct PlannedEntry {
    pub service: Service,
    pub original_alias: String,
    pub alias: String,
    bytes: Vec<u8>,
}

pub struct ImportPlan {
    pub entries: Vec<PlannedEntry>,
    pub skipped: usize,
}

fn read_limited(path: &Path) -> AppResult<Vec<u8>> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.len() > MAX_BYTES as u64 {
        return Err(AppError::msg("备份文件必须是普通文件，且不超过 64 MiB"));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(AppError::msg("备份超过 64 MiB 限制"));
    }
    Ok(bytes)
}

fn validate(service: Service, bytes: &[u8], root: &Path) -> AppResult<()> {
    match service {
        Service::Codex => {
            if bytes.len() > 2 * 1024 * 1024 {
                return Err(AppError::msg("auth.json 超过 2 MiB 限制"));
            }
            identity::parse_auth_bytes(bytes)?;
        }
        Service::ClaudeDesktop => {
            let temp = tempfile::tempdir_in(root)?;
            let path = temp.path().join("Cookies");
            write_new(&path, bytes)?;
            claude::validate_cookie_db_path(&path)?;
        }
    }
    Ok(())
}

fn saved(root: &Path) -> AppResult<Vec<PlannedEntry>> {
    let mut entries = Vec::new();
    let mut size = 0;
    for service in [Service::Codex, Service::ClaudeDesktop] {
        let folder = service.folder(root);
        if !folder.exists() {
            continue;
        }
        for entry in fs::read_dir(folder)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let alias = entry.file_name().to_string_lossy().to_string();
            profile::normalize_alias(&alias)?;
            let path = entry.path().join(service.file());
            if !path.exists() {
                continue;
            }
            let bytes = read_limited(&path)?;
            size += bytes.len();
            if entries.len() >= MAX_ACCOUNTS || size > MAX_BYTES / 4 * 3 {
                return Err(AppError::msg("账户备份最多 500 个账户、48 MiB 原始数据"));
            }
            entries.push(PlannedEntry {
                service,
                original_alias: alias.clone(),
                alias,
                bytes,
            });
        }
    }
    entries.sort_by(|a, b| (a.service.label(), &a.alias).cmp(&(b.service.label(), &b.alias)));
    Ok(entries)
}

pub fn export(root: &Path, destination: &Path) -> AppResult<usize> {
    profile::ensure_private_dir(root)?;
    let _locks = lock_stores(root)?;
    let entries = saved(root)?;
    if entries.is_empty() {
        return Err(AppError::msg("没有可导出的已保存账户"));
    }
    let count = entries.len();
    let mut accounts = Vec::new();
    for entry in entries {
        validate(entry.service, &entry.bytes, root)
            .map_err(|_| AppError::msg(format!("账户 {} 的数据无效，未生成备份", entry.alias)))?;
        accounts.push(Entry {
            service: entry.service,
            alias: entry.alias,
            data: STANDARD.encode(entry.bytes),
        });
    }
    let archive = Archive {
        format: "codex-account-switch-backup".into(),
        version: 1,
        created_at: chrono::Utc::now().to_rfc3339(),
        accounts,
    };
    let bytes = serde_json::to_vec(&archive)?;
    if bytes.len() > MAX_BYTES {
        return Err(AppError::msg("备份超过 64 MiB 限制"));
    }
    write_new(destination, &bytes)?;
    Ok(count)
}

pub fn preview(root: &Path, source: &Path) -> AppResult<ImportPlan> {
    let archive: Archive = serde_json::from_slice(&read_limited(source)?)
        .map_err(|_| AppError::msg("无法读取账户备份：格式损坏或包含未知字段"))?;
    if archive.format != "codex-account-switch-backup" || archive.version != 1 {
        return Err(AppError::msg("不支持的账户备份格式或版本"));
    }
    if archive.accounts.is_empty() || archive.accounts.len() > MAX_ACCOUNTS {
        return Err(AppError::msg("备份必须包含 1–500 个账户"));
    }
    profile::ensure_private_dir(root)?;
    let existing = saved(root)?;
    let mut names = HashSet::new();
    for service in [Service::Codex, Service::ClaudeDesktop] {
        if service.folder(root).exists() {
            for entry in fs::read_dir(service.folder(root))? {
                names.insert((
                    service.label(),
                    entry?.file_name().to_string_lossy().to_lowercase(),
                ));
            }
        }
    }
    let mut plan = ImportPlan {
        entries: Vec::new(),
        skipped: 0,
    };
    let mut source_names = HashSet::new();
    for entry in archive.accounts {
        if profile::normalize_alias(&entry.alias)? != entry.alias
            || !source_names.insert((entry.service.label(), entry.alias.to_lowercase()))
        {
            return Err(AppError::msg("备份中含不合法或重复的账户别名"));
        }
        let bytes = STANDARD
            .decode(&entry.data)
            .map_err(|_| AppError::msg("备份中的账户编码已损坏"))?;
        validate(entry.service, &bytes, root).map_err(|_| {
            AppError::msg(format!("账户 {} 的数据无效；未导入任何账户", entry.alias))
        })?;
        if existing
            .iter()
            .chain(plan.entries.iter())
            .any(|p| p.service == entry.service && p.bytes == bytes)
        {
            plan.skipped += 1;
            continue;
        }
        let mut alias = entry.alias.clone();
        let mut suffix = 2;
        while names.contains(&(entry.service.label(), alias.to_lowercase())) {
            let tail = format!("-import-{suffix}");
            alias = format!(
                "{}{}",
                &entry.alias[..entry.alias.len().min(64 - tail.len())],
                tail
            );
            suffix += 1;
        }
        names.insert((entry.service.label(), alias.to_lowercase()));
        plan.entries.push(PlannedEntry {
            service: entry.service,
            original_alias: entry.alias,
            alias,
            bytes,
        });
    }
    if existing.len() + plan.entries.len() > MAX_ACCOUNTS
        || existing
            .iter()
            .chain(plan.entries.iter())
            .map(|p| p.bytes.len())
            .sum::<usize>()
            > MAX_BYTES / 4 * 3
    {
        return Err(AppError::msg(
            "合并后超过 500 个账户或 48 MiB 原始数据限制，未导入任何账户",
        ));
    }
    Ok(plan)
}

fn lock_stores(root: &Path) -> AppResult<Vec<fs::File>> {
    let mut locks = Vec::new();
    for path in [
        root.join("auth.lock"),
        root.join("claude-desktop/cookies.lock"),
    ] {
        let file = profile::open_private_lock(&path)?;
        file.try_lock_exclusive()
            .map_err(|_| AppError::msg("账户正被其他操作使用，请稍后重试"))?;
        locks.push(file);
    }
    Ok(locks)
}

pub fn apply(root: &Path, plan: &ImportPlan) -> AppResult<usize> {
    let _locks = lock_stores(root)?;
    // 预览后目录可能变化，写入前再次核对全部目标；不覆盖任何既有目录。
    for entry in &plan.entries {
        let folder = entry.service.folder(root);
        let conflict = folder.exists()
            && fs::read_dir(&folder)?.any(|e| {
                e.map(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&entry.alias)
                })
                .unwrap_or(true)
            });
        if conflict {
            return Err(AppError::msg("账户列表已变化，请重新选择备份并预览"));
        }
    }
    let mut created: Vec<(PathBuf, &'static str)> = Vec::new();
    let result = (|| -> AppResult<()> {
        for entry in &plan.entries {
            let folder = entry.service.folder(root);
            profile::ensure_private_dir(&folder)?;
            let destination = folder.join(&entry.alias);
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(&destination)?;
            created.push((destination.clone(), entry.service.file()));
            profile::ensure_private_dir(&destination)?;
            write_new(&destination.join(entry.service.file()), &entry.bytes)?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut incomplete = false;
        for (dir, name) in created.iter().rev() {
            let file = dir.join(name);
            if file.exists() && fs::remove_file(file).is_err() {
                incomplete = true;
            }
            if fs::remove_dir(dir).is_err() {
                incomplete = true;
            }
        }
        return Err(AppError::msg(if incomplete {
            "导入失败，部分新增账户未能回滚，请刷新列表检查".into()
        } else {
            format!("导入失败，新增账户已回滚：{error}")
        }));
    }
    Ok(plan.entries.len())
}

fn write_new(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|e| AppError::msg(format!("无法创建文件（已有文件不会覆盖）：{e}")))?;
    if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn auth(token: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({"OPENAI_API_KEY": token})).unwrap()
    }
    fn seed(root: &Path, alias: &str, token: &str) {
        profile::ProfileStore::new(root.into())
            .save_bytes(alias, &auth(token))
            .unwrap();
    }
    fn archive(path: &Path, accounts: Vec<Entry>) {
        let value = Archive {
            format: "codex-account-switch-backup".into(),
            version: 1,
            created_at: "fixture".into(),
            accounts,
        };
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    }
    fn entry(alias: &str, bytes: &[u8]) -> Entry {
        Entry {
            service: Service::Codex,
            alias: alias.into(),
            data: STANDARD.encode(bytes),
        }
    }

    #[test]
    fn both_services_round_trip_and_second_import_skips_everything() {
        let source = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        let file = source.path().join("accounts.casbackup");
        seed(source.path(), "work", "fixture-work");
        let folder = source.path().join("claude-desktop/profiles/claude");
        fs::create_dir_all(&folder).unwrap();
        let db = rusqlite::Connection::open(folder.join("Cookies")).unwrap();
        db.execute_batch("CREATE TABLE cookies (name TEXT, value TEXT); INSERT INTO cookies VALUES ('fixture', 'local-only');").unwrap();
        drop(db);
        assert_eq!(export(source.path(), &file).unwrap(), 2);
        let plan = preview(dest.path(), &file).unwrap();
        assert_eq!(apply(dest.path(), &plan).unwrap(), 2);
        assert_eq!(
            profile::ProfileStore::new(dest.path().into())
                .read_profile_bytes("work")
                .unwrap(),
            auth("fixture-work")
        );
        claude::validate_cookie_db_path(
            &dest.path().join("claude-desktop/profiles/claude/Cookies"),
        )
        .unwrap();
        let repeat = preview(dest.path(), &file).unwrap();
        assert!(repeat.entries.is_empty());
        assert_eq!(repeat.skipped, 2);
        assert!(!dest.path().join("current").exists());
        assert!(!dest.path().join("claude-desktop/current").exists());
    }

    #[test]
    fn conflicts_preserve_existing_credentials_and_active_account() {
        let root = tempfile::tempdir().unwrap();
        seed(root.path(), "WORK", "existing");
        fs::write(root.path().join("current"), "WORK").unwrap();
        let file = root.path().join("backup.casbackup");
        archive(
            &file,
            vec![
                entry("work", &auth("incoming")),
                entry("copy", &auth("existing")),
            ],
        );
        let plan = preview(root.path(), &file).unwrap();
        assert_eq!(plan.skipped, 1);
        assert_eq!(plan.entries[0].alias, "work-import-2");
        apply(root.path(), &plan).unwrap();
        assert_eq!(
            fs::read(root.path().join("profiles/WORK/auth.json")).unwrap(),
            auth("existing")
        );
        assert_eq!(
            fs::read_to_string(root.path().join("current")).unwrap(),
            "WORK"
        );
    }

    #[test]
    fn malformed_last_entry_and_traversal_do_not_import_partial_data() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("backup.casbackup");
        for invalid in [entry("bad", b"{}"), entry("../escape", &auth("x"))] {
            archive(&file, vec![entry("valid", &auth("valid")), invalid]);
            assert!(preview(root.path(), &file).is_err());
            assert!(!root.path().join("profiles/valid").exists());
        }
    }

    #[test]
    fn changed_destination_is_rejected_before_any_write() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("backup.casbackup");
        archive(
            &file,
            vec![entry("one", &auth("1")), entry("two", &auth("2"))],
        );
        let plan = preview(root.path(), &file).unwrap();
        seed(root.path(), "TWO", "new-existing");
        assert!(apply(root.path(), &plan).is_err());
        assert!(!root.path().join("profiles/one").exists());
    }

    #[test]
    fn write_failure_rolls_back_new_accounts() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("backup.casbackup");
        archive(&file, vec![entry("one", &auth("1"))]);
        let mut plan = preview(root.path(), &file).unwrap();
        // 模拟第二项在文件系统创建目录时失败，验证此前新增账户会回滚。
        let first = &plan.entries[0];
        let bad = PlannedEntry {
            service: first.service,
            original_alias: "too-long".into(),
            alias: "x".repeat(300),
            bytes: auth("2"),
        };
        plan.entries.push(bad);
        assert!(apply(root.path(), &plan).is_err());
        assert!(!root.path().join("profiles/one").exists());
    }

    #[test]
    fn existing_export_and_parent_permissions_are_preserved() {
        let root = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        seed(root.path(), "one", "fixture");
        let path = output.path().join("backup.casbackup");
        #[cfg(unix)]
        let before = {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(output.path()).unwrap().permissions().mode()
        };
        export(root.path(), &path).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(export(root.path(), &path).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(output.path()).unwrap().permissions().mode(),
                before
            );
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn invalid_archive_versions_duplicates_encoding_and_oversize_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("backup.casbackup");
        archive(&file, vec![entry("a", &auth("1")), entry("A", &auth("2"))]);
        assert!(preview(root.path(), &file).is_err());
        archive(
            &file,
            vec![Entry {
                service: Service::Codex,
                alias: "a".into(),
                data: "%%%".into(),
            }],
        );
        assert!(preview(root.path(), &file).is_err());
        archive(&file, vec![entry("a", &auth("1"))]);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        value["version"] = json!(2);
        fs::write(&file, value.to_string()).unwrap();
        assert!(preview(root.path(), &file).is_err());
        fs::File::create(&file)
            .unwrap()
            .set_len(MAX_BYTES as u64 + 1)
            .unwrap();
        assert!(preview(root.path(), &file).is_err());
    }
}
