//! 只读本机 Codex 索引。创建账号只说明来源，聊天累计 Token 不归入账号账单。
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, OpenFlags};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::{AppShared, LocalChatRow};

#[derive(Clone, Debug)]
pub struct Chat {
    pub id: String,
    pub project: String,
    pub title: String,
    pub model: String,
    pub creator: Option<String>,
    pub updated: i64,
    pub tokens: Option<u64>,
    pub archived: bool,
}
#[derive(Clone, Default)]
pub struct Dataset {
    pub chats: Vec<Chat>,
    pub truncated: bool,
    pub source: String,
}

fn database(home: &Path) -> Result<PathBuf, String> {
    let entries = std::fs::read_dir(home).map_err(|_| "本机 Codex 目录不可读".to_string())?;
    entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            let version = n
                .strip_prefix("state_")?
                .strip_suffix(".sqlite")?
                .parse::<u32>()
                .ok()?;
            Some((version, e.path()))
        })
        .max_by_key(|e| e.0)
        .map(|e| e.1)
        .ok_or_else(|| "尚无本机聊天索引；不扫描聊天正文补造数据".into())
}

pub fn read(home: &Path) -> Result<Dataset, String> {
    let path = database(home)?;
    let c = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "本机聊天索引暂时无法只读打开".to_string())?;
    c.busy_timeout(std::time::Duration::from_secs(2))
        .map_err(|e| e.to_string())?;
    c.pragma_update(None, "query_only", true)
        .map_err(|e| e.to_string())?;
    let mut columns = c
        .prepare("PRAGMA table_info(threads)")
        .map_err(|e| e.to_string())?;
    let cols = columns
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if [
        "id",
        "cwd",
        "title",
        "updated_at",
        "tokens_used",
        "archived",
    ]
    .iter()
    .any(|col| !cols.iter().any(|s| s == col))
    {
        return Err("本机索引格式待适配；未把未知结构当作空记录".into());
    }
    // 可选列来自固定白名单，不拼接用户输入。
    let model = if cols.iter().any(|s| s == "model") {
        "model"
    } else {
        "NULL"
    };
    let creator = if cols.iter().any(|s| s == "creator_account_id") {
        "creator_account_id"
    } else {
        "NULL"
    };
    let mut q=c.prepare(&format!("SELECT id,cwd,title,{model},{creator},updated_at,tokens_used,archived FROM threads ORDER BY updated_at DESC,id LIMIT 10001"))
        .map_err(|_|"本机索引查询失败；保留旧数据".to_string())?;
    let mut chats = q
        .query_map([], |r| {
            Ok(Chat {
                id: r.get(0)?,
                project: r.get(1)?,
                title: r.get(2)?,
                model: r
                    .get::<_, Option<String>>(3)?
                    .unwrap_or_else(|| "未记录".into()),
                creator: r.get::<_, Option<String>>(4)?.filter(|s| !s.is_empty()),
                updated: r.get(5)?,
                tokens: u64::try_from(r.get::<_, i64>(6)?).ok(),
                archived: r.get::<_, i64>(7)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "本机索引含无法识别的记录".to_string())?;
    let truncated = chats.len() > 10000;
    chats.truncate(10000);
    Ok(Dataset {
        chats,
        truncated,
        source: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    })
}

pub type Tags = BTreeMap<String, String>;
fn tags_path() -> PathBuf {
    crate::paths::app_home().join("analysis-chat-tags.json")
}
pub fn read_tags(path: &Path) -> crate::error::AppResult<Tags> {
    match std::fs::read(path) {
        Ok(b) => Ok(serde_json::from_slice(&b)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Tags::new()),
        Err(e) => Err(e.into()),
    }
}
pub fn save_tag(path: &Path, id: &str, tag: &str) -> crate::error::AppResult<()> {
    let tag = tag.trim();
    if id.is_empty() || tag.chars().count() > 40 || tag.chars().any(char::is_control) {
        return Err(crate::error::AppError::msg("标签限 40 个字，不含控制字符"));
    }
    let mut all = read_tags(path)?;
    if tag.is_empty() {
        all.remove(id);
    } else {
        all.insert(id.into(), tag.into());
    }
    crate::profile::atomic_write(path, &serde_json::to_vec_pretty(&all)?)
}

pub fn filtered<'a>(
    data: &'a Dataset,
    start: NaiveDate,
    end: NaiveDate,
    account: &str,
    scope: i32,
    search: &str,
    project: &str,
) -> Vec<&'a Chat> {
    let search = search.to_lowercase();
    data.chats
        .iter()
        .filter(|c| {
            let date = DateTime::from_timestamp(c.updated, 0).map(|d| d.date_naive());
            date.is_some_and(|d| d >= start && d <= end)
                && match scope {
                    1 => !account.is_empty() && c.creator.as_deref() == Some(account),
                    2 => c.creator.is_none(),
                    _ => true,
                }
                && (project.is_empty() || c.project == project)
                && (search.is_empty()
                    || c.title.to_lowercase().contains(&search)
                    || c.project.to_lowercase().contains(&search)
                    || c.model.to_lowercase().contains(&search))
        })
        .collect()
}

