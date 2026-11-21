//! 分析页的数据契约、覆盖率、完整期间对比与可复核导出。
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::analytics::{self, Snapshot, UsageResponse};

pub fn validate_response(endpoint: &str, value: &Value) -> Result<(), String> {
    let fail = || "数据格式待适配：统计字段、日期或数值无效，已保留上次成功数据".to_string();
    let rows = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(fail)?;
    if value.get("has_more").and_then(Value::as_bool) == Some(true)
        || ["next_cursor", "next_page", "next_page_token"]
            .iter()
            .any(|k| {
                value
                    .get(k)
                    .is_some_and(|v| !v.is_null() && v.as_str() != Some(""))
            })
    {
        return Err("数据不完整：接口返回未读取的后续分页，未使用部分结果".into());
    }
    if value
        .get("group_by")
        .is_some_and(|g| g.as_str() != Some("day"))
    {
        return Err(fail());
    }
    let finite = |v: &Value| v.as_f64().is_some_and(|n| n.is_finite() && n >= 0.);
    let mut dates = BTreeSet::new();
    for row in rows {
        let date = row
            .get("date")
            .and_then(Value::as_str)
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .ok_or_else(fail)?;
        if !dates.insert(date) {
            return Err("数据格式待适配：同一日期重复，未重复累计".into());
        }
        match endpoint {
            "usage/daily-token-usage-breakdown" => {
                let surfaces = row
                    .get("product_surface_usage_values")
                    .and_then(Value::as_object)
                    .ok_or_else(fail)?;
                if !surfaces.values().all(finite) {
                    return Err(fail());
                }
                for (field, amount) in [("models", "credits"), ("attribution", "value")] {
                    if let Some(items) = row.get(field) {
                        let items = items.as_array().ok_or_else(fail)?;
                        if !items.iter().all(|m| m.get(amount).is_some_and(finite)) {
                            return Err(fail());
                        }
                    }
                }
            }
            "analytics/daily-workspace-usage-counts" => {
                let total = row
                    .pointer("/totals/turns")
                    .and_then(Value::as_u64)
                    .ok_or_else(fail)?;
                for field in ["models", "clients"] {
                    if let Some(items) = row.get(field) {
                        let items = items.as_array().ok_or_else(fail)?;
                        let sum = items.iter().try_fold(0u64, |sum, item| {
                            sum.checked_add(item.get("turns")?.as_u64()?)
                        });
                        if !sum.is_some_and(|sum| sum <= total) {
                            return Err(fail());
                        }
                    }
                }
            }
            "usage/daily-workspace-user-token-usage-breakdown"
                if !row
                    .get("models")
                    .or_else(|| row.get("groups"))
                    .is_some_and(Value::is_array) =>
            {
                return Err(fail());
            }
            _ => {}
        }
    }
    if endpoint == "usage/daily-token-usage-breakdown"
        && !value
            .get("units")
            .and_then(Value::as_str)
            .is_some_and(|u| !u.trim().is_empty())
    {
        return Err(fail());
    }
    Ok(())
}

// 空模型/来源数组表示该维度未提供，不能由其它维度推断为零。
pub fn daily_usage(
    data: &UsageResponse,
    dates: &[NaiveDate],
    group: i32,
    model: &str,
    source: &str,
) -> Vec<Option<f64>> {
    let series = analytics::grouped_usage(data, dates, group, model, source);
    dates
        .iter()
        .enumerate()
        .map(|(i, date)| {
            let rows: Vec<_> = data.data.iter().filter(|r| r.date == *date).collect();
            if rows.len() != 1 {
                return None;
            }
            let day = rows[0];
            let supplied = if group == 3 || (group == 1 && source.is_empty()) {
                !day.models.is_empty()
            } else if group == 0 && model.is_empty() && source.is_empty() {
                !day.product_surface_usage_values.is_empty()
            } else {
                !day.attribution.is_empty()
            };
            supplied.then(|| series.iter().map(|s| s.values[i]).sum())
        })
        .collect()
}

pub fn coverage(covered: usize, total: usize) -> String {
    if covered == 0 {
        format!("无可用日期 · 0/{total} 天")
    } else if covered == total {
        format!("已提供 {covered}/{total} 天 · 不代表当日已完整")
    } else {
        format!("部分数据 · {covered}/{total} 天")
    }
}

