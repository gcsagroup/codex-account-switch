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
    preferences_account: Option<String>,
    attempted: HashMap<CacheKey, i64>,
}

pub fn connect(window: &MainWindow, shared: Arc<Mutex<AppShared>>) {
    connect_valuation(window, Arc::clone(&shared));
    connect_report(window, Arc::clone(&shared));
    crate::local_analysis::connect(window, Arc::clone(&shared));
    let weak = window.as_weak();
    let state = Arc::clone(&shared);
    window.on_load_analysis(move |force| {
        if let Some(window) = weak.upgrade() {
            load(&window, Arc::clone(&state), force, false);
        }
    });
    let weak = window.as_weak();
    window.on_update_analysis(move || {
        if let Some(window) = weak.upgrade() {
            let state = shared.lock().unwrap();
            render_locked(&window, &state);
            persist_preferences(&window, &state);
        }
    });
}

pub fn load_weekly(window: &MainWindow, shared: Arc<Mutex<AppShared>>) {
    load(window, shared, false, true);
}

fn load(window: &MainWindow, shared: Arc<Mutex<AppShared>>, force: bool, weekly: bool) {
    let alias = window.get_current_alias().to_string();
    let mut days = if weekly || window.get_analysis_range() == 0 {
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
                    if !weekly {
                        state.analysis.active = None;
                    }
                    clear(window, t(state.lang, "status.no_token"));
                    return;
                }
            },
            Err(_) => {
                if !weekly {
                    state.analysis.active = None;
                }
                clear(window, t(state.lang, "active.no_account"));
                return;
            }
        };
        if !weekly && state.analysis.preferences_account.as_deref() != Some(&account) {
            match crate::analysis_report::read_preferences(&preferences_path()) {
                Ok(all) => {
                    let prefs = all.get(&account).cloned().unwrap_or_default();
                    window.set_analysis_range(prefs.range.clamp(0, 1));
                    window.set_analysis_group(prefs.group.clamp(0, 3));
                    window.set_analysis_model_filter(if prefs.model.is_empty() {
                        "全部模型".into()
                    } else {
                        prefs.model.into()
                    });
                    window.set_analysis_source_filter(if prefs.source.is_empty() {
                        "全部来源".into()
                    } else {
                        prefs.source.into()
                    });
                    window.set_analysis_show_history(prefs.show_history);
                    window.set_analysis_action_status("".into());
                }
                Err(_) => window
                    .set_analysis_action_status("筛选设置不可读，保留文件并使用当前选择".into()),
            }
            state.analysis.preferences_account = Some(account.clone());
            days = if window.get_analysis_range() == 0 {
                7
            } else {
                30
            };
        }
        let key = (account, days, end);
        if !weekly && state.analysis.active.as_ref().map(|(_, k)| k) != Some(&key) {
            window.set_analysis_detail("".into());
        }
        if !weekly && state.analysis.active.as_ref().map(|(_, k)| &k.0) != Some(&key.0) {
            window.set_val_editing(false);
            window.set_val_error("".into());
            state.analysis.editing_account = None;
        }
        if !weekly {
            state.analysis.active = Some((alias, key.clone()));
            persist_preferences(window, &state);
        }
        state.analysis.cache.retain(|(_, _, date), _| *date == end);
        state
            .analysis
            .attempted
            .retain(|(_, _, date), _| *date == end);
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
        // 自动卡片刷新也限制失败重试，避免个人账号不支持明细接口时反复请求。
        let recently_attempted = state
            .analysis
            .attempted
            .get(&key)
            .is_some_and(|stamp| (0..60).contains(&(Utc::now().timestamp() - stamp)));
        if (!force && ((fresh && !failed) || (weekly && recently_attempted)))
            || !state.analysis.pending.insert(key.clone())
        {
            return;
        }
        state
            .analysis
            .attempted
            .insert(key.clone(), Utc::now().timestamp());
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

