//! 只读重置历史；分页完成后再替换缓存，不推断或执行重置。
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::error::{AppError, AppResult};
use crate::i18n::t;
use crate::{AppShared, MainWindow, Provider};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: String,
    pub kind: String,
    pub occurred_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Page {
    pub events: Vec<Event>,
    pub window_start: DateTime<Utc>,
    pub as_of: DateTime<Utc>,
    pub next_cursor: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct History {
    pub events: Vec<Event>,
    pub window_start: Option<DateTime<Utc>>,
    pub as_of: Option<DateTime<Utc>>,
    pub fetched_at: i64,
}

// 保留游标保护；分页中途失败不能把旧的完整记录替换成半份。
fn collect_pages(mut request: impl FnMut(Option<&str>) -> AppResult<Page>) -> AppResult<History> {
    let mut events = BTreeMap::<String, Event>::new();
    let mut cursor = None;
    let mut seen = HashSet::new();
    let mut result = History::default();
    for _ in 0..100 {
        let page = request(cursor.as_deref())?;
        if page.window_start > page.as_of {
            return Err(AppError::msg("重置历史时间范围无效"));
        }
        if result.window_start.is_none() {
            result.window_start = Some(page.window_start);
            result.as_of = Some(page.as_of);
        }
        for event in page.events {
            if event.id.is_empty() {
                return Err(AppError::msg("重置历史缺少记录标识"));
            }
            if let Some(old) = events.get(&event.id) {
                if old != &event {
                    return Err(AppError::msg("重置历史存在冲突记录，请刷新"));
                }
            } else {
                events.insert(event.id.clone(), event);
            }
        }
        match page.next_cursor.filter(|c| !c.is_empty()) {
            Some(next) if seen.insert(next.clone()) => cursor = Some(next),
            Some(_) => return Err(AppError::msg("重置历史分页游标重复，请重试")),
            None => {
                let start = result.window_start.unwrap();
                let end = result.as_of.unwrap();
                result.events = events
                    .into_values()
                    .filter(|e| e.occurred_at >= start && e.occurred_at <= end)
                    .collect();
                result.events.sort_by(|a, b| {
                    b.occurred_at
                        .cmp(&a.occurred_at)
                        .then_with(|| a.id.cmp(&b.id))
                });
                result.fetched_at = Utc::now().timestamp();
                return Ok(result);
            }
        }
    }
    Err(AppError::msg("重置历史分页超过读取上限，原缓存保留"))
}

fn fetch(token: &str, account: &str) -> AppResult<History> {
    let client = crate::usage::http_client()?;
    collect_pages(|cursor| {
        let query = cursor.map(|c| vec![("cursor", c)]).unwrap_or_default();
        crate::analytics::get(
            &client,
            token,
            account,
            "rate-limit-reset-credits/history",
            &query,
        )
    })
}
fn path() -> std::path::PathBuf {
    crate::paths::app_home().join("cache/reset-history.json")
}
fn read(path: &std::path::Path) -> AppResult<BTreeMap<String, History>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(e.into()),
    }
}
fn save(path: &std::path::Path, account: &str, history: &History) -> AppResult<()> {
    let mut entries = read(path)?;
    entries.insert(account.into(), history.clone());
    crate::profile::atomic_write(path, &serde_json::to_vec_pretty(&entries)?)
}
#[derive(Default)]
pub struct State {
    active: Option<(String, String)>,
    cache: HashMap<String, History>,
    pending: HashSet<String>,
    errors: HashMap<String, String>,
}

pub fn connect(window: &MainWindow, shared: Arc<Mutex<AppShared>>) {
    let weak = window.as_weak();
    window.on_load_reset_history(move |force| {
        let Some(w) = weak.upgrade() else {
            return;
        };
        let alias = w.get_current_alias().to_string();
        let (token, account) = {
            let mut state = shared.lock().unwrap();
            if state.provider != Provider::Codex {
                return;
            }
            let Ok(creds) = state.codex.store().read_profile_creds(&alias) else {
                return;
            };
            let (Some(token), Some(account)) = (creds.access_token, creds.account_id) else {
                return;
            };
            let history = &mut state.reset_history;
            history.active = Some((alias, account.clone()));
            if !history.cache.contains_key(&account) {
                match read(&path()) {
                    Ok(saved) => {
                        if let Some(h) = saved.get(&account) {
                            history.cache.insert(account.clone(), h.clone());
                        }
                    }
                    Err(_) => {
                        history
                            .errors
                            .insert(account.clone(), "重置历史缓存无法读取，原文件保留".into());
                    }
                }
            }
            if history.pending.contains(&account)
                || (!force
                    && !history.errors.contains_key(&account)
                    && history
                        .cache
                        .get(&account)
                        .is_some_and(|h| Utc::now().timestamp() - h.fetched_at < 60))
            {
                render_locked(&w, &state);
                return;
            }
            history.pending.insert(account.clone());
            render_locked(&w, &state);
            (token, account)
        };
        let shared = Arc::clone(&shared);
        let weak = w.as_weak();
        std::thread::spawn(move || {
            let result = fetch(&token, &account);
            let _ = slint::invoke_from_event_loop(move || {
                let mut state = shared.lock().unwrap();
                let history = &mut state.reset_history;
                history.pending.remove(&account);
                match result {
                    Ok(data) => {
                        // 在同一状态锁下串行写缓存，保留其他账号记录。
                        if save(&path(), &account, &data).is_err() {
                            history
                                .errors
                                .insert(account.clone(), "历史已更新，但本地缓存保存失败".into());
                        } else {
                            history.errors.remove(&account);
                        }
                        history.cache.insert(account, data);
                    }
                    Err(e) => {
                        history.errors.insert(account, e.to_string());
                    }
                }
                if let Some(w) = weak.upgrade() {
                    render_locked(&w, &state);
                }
            });
        });
    });
}

