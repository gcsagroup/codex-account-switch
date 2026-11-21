//! 个人分析的只读接口。每个面板独立保留成功数据和失败状态。
use std::collections::{BTreeMap, BTreeSet};

use chrono::{Days, NaiveDate, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Default)]
pub struct Panel<T> {
    pub data: Option<T>,
    pub updated_at: i64,
    pub error: String,
}

impl<T> Panel<T> {
    fn update(&mut self, result: AppResult<T>) {
        match result {
            Ok(data) => {
                self.data = Some(data);
                self.updated_at = Utc::now().timestamp();
                self.error.clear();
            }
            Err(error) => self.error = error.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub usage: Panel<UsageResponse>,
    pub activity: Panel<ActivityResponse>,
    pub plugins: Panel<ToolResponse>,
    pub skills: Panel<ToolResponse>,
    pub tokens: Panel<crate::usage::TokenUsageProfile>,
    pub detailed_tokens: Panel<crate::model_valuation::DetailedUsage>,
    pub comparison_recent: Panel<UsageResponse>,
    pub comparison_previous: Panel<UsageResponse>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageResponse {
    pub data: Vec<UsageDay>,
    pub units: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDay {
    pub date: NaiveDate,
    pub product_surface_usage_values: BTreeMap<String, f64>,
    #[serde(default)]
    pub models: Vec<ModelUsage>,
    #[serde(default)]
    pub attribution: Vec<Attribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub credits: f64,
    #[serde(default)]
    pub speed: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Attribution {
    pub thread_source: String,
    pub turn_trigger: String,
    pub model: String,
    pub surface: String,
    pub value: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityResponse {
    pub data: Vec<ActivityDay>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDay {
    pub date: NaiveDate,
    pub totals: TurnCount,
    #[serde(default)]
    pub models: Vec<ModelTurns>,
    #[serde(default)]
    pub clients: Vec<ClientTurns>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnCount {
    pub turns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTurns {
    pub model: String,
    pub turns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientTurns {
    pub client_id: String,
    pub turns: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolResponse {
    pub data: Vec<ToolDay>,
    #[serde(default)]
    pub data_freshness_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDay {
    pub date: NaiveDate,
    #[serde(alias = "skill_usage_overviews")]
    pub plugin_usage_overviews: Vec<ToolCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCount {
    #[serde(default)]
    pub display_name: String,
    #[serde(default, alias = "skill_name")]
    pub plugin_name: String,
    pub invocation_counts: u64,
}

#[derive(Debug, Clone)]
pub struct Series {
    pub name: String,
    pub values: Vec<f64>,
}

pub fn dates(end: NaiveDate, days: i32) -> Vec<NaiveDate> {
    dates_for_period(end, days)
}

fn dates_for_period(end: NaiveDate, days: i32) -> Vec<NaiveDate> {
    (0..days.clamp(1, 30))
        .rev()
        .filter_map(|offset| end.checked_sub_days(Days::new(offset as u64)))
        .collect()
}

pub fn fetch(
    token: &str,
    account: &str,
    end: NaiveDate,
    days: i32,
    mut previous: Snapshot,
) -> Snapshot {
    let client = match crate::usage::http_client() {
        Ok(client) => client,
        Err(error) => {
            let message = error.to_string();
            previous.usage.error = message.clone();
            previous.activity.error = message.clone();
            previous.plugins.error = message.clone();
            previous.skills.error = message.clone();
            previous.tokens.error = message.clone();
            previous.detailed_tokens.error = message.clone();
            previous.comparison_recent.error = message.clone();
            previous.comparison_previous.error = message;
            return previous;
        }
    };
    let dates = dates(end, days);
    let start = dates[0].to_string();
    let comparison_end = end.pred_opt().unwrap_or(end);
    let end = end.to_string();
    let base = [
        ("start_date", start.as_str()),
        ("end_date", end.as_str()),
        ("group_by", "day"),
    ];
    // 分别读取，某个面板失败不影响其余面板；只在后台线程等待。
    std::thread::scope(|scope| {
        let comparison = scope.spawn(|| {
            let read_period = |end: NaiveDate| {
                let ds = dates_for_period(end, days);
                let start = ds[0].to_string();
                let end = end.to_string();
                get(
                    &client,
                    token,
                    account,
                    "usage/daily-token-usage-breakdown",
                    &[
                        ("start_date", start.as_str()),
                        ("end_date", end.as_str()),
                        ("group_by", "day"),
                    ],
                )
            };
            (
                read_period(comparison_end),
                read_period(comparison_end - chrono::Duration::days(days.into())),
            )
        });
        let detailed = scope.spawn(|| {
            let mut query = base.to_vec();
            query.extend([
                ("breakdown_by", "model"),
                ("modes", "codex"),
                ("modes", "work"),
            ]);
            get(
                &client,
                token,
                account,
                "usage/daily-workspace-user-token-usage-breakdown",
                &query,
            )
        });
        let tokens = scope.spawn(|| crate::usage::fetch_token_usage_profile("", token, account));
        let quota = scope.spawn(|| {
            get(
                &client,
                token,
                account,
                "usage/daily-token-usage-breakdown",
                &base,
            )
        });
        let turns = scope.spawn(|| {
            let mut query = base.to_vec();
            query.push(("workspace_user", "true"));
            get(
                &client,
                token,
                account,
                "analytics/daily-workspace-usage-counts",
                &query,
            )
        });
        let plugins = scope.spawn(|| {
            let mut query = base.to_vec();
            query.extend([("workspace_user", "true"), ("top_plugin_limit", "8")]);
            get(
                &client,
                token,
                account,
                "analytics/daily-plugin-usage-metrics",
                &query,
            )
        });
        let skills = scope.spawn(|| {
            let mut query = base.to_vec();
            query.extend([("workspace_user", "true"), ("top_skill_limit", "6")]);
            get(
                &client,
                token,
                account,
                "analytics/daily-skill-usage-metrics",
                &query,
            )
        });
        let (recent, prior) = comparison.join().unwrap_or_else(|_| {
            (
                Err(AppError::msg("期间对比请求中断")),
                Err(AppError::msg("期间对比请求中断")),
            )
        });
        previous.comparison_recent.update(recent);
        previous.comparison_previous.update(prior);
        previous.detailed_tokens.update(
            detailed
                .join()
                .unwrap_or_else(|_| Err(AppError::msg("Token 明细请求中断"))),
        );
        previous.tokens.update(
            tokens
                .join()
                .unwrap_or_else(|_| Err(AppError::msg("Token 请求中断"))),
        );
        previous.usage.update(
            quota
                .join()
                .unwrap_or_else(|_| Err(AppError::msg("分析请求中断"))),
        );
        previous.activity.update(
            turns
                .join()
                .unwrap_or_else(|_| Err(AppError::msg("分析请求中断"))),
        );
        previous.plugins.update(
            plugins
                .join()
                .unwrap_or_else(|_| Err(AppError::msg("分析请求中断"))),
        );
        previous.skills.update(
            skills
                .join()
                .unwrap_or_else(|_| Err(AppError::msg("分析请求中断"))),
        );
    });
    previous
}

pub(crate) fn get<T: DeserializeOwned>(
    client: &reqwest::blocking::Client,
    token: &str,
    account: &str,
    endpoint: &str,
    query: &[(&str, &str)],
) -> AppResult<T> {
    let response = client
        .get(format!("https://chatgpt.com/backend-api/wham/{endpoint}"))
        .bearer_auth(token)
        .header("ChatGPT-Account-Id", account)
        .query(query)
        .send()?;
    let status = response.status();
    let body = response.text()?;
    if status.as_u16() == 400 && body.contains("No active workspace") {
        return Err(AppError::msg("此个人账号不提供按模型 Token 分类明细"));
    }
    if !status.is_success() {
        return Err(AppError::msg(crate::usage::friendly_http_error(
            status.as_u16(),
            &body,
        )));
    }
    parse_response(endpoint, &body)
}

// 日统计按日期校验；重置历史使用事件结构，分页和时间范围由历史模块校验。
pub(crate) fn parse_response<T: DeserializeOwned>(endpoint: &str, body: &str) -> AppResult<T> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| AppError::msg("数据格式待适配：响应不是有效 JSON"))?;
    if endpoint != "rate-limit-reset-credits/history" {
        crate::analysis_report::validate_response(endpoint, &value).map_err(AppError::msg)?;
    }
    serde_json::from_value(value)
        .map_err(|_| AppError::msg("数据格式待适配：统计字段或单位无法识别"))
}

pub fn usage_series(data: &UsageResponse, dates: &[NaiveDate], by_model: bool) -> Vec<Series> {
    compact_series(usage_series_full(data, dates, by_model), dates.len())
}
fn usage_series_full(data: &UsageResponse, dates: &[NaiveDate], by_model: bool) -> Vec<Series> {
    let mut values: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for day in &data.data {
        let Some(index) = dates.iter().position(|date| *date == day.date) else {
            continue;
        };
        let buckets = if by_model {
            day.models
                .iter()
                .map(|m| (m.model.clone(), m.credits))
                .collect::<Vec<_>>()
        } else {
            day.product_surface_usage_values
                .iter()
                .map(|(surface, value)| {
                    (
                        if surface.starts_with("work_") {
                            "Work"
                        } else {
                            "Codex"
                        }
                        .to_string(),
                        *value,
                    )
                })
                .collect()
        };
        for (name, value) in buckets {
            if value.is_finite() && value > 0.0 {
                values.entry(name).or_insert_with(|| vec![0.0; dates.len()])[index] += value;
            }
        }
    }
    let mut series = values
        .into_iter()
        .map(|(name, values)| Series { name, values })
        .collect::<Vec<_>>();
    series.sort_by(|a, b| {
        sum(&b.values)
            .total_cmp(&sum(&a.values))
            .then(a.name.cmp(&b.name))
    });
    series
}

fn compact_series(mut series: Vec<Series>, days: usize) -> Vec<Series> {
    // 颜色保持唯一，长尾模型聚合后仍保留每日总量。
    if series.len() > 6 {
        let mut other = vec![0.0; days];
        for tail in series.drain(5..) {
            for (index, value) in tail.values.iter().enumerate() {
                other[index] += value;
            }
        }
        series.push(Series {
            name: "Other".into(),
            values: other,
        });
    }
    series
}

/// 分组 0 产品、1 模型、2 来源、3 模型与速度。未知来源保留原名。
pub fn grouped_usage(
    data: &UsageResponse,
    dates: &[NaiveDate],
    group: i32,
    model: &str,
    source: &str,
) -> Vec<Series> {
    if group < 2 && model.is_empty() && source.is_empty() {
        return usage_series(data, dates, group == 1);
    }
    compact_series(
        grouped_usage_full(data, dates, group, model, source),
        dates.len(),
    )
}
pub fn grouped_usage_full(
    data: &UsageResponse,
    dates: &[NaiveDate],
    group: i32,
    model: &str,
    source: &str,
) -> Vec<Series> {
    if group < 2 && model.is_empty() && source.is_empty() {
        return usage_series_full(data, dates, group == 1);
    }
    let mut copy = data.clone();
    for day in &mut copy.data {
        if group == 3 || (group == 1 && source.is_empty()) {
            day.models.retain(|m| model.is_empty() || m.model == model);
            for m in &mut day.models {
                if group != 3 {
                    continue;
                }
                m.model = format!(
                    "{} · {}",
                    m.model,
                    if m.speed.is_empty() {
                        "未知速度"
                    } else {
                        &m.speed
                    }
                );
            }
        } else {
            day.models = day
                .attribution
                .iter()
                .filter(|a| {
                    (model.is_empty() || a.model == model)
                        && (source.is_empty() || source_key(a) == source)
                })
                .map(|a| ModelUsage {
                    model: match group {
                        0 => a.surface.clone(),
                        1 => a.model.clone(),
                        _ => source_key(a),
                    },
                    speed: String::new(),
                    credits: a.value,
                })
                .collect();
        }
    }
    usage_series_full(&copy, dates, true)
}
pub fn source_key(a: &Attribution) -> String {
    let raw = format!(
        "{} / {}",
        if a.thread_source.is_empty() {
            "未知来源"
        } else {
            &a.thread_source
        },
        if a.turn_trigger.is_empty() {
            "未知触发"
        } else {
            &a.turn_trigger
        }
    );
    let label = match a.turn_trigger.as_str() {
        "composer" => "手动对话",
        "automation_heartbeat_scheduled" => "定时任务",
        "app_update_resume" => "更新后恢复",
        "automaticretry" | "automatic_retry" | "capacity_retry_automatic" => "自动重试",
        _ => match a.thread_source.as_str() {
            "computer_history" => "电脑历史",
            "memory_consolidation" => "记忆整理",
            "suggestions" | "ambient_suggestions" => "建议生成",
            "ambient_suggestion_safety" => "建议安全检查",
            "thread_title" => "会话标题",
            _ => return raw,
        },
    };
    format!("{label}（{raw}）")
}
pub fn composition(
    data: &UsageResponse,
    dates: &[NaiveDate],
    group: i32,
    model: &str,
    source: &str,
) -> String {
    let series = grouped_usage(data, dates, group, model, source);
    let total: f64 = series.iter().map(|s| s.values.iter().sum::<f64>()).sum();
    let coverage = dates
        .iter()
        .filter(|d| data.data.iter().any(|day| day.date == **d))
        .count();
    let attributed = data
        .data
        .iter()
        .filter(|d| dates.contains(&d.date))
        .any(|d| !d.attribution.is_empty());
    let mut lines = vec![format!("返回日期 {coverage}/{} · 单位 {}。比例表示筛选范围内的消耗构成，不等于本周已用额度或费用；今天的数据可能仍未汇总。", dates.len(), data.units)];
    if group == 3 {
        lines.push("速度数据来自独立的模型统计，来源筛选仅用于其他分组。".into());
    }
    if !attributed && (group == 2 || !source.is_empty() || (!model.is_empty() && group == 0)) {
        lines.push("接口未提供消耗来源明细，不能将缺失值视为零。".into());
    }
    if total > 0. {
        lines.extend(series.iter().map(|s| {
            let n = s.values.iter().sum::<f64>();
            format!(
                "{}：{:.2}% · {:.4} {}",
                if s.name == "Other" { "其他" } else { &s.name },
                n / total * 100.,
                n,
                data.units
            )
        }));
    } else {
        lines.push("当前筛选没有已返回的正用量；缺失数据不代表零消耗。".into());
    }
    lines.join("\n")
}

pub fn activity_ranking(
    data: &ActivityResponse,
    dates: &[NaiveDate],
    by_client: bool,
) -> (f64, Vec<(String, f64)>) {
    let date_set = dates.iter().collect::<BTreeSet<_>>();
    let mut totals = BTreeMap::new();
    let mut total = 0.0;
    for day in &data.data {
        if !date_set.contains(&day.date) {
            continue;
        }
        total += day.totals.turns as f64;
        let counts = if by_client {
            day.clients
                .iter()
                .map(|m| (m.client_id.clone(), m.turns))
                .collect::<Vec<_>>()
        } else {
            day.models
                .iter()
                .map(|m| (m.model.clone(), m.turns))
                .collect()
        };
        for (name, value) in counts {
            *totals.entry(name).or_insert(0.0) += value as f64;
        }
    }
    let remainder = total - totals.values().sum::<f64>();
    if remainder > 0.0 {
        *totals.entry("Other".into()).or_insert(0.0) += remainder;
    }
    (total, sorted(totals))
}

pub fn tool_ranking(data: &ToolResponse, dates: &[NaiveDate]) -> (f64, Vec<(String, f64)>) {
    let date_set = dates.iter().collect::<BTreeSet<_>>();
    let mut totals = BTreeMap::new();
    for day in &data.data {
        if !date_set.contains(&day.date) {
            continue;
        }
        for tool in &day.plugin_usage_overviews {
            let name = if tool.display_name.trim().is_empty() {
                &tool.plugin_name
            } else {
                &tool.display_name
            };
            *totals
                .entry(if name.is_empty() {
                    "Other".into()
                } else {
                    name.clone()
                })
                .or_insert(0.0) += tool.invocation_counts as f64;
        }
    }
    (totals.values().sum(), sorted(totals))
}

fn sorted(values: BTreeMap<String, f64>) -> Vec<(String, f64)> {
    let mut values = values
        .into_iter()
        .filter(|(_, value)| *value > 0.0)
        .collect::<Vec<_>>();
    values.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    values
}

fn sum(values: &[f64]) -> f64 {
    values.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_groups_products_and_model_speeds_without_changing_units() {
        let payload = r#"{"units":"percent","data":[{"date":"2026-09-07","product_surface_usage_values":{"desktop_app":120,"work_desktop":12},"models":[{"model":"m1","credits":40},{"model":"m1","credits":80},{"model":"m2","credits":12}]}]}"#;
        let data: UsageResponse = serde_json::from_str(payload).unwrap();
        let dates = dates(NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(), 7);
        assert_eq!(data.units, "percent");
        let products = usage_series(&data, &dates, false);
        assert_eq!(products[0].values, vec![0., 0., 0., 0., 0., 0., 120.]);
        assert_eq!(products[1].name, "Work");
        assert_eq!(usage_series(&data, &dates, true)[0].values[6], 120.);
    }

    #[test]
    fn partial_failure_preserves_success_timestamp_and_cached_data() {
        let mut panel = Panel {
            data: Some(42),
            updated_at: 10,
            error: String::new(),
        };
        panel.update(Err(AppError::msg("403")));
        assert_eq!(panel.data, Some(42));
        assert_eq!(panel.updated_at, 10);
        assert_eq!(panel.error, "403");
        panel.update(Ok(43));
        assert_eq!(panel.data, Some(43));
        assert!(panel.error.is_empty());
    }

    #[test]
    fn activity_preserves_unclassified_turns_and_filters_dates() {
        let data: ActivityResponse = serde_json::from_str(r#"{"data":[{"date":"2026-09-07","totals":{"turns":10},"models":[{"model":"m1","turns":7}],"clients":[]},{"date":"2026-08-01","totals":{"turns":999}}]}"#).unwrap();
        let dates = dates(NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(), 7);
        let (total, values) = activity_ranking(&data, &dates, false);
        assert_eq!(total, 10.);
        assert_eq!(values, vec![("m1".into(), 7.), ("Other".into(), 3.)]);
    }

    #[test]
    fn tools_parse_both_endpoints_and_combine_names() {
        let data: ToolResponse = serde_json::from_str(r#"{"data":[{"date":"2026-09-07","skill_usage_overviews":[{"skill_name":"test","display_name":"","invocation_counts":4},{"skill_name":"test","invocation_counts":2}]}]}"#).unwrap();
        let dates = dates(NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(), 7);
        assert_eq!(tool_ranking(&data, &dates), (6., vec![("test".into(), 6.)]));
        assert!(serde_json::from_str::<UsageResponse>(r#"{"unexpected":1}"#).is_err());
    }

    #[test]
    fn thirty_days_cross_month_and_long_tail_preserves_totals() {
        let dates = dates(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(), 30);
        assert_eq!(dates[0].to_string(), "2026-01-31");
        let data = UsageResponse {
            units: "percent".into(),
            data: vec![UsageDay {
                date: dates[29],
                product_surface_usage_values: BTreeMap::new(),
                attribution: vec![],
                models: (0..10)
                    .map(|i| ModelUsage {
                        model: format!("m{i}"),
                        credits: 1.,
                        speed: String::new(),
                    })
                    .collect(),
            }],
        };
        let series = usage_series(&data, &dates, true);
        assert_eq!(series.len(), 6);
        assert_eq!(series.iter().map(|s| s.values[29]).sum::<f64>(), 10.);
    }
    #[test]
    fn attribution_and_speed_preserve_totals_and_unknown_labels() {
        let data: UsageResponse = serde_json::from_str(r#"{"units":"percent","data":[{"date":"2026-10-10","product_surface_usage_values":{"codex":4},"models":[{"model":"x","speed":"standard","credits":1},{"model":"x","speed":"fast","credits":3}],"attribution":[{"thread_source":"user","turn_trigger":"composer","model":"x","value":1},{"thread_source":"future","turn_trigger":"new","model":"x","value":3}]}]}"#).unwrap();
        let ds = dates(NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(), 7);
        for group in [0, 1, 2, 3] {
            assert_eq!(
                grouped_usage(&data, &ds, group, "", "")
                    .iter()
                    .map(|s| s.values.iter().sum::<f64>())
                    .sum::<f64>(),
                4.
            );
        }
        assert_eq!(
            grouped_usage(&data, &ds, 2, "x", "手动对话（user / composer）")[0].values[6],
            1.
        );
        assert!(composition(&data, &ds, 2, "", "").contains("future / new：75.00%"));
        assert!(composition(&data, &ds, 3, "", "").contains("x · fast"));
    }
    #[test]
    fn model_filter_works_without_attribution_and_speed_does_not_invent_source() {
        let data: UsageResponse = serde_json::from_str(r#"{"units":"percent","data":[{"date":"2026-10-10","product_surface_usage_values":{},"models":[{"model":"x","credits":4}]}]}"#).unwrap();
        let ds = dates(NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(), 7);
        assert_eq!(grouped_usage(&data, &ds, 1, "x", "")[0].values[6], 4.);
        assert!(composition(&data, &ds, 2, "", "").contains("接口未提供"));
    }
}
