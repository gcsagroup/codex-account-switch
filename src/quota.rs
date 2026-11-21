//! 只读额度详情。可用次数与此刻可使用次数分开，缺失字段保持未知。
use crate::{
    error::{AppError, AppResult},
    usage::{self, UsageSnapshot},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Status {
    pub rate_limit: Option<Limit>,
    pub code_review_rate_limit: Option<Limit>,
    pub additional_rate_limits: Option<serde_json::Value>,
    #[serde(deserialize_with = "nullable_models")]
    pub model_usage: BTreeMap<String, ModelStatus>,
    pub credits: Option<Credits>,
    pub spend_control: Option<Spend>,
    pub rate_limit_reached_type: Option<String>,
    pub rate_limit_reset_credits: Option<ResetSummary>,
}
fn nullable_models<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, ModelStatus>, D::Error> {
    Ok(Option::<BTreeMap<String, ModelStatus>>::deserialize(deserializer)?.unwrap_or_default())
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Limit {
    pub allowed: Option<bool>,
    pub limit_reached: Option<bool>,
    pub primary_window: Option<serde_json::Value>,
    pub secondary_window: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ModelStatus {
    pub available: Option<bool>,
    pub available_at: Option<serde_json::Value>,
    pub credits_would_enable: Option<bool>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Credits {
    pub has_credits: Option<bool>,
    pub unlimited: Option<bool>,
    pub overage_limit_reached: Option<bool>,
    pub balance: Option<String>,
    pub approx_local_messages: Option<Vec<i64>>,
    pub approx_cloud_messages: Option<Vec<i64>>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Spend {
    pub reached: Option<bool>,
    pub individual_limit: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ResetSummary {
    pub available_count: Option<i64>,
    pub applicable_available_count: Option<i64>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ResetItem {
    pub status: String,
    pub reset_type: String,
    pub is_supported_by_plan: Option<bool>,
    pub expires_at: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ResetDetails {
    pub credits: Vec<ResetItem>,
    pub fetched_at: i64,
    pub error: String,
}

pub fn fetch_resets(token: &str, account: &str, previous: ResetDetails) -> ResetDetails {
    let result = (|| -> AppResult<ResetDetails> {
        let response = usage::http_client()?
            .get("https://chatgpt.com/backend-api/wham/rate-limit-reset-credits")
            .bearer_auth(token)
            .header("ChatGPT-Account-Id", account)
            .send()?;
        let status = response.status();
        let body = response.text()?;
        if !status.is_success() {
            return Err(AppError::msg(usage::friendly_http_error(
                status.as_u16(),
                &body,
            )));
        }
        let mut details: ResetDetails = serde_json::from_str(&body)?;
        details.fetched_at = chrono::Utc::now().timestamp();
        Ok(details)
    })();
    match result {
        Ok(details) => details,
        Err(e) => ResetDetails {
            error: e.to_string(),
            ..previous
        },
    }
}
fn fresh(s: &UsageSnapshot, now: i64) -> bool {
    s.error.is_empty() && s.fetched_at > 0 && (0..=900).contains(&(now - s.fetched_at))
}
fn yn(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "是",
        Some(false) => "否",
        None => "未知",
    }
}
fn count(value: Option<i64>) -> String {
    value
        .filter(|v| *v >= 0)
        .map(|v| v.to_string())
        .unwrap_or_else(|| "未知".into())
}
fn timestamp(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|d| d.timestamp())
}
fn range(value: &Option<Vec<i64>>) -> String {
    value
        .as_ref()
        .filter(|v| v.len() == 2 && v[0] >= 0 && v[1] >= v[0])
        .map(|v| format!("{}–{}", v[0], v[1]))
        .unwrap_or_else(|| "未知".into())
}
fn limit_text(name: &str, limit: &Limit) -> String {
    let windows = [&limit.primary_window, &limit.secondary_window]
        .into_iter()
        .flatten()
        .map(|w| {
            let used = w
                .get("used_percent")
                .and_then(|v| v.as_f64())
                .filter(|v| v.is_finite() && *v >= 0.)
                .map(|v| format!("{v:.0}%"))
                .unwrap_or_else(|| "未知".into());
            let hours = w
                .get("limit_window_seconds")
                .and_then(|v| v.as_i64())
                .map(|v| format!("{} 小时", v / 3600))
                .unwrap_or_else(|| "时长未知".into());
            let reset = usage::format_reset(w.get("reset_at").and_then(|v| v.as_i64()));
            format!("{hours}窗口已用 {used}，重置 {reset}")
        })
        .collect::<Vec<_>>()
        .join("；");
    format!(
        "{name}：允许 {} · 达限 {}{}",
        yn(limit.allowed),
        yn(limit.limit_reached),
        if windows.is_empty() {
            String::new()
        } else {
            format!(" · {windows}")
        }
    )
}
fn extra_text(name: &str, value: &serde_json::Value) -> String {
    let label = value
        .get("limit_name")
        .or_else(|| value.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or(name);
    let value = value.get("rate_limit").unwrap_or(value);
    serde_json::from_value::<Limit>(value.clone())
        .map(|r| limit_text(label, &r))
        .unwrap_or_else(|_| format!("{label}：尚未识别此额度格式"))
}
pub fn reset_label(snapshot: Option<&UsageSnapshot>) -> String {
    let reset = snapshot.and_then(|s| s.quota.rate_limit_reset_credits.as_ref());
    format!(
        "拥有重置 {} · 当前可使用 {}",
        count(
            reset
                .and_then(|r| r.available_count)
                .or_else(|| snapshot.and_then(|s| s.reset_credits_available))
        ),
        count(reset.and_then(|r| r.applicable_available_count))
    )
}
pub fn balance_text(balance: &str) -> String {
    if balance.contains('.') {
        balance
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    } else {
        balance.to_string()
    }
}

/// 列表按整数展示并标记近似值；详情保留原始精度，缺失或无效余额不冒充零。
pub fn balance_summary(s: &UsageSnapshot) -> String {
    if s.quota
        .credits
        .as_ref()
        .is_some_and(|c| c.unlimited == Some(true))
    {
        return "不限".into();
    }
    let Some(balance) = s
        .credits_balance
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0. && *v < i64::MAX as f64)
    else {
        return "—".into();
    };
    format!(
        "{}{}",
        if balance.fract() == 0. { "" } else { "≈ " },
        crate::format_integer(balance.round() as i64)
    )
}
pub fn details(s: &UsageSnapshot, now: i64) -> String {
    let q = &s.quota;
    let mut lines = vec![format!(
        "额度状态：{} · 成功读取 {}",
        if fresh(s, now) {
            "已更新"
        } else {
            "缓存／待刷新"
        },
        if s.fetched_at > 0 {
            usage::format_reset(Some(s.fetched_at))
        } else {
            "尚未成功".into()
        }
    )];
    if let Some(r) = &q.rate_limit {
        lines.push(format!(
            "服务允许使用：{} · 已达上限：{}",
            yn(r.allowed),
            yn(r.limit_reached)
        ));
    }
    if let Some(r) = &q.rate_limit_reached_type {
        lines.push(format!("限制原因：{r}"));
    }
    if let Some(r) = &q.spend_control {
        lines.push(format!(
            "支出限制已触发：{} · 个人上限：{}",
            yn(r.reached),
            r.individual_limit
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "未知".into())
        ));
    }
    let reset = q.rate_limit_reset_credits.as_ref();
    lines.push(format!(
        "重置次数：拥有 {} · 当前可使用 {}",
        count(
            reset
                .and_then(|r| r.available_count)
                .or(s.reset_credits_available)
        ),
        count(reset.and_then(|r| r.applicable_available_count))
    ));
    for credit in &s.reset_details.credits {
        lines.push(format!(
            "重置凭证：{} · 套餐支持 {} · 到期 {}（本地时间）",
            match credit.status.as_str() {
                "available" => "未使用",
                "redeemed" => "已使用",
                "expired" => "已到期",
                _ => &credit.status,
            },
            yn(credit.is_supported_by_plan),
            credit
                .expires_at
                .as_deref()
                .and_then(timestamp)
                .map(|t| usage::format_reset(Some(t)))
                .unwrap_or_else(|| "未知".into())
        ));
    }
    lines.push(format!(
        "重置详情读取：{}{}",
        if s.reset_details.fetched_at > 0 {
            usage::format_reset(Some(s.reset_details.fetched_at))
        } else {
            "尚未成功".into()
        },
        if s.reset_details.error.is_empty() {
            String::new()
        } else {
            format!(" · 缓存：{}", s.reset_details.error)
        }
    ));
    if let Some(c) = &q.credits {
        lines.push(format!(
            "Credits：{} · 有余额 {} · 无限 {} · 超额上限已达 {}",
            balance_text(c.balance.as_deref().unwrap_or("未知")),
            yn(c.has_credits),
            yn(c.unlimited),
            yn(c.overage_limit_reached)
        ));
        lines.push(format!(
            "预计可用消息：本地 {} · 云端 {}（服务端估算，非美元）",
            range(&c.approx_local_messages),
            range(&c.approx_cloud_messages)
        ));
    }
    for (model, status) in &q.model_usage {
        lines.push(format!(
            "{model}：可用 {} · Credits 可启用 {} · 可用时间 {}",
            yn(status.available),
            yn(status.credits_would_enable),
            status
                .available_at
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "未知".into())
        ));
    }
    if let Some(r) = &q.code_review_rate_limit {
        lines.push(limit_text("代码审查独立额度", r));
    }
    if let Some(extra) = &q.additional_rate_limits {
        match extra {
            serde_json::Value::Array(items) => {
                for item in items {
                    lines.push(extra_text("其他独立额度", item));
                }
            }
            serde_json::Value::Object(items) => {
                for (name, item) in items {
                    lines.push(extra_text(name, item));
                }
            }
            _ => lines.push("其他独立额度：服务返回了尚未识别的格式".into()),
        }
    }
    if let Some(p) = &s.token_profile {
        lines.push(format!(
            "活动数据截至：{} · 服务生成：{}{}",
            p.stats_as_of.as_deref().unwrap_or("未知"),
            p.generated_at.as_deref().unwrap_or("未知"),
            if s.activity_error.is_empty() {
                String::new()
            } else {
                format!(" · 缓存：{}", s.activity_error)
            }
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_payload_keeps_owned_and_applicable_resets_distinct() {
        let body = r#"{"rate_limit":{"allowed":true,"limit_reached":false,"primary_window":{"used_percent":1,"limit_window_seconds":604800}},"credits":{"balance":"52305.8451230000","has_credits":true,"unlimited":false,"approx_local_messages":[13076,67998],"approx_cloud_messages":[2092,13076]},"model_usage":{"example-model":{"available":true,"credits_would_enable":false}},"rate_limit_reset_credits":{"available_count":3,"applicable_available_count":0}}"#;
        let s = usage::parse_usage_payload_for_test(body, "example").unwrap();
        assert_eq!(s.secondary.used_percent, 1.);
        assert_eq!(balance_summary(&s), "≈ 52,306");
        assert_eq!(balance_text(&s.credits_balance), "52305.845123");
        assert_eq!(reset_label(Some(&s)), "拥有重置 3 · 当前可使用 0");
        let text = details(&s, s.fetched_at);
        assert!(text.contains("本地 13076–67998 · 云端 2092–13076"));
        assert!(text.contains("example-model：可用 是"));
    }

    #[test]
    fn unknown_invalid_zero_and_unlimited_balances_stay_distinct() {
        let mut s = UsageSnapshot::default();
        for raw in ["", "NaN", "inf", "-1", "invalid", "1e30"] {
            s.credits_balance = raw.into();
            assert_eq!(balance_summary(&s), "—");
        }
        s.credits_balance = "0.0000".into();
        assert_eq!(balance_summary(&s), "0");
        s.credits_balance = "1000.0000".into();
        assert_eq!(balance_summary(&s), "1,000");
        s.quota.credits = Some(Credits {
            unlimited: Some(true),
            ..Default::default()
        });
        assert_eq!(balance_summary(&s), "不限");
        assert_eq!(reset_label(Some(&s)), "拥有重置 未知 · 当前可使用 未知");
        assert!(details(&s, 1000).contains("成功读取 尚未成功"));
    }

    #[test]
    fn missing_percent_never_displays_as_full_remaining_quota() {
        for value in [serde_json::Value::Null, serde_json::json!(-1)] {
            let body = serde_json::json!({"rate_limit":{"primary_window":{
                "used_percent":value,"limit_window_seconds":604800,"reset_at":3000
            }}});
            let s = usage::parse_usage_payload_for_test(&body.to_string(), "example").unwrap();
            assert!(!usage::window_usage_available(&s.secondary, 2000));
            assert_eq!(
                usage::format_usage_row(&s, 2000, "待刷新", "无窗口"),
                ("待刷新".into(), true)
            );
        }
        let s = usage::parse_usage_payload_for_test(r#"{"rate_limit":{"primary_window":{"used_percent":0,"limit_window_seconds":604800,"reset_at":3000}}}"#, "example").unwrap();
        assert!(usage::window_usage_available(&s.secondary, 2000));
        assert!(!usage::window_usage_available(&s.secondary, 3000));
    }

    #[test]
    fn failed_refresh_preserves_quota_and_expiry_with_cache_status() {
        let old = UsageSnapshot {
            fetched_at: 1000,
            quota: Status { rate_limit_reset_credits: Some(ResetSummary { available_count: Some(3), applicable_available_count: Some(0) }), ..Default::default() },
            reset_details: serde_json::from_str(r#"{"credits":[{"status":"available","is_supported_by_plan":true,"expires_at":"2026-11-30T20:17:31Z"}],"fetched_at":1000,"error":"offline"}"#).unwrap(),
            ..Default::default()
        };
        let failed = usage::failed_snapshot(Some(old.clone()), "example", "offline".into(), 2000);
        assert_eq!(failed.quota, old.quota);
        assert_eq!(failed.reset_details, old.reset_details);
        let text = details(&failed, 2000);
        assert!(text.contains("缓存／待刷新"));
        assert!(text.contains("重置凭证：未使用 · 套餐支持 是 · 到期"));
        assert!(text.contains("缓存：offline"));
    }
}
