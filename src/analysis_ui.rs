use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use chrono::{NaiveDate, Utc};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::analytics::{self, Panel, Snapshot};
use crate::i18n::{t, Language};
use crate::{
    AnalysisBar, AnalysisLegend, AnalysisRank, AnalysisSegment, AppShared, MainWindow, Provider,
};

// 缓存按真实账号、日期窗口分开；别名只负责当前页面选择。
type CacheKey = (String, i32, NaiveDate);

#[derive(Default)]
pub struct State {
    cache: HashMap<CacheKey, Snapshot>,
    active: Option<(String, CacheKey)>,
    pending: HashSet<CacheKey>,
    editing_account: Option<String>,
}

pub fn connect(window: &MainWindow, shared: Arc<Mutex<AppShared>>) {
    connect_valuation(window, Arc::clone(&shared));
    let weak = window.as_weak();
    let state = Arc::clone(&shared);
    window.on_load_analysis(move |force| {
        if let Some(window) = weak.upgrade() {
            load(&window, Arc::clone(&state), force);
        }
    });
    let weak = window.as_weak();
    window.on_update_analysis(move || {
        if let Some(window) = weak.upgrade() {
            render_locked(&window, &shared.lock().unwrap());
        }
    });
}

fn load(window: &MainWindow, shared: Arc<Mutex<AppShared>>, force: bool) {
    let alias = window.get_current_alias().to_string();
    let days = if window.get_analysis_range() == 0 {
        7
    } else {
        30
    };
    let end = Utc::now().date_naive();
    let (token, key, previous) = {
        let mut state = shared.lock().unwrap();
        if state.provider != Provider::Codex {
            return;
        }
        let creds = state.codex.store().read_profile_creds(&alias);
        let (token, account) = match creds {
            Ok(creds) => match (creds.access_token, creds.account_id) {
                (Some(token), Some(account)) if !token.is_empty() && !account.is_empty() => {
                    (token, account)
                }
                _ => {
                    state.analysis.active = None;
                    clear(window, t(state.lang, "status.no_token"));
                    return;
                }
            },
            Err(_) => {
                state.analysis.active = None;
                clear(window, t(state.lang, "active.no_account"));
                return;
            }
        };
        let key = (account, days, end);
        if state.analysis.active.as_ref().map(|(_, k)| &k.0) != Some(&key.0) {
            window.set_val_editing(false);
            window.set_val_error("".into());
            state.analysis.editing_account = None;
        }
        state.analysis.active = Some((alias, key.clone()));
        state.analysis.cache.retain(|(_, _, date), _| *date == end);
        let previous = state.analysis.cache.get(&key).cloned().unwrap_or_default();
        render_locked(window, &state);
        let fresh = [
            previous.usage.updated_at,
            previous.activity.updated_at,
            previous.plugins.updated_at,
            previous.skills.updated_at,
            previous.tokens.updated_at,
        ]
        .iter()
        .all(|stamp| Utc::now().timestamp() - stamp < 60);
        let failed = [
            &previous.usage.error,
            &previous.activity.error,
            &previous.plugins.error,
            &previous.skills.error,
            &previous.tokens.error,
        ]
        .iter()
        .any(|error| !error.is_empty());
        if (!force && fresh && !failed) || !state.analysis.pending.insert(key.clone()) {
            return;
        }
        render_locked(window, &state);
        (token, key, previous)
    };
    let weak = window.as_weak();
    std::thread::spawn(move || {
        let snapshot = analytics::fetch(&token, &key.0, key.2, key.1, previous);
        let _ = slint::invoke_from_event_loop(move || {
            let mut state = shared.lock().unwrap();
            state.analysis.pending.remove(&key);
            state.analysis.cache.insert(key, snapshot);
            if let Some(window) = weak.upgrade() {
                render_locked(&window, &state);
            }
        });
    });
}

