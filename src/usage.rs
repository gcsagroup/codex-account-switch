use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::paths;

pub const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UsageWindow {
    pub used_percent: f64,
    pub reset_at: Option<i64>,
    pub limit_window_seconds: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UsageSnapshot {
    pub alias: String,
    pub plan_type: String,
    pub primary: UsageWindow,
    pub secondary: UsageWindow,
    pub credits_balance: String,
    pub token_profile: Option<TokenUsageProfile>,
    pub fetched_at: i64,
    pub error: String,
}

impl UsageSnapshot {
    /// Translate any stored Chinese error messages to the current language.
    pub fn localized_error(&self, lang: crate::i18n::Language) -> String {
        if self.error == "无 OAuth token，跳过额度查询" {
            crate::i18n::t(lang, "status.no_token").to_string()
        } else {
            self.error.clone()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TokenUsageProfile {
    pub lifetime_tokens: Option<i64>,
    pub peak_daily_tokens: Option<i64>,
    pub longest_running_turn_sec: Option<i64>,
    pub current_streak_days: Option<i64>,
    pub longest_streak_days: Option<i64>,
    pub daily_usage_buckets: Option<Vec<TokenUsageDailyBucket>>,
    pub weekly_usage_buckets: Option<Vec<TokenUsageDailyBucket>>,
    pub cumulative_daily_usage_buckets: Option<Vec<TokenUsageDailyBucket>>,
    pub fast_mode_usage_percentage: Option<f64>,
    pub most_used_reasoning_effort: Option<String>,
    pub most_used_reasoning_effort_percentage: Option<f64>,
    pub total_skills_used: Option<i64>,
    pub unique_skills_used: Option<i64>,
    pub total_threads: Option<i64>,
    pub top_invocations: Option<Vec<TokenUsageInvocation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsageDailyBucket {
    pub start_date: String,
    pub tokens: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TokenUsageInvocation {
    #[serde(default)]
    pub plugin_id: Option<String>,
    #[serde(default)]
    pub plugin_name: Option<String>,
    #[serde(default)]
    pub skill_id: Option<String>,
    #[serde(default)]
    pub skill_name: Option<String>,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub usage_count: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct UsageCacheFile {
    snapshots: HashMap<String, UsageSnapshot>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    #[serde(default)]
    plan_type: Option<String>,
    #[serde(default)]
    rate_limit: Option<RateLimit>,
    #[serde(default)]
    credits: Option<Credits>,
}

#[derive(Debug, Deserialize)]
struct RateLimit {
    #[serde(default)]
    primary_window: Option<WindowJson>,
    #[serde(default)]
    secondary_window: Option<WindowJson>,
}

#[derive(Debug, Deserialize)]
struct WindowJson {
    #[serde(default)]
    used_percent: Option<f64>,
    #[serde(default)]
    reset_at: Option<i64>,
    #[serde(default)]
    reset_after_seconds: Option<i64>,
    #[serde(default)]
    limit_window_seconds: Option<i64>,
}

/// Windows longer than 2 days are treated as the weekly bucket.
const WEEKLY_WINDOW_SECONDS: i64 = 2 * 24 * 3600;

#[derive(Debug, Deserialize)]
struct Credits {
    #[serde(default)]
    balance: Option<String>,
}

pub fn load_cache() -> HashMap<String, UsageSnapshot> {
    let path = paths::usage_cache_path();
    read_cache_file(&path).unwrap_or_default().snapshots
}

pub fn save_cache(snapshots: &HashMap<String, UsageSnapshot>) -> AppResult<()> {
    let path = paths::usage_cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = UsageCacheFile {
        snapshots: snapshots.clone(),
    };
    let data = serde_json::to_vec_pretty(&file)?;
    atomic_write(&path, &data)?;
    Ok(())
}

pub fn fetch_usage(alias: &str, access_token: &str, account_id: &str) -> AppResult<UsageSnapshot> {
    if access_token.is_empty() || account_id.is_empty() {
        return Ok(UsageSnapshot {
            alias: alias.to_string(),
            error: "无 ChatGPT OAuth 凭据，无法查询额度".into(),
            fetched_at: now_ts(),
            ..UsageSnapshot::default()
        });
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("codex-account-switch/0.1")
        .build()?;

    let response = client
        .get(USAGE_URL)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("ChatGPT-Account-Id", account_id)
        .send()?;

    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        return Err(AppError::msg(friendly_http_error(status.as_u16(), &body)));
    }

    let parsed: UsageResponse = serde_json::from_str(&body)?;
    Ok(snapshot_from_response(alias, parsed))
}

fn friendly_http_error(status: u16, body: &str) -> String {
    let lower = body.to_ascii_lowercase();
    if status == 401
        || lower.contains("token_expired")
        || lower.contains("token is expired")
        || lower.contains("authentication token")
    {
        return "登录已过期，请重新导入该账号的 auth.json".into();
    }
    if status == 403 {
        return "无权限查询额度（403）".into();
    }
    if status == 429 {
        return "查询过于频繁，请稍后再试".into();
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(msg) = v
            .pointer("/error/message")
            .and_then(|m| m.as_str())
            .filter(|s| !s.is_empty())
        {
            return format!("查询失败 HTTP {status}: {}", truncate(msg, 80));
        }
    }
    format!("查询失败 HTTP {status}")
}

fn snapshot_from_response(alias: &str, parsed: UsageResponse) -> UsageSnapshot {
    let fetched_at = now_ts();
    let raw_primary = window_from(
        parsed
            .rate_limit
            .as_ref()
            .and_then(|r| r.primary_window.as_ref()),
        fetched_at,
    );
    let raw_secondary = window_from(
        parsed
            .rate_limit
            .as_ref()
            .and_then(|r| r.secondary_window.as_ref()),
        fetched_at,
    );
    let (short, weekly) = classify_windows(raw_primary, raw_secondary);

    UsageSnapshot {
        alias: alias.to_string(),
        plan_type: parsed.plan_type.unwrap_or_default(),
        // primary = short/5h bucket, secondary = weekly bucket (normalized)
        primary: short,
        secondary: weekly,
        credits_balance: parsed.credits.and_then(|c| c.balance).unwrap_or_default(),
        token_profile: None,
        fetched_at,
        error: String::new(),
    }
}

fn window_from(w: Option<&WindowJson>, fetched_at: i64) -> UsageWindow {
    let Some(w) = w else {
        return UsageWindow::default();
    };
    let reset_at = w.reset_at.or_else(|| {
        w.reset_after_seconds
            .filter(|s| *s >= 0)
            .map(|s| fetched_at + s)
    });
    UsageWindow {
        used_percent: w.used_percent.unwrap_or(0.0),
        reset_at,
        limit_window_seconds: w.limit_window_seconds,
    }
}

pub fn window_has_data(w: &UsageWindow) -> bool {
    w.reset_at.is_some() || w.limit_window_seconds.is_some() || w.used_percent > 0.0
}

pub fn window_is_expired(w: &UsageWindow, now: i64) -> bool {
    window_has_data(w) && w.reset_at.is_some_and(|reset_at| reset_at <= now)
}

pub fn snapshot_has_expired_window(snap: &UsageSnapshot, now: i64) -> bool {
    window_is_expired(&snap.primary, now) || window_is_expired(&snap.secondary, now)
}

fn is_weekly_window(w: &UsageWindow) -> bool {
    matches!(w.limit_window_seconds, Some(secs) if secs >= WEEKLY_WINDOW_SECONDS)
}

/// Map API primary/secondary into short + weekly by window length.
/// Some plans (e.g. current Pro) only return a weekly window in `primary_window`.
fn classify_windows(a: UsageWindow, b: UsageWindow) -> (UsageWindow, UsageWindow) {
    let mut short = UsageWindow::default();
    let mut weekly = UsageWindow::default();

    for w in [a, b] {
        if !window_has_data(&w) {
            continue;
        }
        if is_weekly_window(&w) {
            // Prefer the higher-usage weekly reading if both look weekly.
            if !window_has_data(&weekly) || w.used_percent >= weekly.used_percent {
                weekly = w;
            }
        } else {
            if !window_has_data(&short) || w.used_percent >= short.used_percent {
                short = w;
            }
        }
    }

    // If API omitted limit_window_seconds but only one window exists, treat as weekly
    // when reset looks far away (>2d), else short.
    if !window_has_data(&short) && !window_has_data(&weekly) {
        return (UsageWindow::default(), UsageWindow::default());
    }
    if !window_has_data(&weekly) && !window_has_data(&short) {
        // unreachable
    }
    if window_has_data(&short) && !window_has_data(&weekly) && short.limit_window_seconds.is_none()
    {
        if let Some(reset_at) = short.reset_at {
            if reset_at - now_ts() >= WEEKLY_WINDOW_SECONDS {
                return (UsageWindow::default(), short);
            }
        }
    }

    (short, weekly)
}

pub fn format_usage_short(
    snap: &UsageSnapshot,
    unavailable_label: &str,
    no_window_label: &str,
) -> String {
    if !snap.error.is_empty() {
        return format!("{} · {unavailable_label}", snap.alias);
    }
    let plan = if snap.plan_type.is_empty() {
        "?"
    } else {
        snap.plan_type.as_str()
    };
    let mut parts = vec![plan.to_string()];
    if window_has_data(&snap.primary) {
        parts.push(format!("5h {:.0}%", snap.primary.used_percent));
    }
    if window_has_data(&snap.secondary) {
        parts.push(format!("wk {:.0}%", snap.secondary.used_percent));
    }
    if parts.len() == 1 {
        parts.push(no_window_label.into());
    }
    parts.join(" · ")
}

pub fn format_reset(ts: Option<i64>) -> String {
    let Some(ts) = ts else {
        return "—".into();
    };
    match chrono::DateTime::from_timestamp(ts, 0) {
        Some(dt) => dt
            .with_timezone(&chrono::Local)
            .format("%m-%d %H:%M")
            .to_string(),
        None => "—".into(),
    }
}

pub fn format_window_reset(
    w: &UsageWindow,
    no_window_label: &str,
    reset_prefix: &str,
    reset_unknown_label: &str,
) -> String {
    if !window_has_data(w) {
        return no_window_label.into();
    }
    match w.reset_at {
        Some(ts) => format!("{reset_prefix} {}", format_reset(Some(ts))),
        None => reset_unknown_label.into(),
    }
}

/// Fetch account token activity from the ChatGPT profile endpoint.
/// This is the same data shown in Codex CLI `/usage` and the web dashboard.
pub fn fetch_token_usage_profile(
    _alias: &str,
    access_token: &str,
    account_id: &str,
) -> AppResult<TokenUsageProfile> {
    if access_token.is_empty() || account_id.is_empty() {
        return Ok(TokenUsageProfile::default());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("codex-account-switch/0.1")
        .build()?;

    let url = "https://chatgpt.com/backend-api/wham/profiles/me";
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("ChatGPT-Account-Id", account_id)
        .send()?;

    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        return Err(AppError::msg(friendly_http_error(status.as_u16(), &body)));
    }

    let parsed: TokenUsageProfileResponse = serde_json::from_str(&body)?;
    Ok(parsed.stats)
}

#[derive(Debug, Deserialize)]
struct TokenUsageProfileResponse {
    stats: TokenUsageProfile,
}

/// Compact reset label for dense list rows.
pub fn format_window_reset_short(w: &UsageWindow) -> String {
    if !window_has_data(w) {
        return "—".into();
    }
    match w.reset_at {
        Some(ts) => format_reset(Some(ts)),
        None => "—".into(),
    }
}

pub fn format_usage_row(
    snap: &UsageSnapshot,
    now: i64,
    pending_label: &str,
    no_window_label: &str,
) -> (String, bool) {
    if !snap.error.is_empty() {
        return (String::new(), true);
    }
    if snapshot_has_expired_window(snap, now) {
        return (pending_label.into(), true);
    }

    let mut parts = Vec::new();
    for (label, window) in [("5h", &snap.primary), ("wk", &snap.secondary)] {
        if !window_has_data(window) {
            continue;
        }
        let mut part = format!("{label} {:.0}%", window.used_percent);
        if let Some(reset_at) = window.reset_at {
            part.push_str(&format!(" · ↻ {}", format_reset(Some(reset_at))));
        }
        parts.push(part);
    }

    if parts.is_empty() {
        return (no_window_label.into(), false);
    }
    let attention = snap.primary.used_percent >= 90.0 || snap.secondary.used_percent >= 90.0;
    (parts.join("   "), attention)
}

pub fn recent_daily_tokens(profile: &TokenUsageProfile, days: usize) -> Vec<i64> {
    recent_daily_tokens_ending_on(profile, chrono::Local::now().date_naive(), days)
}

/// Normalize recent daily activity for compact charts. A logarithmic scale keeps
/// quieter days visible when a single large run would otherwise flatten them.
pub fn normalized_daily_tokens(profile: &TokenUsageProfile, days: usize) -> Vec<f32> {
    normalize_values(&recent_daily_tokens(profile, days))
}

/// Return the most recent API buckets, left-padded to a stable chart width.
pub fn normalized_usage_buckets(
    buckets: Option<&[TokenUsageDailyBucket]>,
    points: usize,
) -> Vec<f32> {
    if points == 0 {
        return Vec::new();
    }
    let mut ordered = buckets.unwrap_or_default().to_vec();
    ordered.sort_by(|a, b| a.start_date.cmp(&b.start_date));
    let values = ordered
        .into_iter()
        .rev()
        .take(points)
        .map(|bucket| bucket.tokens.max(0))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let mut padded = vec![0; points.saturating_sub(values.len())];
    padded.extend(values);
    normalize_values(&padded)
}

/// Normalize cumulative buckets relative to the first visible point so the
/// chart communicates growth instead of rendering a nearly flat full-height wall.
pub fn normalized_cumulative_buckets(
    buckets: Option<&[TokenUsageDailyBucket]>,
    points: usize,
) -> Vec<f32> {
    if points == 0 {
        return Vec::new();
    }
    let mut ordered = buckets.unwrap_or_default().to_vec();
    ordered.sort_by(|a, b| a.start_date.cmp(&b.start_date));
    let values = ordered
        .into_iter()
        .rev()
        .take(points)
        .map(|bucket| bucket.tokens.max(0))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let baseline = values.first().copied().unwrap_or(0);
    let growth = values
        .into_iter()
        .map(|value| value.saturating_sub(baseline))
        .collect::<Vec<_>>();
    let mut padded = vec![0.0; points.saturating_sub(growth.len())];
    padded.extend(normalize_linear_values(&growth));
    padded
}

fn normalize_values(values: &[i64]) -> Vec<f32> {
    let max = values.iter().copied().max().unwrap_or(0).max(0) as f64;
    if max == 0.0 {
        return vec![0.0; values.len()];
    }
    let denominator = max.ln_1p();
    values
        .iter()
        .map(|value| ((*value).max(0) as f64).ln_1p() / denominator)
        .map(|value| value as f32)
        .collect()
}

fn normalize_linear_values(values: &[i64]) -> Vec<f32> {
    let max = values.iter().copied().max().unwrap_or(0).max(0) as f32;
    if max == 0.0 {
        return vec![0.0; values.len()];
    }
    values
        .iter()
        .map(|value| (*value).max(0) as f32 / max)
        .collect()
}

fn recent_daily_tokens_ending_on(
    profile: &TokenUsageProfile,
    end_date: chrono::NaiveDate,
    days: usize,
) -> Vec<i64> {
    let mut by_date = HashMap::new();
    for bucket in profile.daily_usage_buckets.as_deref().unwrap_or_default() {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&bucket.start_date, "%Y-%m-%d") {
            *by_date.entry(date).or_insert(0_i64) += bucket.tokens;
        }
    }

    (0..days)
        .rev()
        .map(|offset| {
            let date = end_date - chrono::Duration::days(offset as i64);
            by_date.get(&date).copied().unwrap_or(0)
        })
        .collect()
}

fn read_cache_file(path: &Path) -> AppResult<UsageCacheFile> {
    let data = fs::read(path)?;
    Ok(serde_json::from_slice(&data)?)
}

fn atomic_write(path: &Path, data: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

#[cfg(test)]
pub fn parse_usage_payload_for_test(body: &str, alias: &str) -> AppResult<UsageSnapshot> {
    let parsed: UsageResponse = serde_json::from_str(body)?;
    Ok(snapshot_from_response(alias, parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wham_usage_payload() {
        let body = r#"{
          "plan_type": "pro",
          "rate_limit": {
            "primary_window": {"used_percent": 42.5, "limit_window_seconds": 18000, "reset_at": 1780000000},
            "secondary_window": {"used_percent": 18.0, "limit_window_seconds": 604800, "reset_at": 1780500000}
          },
          "credits": {"balance": "5.00"}
        }"#;
        let snap = parse_usage_payload_for_test(body, "work").unwrap();
        assert_eq!(snap.plan_type, "pro");
        assert!((snap.primary.used_percent - 42.5).abs() < f64::EPSILON);
        assert!((snap.secondary.used_percent - 18.0).abs() < f64::EPSILON);
        assert_eq!(snap.secondary.reset_at, Some(1780500000));
        assert_eq!(snap.credits_balance, "5.00");
    }

    #[test]
    fn maps_weekly_only_primary_window_to_weekly_bucket() {
        let body = r#"{
          "plan_type": "pro",
          "rate_limit": {
            "primary_window": {
              "used_percent": 100,
              "limit_window_seconds": 604800,
              "reset_after_seconds": 355280,
              "reset_at": 1786201471
            },
            "secondary_window": null
          }
        }"#;
        let snap = parse_usage_payload_for_test(body, "main").unwrap();
        assert!(
            !window_has_data(&snap.primary),
            "short window should be empty"
        );
        assert!((snap.secondary.used_percent - 100.0).abs() < f64::EPSILON);
        assert_eq!(snap.secondary.reset_at, Some(1786201471));
    }

    #[test]
    fn usage_row_omits_missing_short_window_and_shows_weekly_reset() {
        let snap = UsageSnapshot {
            secondary: UsageWindow {
                used_percent: 42.0,
                reset_at: Some(1_780_501_000),
                limit_window_seconds: Some(604_800),
            },
            ..UsageSnapshot::default()
        };

        let (label, attention) = format_usage_row(&snap, 1_780_000_000, "pending", "none");
        assert!(!label.contains("5h"));
        assert!(label.contains("wk 42%"));
        assert!(label.contains('↻'));
        assert!(!attention);
    }

    #[test]
    fn expired_window_is_pending_instead_of_showing_old_percentage() {
        let snap = UsageSnapshot {
            secondary: UsageWindow {
                used_percent: 100.0,
                reset_at: Some(100),
                limit_window_seconds: Some(604_800),
            },
            ..UsageSnapshot::default()
        };

        assert_eq!(
            format_usage_row(&snap, 101, "pending", "none"),
            ("pending".into(), true)
        );
    }

    #[test]
    fn recent_daily_tokens_fills_missing_dates_with_zero() {
        let profile = TokenUsageProfile {
            daily_usage_buckets: Some(vec![
                TokenUsageDailyBucket {
                    start_date: "2026-08-20".into(),
                    tokens: 10,
                },
                TokenUsageDailyBucket {
                    start_date: "2026-08-22".into(),
                    tokens: 30,
                },
            ]),
            ..TokenUsageProfile::default()
        };
        let end = chrono::NaiveDate::from_ymd_opt(2026, 8, 23).unwrap();

        assert_eq!(
            recent_daily_tokens_ending_on(&profile, end, 4),
            vec![10, 0, 30, 0]
        );
    }

    #[test]
    fn parses_complete_activity_profile() {
        let body = r#"{
          "stats": {
            "lifetime_tokens": 49350000000,
            "peak_daily_tokens": 2450000000,
            "current_streak_days": 0,
            "longest_streak_days": 100,
            "fast_mode_usage_percentage": 11,
            "most_used_reasoning_effort": "xhigh",
            "most_used_reasoning_effort_percentage": 58,
            "unique_skills_used": 86,
            "total_skills_used": 13082,
            "total_threads": 3413,
            "top_invocations": [{
              "plugin_name": "superpowers-zh",
              "skill_name": "brainstorming",
              "type": "skill",
              "usage_count": 9238
            }],
            "weekly_usage_buckets": [{"start_date":"2026-08-17","tokens":20}],
            "cumulative_daily_usage_buckets": [{"start_date":"2026-08-23","tokens":30}]
          }
        }"#;
        let parsed: TokenUsageProfileResponse = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.stats.longest_streak_days, Some(100));
        assert_eq!(parsed.stats.unique_skills_used, Some(86));
        let invocation = &parsed.stats.top_invocations.unwrap()[0];
        assert_eq!(invocation.usage_count, 9238);
        assert_eq!(invocation.skill_id, None);
    }

    #[test]
    fn chart_normalization_is_padded_and_log_scaled() {
        let buckets = vec![
            TokenUsageDailyBucket {
                start_date: "2026-08-01".into(),
                tokens: 9,
            },
            TokenUsageDailyBucket {
                start_date: "2026-08-08".into(),
                tokens: 99,
            },
        ];
        let values = normalized_usage_buckets(Some(&buckets), 4);
        assert_eq!(values.len(), 4);
        assert_eq!(&values[..2], &[0.0, 0.0]);
        assert!(values[2] > 0.0 && values[2] < 1.0);
        assert_eq!(values[3], 1.0);
    }

    #[test]
    fn cumulative_chart_shows_growth_from_visible_baseline() {
        let buckets = vec![
            TokenUsageDailyBucket {
                start_date: "2026-08-01".into(),
                tokens: 100,
            },
            TokenUsageDailyBucket {
                start_date: "2026-08-02".into(),
                tokens: 150,
            },
            TokenUsageDailyBucket {
                start_date: "2026-08-03".into(),
                tokens: 300,
            },
        ];
        let values = normalized_cumulative_buckets(Some(&buckets), 3);
        assert_eq!(values[0], 0.0);
        assert!((values[1] - 0.25).abs() < f32::EPSILON);
        assert_eq!(values[2], 1.0);
    }
}