pub fn comparison_text(snapshot: &Snapshot, today: NaiveDate, days: i32) -> String {
    let end = today - Duration::days(1);
    let current_dates = analytics::dates(end, days);
    let previous_dates = analytics::dates(end - Duration::days(days.into()), days);
    let mut lines = vec![format!(
        "完整日历期间 · UTC\n{} — {} 对比 {} — {}。不含今天；与本页模型、来源筛选独立。",
        current_dates[0],
        end,
        previous_dates[0],
        previous_dates[previous_dates.len() - 1]
    )];
    if let Some(profile) = &snapshot.tokens.data {
        let a = crate::valuation::token_series(profile, &current_dates);
        let b = crate::valuation::token_series(profile, &previous_dates);
        if a.covered == days as usize
            && b.covered == days as usize
            && snapshot.tokens.error.is_empty()
        {
            lines.push(format!(
                "Token：{}",
                change(a.total.map(|n| n as f64), b.total.map(|n| n as f64))
            ));
        } else {
            lines.push(format!(
                "Token 暂不计算环比：本期 {}/{} 天，上期 {}/{} 天{}。",
                a.covered,
                days,
                b.covered,
                days,
                if snapshot.tokens.error.is_empty() {
                    ""
                } else {
                    "；刷新失败，保留旧数据"
                }
            ));
        }
    } else {
        lines.push("Token 暂不计算环比：尚无可用数据。".into());
    }
    let (Some(a), Some(b)) = (
        &snapshot.comparison_recent.data,
        &snapshot.comparison_previous.data,
    ) else {
        lines.push("消耗暂不计算环比：对比数据未就绪。".into());
        for p in [&snapshot.comparison_recent, &snapshot.comparison_previous] {
            if !p.error.is_empty() {
                lines.push(p.error.clone());
            }
        }
        return lines.join("\n");
    };
    let av = daily_usage(a, &current_dates, 0, "", "");
    let bv = daily_usage(b, &previous_dates, 0, "", "");
    if !snapshot.comparison_recent.error.is_empty()
        || !snapshot.comparison_previous.error.is_empty()
        || a.units != b.units
        || av.iter().any(Option::is_none)
        || bv.iter().any(Option::is_none)
    {
        lines.push("消耗暂不计算环比：存在缺失日期、单位变化或刷新失败。".into());
        return lines.join("\n");
    }
    // 没有服务端完整性承诺，明确这是已返回日统计的对比。
    lines.push(format!(
        "已返回日消耗（{}）：{}；统计可能仍有延迟。",
        a.units,
        change(
            Some(av.iter().flatten().sum()),
            Some(bv.iter().flatten().sum())
        )
    ));
    for (group, title) in [(1, "模型"), (2, "来源")] {
        if daily_usage(a, &current_dates, group, "", "")
            .iter()
            .any(Option::is_none)
            || daily_usage(b, &previous_dates, group, "", "")
                .iter()
                .any(Option::is_none)
        {
            lines.push(format!("{title}变化：明细不足，未推断主因。"));
            continue;
        }
        let mut delta = BTreeMap::<String, f64>::new();
        for (data, ds, sign) in [(a, &current_dates, 1.), (b, &previous_dates, -1.)] {
            for s in analytics::grouped_usage_full(data, ds, group, "", "") {
                *delta.entry(s.name).or_default() += sign * s.values.iter().sum::<f64>();
            }
        }
        if let Some((name, amount)) = delta.iter().max_by(|a, b| a.1.abs().total_cmp(&b.1.abs())) {
            lines.push(format!(
                "{title}最大变化：{name} {amount:+.4} {}（分组变化，不代表因果）。",
                a.units
            ));
        }
    }
    lines.join("\n")
}

fn change(a: Option<f64>, b: Option<f64>) -> String {
    match (a, b) {
        (Some(a), Some(b)) if b > 0. => format!("{a:.2} / {b:.2} · {:+.1}%", (a - b) / b * 100.),
        (Some(a), Some(0.)) => format!("{a:.2} / 0 · 上期为零，不计算百分比"),
        _ => "数据不足".into(),
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub range: i32,
    pub group: i32,
    pub model: String,
    pub source: String,
    pub show_history: bool,
}

pub fn read_preferences(path: &Path) -> crate::error::AppResult<BTreeMap<String, Preferences>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(e.into()),
    }
}
pub fn save_preferences(
    path: &Path,
    account: &str,
    prefs: Preferences,
) -> crate::error::AppResult<()> {
    let mut all = read_preferences(path)?;
    all.insert(account.into(), prefs);
    crate::profile::atomic_write(path, &serde_json::to_vec_pretty(&all)?)
}