pub fn render_locked(w: &MainWindow, state: &AppShared) {
    let lang = state.lang;
    w.set_tr_reset_history(t(lang, "history.title").into());
    w.set_reset_history_rows(ModelRc::new(
        VecModel::from(Vec::<crate::HistoryRow>::new()),
    ));
    w.set_reset_history_status("".into());
    w.set_reset_history_window(t(lang, "history.window").into());
    w.set_reset_history_busy(false);
    let h = &state.reset_history;
    let Some((alias, account)) = &h.active else {
        return;
    };
    if state.provider != Provider::Codex
        || alias != w.get_current_alias().as_str()
        || !state
            .codex
            .store()
            .read_profile_creds(alias)
            .ok()
            .and_then(|c| c.account_id)
            .is_some_and(|a| a == *account)
    {
        return;
    }
    let busy = h.pending.contains(account);
    w.set_reset_history_busy(busy);
    let data = h.cache.get(account);
    let mut status = if busy {
        t(lang, "new.loading").to_string()
    } else {
        String::new()
    };
    if let Some(error) = h.errors.get(account) {
        status.push_str(&format!(" {error}"));
    }
    if let Some(data) = data {
        // 持久化缓存重启后也只显示当前 30 天窗口内记录。
        let cutoff = Utc::now() - chrono::Duration::days(30);
        let rows: Vec<_> = data
            .events
            .iter()
            .filter(|e| e.occurred_at >= cutoff)
            .map(|e| crate::HistoryRow {
                title: t(
                    lang,
                    match e.kind.as_str() {
                        "granted" => "history.granted",
                        "used" | "redeemed" => "history.used",
                        _ => "history.unknown",
                    },
                )
                .into(),
                time: e
                    .occurred_at
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M %:z")
                    .to_string()
                    .into(),
            })
            .collect();
        if rows.is_empty() {
            status.push_str(t(lang, "history.empty"));
        }
        w.set_reset_history_rows(ModelRc::new(VecModel::from(rows)));
        if !status.trim().is_empty() {
            status.push_str(" · ");
        }
        status.push_str(&format!(
            "{} {}",
            t(lang, "new.last_success"),
            crate::usage::format_reset(Some(data.fetched_at))
        ));
    }
    w.set_reset_history_status(status.into());
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(events: serde_json::Value, cursor: Option<&str>) -> Page {
        serde_json::from_value(serde_json::json!({"events":events,"window_start":"2026-08-17T00:00:00Z","as_of":"2026-09-16T00:00:00Z","next_cursor":cursor})).unwrap()
    }
    #[test]
    fn pages_deduplicate_sort_and_preserve_event_kinds() {
        let e = serde_json::json!({"id":"a","kind":"granted","occurred_at":"2026-09-05T04:20:00Z"});
        let mut n = 0;
        let h=collect_pages(|cursor|{n+=1;Ok(if cursor.is_none(){page(serde_json::json!([e]),Some("next"))}else{page(serde_json::json!([e,{"id":"b","kind":"used","occurred_at":"2026-09-06T00:05:00Z"}]),None)})}).unwrap();
        assert_eq!(n, 2);
        assert_eq!(h.events.len(), 2);
        assert_eq!(h.events[0].kind, "used");
    }
    #[test]
    fn repeated_cursor_and_conflicting_events_fail() {
        assert!(collect_pages(|_| Ok(page(serde_json::json!([]), Some("loop")))).is_err());
        let p = page(
            serde_json::json!([{"id":"a","kind":"used","occurred_at":"2026-09-05T00:00:00Z"},{"id":"a","kind":"granted","occurred_at":"2026-09-05T00:00:00Z"}]),
            None,
        );
        assert!(collect_pages(|_| Ok(p.clone())).is_err());
        assert!(collect_pages(|_| Err(AppError::msg("network"))).is_err());
    }
    #[test]
    fn account_cache_survives_restart_and_corruption_is_not_overwritten() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("history.json");
        let a = History {
            fetched_at: 10,
            ..Default::default()
        };
        save(&p, "a", &a).unwrap();
        save(
            &p,
            "b",
            &History {
                fetched_at: 20,
                ..Default::default()
            },
        )
        .unwrap();
        let entries = read(&p).unwrap();
        assert_eq!(entries["a"].fetched_at, 10);
        assert_eq!(entries["b"].fetched_at, 20);
        std::fs::write(&p, b"broken").unwrap();
        assert!(save(&p, "a", &a).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"broken");
    }
}