fn creator_label(c: &Chat, account: &str) -> &'static str {
    match c.creator.as_deref() {
        None => "创建账号未知",
        Some(a) if !account.is_empty() && a == account => "创建账号匹配",
        _ => "其他创建账号",
    }
}
pub fn export(chats: &[&Chat], tags: &Tags, account: &str, scope: &str) -> String {
    let mut out = String::from("\u{feff}");
    out += &crate::analysis_report::csv_row(&["范围".into(), scope.into()]);
    out += &crate::analysis_report::csv_row(&[
        "口径".into(),
        "按更新时间选择本机聊天；Token 是聊天累计值，非期间消耗；不与云端相加；可能含父子记录重叠"
            .into(),
    ]);
    out += &crate::analysis_report::csv_row(
        &[
            "聊天ID",
            "项目路径",
            "标题",
            "最后记录模型",
            "创建账号状态",
            "更新时间UTC",
            "聊天累计Token",
            "已归档",
            "手动用途",
        ]
        .map(String::from),
    );
    for c in chats {
        out += &crate::analysis_report::csv_row(&[
            c.id.clone(),
            c.project.clone(),
            c.title.clone(),
            c.model.clone(),
            creator_label(c, account).into(),
            DateTime::from_timestamp(c.updated, 0)
                .map(|d| d.to_rfc3339())
                .unwrap_or_default(),
            c.tokens.map(|n| n.to_string()).unwrap_or_default(),
            c.archived.to_string(),
            tags.get(&c.id).cloned().unwrap_or_default(),
        ]);
    }
    out
}

#[derive(Default)]
struct State {
    data: Option<Dataset>,
    busy: bool,
    error: String,
    updated: i64,
    selected: Option<String>,
}