// 卡片固定统计最近七个 UTC 日期，与分析页的 7/30 天选择互不干扰。
fn weekly_totals(
    snapshot: &Snapshot,
    end: NaiveDate,
    assumption: &crate::valuation::Assumptions,
) -> (
    crate::valuation::TokenSeries,
    crate::model_valuation::AutomaticCost,
) {
    let dates = analytics::dates(end, 7);
    let empty = crate::usage::TokenUsageProfile::default();
    let mut series =
        crate::valuation::token_series(snapshot.tokens.data.as_ref().unwrap_or(&empty), &dates);
    crate::model_valuation::merge_reported_totals(
        &mut series,
        &dates,
        snapshot.detailed_tokens.data.as_ref(),
    );
    let cost = crate::model_valuation::calculate(
        &dates,
        &series,
        snapshot.detailed_tokens.data.as_ref(),
        snapshot.activity.data.as_ref(),
        assumption,
    );
    (series, cost)
}

fn render_weekly(window: &MainWindow, state: &AppShared) {
    let lang = state.lang;
    window.set_weekly_token_title(t(lang, "weekly.tokens").into());
    window.set_weekly_value_title(t(lang, "val.auto_value").into());
    window.set_weekly_tokens("—".into());
    window.set_weekly_value("—".into());
    window.set_weekly_note(t(lang, "weekly.scope").into());
    let account = state
        .codex
        .store()
        .read_profile_creds(&window.get_current_alias())
        .ok()
        .and_then(|c| c.account_id);
    let Some(account) = account.filter(|_| state.provider == Provider::Codex) else {
        return;
    };
    let end = Utc::now().date_naive();
    let key = (account.clone(), 7, end);
    let empty = Snapshot::default();
    let snapshot = state.analysis.cache.get(&key).unwrap_or(&empty);
    let busy = state.analysis.pending.contains(&key);
    let settings = crate::valuation::read_settings(&crate::valuation::settings_path());
    let assumption = crate::valuation::effective_assumption(
        settings.as_ref().ok().and_then(|s| s.get(&account)),
    );
    let (series, cost) = weekly_totals(snapshot, end, &assumption);
    window.set_weekly_tokens(
        series
            .total
            .map(|n| number(n as f64))
            .unwrap_or_else(|| "—".into())
            .into(),
    );
    if settings.is_ok() {
        window.set_weekly_value(
            cost.total
                .map(|v| format!("≈ {}", crate::valuation::money(v)))
                .unwrap_or_else(|| "—".into())
                .into(),
        );
    }
    let summary = t(lang, "val.auto_summary")
        .replace("{exact}", &cost.exact_days.to_string())
        .replace("{estimated}", &cost.estimated_days.to_string())
        .replace("{priced}", &number(cost.priced_tokens))
        .replace("{unpriced}", &number(cost.unpriced_tokens));
    let coverage = t(lang, "val.coverage")
        .replace("{n}", &series.covered.to_string())
        .replace("{days}", "7");
    let status = [
        panel_message(&snapshot.tokens, lang, busy),
        panel_message(&snapshot.activity, lang, busy),
    ]
    .join(" · ");
    window.set_weekly_note(
        format!(
            "{} — {} · UTC · {}\n{}\n{}\n{}{}",
            analytics::dates(end, 7)[0],
            end,
            t(lang, "weekly.scope"),
            coverage,
            summary,
            status,
            if settings.is_err() {
                format!(" · {}", t(lang, "val.storage_error"))
            } else {
                String::new()
            }
        )
        .into(),
    );
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
    window.set_analysis_overview("尚无可用账号统计".into());
    window.set_analysis_consumption("—".into());
    window.set_analysis_dominant("—".into());
    window.set_analysis_coverage("尚无数据".into());
    window.set_analysis_comparison("尚无可比较数据".into());
    window.set_analysis_daily_rows(model(vec![]));
    window.set_analysis_consumption_rows(model(vec![]));
    window.set_analysis_detail("".into());
    window.set_analysis_heat_cells(model(vec![]));
    window.set_analysis_rank_note("尚无可用消耗明细".into());
}