fn clear(window: &MainWindow, message: &str) {
    clear_valuation(window, message);
    window.set_analysis_bars(model(vec![]));
    window.set_analysis_legend(model(vec![]));
    window.set_analysis_activity_rows(model(vec![]));
    window.set_analysis_tool_rows(model(vec![]));
    window.set_analysis_turn_total("—".into());
    window.set_analysis_tool_total("—".into());
    window.set_analysis_status(message.into());
    window.set_analysis_usage_message("".into());
    window.set_analysis_activity_message("".into());
    window.set_analysis_tool_message("".into());
    window.set_analysis_hover("".into());
    window.set_analysis_max("—".into());
    window.set_analysis_unit("".into());
    window.set_analysis_date_range("".into());
    window.set_analysis_busy(false);
}

pub fn render_locked(window: &MainWindow, state: &AppShared) {
    let lang = state.lang;
    let Some((alias, key)) = &state.analysis.active else {
        clear(window, t(lang, "activity.no_data"));
        return;
    };
    let days = if window.get_analysis_range() == 0 {
        7
    } else {
        30
    };
    let principal_matches = state
        .codex
        .store()
        .read_profile_creds(alias)
        .ok()
        .and_then(|creds| creds.account_id)
        .is_some_and(|account| account == key.0);
    if !principal_matches
        || !selection_matches(
            alias,
            &window.get_current_alias(),
            key.1,
            days,
            state.provider,
        )
    {
        clear(window, t(lang, "activity.no_data"));
        return;
    }
    let empty = Snapshot::default();
    let snapshot = state.analysis.cache.get(key).unwrap_or(&empty);
    let busy = state.analysis.pending.contains(key);
    window.set_analysis_busy(busy);
    window.set_analysis_status(if busy { t(lang, "new.loading") } else { "" }.into());
    let dates = analytics::dates(key.2, days);
    window.set_analysis_date_range(format!("{} — {} · UTC", dates[0], key.2).into());
    render_valuation(window, snapshot, &key.0, &dates, lang, busy);
    window.set_analysis_usage_message(panel_message(&snapshot.usage, lang, busy).into());
    window.set_analysis_activity_message(panel_message(&snapshot.activity, lang, busy).into());
    window.set_analysis_hover("".into());
    window.set_analysis_unit("—".into());
    window.set_analysis_max("—".into());
    window.set_analysis_bars(model(vec![]));
    window.set_analysis_legend(model(vec![]));

    if let Some(data) = &snapshot.usage.data {
        let unit = if data.units == "percent" {
            t(lang, "new.unit_percent")
        } else {
            &data.units
        };
        window.set_analysis_unit(unit.into());
        let series = analytics::usage_series(data, &dates, window.get_analysis_group() != 0);
        let totals = (0..dates.len())
            .map(|i| series.iter().map(|s| s.values[i]).sum::<f64>())
            .collect::<Vec<_>>();
        let max = totals.iter().copied().fold(0.0, f64::max).max(1.0);
        let bars = dates
            .iter()
            .enumerate()
            .map(|(index, date)| {
                let mut offset = 0.0;
                let mut details = vec![format!("{date} · {}", decimal(totals[index]))];
                let segments = series
                    .iter()
                    .enumerate()
                    .map(|(color, series)| {
                        let fraction = (series.values[index] / max) as f32;
                        let segment = AnalysisSegment {
                            fraction,
                            offset,
                            color_index: color as i32,
                        };
                        offset += fraction;
                        if series.values[index] > 0.0 {
                            details.push(format!(
                                "{} {}",
                                display_name(&series.name, lang),
                                decimal(series.values[index])
                            ));
                        }
                        segment
                    })
                    .collect();
                AnalysisBar {
                    label: if days == 7 || index % 5 == 0 || index == dates.len() - 1 {
                        date.format("%m-%d").to_string().into()
                    } else {
                        "".into()
                    },
                    detail: details.join("  ·  ").into(),
                    segments: model(segments),
                }
            })
            .collect();
        window.set_analysis_max(decimal(max).into());
        window.set_analysis_bars(model(bars));
        window.set_analysis_legend(model(
            series
                .iter()
                .enumerate()
                .map(|(index, series)| AnalysisLegend {
                    name: display_name(&series.name, lang).into(),
                    color_index: index as i32,
                })
                .collect(),
        ));
        if series.is_empty() && snapshot.usage.error.is_empty() {
            window.set_analysis_hover(t(lang, "new.no_activity").into());
        }
    }

    let (total, rows) = snapshot
        .activity
        .data
        .as_ref()
        .map(|data| {
            analytics::activity_ranking(data, &dates, window.get_analysis_activity_group() != 0)
        })
        .map(|(total, rows)| (number(total), ranking(rows, lang)))
        .unwrap_or_else(|| ("—".into(), vec![]));
    window.set_analysis_turn_total(total.into());
    window.set_analysis_activity_rows(model(rows));
    let panel = if window.get_analysis_tool_group() == 0 {
        &snapshot.plugins
    } else {
        &snapshot.skills
    };
    let (total, rows) = panel
        .data
        .as_ref()
        .map(|data| analytics::tool_ranking(data, &dates))
        .map(|(total, rows)| (number(total), ranking(rows, lang)))
        .unwrap_or_else(|| ("—".into(), vec![]));
    window.set_analysis_tool_total(total.into());
    window.set_analysis_tool_rows(model(rows));
    let mut message = panel_message(panel, lang, busy);
    if panel.error.is_empty() {
        if let Some(data) = &panel.data {
            if let Some(timestamp) = data
                .data_freshness_ts
                .as_ref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            {
                message = format!(
                    "{} {} · {}",
                    t(lang, "new.data_updated"),
                    timestamp.format("%m-%d %H:%M UTC"),
                    t(
                        lang,
                        if window.get_analysis_tool_group() == 0 {
                            "new.top_plugins"
                        } else {
                            "new.top_skills"
                        }
                    )
                );
            }
        }
    }
    window.set_analysis_tool_message(message.into());
}