pub fn connect(w: &crate::MainWindow, shared: Arc<Mutex<AppShared>>) {
    let state = Arc::new(Mutex::new(State::default()));
    let weak = w.as_weak();
    let st = state.clone();
    let sh = shared.clone();
    w.on_load_local_analysis(move || {
        let Some(w) = weak.upgrade() else { return };
        if st.lock().unwrap().busy {
            return;
        }
        st.lock().unwrap().busy = true;
        render(&w, &st.lock().unwrap(), &sh.lock().unwrap());
        let weak = w.as_weak();
        let st = st.clone();
        let sh = sh.clone();
        let home = crate::paths::codex_home();
        std::thread::spawn(move || {
            let result = read(&home);
            let _ = slint::invoke_from_event_loop(move || {
                let mut state = st.lock().unwrap();
                state.busy = false;
                match result {
                    Ok(data) => {
                        state.data = Some(data);
                        state.error.clear();
                        state.updated = Utc::now().timestamp();
                    }
                    Err(e) => state.error = e,
                }
                if let Some(w) = weak.upgrade() {
                    render(&w, &state, &sh.lock().unwrap());
                }
            });
        });
    });
    let weak = w.as_weak();
    let st = state.clone();
    let sh = shared.clone();
    w.on_render_local_analysis(move || {
        if let Some(w) = weak.upgrade() {
            render(&w, &st.lock().unwrap(), &sh.lock().unwrap());
        }
    });
    let weak = w.as_weak();
    let st = state.clone();
    let sh = shared.clone();
    w.on_select_local_chat(move |id| {
        if let Some(w) = weak.upgrade() {
            let mut s = st.lock().unwrap();
            s.selected = Some(id.to_string());
            render(&w, &s, &sh.lock().unwrap());
        }
    });
    let weak = w.as_weak();
    let st = state.clone();
    let sh = shared.clone();
    w.on_save_local_tag(move || {
        let Some(w) = weak.upgrade() else { return };
        let s = st.lock().unwrap();
        if let Some(id) = &s.selected {
            match save_tag(&tags_path(), id, &w.get_local_tag()) {
                Ok(()) => {
                    render(&w, &s, &sh.lock().unwrap());
                    w.set_local_action_status("手动标签已保存；未修改 Codex 聊天".into());
                }
                Err(_) => w.set_local_action_status(
                    "标签保存失败：请检查格式或已有标签文件；旧标签未覆盖".into(),
                ),
            }
        }
    });
    let weak = w.as_weak();
    let st = state;
    let sh = shared;
    w.on_export_local_analysis(move || {
        let Some(w) = weak.upgrade() else { return };
        let text = {
            let s = st.lock().unwrap();
            let a = account(&w, &sh.lock().unwrap());
            let Some(data) = &s.data else { return };
            let ds = dates(&w);
            let tags = match read_tags(&tags_path()) {
                Ok(t) => t,
                Err(_) => {
                    w.set_local_action_status("标签文件不可读，导出已停止".into());
                    return;
                }
            };
            let rows = filtered(
                data,
                ds[0],
                ds[ds.len() - 1],
                &a,
                w.get_local_scope(),
                &w.get_local_search(),
                project(&w).as_str(),
            );
            export(
                &rows,
                &tags,
                &a,
                &format!(
                    "{} — {} · UTC · {} · 范围选项 {} · 项目 {} · 搜索 {} · 最近读取 {} · {}",
                    ds[0],
                    ds[ds.len() - 1],
                    data.source,
                    w.get_local_scope(),
                    w.get_local_project_filter(),
                    w.get_local_search(),
                    s.updated,
                    if s.error.is_empty() {
                        "读取成功"
                    } else {
                        "保留旧数据"
                    }
                ),
            )
        };
        if let Some(path) = rfd::FileDialog::new()
            .set_title("导出本机聊天明细（包含标题和路径）")
            .add_filter("CSV", &["csv"])
            .set_file_name("codex-local-analysis.csv")
            .save_file()
        {
            let result = crate::profile::atomic_write(&path, text.as_bytes());
            w.set_local_action_status(
                if result.is_ok() {
                    "本机明细已导出"
                } else {
                    "导出失败"
                }
                .into(),
            );
        }
    });
}
fn account(w: &crate::MainWindow, s: &AppShared) -> String {
    s.codex
        .store()
        .read_profile_creds(&w.get_current_alias())
        .ok()
        .and_then(|c| c.account_id)
        .unwrap_or_default()
}
fn dates(w: &crate::MainWindow) -> Vec<NaiveDate> {
    crate::analytics::dates(
        Utc::now().date_naive(),
        if w.get_analysis_range() == 0 { 7 } else { 30 },
    )
}
fn project(w: &crate::MainWindow) -> String {
    let p = w.get_local_project_filter().to_string();
    if p == "全部项目" {
        String::new()
    } else {
        p
    }
}
fn render(w: &crate::MainWindow, s: &State, shared: &AppShared) {
    w.set_local_busy(s.busy);
    w.set_local_rows(ModelRc::new(VecModel::from(Vec::<LocalChatRow>::new())));
    w.set_local_detail("选择一条聊天查看明细并添加手动用途标签".into());
    w.set_local_selected(false);
    w.set_local_tag("".into());
    let Some(data) = &s.data else {
        w.set_local_status(if s.busy {
            "正在只读加载本机聊天索引".into()
        } else if !s.error.is_empty() {
            s.error.clone().into()
        } else {
            "点击读取本机记录；不读取聊天正文".into()
        });
        return;
    };
    let account = account(w, shared);
    let ds = dates(w);
    let mut projects = data
        .chats
        .iter()
        .map(|c| c.project.clone())
        .collect::<Vec<_>>();
    projects.sort();
    projects.dedup();
    let chosen = w.get_local_project_filter().to_string();
    let index = projects
        .iter()
        .position(|p| p == &chosen)
        .map(|n| n + 1)
        .unwrap_or(0);
    w.set_local_project_options(ModelRc::new(VecModel::from(
        std::iter::once("全部项目".into())
            .chain(projects.iter().map(|s| s.clone().into()))
            .collect::<Vec<slint::SharedString>>(),
    )));
    w.set_local_project_index(index as i32);
    if index == 0 {
        w.set_local_project_filter("全部项目".into());
    }
    let tags_result = read_tags(&tags_path());
    let tags = tags_result.as_ref().cloned().unwrap_or_default();
    let rows = filtered(
        data,
        ds[0],
        ds[ds.len() - 1],
        &account,
        w.get_local_scope(),
        &w.get_local_search(),
        &project(w),
    );
    let page_count = rows.len().div_ceil(12).max(1);
    let page = (w.get_local_page().max(0) as usize).min(page_count - 1);
    w.set_local_page(page as i32);
    w.set_local_page_count(page_count as i32);
    w.set_local_status(format!("本机索引 {} · 最近成功读取 {}\n{} — {} · UTC · 按聊天更新时间筛选 · {} 条 · 第 {}/{} 页{}{}{}",
        data.source,crate::usage::format_reset(Some(s.updated)),ds[0],ds[ds.len()-1],rows.len(),page+1,page_count,
        if data.truncated{"；仅读取最近 10000 条"}else{""},if s.error.is_empty(){String::new()}else{format!("；刷新失败，保留旧数据：{}",s.error)},if tags_result.is_err(){"；手动标签文件不可读"}else{""}).into());
    let unknown = rows.iter().filter(|c| c.creator.is_none()).count();
    let mut groups = BTreeMap::<String, usize>::new();
    for c in &rows {
        let key = match w.get_local_group() {
            1 => c.model.clone(),
            2 => tags.get(&c.id).cloned().unwrap_or_else(|| "未分类".into()),
            _ => c.project.clone(),
        };
        *groups.entry(key).or_default() += 1;
    }
    let mut groups = groups.into_iter().collect::<Vec<_>>();
    groups.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    w.set_local_summary(format!("{} 条创建账号未知。创建账号不证明整段聊天的用量归属。\n{}\n用途来自本地手动标签；未接入官方工作区分类。以下为记录数量，不是完成任务数。",
        unknown,groups.iter().take(8).map(|(k,n)|format!("{}：{} 条",if k.is_empty(){"未记录项目"}else{k},n)).collect::<Vec<_>>().join("\n")).into());
    w.set_local_rows(ModelRc::new(VecModel::from(
        rows.iter()
            .skip(page * 12)
            .take(12)
            .map(|c| LocalChatRow {
                id: c.id.clone().into(),
                title: if c.title.trim().is_empty() {
                    "未命名聊天".into()
                } else {
                    c.title.clone().into()
                },
                detail: format!(
                    "{} · {} · {}{}",
                    c.project,
                    c.model,
                    creator_label(c, &account),
                    if c.archived { " · 已归档" } else { "" }
                )
                .into(),
                tokens: c
                    .tokens
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "未提供".into())
                    .into(),
                tag: tags
                    .get(&c.id)
                    .cloned()
                    .unwrap_or_else(|| "未分类".into())
                    .into(),
            })
            .collect::<Vec<_>>(),
    )));
    if let Some(c) = s
        .selected
        .as_ref()
        .and_then(|id| rows.iter().find(|c| &c.id == id))
    {
        w.set_local_selected(true);
        w.set_local_tag(tags.get(&c.id).cloned().unwrap_or_default().into());
        w.set_local_detail(format!("{}\n项目：{}\n聊天 ID：{}\n最后记录模型：{} · {}\n更新时间：{}\n累计 Token：{}；这是整个聊天累计值，不是所选期间消耗。父子聊天可能有重叠，不汇总为账号账单。",
            c.title,c.project,c.id,c.model,creator_label(c,&account),DateTime::from_timestamp(c.updated,0).map(|d|d.to_rfc3339()).unwrap_or_default(),c.tokens.map(|n|n.to_string()).unwrap_or_else(||"未提供".into())).into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_index_is_read_only_handles_old_schema_and_preserves_unknown_identity() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("state_5.sqlite");
        let c = Connection::open(&p).unwrap();
        c.execute_batch("CREATE TABLE threads(id TEXT,cwd TEXT,title TEXT,updated_at INTEGER,tokens_used INTEGER,archived INTEGER);INSERT INTO threads VALUES('a','/p','标题',1794614400,123,0);").unwrap();
        drop(c);
        let before = std::fs::read(&p).unwrap();
        let d = read(dir.path()).unwrap();
        assert_eq!(d.chats.len(), 1);
        assert!(d.chats[0].creator.is_none());
        assert_eq!(before, std::fs::read(p).unwrap());
        let day = NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
        assert!(filtered(&d, day, day, "account", 1, "", "").is_empty());
        assert_eq!(filtered(&d, day, day, "account", 2, "标题", "").len(), 1);
    }
    #[test]
    fn manual_tags_survive_reload_and_bad_file_is_not_overwritten() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("tags.json");
        save_tag(&p, "a", "测试验证").unwrap();
        assert_eq!(read_tags(&p).unwrap()["a"], "测试验证");
        save_tag(&p, "b", "文档").unwrap();
        assert_eq!(read_tags(&p).unwrap().len(), 2);
        assert!(save_tag(&p, "a", &"a".repeat(41)).is_err());
        std::fs::write(&p, "bad").unwrap();
        assert!(save_tag(&p, "a", "new").is_err());
        assert_eq!(std::fs::read_to_string(p).unwrap(), "bad");
    }
    #[test]
    fn export_labels_cumulative_scope_and_never_exports_raw_account_id() {
        let chat = Chat {
            id: "x".into(),
            project: "/p".into(),
            title: "=CMD()".into(),
            model: "m".into(),
            creator: Some("private-id".into()),
            updated: 0,
            tokens: Some(0),
            archived: false,
        };
        let csv = export(&[&chat], &Tags::new(), "private-id", "7天");
        assert!(!csv.contains("private-id"));
        assert!(csv.contains("'=CMD()"));
        assert!(csv.contains("非期间消耗"));
    }
}