pub fn render_locked(window: &MainWindow, state: &AppShared) {
    render_weekly(window, state);
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
    render_report(window, snapshot, &dates, lang, busy);
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
        let mut models = std::collections::BTreeSet::new();
        let mut sources = std::collections::BTreeSet::new();
        for day in &data.data {
            for m in &day.models {
                models.insert(m.model.clone());
            }
            for a in &day.attribution {
                models.insert(a.model.clone());
                sources.insert(analytics::source_key(a));
            }
        }
        // 选项增减会改变 ComboBox 的索引，先按文本保留筛选再更新列表。
        let selected_model = window.get_analysis_model_filter().to_string();
        let selected_source = window.get_analysis_source_filter().to_string();
        // 临时缺少维度数据时保留用户选项，不能悄悄改成全量。
        if selected_model != "全部模型" {
            models.insert(selected_model.clone());
        }
        if selected_source != "全部来源" {
            sources.insert(selected_source.clone());
        }
        let model_index = models
            .iter()
            .position(|name| *name == selected_model)
            .map(|i| i + 1)
            .unwrap_or(0);
        let source_index = sources
            .iter()
            .position(|name| *name == selected_source)
            .map(|i| i + 1)
            .unwrap_or(0);
        window.set_analysis_model_options(model(
            std::iter::once("全部模型".into())
                .chain(models.into_iter().map(Into::into))
                .collect(),
        ));
        window.set_analysis_source_options(model(
            std::iter::once("全部来源".into())
                .chain(sources.into_iter().map(Into::into))
                .collect(),
        ));
        window.set_analysis_model_index(model_index as i32);
        window.set_analysis_source_index(source_index as i32);
        window.set_analysis_model_filter(selected_model.clone().into());
        window.set_analysis_source_filter(selected_source.clone().into());
        let m = selected_model;
        let source = selected_source;
        let m = if m == "全部模型" { "" } else { &m };
        let source = if source == "全部来源" || window.get_analysis_group() == 3 {
            ""
        } else {
            &source
        };
        let series = analytics::grouped_usage(data, &dates, window.get_analysis_group(), m, source);
        let cutoff = snapshot
            .tokens
            .data
            .as_ref()
            .and_then(|p| p.stats_as_of.as_deref())
            .unwrap_or("未知");
        window.set_analysis_composition(
            format!(
                "{}\nToken 统计截至：{cutoff}；消耗接口未提供统计截止时间。",
                analytics::composition(data, &dates, window.get_analysis_group(), m, source)
            )
            .into(),
        );
        let availability = crate::analysis_report::daily_usage(
            data,
            &dates,
            window.get_analysis_group(),
            m,
            source,
        );
        let totals = (0..dates.len())
            .map(|i| series.iter().map(|s| s.values[i]).sum::<f64>())
            .collect::<Vec<_>>();
        let max = totals.iter().copied().fold(0.0, f64::max).max(1.0);
        let bars = dates
            .iter()
            .enumerate()
            .map(|(index, date)| {
                let mut offset = 0.0;
                let returned = availability[index].is_some();
                let mut details = vec![if !returned {
                    format!("{date} · 日期或当前维度未提供，不能视为零")
                } else if *date == key.2 {
                    format!("{date} · {}（当日可能未完整）", decimal(totals[index]))
                } else {
                    format!("{date} · {}", decimal(totals[index]))
                }];
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
        if series.is_empty() {
            let bars = dates
                .iter()
                .enumerate()
                .map(|(i, date)| AnalysisBar {
                    label: date.format("%m-%d").to_string().into(),
                    detail: format!(
                        "{date} · {}",
                        availability[i]
                            .map(|v| format!("{v} {}（服务端返回）", data.units))
                            .unwrap_or_else(|| "此日期或维度未提供".into())
                    )
                    .into(),
                    segments: model(if availability[i].is_some() {
                        vec![AnalysisSegment {
                            fraction: 0.,
                            offset: 0.,
                            color_index: 0,
                        }]
                    } else {
                        vec![]
                    }),
                })
                .collect();
            window.set_analysis_bars(model(bars));
        }
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
        .map(|(total, rows)| {
            (
                if snapshot
                    .activity
                    .data
                    .as_ref()
                    .is_some_and(|d| d.data.iter().any(|r| dates.contains(&r.date)))
                {
                    number(total)
                } else {
                    "—".into()
                },
                ranking(rows, lang),
            )
        })
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
        .map(|(total, rows)| {
            (
                if panel
                    .data
                    .as_ref()
                    .is_some_and(|d| d.data.iter().any(|r| dates.contains(&r.date)))
                {
                    number(total)
                } else {
                    "—".into()
                },
                ranking(rows, lang),
            )
        })
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
                    "{} · {} {} · {}",
                    message,
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
    window.set_analysis_tool_message(
        format!("{message}\n仅返回每日榜单条目，不代表全部调用；不适用消耗筛选。").into(),
    );
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

fn panel_message<T>(panel: &Panel<T>, _lang: Language, busy: bool) -> String {
    let fetched = if panel.updated_at > 0 {
        crate::usage::format_reset(Some(panel.updated_at))
    } else {
        "尚无成功读取".into()
    };
    if !panel.error.is_empty() {
        format!(
            "{} · {} · 最近成功读取 {fetched}",
            if panel.data.is_some() {
                "刷新失败，保留旧数据"
            } else if panel.error.contains("不提供") {
                "此账号不支持"
            } else {
                "读取失败"
            },
            panel.error
        )
    } else if panel.data.is_none() {
        if busy { "正在读取" } else { "尚无数据" }.into()
    } else {
        format!(
            "{}最近成功读取 {fetched}；读取时间不代表统计已完整",
            if busy { "正在刷新 · " } else { "" }
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
            a.speed = previous.speed;
            a.long_context = previous.long_context;
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
        a.speed = w.get_val_speed_index();
        a.long_context = w.get_val_context_index() == 1;
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
        w.set_val_speed_index(a.speed);
        w.set_val_context_index(i32::from(a.long_context));
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
    // 账号 Token 总量不随估值模式改变。
    crate::model_valuation::merge_reported_totals(
        &mut series,
        dates,
        snapshot.detailed_tokens.data.as_ref(),
    );
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
        let price_dates = result
            .rows
            .iter()
            .filter_map(|row| {
                crate::valuation::OFFICIAL_PRICES
                    .iter()
                    .position(|p| p.0 == row.model)
                    .and_then(crate::valuation::official_assumption)
                    .map(|a| a.price_date)
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(" / ");
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
                        if price_dates.is_empty() {
                            "无参与计价模型；当前价目核验 2026-11-14"
                        } else {
                            &price_dates
                        }
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
                "{status}\n分类明确部分：{} · 分摊估算部分：{} · 未计价：{} Token\n{}\n{}: {freshness}\n{}",
                if result.rows.iter().any(|r| !r.estimated && r.usd.is_some()) { crate::valuation::money(result.classified_usd) } else { "无可计价明细".into() },
                if result.rows.iter().any(|r| r.estimated && r.usd.is_some()) { crate::valuation::money(result.estimated_usd) } else { "无可计价明细".into() }, number(result.unpriced_tokens),
                t(lang, "val.auto_note"), t(lang, "val.exact_source"),
                if result.total.is_none() {
                    if series.total.is_none() { "未计价原因：尚无可用 Token 日期。" }
                    else if result.rows.is_empty() { "未计价原因：缺少同日模型或分类明细，无法可靠分摊。" }
                    else { "未计价原因：模型或所选速度／上下文组合没有已核验价格。" }
                } else { "以上金额均为 API 情景折算；分类明确不代表实际账单。" }
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
    w.set_val_coverage(format!("{} — {} · UTC · {}\nToken 统计截至 {} · 当前模型、来源筛选不适用此账号总量。今天可能未完整。",
        dates[0], dates[dates.len()-1], crate::analysis_report::coverage(series.covered,dates.len()),
        panel.data.as_ref().and_then(|p|p.stats_as_of.as_deref()).unwrap_or("未提供")).into());
    if let Some(a) = assumption {
        w.set_val_basis(
            format!(
                "{}\n{}",
                w.get_val_basis(),
                crate::valuation::price_reference(a)
            )
            .into(),
        );
    }
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
                        "未计价：缺少模型／分类，或所选价格组合未提供"
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

fn preferences_path() -> std::path::PathBuf {
    crate::paths::app_home().join("analysis-preferences.json")
}
fn persist_preferences(w: &MainWindow, state: &AppShared) {
    let Some(account) = valuation_account(w, state) else {
        return;
    };
    let prefs = crate::analysis_report::Preferences {
        range: w.get_analysis_range(),
        group: w.get_analysis_group(),
        model: w.get_analysis_model_filter().to_string(),
        source: w.get_analysis_source_filter().to_string(),
        show_history: w.get_analysis_show_history(),
    };
    if crate::analysis_report::save_preferences(&preferences_path(), &account, prefs).is_err() {
        w.set_analysis_action_status("筛选设置保存失败；已有设置文件未覆盖".into());
    }
}

fn connect_report(w: &MainWindow, shared: Arc<Mutex<AppShared>>) {
    let weak = w.as_weak();
    let sh = shared.clone();
    w.on_export_analysis(move || {
        let Some(w) = weak.upgrade() else { return };
        let text = {
            let state = sh.lock().unwrap();
            if valuation_account(&w, &state).is_none() {
                return;
            }
            let Some((alias, key)) = &state.analysis.active else {
                return;
            };
            if !selection_matches(
                alias,
                &w.get_current_alias(),
                key.1,
                if w.get_analysis_range() == 0 { 7 } else { 30 },
                state.provider,
            ) {
                return;
            }
            let Some(snapshot) = state.analysis.cache.get(key) else {
                return;
            };
            let dates = analytics::dates(key.2, key.1);
            let (m, source) = report_filters(&w);
            let mut text = crate::analysis_report::export_cloud(
                snapshot,
                alias,
                &dates,
                w.get_analysis_group(),
                &m,
                &source,
            );
            let settings = crate::valuation::read_settings(&crate::valuation::settings_path());
            if let Ok(settings) = settings {
                let a = crate::valuation::effective_assumption(settings.get(&key.0));
                let mut ts = crate::valuation::token_series(
                    snapshot.tokens.data.as_ref().unwrap_or(&Default::default()),
                    &dates,
                );
                crate::model_valuation::merge_reported_totals(
                    &mut ts,
                    &dates,
                    snapshot.detailed_tokens.data.as_ref(),
                );
                let cost = crate::model_valuation::calculate(
                    &dates,
                    &ts,
                    snapshot.detailed_tokens.data.as_ref(),
                    snapshot.activity.data.as_ref(),
                    &a,
                );
                text += &crate::analysis_report::csv_row(&[
                    "估值基准".into(),
                    crate::valuation::scenario_label(&a),
                    "估算输入占比".into(),
                    a.input_percent.to_string(),
                    "缓存占输入".into(),
                    a.cached_input_percent.to_string(),
                ]);
                text += &crate::analysis_report::csv_row(&[
                    "估值模型".into(),
                    "Token".into(),
                    "金额USD".into(),
                    "Token口径".into(),
                    "价格核验日期".into(),
                ]);
                if !w.get_val_automatic() {
                    text += &crate::analysis_report::csv_row(&[
                        "单一模型情景".into(),
                        a.model.clone(),
                        ts.total
                            .and_then(|n| a.usd(n))
                            .map(|n| n.to_string())
                            .unwrap_or_default(),
                        crate::valuation::price_reference(&a),
                    ]);
                }
                text += &crate::analysis_report::csv_row(&[
                    "下列为自动按模型明细".into(),
                    "与单一模型情景分开，不相加".into(),
                ]);
                for row in cost.rows {
                    text += &crate::analysis_report::csv_row(&[
                        row.model.clone(),
                        row.tokens.to_string(),
                        row.usd.map(|n| n.to_string()).unwrap_or_default(),
                        if row.estimated {
                            "轮次分摊估算"
                        } else {
                            "分类明确"
                        }
                        .into(),
                        if crate::valuation::OFFICIAL_PRICES
                            .iter()
                            .any(|p| p.0 == row.model)
                        {
                            crate::valuation::price_date(&row.model).to_string()
                        } else {
                            "未计价".into()
                        },
                    ]);
                }
                text += &crate::analysis_report::csv_row(&[
                    "未计价Token".into(),
                    cost.unpriced_tokens.to_string(),
                ]);
            } else {
                text += &crate::analysis_report::csv_row(&[
                    "估值状态".into(),
                    "设置不可读，未导出估值".into(),
                ]);
            }
            text
        };
        if let Some(path) = rfd::FileDialog::new()
            .set_title("导出账号分析")
            .add_filter("CSV", &["csv"])
            .set_file_name("codex-account-analysis.csv")
            .save_file()
        {
            w.set_analysis_action_status(
                if crate::profile::atomic_write(&path, text.as_bytes()).is_ok() {
                    "分析明细已导出；包含统计范围、缺失状态与价格依据"
                } else {
                    "导出失败"
                }
                .into(),
            );
        }
    });
    let weak = w.as_weak();
    let sh = shared.clone();
    w.on_select_analysis_rank(move |name| {
        let Some(w) = weak.upgrade() else { return };
        let s = sh.lock().unwrap();
        let Some((_, key)) = &s.analysis.active else {
            return;
        };
        let Some(snapshot) = s.analysis.cache.get(key) else {
            return;
        };
        let Some(data) = &snapshot.usage.data else {
            return;
        };
        let dates = analytics::dates(key.2, key.1);
        let (m, source) = report_filters(&w);
        let valid =
            crate::analysis_report::daily_usage(data, &dates, w.get_analysis_group(), &m, &source);
        if let Some(row) =
            analytics::grouped_usage_full(data, &dates, w.get_analysis_group(), &m, &source)
                .into_iter()
                .find(|r| r.name == name.as_str())
        {
            w.set_analysis_detail(
                format!(
                    "{} · {} · {}\n{}",
                    name,
                    w.get_analysis_date_range(),
                    data.units,
                    dates
                        .iter()
                        .enumerate()
                        .map(|(i, d)| format!(
                            "{d}：{}",
                            if valid[i].is_some() {
                                format!("{:.4}", row.values[i])
                            } else {
                                "此维度未提供".into()
                            }
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
                .into(),
            );
        }
    });
    let weak = w.as_weak();
    w.on_change_price_scenario(move || {
        let Some(w) = weak.upgrade() else { return };
        let s = shared.lock().unwrap();
        let Some(account) = valuation_account(&w, &s) else {
            return;
        };
        let result =
            crate::valuation::read_settings(&crate::valuation::settings_path()).and_then(|all| {
                let mut a = crate::valuation::effective_assumption(all.get(&account));
                a.speed = w.get_val_speed_index();
                a.long_context = w.get_val_context_index() == 1;
                crate::valuation::save_setting(&crate::valuation::settings_path(), &account, a)
            });
        render_locked(&w, &s);
        w.set_val_error(
            if result.is_err() {
                "价格情景保存失败；旧设置未覆盖"
            } else {
                ""
            }
            .into(),
        );
    });
}
fn report_filters(w: &MainWindow) -> (String, String) {
    let m = w.get_analysis_model_filter().to_string();
    let s = w.get_analysis_source_filter().to_string();
    (
        if m == "全部模型" {
            String::new()
        } else {
            m
        },
        if s == "全部来源" || w.get_analysis_group() == 3 {
            String::new()
        } else {
            s
        },
    )
}
fn render_report(
    w: &MainWindow,
    snapshot: &Snapshot,
    dates: &[NaiveDate],
    lang: Language,
    busy: bool,
) {
    let (m, source) = report_filters(w);
    let mut tokens = crate::valuation::token_series(
        snapshot.tokens.data.as_ref().unwrap_or(&Default::default()),
        dates,
    );
    crate::model_valuation::merge_reported_totals(
        &mut tokens,
        dates,
        snapshot.detailed_tokens.data.as_ref(),
    );
    let global = snapshot
        .usage
        .data
        .as_ref()
        .map(|u| crate::analysis_report::daily_usage(u, dates, 0, "", ""))
        .unwrap_or_else(|| vec![None; dates.len()]);
    let selected = snapshot
        .usage
        .data
        .as_ref()
        .map(|u| crate::analysis_report::daily_usage(u, dates, w.get_analysis_group(), &m, &source))
        .unwrap_or_else(|| vec![None; dates.len()]);
    let consumed = global.iter().flatten().count();
    let turns = snapshot
        .activity
        .data
        .as_ref()
        .map(|a| a.data.iter().filter(|d| dates.contains(&d.date)).count())
        .unwrap_or(0);
    let unit = snapshot
        .usage
        .data
        .as_ref()
        .map(|u| u.units.as_str())
        .unwrap_or("单位未提供");
    w.set_analysis_consumption(if consumed == 0 {
        "—".into()
    } else {
        format!("{} {}", decimal(global.iter().flatten().sum()), unit).into()
    });
    let inconsistent =
        (0..dates.len()).any(|i| tokens.daily[i].is_some_and(|n| n > 0) && global[i] == Some(0.));
    w.set_analysis_coverage(
        format!(
            "Token {}/{} · 消耗 {}/{} · 轮次 {}/{}{}{}",
            tokens.covered,
            dates.len(),
            consumed,
            dates.len(),
            turns,
            dates.len(),
            if inconsistent {
                " · 跨接口数据未对齐"
            } else {
                ""
            },
            if !snapshot.tokens.error.is_empty()
                || !snapshot.usage.error.is_empty()
                || !snapshot.activity.error.is_empty()
            {
                " · 部分请求失败或不支持，查看数据状态"
            } else {
                ""
            }
        )
        .into(),
    );
    w.set_analysis_overview(format!("账号统计 · {} — {} · UTC · 包含今天，今日可能未完整\n{}\nToken：{}\n消耗：{} · 统计截止：接口未提供\n轮次：{} · 统计截止：接口未提供",
        dates[0],dates[dates.len()-1],if inconsistent{"跨接口未对齐：存在 Token 正用量而消耗返回零的日期；不能据此判断没有使用，也不推算费用。"}else{"各指标独立核对覆盖率；未返回不等于零。"},
        panel_message(&snapshot.tokens,lang,busy),panel_message(&snapshot.usage,lang,busy),panel_message(&snapshot.activity,lang,busy)).into());
    w.set_analysis_comparison(
        crate::analysis_report::comparison_text(
            snapshot,
            dates[dates.len() - 1],
            dates.len() as i32,
        )
        .into(),
    );
    let full = snapshot
        .usage
        .data
        .as_ref()
        .map(|u| analytics::grouped_usage_full(u, dates, w.get_analysis_group(), &m, &source))
        .unwrap_or_default();
    let global_models = snapshot
        .usage
        .data
        .as_ref()
        .map(|u| analytics::grouped_usage_full(u, dates, 1, "", ""))
        .unwrap_or_default();
    w.set_analysis_dominant(
        global_models
            .first()
            .map(|r| r.name.clone())
            .unwrap_or_else(|| "模型明细未提供".into())
            .into(),
    );
    let sum = full
        .iter()
        .map(|s| s.values.iter().sum::<f64>())
        .sum::<f64>();
    let max = full
        .iter()
        .map(|s| s.values.iter().sum::<f64>())
        .fold(0., f64::max)
        .max(f64::MIN_POSITIVE);
    w.set_analysis_consumption_rows(model(
        full.iter()
            .take(if w.get_analysis_show_all() {
                usize::MAX
            } else {
                6
            })
            .map(|s| {
                let n = s.values.iter().sum::<f64>();
                crate::AnalysisRank {
                    name: s.name.clone().into(),
                    value: format!(
                        "{n:.4} {unit} · {:.1}%",
                        if sum > 0. { n / sum * 100. } else { 0. }
                    )
                    .into(),
                    ratio: (n / max) as f32,
                }
            })
            .collect(),
    ));
    w.set_analysis_rank_note(format!("当前筛选：{} · {} · {}\n{}；共 {} 个分类。占比仅在已返回筛选数据内计算。点击条目查看逐日明细。",
        ["产品","模型","来源","模型与速度"][w.get_analysis_group().clamp(0,3)as usize],w.get_analysis_model_filter(),if w.get_analysis_group()==3{"来源筛选不适用".into()}else{w.get_analysis_source_filter()},
        crate::analysis_report::coverage(selected.iter().flatten().count(),dates.len()),full.len()).into());
    w.set_analysis_daily_rows(model(dates.iter().enumerate().map(|(i,d)|crate::AnalysisDayRow{
        date:d.to_string().into(),tokens:tokens.daily[i].map(|n|number(n as f64)).unwrap_or_else(||"未提供".into()).into(),
        usage:selected[i].map(|n|format!("{n:.4} {unit}")).unwrap_or_else(||"当前维度未提供".into()).into(),
        detail:format!("{d} · UTC\n账号 Token：{}\n筛选消耗：{}\n模型、来源筛选仅适用于消耗；今天可能未完整。",tokens.daily[i].map(|n|n.to_string()).unwrap_or_else(||"未提供".into()),selected[i].map(|n|format!("{n:.6} {unit}")).unwrap_or_else(||"当前维度未提供".into())).into(),
    }).collect()));
    let end = dates[dates.len() - 1];
    let heat_dates = (0..364)
        .rev()
        .map(|i| end - chrono::Duration::days(i))
        .collect::<Vec<_>>();
    let heat = crate::valuation::token_series(
        snapshot.tokens.data.as_ref().unwrap_or(&Default::default()),
        &heat_dates,
    );
    let max = heat
        .daily
        .iter()
        .flatten()
        .copied()
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    w.set_analysis_heat_cells(model(
        heat_dates
            .iter()
            .enumerate()
            .map(|(i, d)| crate::AnalysisHeatCell {
                intensity: heat.daily[i]
                    .map(|v| (v as f64 / max) as f32)
                    .unwrap_or(-1.),
                detail: format!(
                    "{d} · {}",
                    heat.daily[i]
                        .map(|n| format!("{} Token", number(n as f64)))
                        .unwrap_or_else(|| "日期未提供".into())
                )
                .into(),
            })
            .collect(),
    ));
    w.set_analysis_heat_months(model(
        heat_dates
            .iter()
            .enumerate()
            .filter(|(i, d)| *i == 0 || chrono::Datelike::day(*d) == 1)
            .map(|(i, d)| crate::AnalysisHeatMonth {
                label: d.format("%m月").to_string().into(),
                column: (i / 7) as i32,
            })
            .collect(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekly_value_filters_dates_and_prices_each_reported_model() {
        let end = NaiveDate::from_ymd_opt(2026, 9, 16).unwrap();
        let mut snapshot = Snapshot::default();
        let row = |date: &str, name: &str| {
            serde_json::json!({
                "date": date, "models": [{"model": name,
                "uncached_text_input_tokens": 1_000_000,
                "cached_text_input_tokens": 0, "text_output_tokens": 0}]
            })
        };
        snapshot.detailed_tokens.data = Some(
            serde_json::from_value(serde_json::json!({"data": [
                row("2026-09-09", "gpt-6-sol"), row("2026-09-10", "gpt-6-astra"),
                row("2026-09-16", "gpt-5.5"), row("2026-09-17", "gpt-5.5")
            ]}))
            .unwrap(),
        );
        let (series, cost) = weekly_totals(
            &snapshot,
            end,
            &crate::valuation::effective_assumption(None),
        );
        assert_eq!(series.daily.len(), 7);
        assert_eq!(series.covered, 2);
        assert_eq!(series.total, Some(2_000_000));
        assert_eq!(cost.total, Some(15.));
        assert_eq!(cost.exact_days, 2);
        assert_eq!(cost.estimated_days, 0);
    }

    #[test]
    fn weekly_missing_and_unknown_models_are_not_zero_cost() {
        let end = NaiveDate::from_ymd_opt(2026, 9, 16).unwrap();
        let mut snapshot = Snapshot::default();
        let assumption = crate::valuation::effective_assumption(None);
        let (series, cost) = weekly_totals(&snapshot, end, &assumption);
        assert_eq!(series.total, None);
        assert_eq!(cost.total, None);
        snapshot.detailed_tokens.data = Some(serde_json::from_value(serde_json::json!({"data":[{
            "date":"2026-09-16", "models":[{"model":"unknown",
            "uncached_text_input_tokens":1000,"cached_text_input_tokens":0,"text_output_tokens":0}]
        }]})).unwrap());
        let (series, cost) = weekly_totals(&snapshot, end, &assumption);
        assert_eq!(series.total, Some(1000));
        assert_eq!(cost.total, None);
        assert_eq!(cost.unpriced_tokens, 1000.);
    }

    #[test]
    fn weekly_personal_usage_uses_only_same_day_model_activity() {
        let end = NaiveDate::from_ymd_opt(2026, 9, 16).unwrap();
        let mut snapshot = Snapshot::default();
        snapshot.tokens.data = Some(crate::usage::TokenUsageProfile {
            daily_usage_buckets: Some(vec![
                crate::usage::TokenUsageDailyBucket {
                    start_date: "2026-09-15".into(),
                    tokens: 1_000_000,
                },
                crate::usage::TokenUsageDailyBucket {
                    start_date: "2026-09-16".into(),
                    tokens: 1_000_000,
                },
            ]),
            ..Default::default()
        });
        snapshot.activity.data = Some(
            serde_json::from_value(serde_json::json!({"data":[{
                "date":"2026-09-16","totals":{"turns":2},"models":[
                    {"model":"gpt-6-astra","turns":1},{"model":"gpt-5.5","turns":1}]
            }]}))
            .unwrap(),
        );
        let (series, cost) = weekly_totals(
            &snapshot,
            end,
            &crate::valuation::effective_assumption(None),
        );
        assert_eq!(series.total, Some(2_000_000));
        assert_eq!(cost.estimated_days, 1);
        assert_eq!(cost.unpriced_tokens, 1_000_000.);
        assert!((cost.total.unwrap() - 11.3).abs() < 0.000001);
    }

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