fn selection_matches(
    request_alias: &str,
    selected_alias: &str,
    request_days: i32,
    selected_days: i32,
    provider: Provider,
) -> bool {
    provider == Provider::Codex && request_alias == selected_alias && request_days == selected_days
}

fn panel_message<T>(panel: &Panel<T>, lang: Language, busy: bool) -> String {
    if !panel.error.is_empty() {
        format!(
            "{}{}",
            if panel.data.is_some() {
                format!("{} · ", t(lang, "new.cached"))
            } else {
                String::new()
            },
            panel.error
        )
    } else if panel.data.is_none() {
        t(lang, if busy { "new.loading" } else { "new.no_data" }).into()
    } else {
        format!(
            "{} {}",
            t(lang, "new.last_success"),
            crate::usage::format_reset(Some(panel.updated_at))
        )
    }
}

fn ranking(mut values: Vec<(String, f64)>, lang: Language) -> Vec<AnalysisRank> {
    let mut other = 0.0;
    values.retain(|(name, value)| {
        if name == "Other" {
            other += value;
            false
        } else {
            true
        }
    });
    if values.len() > 6 || (other > 0.0 && values.len() > 5) {
        other += values.drain(5..).map(|(_, value)| value).sum::<f64>();
    }
    if other > 0.0 {
        values.push(("Other".into(), other));
    }
    let max = values.iter().map(|(_, value)| *value).fold(1.0, f64::max);
    values
        .into_iter()
        .map(|(name, value)| AnalysisRank {
            name: display_name(&name, lang).into(),
            value: number(value).into(),
            ratio: (value / max) as f32,
        })
        .collect()
}

fn display_name(name: &str, lang: Language) -> String {
    match name {
        "Other" => t(lang, "new.other").into(),
        "desktop_app" | "CODEX_DESKTOP_APP" => "Codex Desktop".into(),
        "work_desktop" => "Work Desktop".into(),
        "cli" => "CLI".into(),
        "vscode" => "VS Code".into(),
        _ => name.into(),
    }
}