// 文本单元格避免被电子表格当作公式执行；保留数值列为数字。
pub fn csv_cell(value: &str) -> String {
    let safe = if value
        .trim_start()
        .starts_with(['=', '+', '-', '@', '\t', '\r'])
    {
        format!("'{value}")
    } else {
        value.to_string()
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}
pub fn csv_row(values: &[String]) -> String {
    values
        .iter()
        .map(|s| csv_cell(s))
        .collect::<Vec<_>>()
        .join(",")
        + "\r\n"
}

pub fn export_cloud(
    snapshot: &Snapshot,
    account_label: &str,
    dates: &[NaiveDate],
    group: i32,
    model: &str,
    source: &str,
) -> String {
    let mut out = String::from("\u{feff}");
    for (k, v) in [
        ("范围", account_label.to_string()),
        (
            "日期口径",
            format!("{} — {} · UTC", dates[0], dates[dates.len() - 1]),
        ),
        (
            "筛选作用",
            format!("消耗明细：分组 {group} / 模型 {model} / 来源 {source}；Token 为账号总量"),
        ),
        (
            "价格版本",
            "已核验价格快照；所选情景与逐模型核验日期见下方估值基准；不是账单".into(),
        ),
    ] {
        out += &csv_row(&[k.into(), v]);
    }
    let mut tokens = crate::valuation::token_series(
        snapshot.tokens.data.as_ref().unwrap_or(&Default::default()),
        dates,
    );
    crate::model_valuation::merge_reported_totals(
        &mut tokens,
        dates,
        snapshot.detailed_tokens.data.as_ref(),
    );
    let usage = snapshot.usage.data.as_ref();
    let daily = usage
        .map(|d| daily_usage(d, dates, group, model, source))
        .unwrap_or_else(|| vec![None; dates.len()]);
    out += &csv_row(&[
        "Token覆盖".into(),
        coverage(tokens.covered, dates.len()),
        "消耗覆盖".into(),
        coverage(daily.iter().flatten().count(), dates.len()),
    ]);
    out += &csv_row(&[
        "Token统计截至".into(),
        snapshot
            .tokens
            .data
            .as_ref()
            .and_then(|p| p.stats_as_of.clone())
            .unwrap_or_else(|| "未提供".into()),
        "消耗统计截至".into(),
        "接口未提供".into(),
    ]);
    out += &csv_row(&[
        "Token最近成功(Unix秒)".into(),
        snapshot.tokens.updated_at.to_string(),
        "消耗最近成功(Unix秒)".into(),
        snapshot.usage.updated_at.to_string(),
    ]);
    out += &csv_row(&[
        "Token刷新错误".into(),
        snapshot.tokens.error.clone(),
        "消耗刷新错误".into(),
        snapshot.usage.error.clone(),
    ]);
    out += &csv_row(&[
        "日期".into(),
        "账号Token合计".into(),
        "已返回筛选消耗".into(),
        "消耗单位".into(),
        "状态".into(),
    ]);
    for (i, d) in dates.iter().enumerate() {
        out += &csv_row(&[
            d.to_string(),
            tokens.daily[i].map(|n| n.to_string()).unwrap_or_default(),
            daily[i].map(|n| n.to_string()).unwrap_or_default(),
            usage.map(|u| u.units.clone()).unwrap_or_default(),
            "空白为缺失；今天可能未完整".into(),
        ]);
    }
    if let Some(data) = usage {
        out += &csv_row(&["消耗分组".into(), "日期".into(), "值".into(), "单位".into()]);
        for s in analytics::grouped_usage_full(data, dates, group, model, source) {
            for (i, date) in dates.iter().enumerate() {
                out += &csv_row(&[
                    s.name.clone(),
                    date.to_string(),
                    daily[i]
                        .map(|_| s.values[i].to_string())
                        .unwrap_or_default(),
                    data.units.clone(),
                ]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_dates_negative_values_pagination_and_unknown_shape() {
        let endpoint = "usage/daily-token-usage-breakdown";
        for body in [
            r#"{"data":[],"units":"percent","has_more":true}"#,
            r#"{"data":[{"date":"2026-11-01","product_surface_usage_values":{"cli":-1}}],"units":"percent"}"#,
            r#"{"data":[{"date":"2026-11-01","product_surface_usage_values":{}},{"date":"2026-11-01","product_surface_usage_values":{}}],"units":"percent"}"#,
            r#"{"results":[],"units":"percent"}"#,
        ] {
            assert!(validate_response(endpoint, &serde_json::from_str(body).unwrap()).is_err());
        }
        assert!(
            validate_response(endpoint, &serde_json::json!({"data":[],"units":"percent"})).is_ok()
        );
    }
    #[test]
    fn zero_surface_and_absent_model_are_different() {
        let d: UsageResponse=serde_json::from_value(serde_json::json!({"units":"percent","data":[{"date":"2026-11-01","product_surface_usage_values":{"cli":0},"models":[]}]})).unwrap();
        let ds = analytics::dates(NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(), 1);
        assert_eq!(daily_usage(&d, &ds, 0, "", ""), vec![Some(0.)]);
        assert_eq!(daily_usage(&d, &ds, 1, "", ""), vec![None]);
        assert_eq!(daily_usage(&d, &ds, 2, "", ""), vec![None]);
    }
    #[test]
    fn comparisons_refuse_incomplete_or_zero_baseline_percentages() {
        assert!(change(Some(3.), Some(0.)).contains("不计算"));
        assert!(comparison_text(
            &Snapshot::default(),
            NaiveDate::from_ymd_opt(2026, 11, 14).unwrap(),
            7
        )
        .contains("11-13"));
        assert!(comparison_text(
            &Snapshot::default(),
            NaiveDate::from_ymd_opt(2026, 11, 14).unwrap(),
            7
        )
        .contains("暂不计算"));
    }
    #[test]
    fn preferences_survive_restart_and_do_not_overwrite_corrupt_data() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("prefs.json");
        save_preferences(
            &p,
            "a",
            Preferences {
                model: "model-a".into(),
                ..Default::default()
            },
        )
        .unwrap();
        save_preferences(
            &p,
            "b",
            Preferences {
                range: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(read_preferences(&p).unwrap()["a"].model, "model-a");
        std::fs::write(&p, "bad").unwrap();
        assert!(save_preferences(&p, "c", Preferences::default()).is_err());
        assert_eq!(std::fs::read_to_string(p).unwrap(), "bad");
    }
    #[test]
    fn personal_sparse_response_keeps_independent_coverage() {
        let ds = analytics::dates(NaiveDate::from_ymd_opt(2026, 11, 14).unwrap(), 7);
        let body=serde_json::json!({"units":"percent","group_by":"day","data":ds.iter().map(|d|serde_json::json!({
            "date":d,"product_surface_usage_values":{"desktop_app":0},"models":[],"attribution":[]
        })).collect::<Vec<_>>()}).to_string();
        let usage: UsageResponse =
            analytics::parse_response("usage/daily-token-usage-breakdown", &body).unwrap();
        let activity: crate::analytics::ActivityResponse = analytics::parse_response(
            "analytics/daily-workspace-usage-counts",
            r#"{"data":[],"group_by":"day"}"#,
        )
        .unwrap();
        let tokens = crate::usage::TokenUsageProfile {
            daily_usage_buckets: Some(vec![crate::usage::TokenUsageDailyBucket {
                start_date: "2026-11-13".into(),
                tokens: 100,
            }]),
            ..Default::default()
        };
        assert_eq!(crate::valuation::token_series(&tokens, &ds).covered, 1);
        assert_eq!(
            daily_usage(&usage, &ds, 0, "", "").iter().flatten().count(),
            7
        );
        assert_eq!(
            daily_usage(&usage, &ds, 1, "", "").iter().flatten().count(),
            0
        );
        assert!(activity.data.is_empty());
    }
    #[test]
    fn complete_period_comparison_preserves_units_and_stops_on_failure() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
        let period = |end, n| {
            serde_json::from_value::<UsageResponse>(serde_json::json!({"units":"credits","data":analytics::dates(end,7).iter().map(|d|serde_json::json!({"date":d,"product_surface_usage_values":{"cli":n}})).collect::<Vec<_>>()})).unwrap()
        };
        let mut snapshot = Snapshot::default();
        snapshot.comparison_recent.data = Some(period(today - Duration::days(1), 2));
        snapshot.comparison_previous.data = Some(period(today - Duration::days(8), 1));
        assert!(comparison_text(&snapshot, today, 7).contains("+100.0%"));
        snapshot.comparison_recent.error = "network".into();
        assert!(!comparison_text(&snapshot, today, 7).contains("+100.0%"));
    }
    #[test]
    fn exports_leave_missing_values_blank_and_escape_formulas() {
        assert_eq!(csv_cell("=SUM(A1)"), "\"'=SUM(A1)\"");
        let ds = analytics::dates(NaiveDate::from_ymd_opt(2026, 11, 14).unwrap(), 7);
        let csv = export_cloud(&Snapshot::default(), "账号", &ds, 0, "", "");
        assert!(csv.contains("0/7"));
        assert!(csv.contains("\"2026-11-14\",\"\",\"\""));
    }
}
