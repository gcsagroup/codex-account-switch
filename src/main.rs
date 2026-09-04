#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod claude;
mod error;
mod identity;
mod login;
mod paths;
mod profile;
mod restart;
mod switcher;
mod usage;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use fs4::fs_std::FileExt;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::identity::short_account_id;
use crate::profile::ProfileStore;
use crate::switcher::Switcher;
use crate::usage::{
    format_reset, format_usage_row, format_usage_short, format_window_reset,
    format_window_reset_short, normalized_cumulative_buckets, normalized_daily_tokens,
    normalized_usage_buckets, recent_daily_tokens, window_has_data, window_is_expired,
    UsageSnapshot,
};

mod i18n;

slint::include_modules!();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provider {
    Codex,
    Claude,
}

struct AppShared {
    codex: Switcher,
    claude: claude::ClaudeSwitcher,
    provider: Provider,
    preview_alias: Option<String>,
    usage: HashMap<String, UsageSnapshot>,
    usage_refreshing: Arc<AtomicBool>,
    lang: i18n::Language,
}

impl AppShared {
    fn profiles(&self) -> crate::error::AppResult<Vec<crate::profile::Profile>> {
        match self.provider {
            Provider::Codex => self.codex.store().list(),
            Provider::Claude => self.claude.store().list(),
        }
    }

    fn current_alias(&self) -> Option<String> {
        match self.provider {
            Provider::Codex => self.codex.store().read_current_alias(),
            Provider::Claude => self.claude.store().read_current_alias(),
        }
    }

    fn save_live_as(&self, alias: &str) -> crate::error::AppResult<()> {
        match self.provider {
            Provider::Codex => self.codex.save_live_as(alias),
            Provider::Claude => self.claude.save_live_as(alias),
        }
    }

    fn use_profile(&self, alias: &str) -> crate::error::AppResult<()> {
        match self.provider {
            Provider::Codex => self.codex.use_profile(alias),
            Provider::Claude => self.claude.use_profile(alias),
        }
    }

    fn remove(&self, alias: &str) -> crate::error::AppResult<()> {
        match self.provider {
            Provider::Codex => self.codex.store().remove(alias),
            Provider::Claude => self.claude.store().remove(alias),
        }
    }

    fn restore_last_removed(&self) -> crate::error::AppResult<String> {
        match self.provider {
            Provider::Codex => self.codex.store().restore_last_removed(),
            Provider::Claude => self.claude.store().restore_last_removed(),
        }
    }

    fn can_restore_last_removed(&self) -> bool {
        match self.provider {
            Provider::Codex => self.codex.store().can_restore_last_removed(),
            Provider::Claude => self.claude.store().can_restore_last_removed(),
        }
    }

    fn rename(&self, from: &str, to: &str) -> crate::error::AppResult<()> {
        match self.provider {
            Provider::Codex => self.codex.store().rename(from, to),
            Provider::Claude => self.claude.store().rename(from, to),
        }
    }

    fn import_file(
        &self,
        path: &std::path::Path,
        alias: Option<&str>,
    ) -> crate::error::AppResult<String> {
        match self.provider {
            Provider::Codex => self.codex.store().import_file(path, alias),
            Provider::Claude => self.claude.store().import_file(path, alias),
        }
    }

    fn import_bytes(&self, bytes: &[u8], alias: Option<&str>) -> crate::error::AppResult<String> {
        match self.provider {
            Provider::Codex => self.codex.store().import_bytes(bytes, alias),
            Provider::Claude => self.claude.store().import_bytes(bytes, alias),
        }
    }
}

type SharedLoginCancel = Arc<Mutex<Option<login::LoginCancelHandle>>>;

fn main() {
    if let Err(err) = run_app() {
        let msg = format!("codex-account-switch 启动失败: {err:#}\n");
        eprintln!("{msg}");
        let _ = append_log(&msg);
        std::process::exit(1);
    }
}

fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    install_panic_hook();
    append_log("starting")?;

    let store = ProfileStore::default_store();
    store.ensure_dirs()?;
    let _instance_lock = acquire_instance_lock()?;
    let codex = Switcher::new(store, paths::live_auth_path());
    let claude_store = claude::ClaudeProfileStore::default_store();
    claude_store.ensure_dirs()?;
    let claude = claude::ClaudeSwitcher::new(claude_store);
    let usage = usage::load_cache();

    let lang = i18n::Language::preferred_or_system();
    let shared = Arc::new(Mutex::new(AppShared {
        codex,
        claude,
        provider: Provider::Codex,
        preview_alias: None,
        usage,
        usage_refreshing: Arc::new(AtomicBool::new(false)),
        lang,
    }));
    let running = Arc::new(AtomicBool::new(true));

    let window = MainWindow::new()?;
    let tray = AppTray::new()?;

    // Set initial language index in UI
    window.set_lang_index(match lang {
        i18n::Language::En => 0,
        i18n::Language::ZhCn => 1,
        i18n::Language::ZhTw => 2,
        i18n::Language::Ja => 3,
        i18n::Language::Ko => 4,
        i18n::Language::Fr => 5,
        i18n::Language::Es => 6,
    });

    refresh_ui(&window, &tray, &shared);
    window.set_status_text(i18n::t(lang, "status.ready").into());

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_set_provider(move |idx| {
            {
                let mut state = shared.lock().unwrap();
                state.provider = if idx == 1 {
                    Provider::Claude
                } else {
                    Provider::Codex
                };
                state.preview_alias = None;
            }
            if let (Some(w), Some(t)) = (window_weak.upgrade(), tray_weak.upgrade()) {
                w.set_provider_index(idx.clamp(0, 1));
                w.set_import_tab(0);
                w.set_rename_from(SharedString::new());
                w.set_rename_to(SharedString::new());
                refresh_ui(&w, &t, &shared);
            }
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_preview_account(move |alias| {
            let alias = alias.to_string();
            let (lang, exists) = {
                let mut state = shared.lock().unwrap();
                let exists = state
                    .profiles()
                    .unwrap_or_default()
                    .iter()
                    .any(|profile| profile.alias == alias);
                if exists {
                    state.preview_alias = Some(alias.clone());
                }
                (state.lang, exists)
            };
            if !exists {
                return;
            }
            if let (Some(w), Some(t)) = (window_weak.upgrade(), tray_weak.upgrade()) {
                refresh_ui(&w, &t, &shared);
                w.set_status_text(
                    i18n::t(lang, "status.previewing")
                        .replace("{alias}", &alias)
                        .into(),
                );
            }
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_restore_account(move || {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            let lang = shared.lock().unwrap().lang;
            let result = shared.lock().unwrap().restore_last_removed();
            match result {
                Ok(alias) => {
                    w.set_status_text(
                        format!("{} {}", i18n::t(lang, "status.restored"), alias).into(),
                    );
                    if let Some(t) = tray_weak.upgrade() {
                        refresh_ui(&w, &t, &shared);
                    }
                }
                Err(error) => w.set_status_text(
                    format!("{}: {error}", i18n::t(lang, "status.restore_failed")).into(),
                ),
            }
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_set_language(move |idx| {
            let new_lang = match idx {
                0 => i18n::Language::En,
                1 => i18n::Language::ZhCn,
                2 => i18n::Language::ZhTw,
                3 => i18n::Language::Ja,
                4 => i18n::Language::Ko,
                5 => i18n::Language::Fr,
                6 => i18n::Language::Es,
                _ => i18n::Language::En,
            };
            {
                let mut state = shared.lock().unwrap();
                state.lang = new_lang;
            }
            let _ = new_lang.save_preferred();
            if let (Some(w), Some(t)) = (window_weak.upgrade(), tray_weak.upgrade()) {
                w.set_lang_index(match new_lang {
                    i18n::Language::En => 0,
                    i18n::Language::ZhCn => 1,
                    i18n::Language::ZhTw => 2,
                    i18n::Language::Ja => 3,
                    i18n::Language::Ko => 4,
                    i18n::Language::Fr => 5,
                    i18n::Language::Es => 6,
                });
                refresh_ui(&w, &t, &shared);
                w.set_status_text(i18n::t(new_lang, "status.lang_switched").into());
            }
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_refresh(move || {
            let lang = shared.lock().unwrap().lang;
            if let (Some(w), Some(t)) = (window_weak.upgrade(), tray_weak.upgrade()) {
                refresh_ui(&w, &t, &shared);
                w.set_status_text(i18n::t(lang, "status.list_refreshed").into());
            }
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_save_current(move || {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            let alias = w.get_save_alias().to_string();
            let lang = shared.lock().unwrap().lang;
            if alias.trim().is_empty() {
                w.set_status_text(i18n::t(lang, "status.enter_alias").into());
                return;
            }
            let result = {
                let state = shared.lock().unwrap();
                state.save_live_as(&alias)
            };
            match result {
                Ok(()) => {
                    w.set_save_alias(SharedString::new());
                    w.set_status_text(
                        format!("{} {}", i18n::t(lang, "status.saved_live"), alias).into(),
                    );
                    if let Some(t) = tray_weak.upgrade() {
                        refresh_ui(&w, &t, &shared);
                    }
                }
                Err(e) => w.set_status_text(
                    format!("{}: {}", i18n::t(lang, "status.save_failed"), e).into(),
                ),
            }
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_import_live(move || {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            let result = {
                let state = shared.lock().unwrap();
                match state.provider {
                    Provider::Codex => state.codex.adopt_live_profile(None),
                    Provider::Claude => state.claude.save_live_auto(None),
                }
            };
            finish_import_result(&w, &tray_weak, &shared, result);
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_import_auth(move || {
            let Some(w) = window_weak.upgrade() else {
                return;
            };

            let typed = w.get_browse_path().to_string();
            let start = resolve_browse_start(&typed);

            // If the user pasted a concrete file path, import it directly.
            if start.is_file() {
                finish_import(&w, &tray_weak, &shared, &start);
                return;
            }

            let open_dir = if start.is_dir() {
                Some(start.clone())
            } else if let Some(parent) = start.parent().filter(|p| p.is_dir()) {
                Some(parent.to_path_buf())
            } else if typed.trim().is_empty() {
                let provider = shared.lock().unwrap().provider;
                Some(match provider {
                    Provider::Codex => paths::codex_home(),
                    Provider::Claude => paths::app_home().join("claude").join("profiles"),
                })
            } else {
                None
            };

            let lang = shared.lock().unwrap().lang;
            let Some(open_dir) = open_dir else {
                w.set_status_text(
                    format!(
                        "{}: {}",
                        i18n::t(lang, "status.invalid_path"),
                        start.display()
                    )
                    .into(),
                );
                return;
            };

            w.set_status_text(
                format!(
                    "{}: {}",
                    i18n::t(lang, "status.opening_dir"),
                    open_dir.display()
                )
                .into(),
            );

            let dialog = rfd::FileDialog::new()
                .set_title(format!(
                    "{} — {}",
                    i18n::t(lang, "dialog.select_file"),
                    open_dir.display()
                ))
                .add_filter("JSON", &["json"])
                .add_filter("*", &["*"])
                .set_directory(&open_dir);

            let Some(path) = dialog.pick_file() else {
                w.set_status_text(i18n::t(lang, "status.import_cancelled").into());
                return;
            };

            if let Some(parent) = path.parent() {
                w.set_browse_path(parent.to_string_lossy().to_string().into());
            }

            finish_import(&w, &tray_weak, &shared, &path);
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_use_account(move |alias| {
            apply_switch(&window_weak, &tray_weak, &shared, &alias, false);
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_use_account_restart(move |alias| {
            apply_switch(&window_weak, &tray_weak, &shared, &alias, true);
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_remove_account(move |alias| {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            let lang = shared.lock().unwrap().lang;
            let result = {
                let state = shared.lock().unwrap();
                state.remove(&alias)
            };
            match result {
                Ok(()) => {
                    {
                        let mut state = shared.lock().unwrap();
                        if state.provider == Provider::Codex {
                            state.usage.remove(alias.as_str());
                            let _ = usage::save_cache(&state.usage);
                        }
                        if state.preview_alias.as_deref() == Some(alias.as_str()) {
                            state.preview_alias = None;
                        }
                    }
                    w.set_status_text(
                        format!("{} {}", i18n::t(lang, "status.deleted"), alias).into(),
                    );
                    if let Some(t) = tray_weak.upgrade() {
                        refresh_ui(&w, &t, &shared);
                    }
                }
                Err(e) => w.set_status_text(
                    format!("{}: {}", i18n::t(lang, "status.delete_failed"), e).into(),
                ),
            }
        });
    }

    {
        let window_weak = window.as_weak();
        window.on_begin_rename(move |alias| {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            w.set_rename_from(alias.clone());
            w.set_rename_to(alias);
        });
    }

    {
        let window_weak = window.as_weak();
        window.on_cancel_rename(move || {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            w.set_rename_from(SharedString::new());
            w.set_rename_to(SharedString::new());
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_confirm_rename(move || {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            let lang = shared.lock().unwrap().lang;
            let from = w.get_rename_from().to_string();
            let to = w.get_rename_to().to_string();
            if from.is_empty() {
                w.set_status_text(i18n::t(lang, "status.no_rename_pending").into());
                return;
            }
            let result = {
                let state = shared.lock().unwrap();
                state.rename(&from, &to)
            };
            match result {
                Ok(()) => {
                    {
                        let mut state = shared.lock().unwrap();
                        if state.provider == Provider::Codex {
                            if let Some(snap) = state.usage.remove(&from) {
                                let mut snap = snap;
                                snap.alias = to.clone();
                                state.usage.insert(to.clone(), snap);
                                let _ = usage::save_cache(&state.usage);
                            }
                        }
                        if state.preview_alias.as_deref() == Some(from.as_str()) {
                            state.preview_alias = Some(to.clone());
                        }
                    }
                    w.set_rename_from(SharedString::new());
                    w.set_rename_to(SharedString::new());
                    w.set_status_text(
                        format!("{} {} → {}", i18n::t(lang, "status.renamed"), from, to).into(),
                    );
                    if let Some(t) = tray_weak.upgrade() {
                        refresh_ui(&w, &t, &shared);
                    }
                }
                Err(e) => w.set_status_text(
                    format!("{}: {}", i18n::t(lang, "status.rename_failed"), e).into(),
                ),
            }
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_import_paste(move || {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            let text = w.get_paste_json().to_string();
            let alias = w.get_paste_alias().to_string();
            let alias_opt = if alias.trim().is_empty() {
                None
            } else {
                Some(alias.as_str())
            };
            finish_import_bytes(&w, &tray_weak, &shared, text.as_bytes(), alias_opt);
        });
    }

    let login_cancel: SharedLoginCancel = Arc::new(Mutex::new(None));

    // The window close button is an explicit application exit. Keep it on
    // the same path as the in-app and tray Quit actions so background work,
    // the tray icon, and the event loop are all stopped together.
    {
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        let running = Arc::clone(&running);
        let login_cancel = Arc::clone(&login_cancel);
        window.window().on_close_requested(move || {
            if let Some(cancel) = login_cancel.lock().unwrap().take() {
                cancel.cancel();
            }
            request_quit(&window_weak, &tray_weak, &running);
            slint::CloseRequestResponse::HideWindow
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        let login_cancel = Arc::clone(&login_cancel);
        window.on_login_browser(move || {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            if w.get_login_running() {
                return;
            }
            let alias = w.get_login_alias().to_string();
            let alias_opt = if alias.trim().is_empty() {
                None
            } else {
                Some(alias)
            };
            let (lang, provider) = {
                let state = shared.lock().unwrap();
                (state.lang, state.provider)
            };
            if provider == Provider::Claude {
                let result = shared.lock().unwrap().claude.open_desktop();
                match result {
                    Ok(()) => w.set_status_text(i18n::t(lang, "status.claude_opened").into()),
                    Err(e) => w.set_status_text(
                        format!("{}: {e}", i18n::t(lang, "status.claude_open_failed")).into(),
                    ),
                }
                return;
            }
            w.set_login_running(true);
            w.set_status_text(i18n::t(lang, "status.browser_login_started").into());

            let (cancel, wait_fn) = match login::start_codex_login() {
                Ok(v) => v,
                Err(e) => {
                    w.set_login_running(false);
                    w.set_status_text(
                        format!("{}: {}", i18n::t(lang, "status.browser_login_failed"), e).into(),
                    );
                    return;
                }
            };

            let shared = Arc::clone(&shared);
            let window_weak = window_weak.clone();
            let tray_weak = tray_weak.clone();
            let login_cancel = Arc::clone(&login_cancel);
            thread::spawn(move || {
                let mut cancel = cancel;
                let mut wait_fn = wait_fn;
                *login_cancel.lock().unwrap() = Some(cancel.clone_for_ui());

                loop {
                    if login_cancel.lock().unwrap().is_none() {
                        return;
                    }
                    match wait_fn(&mut cancel) {
                        Ok(Some(_)) => break,
                        Ok(None) => std::thread::sleep(Duration::from_millis(500)),
                        Err(e) => {
                            let _ = slint::invoke_from_event_loop({
                                let window_weak = window_weak.clone();
                                let login_cancel = Arc::clone(&login_cancel);
                                let shared = Arc::clone(&shared);
                                move || {
                                    *login_cancel.lock().unwrap() = None;
                                    let lang = shared.lock().unwrap().lang;
                                    if let Some(w) = window_weak.upgrade() {
                                        w.set_login_running(false);
                                        w.set_status_text(
                                            format!(
                                                "{}: {}",
                                                i18n::t(lang, "status.browser_login_failed"),
                                                e
                                            )
                                            .into(),
                                        );
                                    }
                                }
                            });
                            return;
                        }
                    }
                }
                let result = {
                    let state = shared.lock().unwrap();
                    match provider {
                        Provider::Codex => state.codex.adopt_live_profile(alias_opt.as_deref()),
                        Provider::Claude => state.claude.save_live_auto(alias_opt.as_deref()),
                    }
                };

                let _ = slint::invoke_from_event_loop(move || {
                    *login_cancel.lock().unwrap() = None;
                    let Some(w) = window_weak.upgrade() else {
                        return;
                    };
                    let lang = shared.lock().unwrap().lang;
                    w.set_login_running(false);
                    match result {
                        Ok(alias) => {
                            w.set_login_alias(SharedString::new());
                            w.set_status_text(
                                format!("{} {}", i18n::t(lang, "status.browser_login_ok"), alias)
                                    .into(),
                            );
                            if let Some(t) = tray_weak.upgrade() {
                                refresh_ui(&w, &t, &shared);
                            }
                            spawn_usage_refresh(
                                w.as_weak(),
                                tray_weak.clone(),
                                Arc::clone(&shared),
                                true,
                            );
                        }
                        Err(e) => {
                            w.set_status_text(
                                format!("{}: {}", i18n::t(lang, "status.browser_login_failed"), e)
                                    .into(),
                            );
                        }
                    }
                });
            });
        });
    }

    {
        let window_weak = window.as_weak();
        tray.on_show_window(move || {
            if let Some(window) = window_weak.upgrade() {
                let _ = window.show();
                window.window().request_redraw();
            }
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let login_cancel = Arc::clone(&login_cancel);
        window.on_cancel_login(move || {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            let mut guard = login_cancel.lock().unwrap();
            if let Some(cancel) = guard.take() {
                cancel.cancel();
                w.set_login_running(false);
                let lang = shared.lock().unwrap().lang;
                w.set_status_text(i18n::t(lang, "status.login_cancelled_by_user").into());
            }
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_import_clipboard(move || {
            let Some(w) = window_weak.upgrade() else {
                return;
            };
            let lang = shared.lock().unwrap().lang;
            let text = match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                Ok(t) => t,
                Err(e) => {
                    w.set_status_text(
                        format!("{}: {}", i18n::t(lang, "status.clipboard_failed"), e).into(),
                    );
                    return;
                }
            };
            if text.trim().is_empty() {
                w.set_status_text(i18n::t(lang, "status.clipboard_empty").into());
                return;
            }
            w.set_paste_json(text.clone().into());
            let alias = w.get_paste_alias().to_string();
            let alias_opt = if alias.trim().is_empty() {
                None
            } else {
                Some(alias.as_str())
            };
            finish_import_bytes(&w, &tray_weak, &shared, text.as_bytes(), alias_opt);
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        window.on_refresh_usage(move || {
            spawn_usage_refresh(
                window_weak.clone(),
                tray_weak.clone(),
                Arc::clone(&shared),
                true,
            );
        });
    }

    {
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        let running = Arc::clone(&running);
        let login_cancel = Arc::clone(&login_cancel);
        window.on_quit(move || {
            if let Some(cancel) = login_cancel.lock().unwrap().take() {
                cancel.cancel();
            }
            request_quit(&window_weak, &tray_weak, &running);
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        tray.on_refresh_usage(move || {
            spawn_usage_refresh(
                window_weak.clone(),
                tray_weak.clone(),
                Arc::clone(&shared),
                true,
            );
        });
    }

    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        tray.on_switch_account(move |alias| {
            apply_switch(&window_weak, &tray_weak, &shared, &alias, false);
        });
    }

    {
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        let running = Arc::clone(&running);
        let login_cancel = Arc::clone(&login_cancel);
        tray.on_quit(move || {
            if let Some(cancel) = login_cancel.lock().unwrap().take() {
                cancel.cancel();
            }
            request_quit(&window_weak, &tray_weak, &running);
        });
    }

    // Background poll current account every 60s.
    {
        let shared = Arc::clone(&shared);
        let window_weak = window.as_weak();
        let tray_weak = tray.as_weak();
        let running = Arc::clone(&running);
        thread::spawn(move || {
            let mut poll_count = 0_u8;
            while running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(60));
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                poll_count = poll_count.wrapping_add(1);
                spawn_usage_refresh(
                    window_weak.clone(),
                    tray_weak.clone(),
                    Arc::clone(&shared),
                    poll_count.is_multiple_of(10),
                );
            }
        });
    }

    // Initial usage fetch for all profiles.
    spawn_usage_refresh(window.as_weak(), tray.as_weak(), Arc::clone(&shared), true);

    window.show()?;
    tray.show()?;
    append_log("ui shown; entering event loop")?;
    // Important: do NOT use run_event_loop() — hiding the last window would quit
    // the process. Tray apps must stay alive until an explicit quit.
    slint::run_event_loop_until_quit()?;
    running.store(false, Ordering::SeqCst);
    let _ = tray.hide();
    let _ = window.hide();
    append_log("event loop exited")?;
    Ok(())
}

fn request_quit(
    window_weak: &slint::Weak<MainWindow>,
    tray_weak: &slint::Weak<AppTray>,
    running: &Arc<AtomicBool>,
) {
    running.store(false, Ordering::SeqCst);
    let _ = append_log("quit requested");
    if let Some(t) = tray_weak.upgrade() {
        // Visible tray keeps the event loop alive — must hide first.
        t.set_tray_visible(false);
        let _ = t.hide();
    }
    if let Some(w) = window_weak.upgrade() {
        let _ = w.hide();
    }
    if let Err(e) = slint::quit_event_loop() {
        let _ = append_log(&format!("quit_event_loop error: {e}"));
    }
}

fn log_path() -> std::path::PathBuf {
    paths::app_home().join("app.log")
}

fn acquire_instance_lock() -> crate::error::AppResult<File> {
    let path = paths::app_home().join("instance.lock");
    let file = profile::open_private_lock(&path)?;
    match file.try_lock_exclusive() {
        Ok(true) => Ok(file),
        Ok(false) => Err(crate::error::AppError::msg(
            "Codex Account Switch 已有实例在运行，拒绝启动第二个实例",
        )),
        Err(error) => Err(crate::error::AppError::msg(format!(
            "检查 Codex Account Switch 实例锁失败: {error}"
        ))),
    }
}

fn append_log(line: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let path = log_path();
    if let Some(parent) = path.parent() {
        profile::ensure_private_dir(parent)?;
    }
    if std::fs::metadata(&path)
        .map(|metadata| metadata.len() > 2 * 1024 * 1024)
        .unwrap_or(false)
    {
        let backup = path.with_extension("log.1");
        if backup.exists() {
            std::fs::remove_file(&backup)?;
        }
        std::fs::rename(&path, backup)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut f = options.open(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(f, "[{ts}] {line}")?;
    Ok(())
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = append_log(&format!("panic: {info}"));
        default_hook(info);
    }));
}

fn home_dir() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn expand_user_path(input: &str) -> std::path::PathBuf {
    let trimmed = input.trim().trim_matches(|c| c == '"' || c == '\'');
    // Finder “拷贝路径” sometimes wraps in quotes; also strip trailing slashes.
    let trimmed = trimmed.trim_end_matches('/');
    if trimmed.is_empty() {
        return paths::codex_home();
    }
    if trimmed == "~" {
        return home_dir();
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    std::path::PathBuf::from(trimmed)
}

fn resolve_browse_start(typed: &str) -> std::path::PathBuf {
    let path = expand_user_path(typed);
    if path.exists() {
        if let Ok(canon) = path.canonicalize() {
            return canon;
        }
    }
    path
}

fn finish_import(
    window: &MainWindow,
    tray_weak: &slint::Weak<AppTray>,
    shared: &Arc<Mutex<AppShared>>,
    path: &std::path::Path,
) {
    let lang = shared.lock().unwrap().lang;
    let result = {
        let state = shared.lock().unwrap();
        state.import_file(path, None)
    };
    match result {
        Ok(alias) => {
            window.set_status_text(
                format!(
                    "{} {} → {}",
                    i18n::t(lang, "status.imported_as"),
                    path.display(),
                    alias
                )
                .into(),
            );
            if let Some(t) = tray_weak.upgrade() {
                refresh_ui(window, &t, shared);
            }
            spawn_usage_refresh(
                window.as_weak(),
                tray_weak.clone(),
                Arc::clone(shared),
                true,
            );
        }
        Err(e) => window
            .set_status_text(format!("{}: {}", i18n::t(lang, "status.import_failed"), e).into()),
    }
}

fn finish_import_result(
    window: &MainWindow,
    tray_weak: &slint::Weak<AppTray>,
    shared: &Arc<Mutex<AppShared>>,
    result: crate::error::AppResult<String>,
) {
    let lang = shared.lock().unwrap().lang;
    match result {
        Ok(alias) => {
            window.set_status_text(
                format!("{} {}", i18n::t(lang, "status.import_text_ok"), alias).into(),
            );
            if let Some(t) = tray_weak.upgrade() {
                refresh_ui(window, &t, shared);
            }
            spawn_usage_refresh(
                window.as_weak(),
                tray_weak.clone(),
                Arc::clone(shared),
                true,
            );
        }
        Err(e) => window
            .set_status_text(format!("{}: {}", i18n::t(lang, "status.import_failed"), e).into()),
    }
}

fn finish_import_bytes(
    window: &MainWindow,
    tray_weak: &slint::Weak<AppTray>,
    shared: &Arc<Mutex<AppShared>>,
    bytes: &[u8],
    alias: Option<&str>,
) {
    let lang = shared.lock().unwrap().lang;
    let result = {
        let state = shared.lock().unwrap();
        state.import_bytes(bytes, alias)
    };
    match result {
        Ok(alias) => {
            window.set_paste_json(SharedString::new());
            window.set_paste_alias(SharedString::new());
            window.set_status_text(
                format!("{} {}", i18n::t(lang, "status.import_text_ok"), alias).into(),
            );
            if let Some(t) = tray_weak.upgrade() {
                refresh_ui(window, &t, shared);
            }
            spawn_usage_refresh(
                window.as_weak(),
                tray_weak.clone(),
                Arc::clone(shared),
                true,
            );
        }
        Err(e) => window.set_status_text(
            format!("{}: {}", i18n::t(lang, "status.import_text_failed"), e).into(),
        ),
    }
}

fn apply_switch(
    window_weak: &slint::Weak<MainWindow>,
    tray_weak: &slint::Weak<AppTray>,
    shared: &Arc<Mutex<AppShared>>,
    alias: &str,
    restart: bool,
) {
    let result = {
        let state = shared.lock().unwrap();
        state.use_profile(alias)
    };
    let (lang, provider) = {
        let state = shared.lock().unwrap();
        (state.lang, state.provider)
    };
    let Some(w) = window_weak.upgrade() else {
        return;
    };
    match result {
        Ok(()) => {
            shared.lock().unwrap().preview_alias = Some(alias.to_string());
            if let Some(t) = tray_weak.upgrade() {
                refresh_ui(&w, &t, shared);
            }
            spawn_usage_refresh(
                window_weak.clone(),
                tray_weak.clone(),
                Arc::clone(shared),
                false,
            );

            if restart && provider == Provider::Codex {
                w.set_status_text(
                    i18n::t(lang, "status.switching_restart")
                        .replace("{alias}", alias)
                        .into(),
                );
                let window_weak = window_weak.clone();
                let alias = alias.to_string();
                let shared = Arc::clone(shared);
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(300));
                    let lang = shared.lock().unwrap().lang;
                    let msg = match restart::restart_codex_app() {
                        Ok(m) => format!(
                            "{}; {}",
                            i18n::t(lang, "status.switch_ok_restart").replace("{alias}", &alias),
                            m
                        ),
                        Err(e) => format!(
                            "{}; {}: {}",
                            i18n::t(lang, "status.switch_ok_restart").replace("{alias}", &alias),
                            i18n::t(lang, "status.restart_failed_manual"),
                            e
                        ),
                    };
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = window_weak.upgrade() {
                            w.set_status_text(msg.into());
                        }
                    });
                });
            } else if provider == Provider::Claude {
                w.set_status_text(
                    i18n::t(lang, "status.claude_switched")
                        .replace("{alias}", alias)
                        .into(),
                );
            } else {
                w.set_status_text(
                    i18n::t(lang, "status.switch_hint")
                        .replace("{alias}", alias)
                        .into(),
                );
            }
        }
        Err(e) => {
            w.set_status_text(format!("{}: {}", i18n::t(lang, "status.switch_failed"), e).into())
        }
    }
}

fn spawn_usage_refresh(
    window_weak: slint::Weak<MainWindow>,
    tray_weak: slint::Weak<AppTray>,
    shared: Arc<Mutex<AppShared>>,
    all: bool,
) {
    let refresh_flag = {
        let state = shared.lock().unwrap();
        Arc::clone(&state.usage_refreshing)
    };
    if refresh_flag
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    thread::spawn(move || {
        let _refresh_guard = UsageRefreshGuard(refresh_flag);
        let (lang, provider) = {
            let state = shared.lock().unwrap();
            (state.lang, state.provider)
        };
        if provider == Provider::Claude {
            let _ = slint::invoke_from_event_loop(move || {
                if let (Some(w), Some(t)) = (window_weak.upgrade(), tray_weak.upgrade()) {
                    refresh_ui(&w, &t, &shared);
                }
            });
            return;
        }
        let jobs: Vec<(String, Option<String>, Option<String>)> = {
            let state = shared.lock().unwrap();
            let profiles = state.codex.store().list().unwrap_or_default();
            let current = state.codex.store().read_current_alias();
            profiles
                .into_iter()
                .filter(|p| all || current.as_deref() == Some(p.alias.as_str()))
                .filter_map(|p| {
                    let creds = state.codex.store().read_profile_creds(&p.alias).ok()?;
                    Some((
                        p.alias,
                        creds.access_token,
                        creds.account_id.or_else(|| {
                            if p.identity.account_id.is_empty() {
                                None
                            } else {
                                Some(p.identity.account_id.clone())
                            }
                        }),
                    ))
                })
                .collect()
        };

        let mut updates = Vec::new();
        for (alias, token, account_id) in jobs {
            let previous = {
                let state = shared.lock().unwrap();
                state.usage.get(&alias).cloned()
            };
            let snap = match (token, account_id) {
                (Some(token), Some(account_id)) => {
                    let previous_token_profile = previous
                        .as_ref()
                        .and_then(|snapshot| snapshot.token_profile.clone());
                    let usage_result = usage::fetch_usage(&alias, &token, &account_id);
                    let token_result =
                        usage::fetch_token_usage_profile(&alias, &token, &account_id);
                    match (usage_result, token_result) {
                        (Ok(mut s), Ok(profile)) => {
                            s.token_profile = Some(profile);
                            s
                        }
                        (Ok(mut s), Err(error)) => {
                            s.token_profile = previous_token_profile;
                            let _ = append_log(&format!(
                                "token activity refresh failed for {alias}: {error}"
                            ));
                            s
                        }
                        (Err(e), _) => {
                            let mut failed = previous.unwrap_or_default();
                            failed.alias = alias.clone();
                            failed.error = e.to_string();
                            failed.fetched_at = chrono::Utc::now().timestamp();
                            failed
                        }
                    }
                }
                _ => {
                    let mut failed = previous.unwrap_or_default();
                    failed.alias = alias.clone();
                    failed.error = i18n::t(lang, "status.no_token").into();
                    failed.fetched_at = chrono::Utc::now().timestamp();
                    failed
                }
            };
            updates.push(snap);
        }

        {
            let mut state = shared.lock().unwrap();
            let valid_aliases: HashSet<String> = state
                .codex
                .store()
                .list()
                .unwrap_or_default()
                .into_iter()
                .map(|profile| profile.alias)
                .collect();
            for snap in updates {
                if valid_aliases.contains(&snap.alias) {
                    state.usage.insert(snap.alias.clone(), snap);
                }
            }
            let _ = usage::save_cache(&state.usage);
        }

        let _ = slint::invoke_from_event_loop(move || {
            let lang = shared.lock().unwrap().lang;
            if let (Some(w), Some(t)) = (window_weak.upgrade(), tray_weak.upgrade()) {
                refresh_ui(&w, &t, &shared);
                w.set_status_text(i18n::t(lang, "status.usage_updated").into());
            }
        });
    });
}

struct UsageRefreshGuard(Arc<AtomicBool>);

impl Drop for UsageRefreshGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn refresh_ui(window: &MainWindow, tray: &AppTray, shared: &Arc<Mutex<AppShared>>) {
    let state = shared.lock().unwrap();
    let lang = state.lang;
    window.set_provider_index(match state.provider {
        Provider::Codex => 0,
        Provider::Claude => 1,
    });
    window.set_can_restore(state.can_restore_last_removed());
    let profiles = state.profiles().unwrap_or_default();
    let active_alias = state.current_alias().unwrap_or_default();
    let available_aliases = profiles
        .iter()
        .map(|profile| profile.alias.clone())
        .collect::<Vec<_>>();
    let detail_alias = resolve_detail_alias(
        &available_aliases,
        &active_alias,
        state.preview_alias.as_deref(),
    );
    window.set_active_alias(active_alias.clone().into());
    let supports_usage = state.provider == Provider::Codex;
    let now = chrono::Utc::now().timestamp();

    let rows: Vec<AccountRow> = profiles
        .iter()
        .map(|p| {
            let snap = supports_usage.then(|| state.usage.get(&p.alias)).flatten();
            let (usage_summary, usage_attention) = snap
                .map(|s| {
                    format_usage_row(
                        s,
                        now,
                        i18n::t(lang, "usage.pending_refresh"),
                        i18n::t(lang, "usage.no_window_short"),
                    )
                })
                .unwrap_or_else(|| (i18n::t(lang, "usage.pending_refresh").into(), false));
            AccountRow {
                alias: p.alias.clone().into(),
                email: p.identity.email.clone().into(),
                plan: p.identity.plan.clone().into(),
                account_id: short_account_id(&p.identity.account_id).into(),
                is_current: p.is_current,
                is_valid: p.is_valid,
                primary_pct: snap.map(|s| s.primary.used_percent as f32).unwrap_or(0.0),
                secondary_pct: snap.map(|s| s.secondary.used_percent as f32).unwrap_or(0.0),
                primary_reset: snap
                    .map(|s| format_window_reset_short(&s.primary))
                    .unwrap_or_else(|| "—".into())
                    .into(),
                secondary_reset: snap
                    .map(|s| format_window_reset_short(&s.secondary))
                    .unwrap_or_else(|| "—".into())
                    .into(),
                primary_available: snap.is_some_and(|s| {
                    window_has_data(&s.primary) && !window_is_expired(&s.primary, now)
                }),
                secondary_available: snap.is_some_and(|s| {
                    window_has_data(&s.secondary) && !window_is_expired(&s.secondary, now)
                }),
                credits: snap
                    .map(|s| s.credits_balance.clone())
                    .unwrap_or_default()
                    .into(),
                reset_credits: format_reset_credits_available(
                    snap.and_then(|s| s.reset_credits_available),
                    lang,
                )
                .into(),
                usage_summary: usage_summary.into(),
                usage_updated: snap
                    .map(|s| {
                        format!(
                            "{} {}",
                            i18n::t(lang, "usage.updated_prefix"),
                            format_reset(Some(s.fetched_at))
                        )
                    })
                    .unwrap_or_default()
                    .into(),
                usage_attention,
                usage_error: snap
                    .map(|s| {
                        let e = s.localized_error(lang);
                        let e = e.trim();
                        if e.is_empty() {
                            String::new()
                        } else if e.chars().count() > 36 {
                            e.chars().take(36).collect::<String>() + "…"
                        } else {
                            e.to_string()
                        }
                    })
                    .unwrap_or_default()
                    .into(),
            }
        })
        .collect();

    let model = ModelRc::new(VecModel::from(rows.clone()));
    window.set_accounts(model.clone());
    tray.set_accounts(model);

    // Set localized strings
    window.set_tr_app_title(i18n::t(lang, "app.title").into());
    window.set_tr_tray_refresh(i18n::t(lang, "tray.refresh").into());
    window.set_tr_tray_switch(i18n::t(lang, "tray.switch").into());
    window.set_tr_tray_quit(i18n::t(lang, "tray.quit").into());
    window.set_tr_btn_about(i18n::t(lang, "btn.about").into());
    window.set_tr_about_description(i18n::t(lang, "about.description").into());
    window.set_tr_about_third_party(i18n::t(lang, "about.third_party").into());
    window.set_tr_about_notices(i18n::t(lang, "about.notices").into());
    window.set_tr_about_close(i18n::t(lang, "about.close").into());
    window.set_tr_btn_restore(i18n::t(lang, "btn.restore").into());
    window.set_tr_btn_refresh_usage(i18n::t(lang, "btn.refresh_usage").into());
    window.set_tr_btn_refresh_list(i18n::t(lang, "btn.refresh_list").into());
    window.set_tr_btn_quit(i18n::t(lang, "btn.quit").into());
    window.set_tr_active_title(i18n::t(lang, "active.title").into());
    window.set_tr_detail_title(i18n::t(lang, "detail.title").into());
    window.set_tr_active_no_account(i18n::t(lang, "active.no_account").into());
    window.set_tr_active_hint(i18n::t(lang, "active.hint").into());
    window.set_tr_active_restart_hint(i18n::t(lang, "active.restart_hint").into());
    window.set_tr_token_title(i18n::t(lang, "token.title").into());
    window.set_tr_token_lifetime(i18n::t(lang, "token.lifetime").into());
    window.set_tr_token_peak(i18n::t(lang, "token.peak").into());
    window.set_tr_token_streak(i18n::t(lang, "token.streak").into());
    window.set_tr_token_longest(i18n::t(lang, "token.longest").into());
    window.set_tr_token_last_seven(i18n::t(lang, "token.last_seven").into());
    window.set_tr_activity_open(i18n::t(lang, "activity.open").into());
    window.set_tr_activity_title(i18n::t(lang, "activity.title").into());
    window.set_tr_activity_subtitle(i18n::t(lang, "activity.subtitle").into());
    window.set_tr_activity_back(i18n::t(lang, "activity.back").into());
    window.set_tr_activity_daily(i18n::t(lang, "activity.daily").into());
    window.set_tr_activity_weekly(i18n::t(lang, "activity.weekly").into());
    window.set_tr_activity_cumulative(i18n::t(lang, "activity.cumulative").into());
    window.set_tr_activity_longest_streak(i18n::t(lang, "activity.longest_streak").into());
    window.set_tr_activity_chart_title(i18n::t(lang, "activity.chart_title").into());
    window.set_tr_activity_chart_hint(i18n::t(lang, "activity.chart_hint").into());
    window.set_tr_activity_insights(i18n::t(lang, "activity.insights").into());
    window.set_tr_activity_fast_mode(i18n::t(lang, "activity.fast_mode").into());
    window.set_tr_activity_reasoning(i18n::t(lang, "activity.reasoning").into());
    window.set_tr_activity_unique_skills(i18n::t(lang, "activity.unique_skills").into());
    window.set_tr_activity_total_skills(i18n::t(lang, "activity.total_skills").into());
    window.set_tr_activity_threads(i18n::t(lang, "activity.threads").into());
    window.set_tr_activity_top(i18n::t(lang, "activity.top").into());
    window.set_tr_activity_runs(i18n::t(lang, "activity.runs").into());
    window.set_tr_activity_no_data(i18n::t(lang, "activity.no_data").into());
    window.set_tr_import_tab_login(i18n::t(lang, "import.tab.login").into());
    window.set_tr_import_tab_save(i18n::t(lang, "import.tab.save").into());
    window.set_tr_import_tab_path(i18n::t(lang, "import.tab.path").into());
    window.set_tr_import_tab_paste(i18n::t(lang, "import.tab.paste").into());
    window.set_tr_login_hint(i18n::t(lang, "login.hint").into());
    window.set_tr_login_alias_placeholder(i18n::t(lang, "login.alias_placeholder").into());
    window.set_tr_login_open(i18n::t(lang, "login.open").into());
    window.set_tr_login_running(i18n::t(lang, "login.running").into());
    window.set_tr_login_cancel(i18n::t(lang, "login.cancel").into());
    window.set_tr_save_alias_placeholder(i18n::t(lang, "save.alias_placeholder").into());
    window.set_tr_save_button(i18n::t(lang, "save.button").into());
    window.set_tr_save_import(i18n::t(lang, "save.import").into());
    window.set_tr_path_placeholder(i18n::t(lang, "path.placeholder").into());
    window.set_tr_path_open(i18n::t(lang, "path.open").into());
    window.set_tr_paste_placeholder(i18n::t(lang, "paste.placeholder").into());
    window.set_tr_paste_alias_placeholder(i18n::t(lang, "paste.alias_placeholder").into());
    window.set_tr_paste_import(i18n::t(lang, "paste.import").into());
    window.set_tr_paste_clipboard(i18n::t(lang, "paste.clipboard").into());
    window.set_tr_rename_title(i18n::t(lang, "rename.title").into());
    window.set_tr_rename_new_placeholder(i18n::t(lang, "rename.new_placeholder").into());
    window.set_tr_rename_confirm(i18n::t(lang, "rename.confirm").into());
    window.set_tr_rename_cancel(i18n::t(lang, "rename.cancel").into());
    window.set_tr_profiles_title(i18n::t(lang, "profiles.title").into());
    window.set_tr_profiles_empty(i18n::t(lang, "profiles.empty").into());
    window.set_tr_profiles_empty_hint(
        i18n::t(
            lang,
            if state.provider == Provider::Claude {
                "profiles.empty_hint_claude"
            } else {
                "profiles.empty_hint"
            },
        )
        .into(),
    );
    window.set_tr_profiles_count(i18n::t(lang, "profiles.count").into());
    window.set_tr_btn_switch(i18n::t(lang, "btn.switch").into());
    window.set_tr_btn_restart(i18n::t(lang, "btn.restart").into());
    window.set_tr_btn_rename(i18n::t(lang, "btn.rename").into());
    window.set_tr_btn_delete(i18n::t(lang, "btn.delete").into());
    window.set_tr_label_in_use(i18n::t(lang, "label.in_use").into());
    window.set_tr_label_preview(i18n::t(lang, "label.preview").into());
    window.set_tr_label_no_email(i18n::t(lang, "label.no_email").into());
    window.set_tr_label_invalid(i18n::t(lang, "label.invalid").into());
    window.set_tr_gauge_5h(i18n::t(lang, "gauge.5h").into());
    window.set_tr_gauge_weekly(i18n::t(lang, "gauge.weekly").into());
    window.set_tr_usage_reset(i18n::t(lang, "usage.reset_prefix").into());
    window.set_tr_claude_session_hint(i18n::t(lang, "claude.session_hint").into());
    window.set_tr_claude_login_hint(i18n::t(lang, "claude.login_hint").into());
    window.set_tr_claude_paste_placeholder(i18n::t(lang, "claude.paste_placeholder").into());
    window.set_tr_claude_open(i18n::t(lang, "claude.open").into());

    tray.set_tr_tray_show(i18n::t(lang, "tray.show").into());
    tray.set_tr_tray_refresh(i18n::t(lang, "tray.refresh").into());
    tray.set_tr_tray_switch(i18n::t(lang, "tray.switch").into());
    tray.set_tr_tray_quit(i18n::t(lang, "tray.quit").into());

    if let Some(current) = profiles.iter().find(|p| p.alias == detail_alias) {
        let snap = supports_usage
            .then(|| state.usage.get(&current.alias))
            .flatten();
        window.set_current_alias(current.alias.clone().into());
        window.set_current_email(current.identity.email.clone().into());
        window.set_current_plan(
            snap.and_then(|s| {
                if s.plan_type.is_empty() {
                    None
                } else {
                    Some(s.plan_type.clone())
                }
            })
            .unwrap_or_else(|| current.identity.plan.clone())
            .into(),
        );
        let primary_available = snap
            .is_some_and(|s| window_has_data(&s.primary) && !window_is_expired(&s.primary, now));
        let secondary_available = snap.is_some_and(|s| {
            window_has_data(&s.secondary) && !window_is_expired(&s.secondary, now)
        });
        window.set_current_primary_available(primary_available);
        window.set_current_secondary_available(secondary_available);
        window.set_current_primary(if primary_available {
            snap.map(|s| s.primary.used_percent as f32).unwrap_or(0.0)
        } else {
            0.0
        });
        window.set_current_secondary(if secondary_available {
            snap.map(|s| s.secondary.used_percent as f32).unwrap_or(0.0)
        } else {
            0.0
        });
        window.set_current_primary_reset(
            snap.map(|s| {
                if window_is_expired(&s.primary, now) {
                    i18n::t(lang, "usage.pending_refresh").into()
                } else {
                    format_window_reset(
                        &s.primary,
                        i18n::t(lang, "usage.no_window"),
                        i18n::t(lang, "usage.reset_prefix"),
                        i18n::t(lang, "usage.reset_unknown"),
                    )
                }
            })
            .unwrap_or_else(|| i18n::t(lang, "usage.no_window").into())
            .into(),
        );
        window.set_current_secondary_reset(
            snap.map(|s| {
                if window_is_expired(&s.secondary, now) {
                    i18n::t(lang, "usage.pending_refresh").into()
                } else {
                    format_window_reset(
                        &s.secondary,
                        i18n::t(lang, "usage.no_window"),
                        i18n::t(lang, "usage.reset_prefix"),
                        i18n::t(lang, "usage.reset_unknown"),
                    )
                }
            })
            .unwrap_or_else(|| i18n::t(lang, "usage.no_window").into())
            .into(),
        );
        window.set_current_credits(
            snap.map(|s| s.credits_balance.clone())
                .unwrap_or_default()
                .into(),
        );
        window.set_current_reset_credits(
            format_reset_credits_available(snap.and_then(|s| s.reset_credits_available), lang)
                .into(),
        );
        window.set_current_token_lifetime(
            snap.and_then(|s| s.token_profile.as_ref())
                .and_then(|p| p.lifetime_tokens)
                .map(|v| format_tokens_localized(v, lang))
                .unwrap_or_default()
                .into(),
        );
        window.set_current_token_peak(
            snap.and_then(|s| s.token_profile.as_ref())
                .and_then(|p| p.peak_daily_tokens)
                .map(|v| format_tokens_localized(v, lang))
                .unwrap_or_default()
                .into(),
        );
        window.set_current_token_streak(
            snap.and_then(|s| s.token_profile.as_ref())
                .and_then(|p| p.current_streak_days)
                .map(|v| format_duration_days(v, lang))
                .unwrap_or_default()
                .into(),
        );
        window.set_current_token_longest(
            snap.and_then(|s| s.token_profile.as_ref())
                .and_then(|p| p.longest_running_turn_sec)
                .map(|v| format_duration_seconds(v, lang))
                .unwrap_or_default()
                .into(),
        );
        let trend = snap
            .and_then(|s| s.token_profile.as_ref())
            .map(|profile| recent_daily_tokens(profile, 7))
            .unwrap_or_default();
        let trend_max = trend.iter().copied().max().unwrap_or(0);
        let normalized: Vec<f32> = trend
            .iter()
            .map(|value| {
                if trend_max == 0 {
                    0.0
                } else {
                    *value as f32 / trend_max as f32
                }
            })
            .collect();
        let week_total: i64 = trend.iter().sum();
        window.set_current_token_trend(ModelRc::new(VecModel::from(normalized)));
        window.set_current_token_week(
            if week_total > 0 {
                format_tokens_localized(week_total, lang)
            } else {
                String::new()
            }
            .into(),
        );
        let token_profile = snap.and_then(|s| s.token_profile.as_ref());
        window.set_current_token_longest_streak(
            token_profile
                .and_then(|profile| profile.longest_streak_days)
                .map(|value| format_duration_days(value, lang))
                .unwrap_or_default()
                .into(),
        );
        window.set_current_token_daily(ModelRc::new(VecModel::from(
            token_profile
                .map(|profile| normalized_daily_tokens(profile, 364))
                .unwrap_or_default(),
        )));
        window.set_current_token_weekly(ModelRc::new(VecModel::from(
            token_profile
                .map(|profile| {
                    normalized_usage_buckets(profile.weekly_usage_buckets.as_deref(), 52)
                })
                .unwrap_or_default(),
        )));
        window.set_current_token_cumulative(ModelRc::new(VecModel::from(
            token_profile
                .map(|profile| {
                    normalized_cumulative_buckets(
                        profile.cumulative_daily_usage_buckets.as_deref(),
                        52,
                    )
                })
                .unwrap_or_default(),
        )));
        window.set_current_fast_mode(
            token_profile
                .and_then(|profile| profile.fast_mode_usage_percentage)
                .map(|value| format!("{value:.0}%"))
                .unwrap_or_else(|| "—".into())
                .into(),
        );
        window.set_current_reasoning(
            token_profile
                .and_then(|profile| {
                    let effort = profile.most_used_reasoning_effort.as_deref()?;
                    let percentage = profile.most_used_reasoning_effort_percentage?;
                    Some(format!(
                        "{} · {percentage:.0}%",
                        format_reasoning_effort(effort, lang)
                    ))
                })
                .unwrap_or_else(|| "—".into())
                .into(),
        );
        window.set_current_unique_skills(
            token_profile
                .and_then(|profile| profile.unique_skills_used)
                .map(format_integer)
                .unwrap_or_else(|| "—".into())
                .into(),
        );
        window.set_current_total_skills(
            token_profile
                .and_then(|profile| profile.total_skills_used)
                .map(format_integer)
                .unwrap_or_else(|| "—".into())
                .into(),
        );
        window.set_current_total_threads(
            token_profile
                .and_then(|profile| profile.total_threads)
                .map(format_integer)
                .unwrap_or_else(|| "—".into())
                .into(),
        );
        let invocations = token_profile
            .and_then(|profile| profile.top_invocations.as_deref())
            .unwrap_or_default()
            .iter()
            .take(5)
            .map(|invocation| {
                let name = [
                    invocation.skill_name.as_deref(),
                    invocation.plugin_name.as_deref(),
                    invocation.skill_id.as_deref(),
                    invocation.plugin_id.as_deref(),
                ]
                .into_iter()
                .flatten()
                .find(|value| !value.trim().is_empty())
                .unwrap_or("—");
                ActivityInvocation {
                    name: name.into(),
                    kind: invocation.r#type.to_uppercase().into(),
                    count: format_integer(invocation.usage_count).into(),
                }
            })
            .collect::<Vec<_>>();
        window.set_current_top_invocations(ModelRc::new(VecModel::from(invocations)));
    } else {
        window.set_current_alias(SharedString::new());
        window.set_current_email(SharedString::new());
        window.set_current_plan(SharedString::new());
        window.set_current_primary(0.0);
        window.set_current_secondary(0.0);
        window.set_current_primary_reset("—".into());
        window.set_current_secondary_reset("—".into());
        window.set_current_credits(SharedString::new());
        window.set_current_reset_credits(SharedString::new());
        window.set_current_token_lifetime(SharedString::new());
        window.set_current_token_peak(SharedString::new());
        window.set_current_token_streak(SharedString::new());
        window.set_current_token_longest(SharedString::new());
        window.set_current_token_trend(ModelRc::new(VecModel::from(Vec::<f32>::new())));
        window.set_current_token_week(SharedString::new());
        window.set_current_token_longest_streak(SharedString::new());
        window.set_current_token_daily(ModelRc::new(VecModel::from(Vec::<f32>::new())));
        window.set_current_token_weekly(ModelRc::new(VecModel::from(Vec::<f32>::new())));
        window.set_current_token_cumulative(ModelRc::new(VecModel::from(Vec::<f32>::new())));
        window.set_current_fast_mode(SharedString::new());
        window.set_current_reasoning(SharedString::new());
        window.set_current_unique_skills(SharedString::new());
        window.set_current_total_skills(SharedString::new());
        window.set_current_total_threads(SharedString::new());
        window.set_current_top_invocations(ModelRc::new(VecModel::from(
            Vec::<ActivityInvocation>::new(),
        )));
        window.set_current_primary_available(false);
        window.set_current_secondary_available(false);
    }

    if let Some(active) = profiles
        .iter()
        .find(|profile| profile.alias == active_alias)
    {
        let snap = supports_usage
            .then(|| state.usage.get(&active.alias))
            .flatten();
        let tip = if state.provider == Provider::Claude {
            format!("Claude Desktop · {}", active.alias)
        } else {
            snap.map(|snapshot| {
                format_usage_short(
                    snapshot,
                    i18n::t(lang, "usage.unavailable"),
                    i18n::t(lang, "usage.no_window_short"),
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "{} · {}",
                    active.alias,
                    i18n::t(lang, "usage.pending_refresh")
                )
            })
        };
        tray.set_tray_tooltip(tip.into());
        tray.set_tray_title(if state.provider == Provider::Claude {
            "Claude".into()
        } else {
            format!(
                "{:.0}%/{:.0}%",
                snap.map(|snapshot| snapshot.primary.used_percent)
                    .unwrap_or(0.0),
                snap.map(|snapshot| snapshot.secondary.used_percent)
                    .unwrap_or(0.0)
            )
            .into()
        });
    } else {
        tray.set_tray_tooltip(if state.provider == Provider::Claude {
            "Claude Desktop Switch".into()
        } else {
            "Codex Account Switch".into()
        });
        tray.set_tray_title(if state.provider == Provider::Claude {
            "Claude".into()
        } else {
            "Codex".into()
        });
    }
}

fn resolve_detail_alias(
    available_aliases: &[String],
    active_alias: &str,
    preview_alias: Option<&str>,
) -> String {
    preview_alias
        .filter(|preview| available_aliases.iter().any(|alias| alias == *preview))
        .unwrap_or(active_alias)
        .to_string()
}

fn format_reset_credits_available(count: Option<i64>, lang: i18n::Language) -> String {
    count
        .map(|count| {
            format!(
                "{} · {count}",
                i18n::t(lang, "usage.reset_credits_available")
            )
        })
        .unwrap_or_default()
}

fn format_tokens_localized(value: i64, lang: i18n::Language) -> String {
    match lang {
        i18n::Language::ZhCn => format_east_asian_number(value, "亿", "万"),
        i18n::Language::ZhTw => format_east_asian_number(value, "億", "萬"),
        i18n::Language::Ja => format_east_asian_number(value, "億", "万"),
        i18n::Language::Ko => format_east_asian_number(value, "억", "만"),
        i18n::Language::Es if value >= 1_000_000 => {
            format!("{:.1} M", value as f64 / 1_000_000.0)
        }
        i18n::Language::Es if value >= 1_000 => {
            format!("{:.1} mil", value as f64 / 1_000.0)
        }
        _ if value >= 1_000_000 => format!("{:.1} M", value as f64 / 1_000_000.0),
        _ if value >= 1_000 => format!("{:.1} K", value as f64 / 1_000.0),
        _ => value.to_string(),
    }
}

fn format_east_asian_number(value: i64, large_unit: &str, small_unit: &str) -> String {
    if value >= 100_000_000 {
        format!("{:.1}{large_unit}", value as f64 / 100_000_000.0)
    } else if value >= 10_000 {
        format!("{:.1}{small_unit}", value as f64 / 10_000.0)
    } else {
        value.to_string()
    }
}

fn format_duration_days(value: i64, lang: i18n::Language) -> String {
    match lang {
        i18n::Language::ZhCn | i18n::Language::ZhTw => format!("{value} 天"),
        i18n::Language::Ja => format!("{value} 日"),
        i18n::Language::Ko => format!("{value}일"),
        i18n::Language::Fr => format!("{value} j"),
        i18n::Language::Es => format!("{value} días"),
        i18n::Language::En => format!("{value} days"),
    }
}

fn format_duration_seconds(value: i64, lang: i18n::Language) -> String {
    let hours = value.max(0) / 3600;
    let minutes = (value.max(0) % 3600) / 60;
    if hours > 0 {
        return match lang {
            i18n::Language::ZhCn | i18n::Language::ZhTw => {
                format!("{hours} 小时 {minutes} 分")
            }
            i18n::Language::Ja => format!("{hours}時間{minutes}分"),
            i18n::Language::Ko => format!("{hours}시간 {minutes}분"),
            i18n::Language::Fr => format!("{hours} h {minutes} min"),
            i18n::Language::Es => format!("{hours} h {minutes} min"),
            i18n::Language::En => format!("{hours}h {minutes}m"),
        };
    }
    match lang {
        i18n::Language::ZhCn | i18n::Language::ZhTw => format!("{value} 秒"),
        i18n::Language::Ja => format!("{value} 秒"),
        i18n::Language::Ko => format!("{value}초"),
        i18n::Language::Fr => format!("{value} s"),
        i18n::Language::Es => format!("{value} s"),
        i18n::Language::En => format!("{value} s"),
    }
}

fn format_integer(value: i64) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    if negative {
        format!("-{grouped}")
    } else {
        grouped
    }
}

fn format_reasoning_effort(value: &str, lang: i18n::Language) -> String {
    let level = match value.to_ascii_lowercase().as_str() {
        "minimal" | "none" => 0,
        "low" => 1,
        "medium" => 2,
        "high" => 3,
        "xhigh" | "extra_high" | "very_high" => 4,
        _ => return value.to_string(),
    };
    match lang {
        i18n::Language::ZhCn => ["最小", "低", "中", "高", "极高"][level].into(),
        i18n::Language::ZhTw => ["最小", "低", "中", "高", "極高"][level].into(),
        i18n::Language::Ja => ["最小", "低", "中", "高", "最高"][level].into(),
        i18n::Language::Ko => ["최소", "낮음", "중간", "높음", "매우 높음"][level].into(),
        i18n::Language::Fr => ["Minimal", "Faible", "Moyen", "Élevé", "Très élevé"][level].into(),
        i18n::Language::Es => ["Mínimo", "Bajo", "Medio", "Alto", "Muy alto"][level].into(),
        i18n::Language::En => ["Minimal", "Low", "Medium", "High", "Extra high"][level].into(),
    }
}

#[cfg(test)]
mod detail_preview_tests {
    use super::{format_reset_credits_available, resolve_detail_alias};
    use crate::i18n::Language;

    #[test]
    fn valid_preview_alias_controls_detail_panel() {
        let aliases = vec!["active".to_string(), "preview".to_string()];
        assert_eq!(
            resolve_detail_alias(&aliases, "active", Some("preview")),
            "preview"
        );
    }

    #[test]
    fn stale_preview_alias_falls_back_to_active_account() {
        let aliases = vec!["active".to_string()];
        assert_eq!(
            resolve_detail_alias(&aliases, "active", Some("removed")),
            "active"
        );
    }

    #[test]
    fn reset_credit_label_keeps_zero_visible_and_hides_unknown() {
        assert_eq!(
            format_reset_credits_available(Some(0), Language::ZhCn),
            "可用重置次数 · 0"
        );
        assert_eq!(
            format_reset_credits_available(Some(2), Language::En),
            "Resets available · 2"
        );
        assert_eq!(format_reset_credits_available(None, Language::ZhCn), "");
    }
}