fn model<T: Clone + 'static>(values: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(values))
}

fn decimal(value: f64) -> String {
    if value > 0.0 && value < 0.1 {
        "<0.1".into()
    } else if (value - value.round()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}
fn number(value: f64) -> String {
    let raw = format!("{value:.0}");
    let mut result = String::new();
    for (index, c) in raw.chars().enumerate() {
        if index > 0 && (raw.len() - index).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

pub fn localize(window: &MainWindow, lang: Language) {
    window.set_val_text(crate::ValLabels {
        title: t(lang, "val.title").into(),
        tokens: t(lang, "val.tokens").into(),
        value: t(lang, "val.value").into(),
        configure: t(lang, "val.configure").into(),
        model: t(lang, "val.model").into(),
        custom: t(lang, "val.custom").into(),
        input_ratio: t(lang, "val.input_ratio").into(),
        cache_ratio: t(lang, "val.cache_ratio").into(),
        input_price: t(lang, "val.input_price").into(),
        cache_price: t(lang, "val.cache_price").into(),
        output_price: t(lang, "val.output_price").into(),
        price_date: t(lang, "val.price_date").into(),
        apply: t(lang, "val.apply").into(),
        cancel: t(lang, "val.cancel").into(),
        trend: t(lang, "val.trend").into(),
        token_mode: t(lang, "val.token_mode").into(),
        cost_mode: t(lang, "val.cost_mode").into(),
        scope: t(lang, "val.scope").into(),
        source: t(lang, "val.source").into(),
        hint: t(lang, "val.hint").into(),
    });
    macro_rules! label { ($($setter:ident => $key:literal),* $(,)?) => { $(window.$setter(t(lang, $key).into());)* }; }
    label! {
        set_tr_nav_accounts => "new.nav_accounts", set_tr_nav_analysis => "new.nav_analysis",
        set_tr_val_auto => "val.auto", set_tr_val_auto_value => "val.auto_value", set_tr_val_manual => "val.manual",
        set_tr_network => "new.network", set_tr_network_hint => "new.network_hint",
        set_tr_add_account => "new.add_account", set_tr_cached => "new.cached", set_tr_more => "new.more",
        set_tr_loading => "new.loading", set_tr_days => "new.days", set_tr_weekly_used => "new.weekly_used", set_tr_plan_usage => "new.plan_usage",
        set_tr_by_product => "new.by_product", set_tr_by_model => "new.by_model", set_tr_by_client => "new.by_client",
        set_tr_product_activity => "new.product_activity", set_tr_tool_activity => "new.tool_activity",
        set_tr_turns => "new.turns", set_tr_returned_total => "new.returned_total",
        set_tr_plugins => "new.plugins", set_tr_skills => "new.skills",
        set_tr_analysis_delay => "new.analysis_delay", set_tr_analysis_source => "new.analysis_source",
    }
}

fn connect_valuation(window: &MainWindow, shared: Arc<Mutex<AppShared>>) {
    window.set_val_models(model(
        crate::valuation::OFFICIAL_PRICES
            .iter()
            .map(|p| p.0.into())
            .collect(),
    ));
    let weak = window.as_weak();
    let state_select = Arc::clone(&shared);
    window.on_select_valuation_model(move |index| {
        let Some(w) = weak.upgrade() else {
            return;
        };
        let mut state = state_select.lock().unwrap();
        let Some(account) = valuation_account(&w, &state) else {
            return;
        };
        let Some(mut a) = crate::valuation::official_assumption(index as usize) else {
            return;
        };
        let path = crate::valuation::settings_path();
        let result = crate::valuation::read_settings(&path).and_then(|settings| {
            let previous = crate::valuation::effective_assumption(settings.get(&account));
            a.input_percent = previous.input_percent;
            a.cached_input_percent = previous.cached_input_percent;
            crate::valuation::save_setting(&path, &account, a)
        });
        state.analysis.editing_account = None;
        w.set_val_editing(false);
        render_locked(&w, &state);
        w.set_val_error(
            if result.is_err() {
                t(state.lang, "val.storage_error")
            } else {
                ""
            }
            .into(),
        );
    });
    window.on_open_pricing(|| {
        const URL: &str = "https://developers.openai.com/api/docs/pricing";
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(URL).spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", URL])
            .spawn();
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let _ = std::process::Command::new("xdg-open").arg(URL).spawn();
    });

    let weak = window.as_weak();
    let shared_begin = Arc::clone(&shared);
    window.on_begin_valuation(move || {
        let Some(w) = weak.upgrade() else {
            return;
        };
        let mut state = shared_begin.lock().unwrap();
        let Some(account) = valuation_account(&w, &state) else {
            return;
        };
        let settings = match crate::valuation::read_settings(&crate::valuation::settings_path()) {
            Ok(s) => s,
            Err(_) => {
                w.set_val_error(t(state.lang, "val.storage_error").into());
                return;
            }
        };
        let a = crate::valuation::effective_assumption(settings.get(&account));
        state.analysis.editing_account = Some(account);
        w.set_val_input_percent(a.input_percent as f32);
        w.set_val_cache_percent(a.cached_input_percent as f32);
        w.set_val_error("".into());
        w.set_val_editing(true);
    });
    let weak = window.as_weak();
    window.on_apply_valuation(move || {
        let Some(w) = weak.upgrade() else {
            return;
        };
        let mut state = shared.lock().unwrap();
        let Some(account) = valuation_account(&w, &state) else {
            return;
        };
        if state.analysis.editing_account.as_ref() != Some(&account) {
            return;
        }
        let Some(mut a) = crate::valuation::official_assumption(w.get_val_model_index() as usize)
        else {
            return;
        };
        a.input_percent = f64::from(w.get_val_input_percent().round());
        a.cached_input_percent = f64::from(w.get_val_cache_percent().round());
        if !a.validate() {
            w.set_val_error(t(state.lang, "val.invalid").into());
            return;
        }
        if crate::valuation::save_setting(&crate::valuation::settings_path(), &account, a).is_err()
        {
            w.set_val_error(t(state.lang, "val.storage_error").into());
            return;
        }
        state.analysis.editing_account = None;
        w.set_val_editing(false);
        w.set_val_error("".into());
        render_locked(&w, &state);
    });
}

fn valuation_account(window: &MainWindow, state: &AppShared) -> Option<String> {
    if state.provider != Provider::Codex {
        return None;
    }
    let (alias, key) = state.analysis.active.as_ref()?;
    if alias != window.get_current_alias().as_str() {
        return None;
    }
    let account = state
        .codex
        .store()
        .read_profile_creds(alias)
        .ok()?
        .account_id?;
    (account == key.0).then_some(account)
}

fn clear_valuation(w: &MainWindow, message: &str) {
    w.set_val_model_rows(model(vec![]));
    w.set_val_auto_status("".into());
    w.set_val_total("—".into());
    w.set_val_cost("—".into());
    w.set_val_message(message.into());
    w.set_val_coverage("".into());
    w.set_val_basis("".into());
    w.set_val_bars(model(vec![]));
    w.set_val_max("—".into());
    w.set_val_hover("".into());
    w.set_val_editing(false);
    w.set_val_available(false);
    w.set_val_error("".into());
}

fn render_valuation(
    w: &MainWindow,
    snapshot: &Snapshot,
    account: &str,
    dates: &[NaiveDate],
    lang: Language,
    busy: bool,
) {
    let panel = &snapshot.tokens;
    w.set_val_model_rows(model(vec![]));
    w.set_val_auto_status("".into());
    w.set_val_available(true);
    w.set_val_total("—".into());
    w.set_val_cost("—".into());
    let mut message = panel_message(panel, lang, busy);
    if !panel.error.is_empty() && panel.data.is_some() {
        message.push_str(&format!(
            " · {} {}",
            t(lang, "new.last_success"),
            crate::usage::format_reset(Some(panel.updated_at))
        ));
    }
    w.set_val_message(message.into());
    w.set_val_bars(model(vec![]));
    w.set_val_max("—".into());
    w.set_val_hover("".into());
    w.set_val_coverage("".into());
    let settings = crate::valuation::read_settings(&crate::valuation::settings_path());
    let effective = settings
        .as_ref()
        .ok()
        .map(|s| crate::valuation::effective_assumption(s.get(account)));
    let assumption = effective.as_ref();
    if let Some(a) = assumption {
        w.set_val_model_index(
            crate::valuation::OFFICIAL_PRICES
                .iter()
                .position(|p| p.0 == a.model)
                .unwrap_or(0) as i32,
        );
    }
    w.set_val_basis(
        match assumption {
            Some(a) => format!(
                "{} · {} · {}: {}% · {}: {}%\n{}: ${} / ${} / ${} · {}: {}",
                a.model,
                t(lang, "val.custom"),
                t(lang, "val.input_ratio"),
                a.input_percent,
                t(lang, "val.cache_ratio"),
                a.cached_input_percent,
                t(lang, "val.rates"),
                a.input_price,
                a.cached_input_price,
                a.output_price,
                t(lang, "val.price_date"),
                a.price_date
            ),
            None => t(
                lang,
                if settings.is_err() {
                    "val.storage_error"
                } else {
                    "val.unset"
                },
            )
            .into(),
        }
        .into(),
    );
    let empty_profile = crate::usage::TokenUsageProfile::default();
    let mut series =
        crate::valuation::token_series(panel.data.as_ref().unwrap_or(&empty_profile), dates);
    if w.get_val_automatic() {
        crate::model_valuation::merge_reported_totals(
            &mut series,
            dates,
            snapshot.detailed_tokens.data.as_ref(),
        );
    }
    let automatic = if w.get_val_automatic() {
        assumption.map(|a| {
            crate::model_valuation::calculate(
                dates,
                &series,
                snapshot.detailed_tokens.data.as_ref(),
                snapshot.activity.data.as_ref(),
                a,
            )
        })
    } else {
        None
    };
    if let Some(result) = &automatic {
        w.set_val_basis(
            assumption
                .map(|a| {
                    format!(
                        "{}: {}% · {}: {}% · {}: {}",
                        t(lang, "val.input_ratio"),
                        a.input_percent,
                        t(lang, "val.cache_ratio"),
                        a.cached_input_percent,
                        t(lang, "val.price_date"),
                        a.price_date
                    )
                })
                .unwrap_or_default()
                .into(),
        );
        let status = t(lang, "val.auto_summary")
            .replace("{exact}", &result.exact_days.to_string())
            .replace("{estimated}", &result.estimated_days.to_string())
            .replace("{priced}", &number(result.priced_tokens))
            .replace("{unpriced}", &number(result.unpriced_tokens));
        let freshness = panel_message(&snapshot.detailed_tokens, lang, busy);
        w.set_val_auto_status(
            format!(
                "{status}\n{}\n{}: {freshness}",
                t(lang, "val.auto_note"),
                t(lang, "val.exact_source")
            )
            .into(),
        );
        w.set_val_model_rows(model(
            result
                .rows
                .iter()
                .map(|row| crate::ValuationRow {
                    model: row.model.clone().into(),
                    cost: row
                        .usd
                        .map(|v| format!("≈ {}", crate::valuation::money(v)))
                        .unwrap_or_else(|| t(lang, "val.unpriced").into())
                        .into(),
                    detail: format!(
                        "{} · {} Token · {}: {} / {} / {}",
                        t(
                            lang,
                            if row.estimated {
                                "val.estimated"
                            } else {
                                "val.measured"
                            }
                        ),
                        number(row.tokens),
                        t(lang, "val.token_types"),
                        number(row.input),
                        number(row.cached),
                        number(row.output)
                    )
                    .into(),
                })
                .collect(),
        ));
        w.set_val_cost(
            result
                .total
                .map(|v| format!("≈ {}", crate::valuation::money(v)))
                .unwrap_or_else(|| "—".into())
                .into(),
        );
    }
    w.set_val_coverage(
        t(lang, "val.coverage")
            .replace("{n}", &series.covered.to_string())
            .replace("{days}", &dates.len().to_string())
            .into(),
    );
    if let Some(total) = series.total {
        w.set_val_total(number(total as f64).into());
        if let Some(cost) = assumption
            .filter(|_| !w.get_val_automatic())
            .and_then(|a| a.usd(total))
        {
            w.set_val_cost(crate::valuation::money(cost).into());
        }
    }
    let cost_mode = w.get_val_chart_mode() == 1;
    let values: Vec<_> = series
        .daily
        .iter()
        .enumerate()
        .map(|(i, tokens)| {
            if cost_mode && w.get_val_automatic() {
                return automatic.as_ref().and_then(|a| a.daily[i]);
            }
            tokens.and_then(|n| {
                if cost_mode {
                    assumption.and_then(|a| a.usd(n))
                } else {
                    Some(n as f64)
                }
            })
        })
        .collect();
    let max = values
        .iter()
        .flatten()
        .copied()
        .fold(0.0, f64::max)
        .max(1.0);
    if values.iter().any(Option::is_some) {
        w.set_val_max(
            if cost_mode {
                crate::valuation::money(max)
            } else {
                number(max)
            }
            .into(),
        );
    }
    w.set_val_bars(model(
        dates
            .iter()
            .enumerate()
            .map(|(i, date)| {
                let detail = match (series.daily[i], values[i]) {
                    (None, _) => format!("{date} · {}", t(lang, "val.missing")),
                    (Some(n), None) => format!(
                        "{date} · {} Token · {}",
                        number(n as f64),
                        t(lang, "val.unset")
                    ),
                    (Some(n), Some(v)) => format!(
                        "{date} · {} Token{}",
                        number(n as f64),
                        if cost_mode {
                            format!(" · {}", crate::valuation::money(v))
                        } else {
                            String::new()
                        }
                    ),
                };
                AnalysisBar {
                    label: if dates.len() == 7 || i % 5 == 0 || i == dates.len() - 1 {
                        date.format("%m-%d").to_string().into()
                    } else {
                        "".into()
                    },
                    detail: detail.into(),
                    segments: model(
                        values[i]
                            .map(|value| {
                                vec![AnalysisSegment {
                                    fraction: (value / max) as f32,
                                    offset: 0.,
                                    color_index: 0,
                                }]
                            })
                            .unwrap_or_default(),
                    ),
                }
            })
            .collect(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_response_cannot_replace_another_account_or_range() {
        assert!(selection_matches("a", "a", 7, 7, Provider::Codex));
        assert!(!selection_matches("a", "b", 7, 7, Provider::Codex));
        assert!(!selection_matches("a", "a", 7, 30, Provider::Codex));
        assert!(!selection_matches("a", "a", 7, 7, Provider::Claude));
    }

    #[test]
    fn missing_data_is_not_zero_and_rank_tail_is_counted() {
        let panel: Panel<usize> = Panel::default();
        assert!(!panel_message(&panel, Language::ZhCn, false).is_empty());
        let rows = ranking(
            (0..10).map(|i| (i.to_string(), 1.0)).collect(),
            Language::ZhCn,
        );
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[5].value.as_str(), "5");
        assert_eq!(number(3009.), "3,009");
    }

    #[test]
    fn existing_other_merges_with_tail_and_small_usage_stays_visible() {
        let mut values = (0..8).map(|i| (i.to_string(), 1.)).collect::<Vec<_>>();
        values.insert(0, ("Other".into(), 10.));
        let rows = ranking(values, Language::ZhCn);
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[5].value.as_str(), "13");
        assert_eq!(decimal(0.001), "<0.1");
        assert_eq!(decimal(0.), "0");
    }
}
