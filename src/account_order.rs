//! 只根据成功的额度快照建议使用次序，不切换账号、不消费重置次数。
use crate::i18n::{t, Language};
use crate::usage::{format_reset, window_has_data, window_is_expired, UsageSnapshot};

// 完整账号列表每 10 分钟刷新；额外留出网络请求时间。
const MAX_AGE: i64 = 15 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Ready,
    Waiting,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone)]
pub struct Advice {
    pub status: Availability,
    pub at: Option<i64>,
    pub remaining: f64,
    pub reason: &'static str,
}

pub fn evaluate(valid: bool, snapshot: Option<&UsageSnapshot>, now: i64) -> Advice {
    let unknown = |reason| Advice {
        status: Availability::Unknown,
        at: None,
        remaining: 0.,
        reason,
    };
    if !valid {
        return Advice {
            status: Availability::Invalid,
            ..unknown("order.invalid")
        };
    }
    let Some(s) = snapshot else {
        return unknown("order.refresh");
    };
    if !s.error.is_empty()
        || s.fetched_at <= 0
        || s.fetched_at > now
        || now - s.fetched_at > MAX_AGE
    {
        return unknown("order.refresh");
    }
    let windows: Vec<_> = [&s.primary, &s.secondary]
        .into_iter()
        .filter(|w| window_has_data(w))
        .collect();
    if windows.is_empty()
        || windows
            .iter()
            .any(|w| !w.used_percent.is_finite() || w.used_percent < 0.)
    {
        return unknown("order.refresh");
    }
    if windows.iter().any(|w| window_is_expired(w, now)) {
        return unknown("order.elapsed");
    }
    let blocked: Vec<_> = windows.iter().filter(|w| w.used_percent >= 100.).collect();
    if !blocked.is_empty() {
        let at = if blocked.iter().all(|w| w.reset_at.is_some()) {
            blocked.iter().filter_map(|w| w.reset_at).max()
        } else {
            None
        };
        return Advice {
            status: Availability::Waiting,
            at,
            remaining: 0.,
            reason: "order.waiting",
        };
    }
    // 多窗口账号优先消化较早到期的周额度；不能用 5 小时的近重置掩盖周额度顺序。
    let at = if window_has_data(&s.secondary) {
        s.secondary.reset_at
    } else {
        s.primary.reset_at
    };
    Advice {
        status: Availability::Ready,
        at,
        remaining: windows
            .iter()
            .map(|w| 100. - w.used_percent)
            .fold(100., f64::min),
        reason: "order.ready",
    }
}

pub fn ranked(items: &[(String, Advice)], status: Availability) -> Vec<&(String, Advice)> {
    let mut rows: Vec<_> = items.iter().filter(|(_, a)| a.status == status).collect();
    rows.sort_by(|a, b| {
        a.1.at
            .unwrap_or(i64::MAX)
            .cmp(&b.1.at.unwrap_or(i64::MAX))
            .then_with(|| b.1.remaining.total_cmp(&a.1.remaining))
            .then_with(|| a.0.cmp(&b.0))
    });
    rows
}

pub fn reset_summary(snapshot: Option<&UsageSnapshot>, now: i64, lang: Language) -> String {
    let Some(s) = snapshot else {
        return t(lang, "order.reset_unknown").into();
    };
    let mut parts = Vec::new();
    for (key, w) in [("order.weekly", &s.secondary), ("order.short", &s.primary)] {
        if !window_has_data(w) {
            continue;
        }
        let time = if window_is_expired(w, now) {
            t(lang, "order.elapsed").into()
        } else if w.reset_at.is_some() {
            format_reset(w.reset_at)
        } else {
            t(lang, "order.unknown_time").into()
        };
        parts.push(format!("{} {time}", t(lang, key)));
    }
    if parts.is_empty() {
        return t(lang, "order.reset_unknown").into();
    }
    let stale = !s.error.is_empty()
        || s.fetched_at <= 0
        || s.fetched_at > now
        || now - s.fetched_at > MAX_AGE;
    format!(
        "{}{}：{}",
        if stale {
            format!("{} · ", t(lang, "new.cached"))
        } else {
            String::new()
        },
        t(lang, "order.next_reset"),
        parts.join(" · ")
    )
}

pub fn row_label(alias: &str, items: &[(String, Advice)], lang: Language) -> String {
    let Some((_, a)) = items.iter().find(|(name, _)| name == alias) else {
        return String::new();
    };
    match a.status {
        Availability::Ready | Availability::Waiting => {
            let rank = ranked(items, a.status)
                .iter()
                .position(|(name, _)| name == alias)
                .unwrap()
                + 1;
            let label = t(
                lang,
                if a.status == Availability::Ready {
                    "order.use_rank"
                } else {
                    "order.wait_rank"
                },
            );
            let detail = if a.status == Availability::Waiting {
                a.at.map(|at| {
                    format!(
                        "{} {}",
                        t(lang, "order.after_reset"),
                        format_reset(Some(at))
                    )
                })
                .unwrap_or_else(|| t(lang, "order.unknown_time").into())
            } else if a.at.is_none() {
                t(lang, "order.unknown_time").into()
            } else {
                t(lang, a.reason).into()
            };
            format!("{label} #{rank} · {detail}")
        }
        _ => t(lang, a.reason).into(),
    }
}

pub fn summary(items: &[(String, Advice)], lang: Language) -> String {
    let mut lines = Vec::new();
    for (status, key) in [
        (Availability::Ready, "order.use_sequence"),
        (Availability::Waiting, "order.wait_sequence"),
    ] {
        let list = ranked(items, status);
        if !list.is_empty() {
            lines.push(format!(
                "{}：{}",
                t(lang, key),
                list.iter()
                    .map(|(alias, _)| alias.as_str())
                    .collect::<Vec<_>>()
                    .join(" → ")
            ));
        }
    }
    if !items.iter().any(|(_, a)| a.status == Availability::Ready) {
        lines.insert(0, t(lang, "order.none_ready").into());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::UsageWindow;
    fn snapshot(used: f64, reset: Option<i64>) -> UsageSnapshot {
        UsageSnapshot {
            fetched_at: 1000,
            secondary: UsageWindow {
                used_percent: used,
                reset_at: reset,
                limit_window_seconds: Some(604800),
            },
            ..Default::default()
        }
    }
    #[test]
    fn available_accounts_use_weekly_deadline_then_remaining_and_stable_name() {
        let mut early = snapshot(80., Some(2000));
        early.primary = UsageWindow {
            used_percent: 20.,
            reset_at: Some(1800),
            limit_window_seconds: Some(18000),
        };
        let mut late = snapshot(10., Some(3000));
        late.primary.reset_at = Some(1100);
        let items = vec![
            ("late".into(), evaluate(true, Some(&late), 1000)),
            ("early".into(), evaluate(true, Some(&early), 1000)),
            (
                "more".into(),
                evaluate(true, Some(&snapshot(10., Some(2000))), 1000),
            ),
            (
                "empty".into(),
                evaluate(true, Some(&snapshot(100., Some(1500))), 1000),
            ),
        ];
        assert_eq!(
            ranked(&items, Availability::Ready)
                .iter()
                .map(|r| r.0.as_str())
                .collect::<Vec<_>>(),
            vec!["more", "early", "late"]
        );
        assert_eq!(ranked(&items, Availability::Waiting)[0].0, "empty");
    }
    #[test]
    fn depleted_windows_wait_until_every_blocking_window_resets() {
        let mut s = snapshot(100., Some(3000));
        s.primary = UsageWindow {
            used_percent: 100.,
            reset_at: Some(2000),
            limit_window_seconds: Some(18000),
        };
        assert_eq!(evaluate(true, Some(&s), 1000).at, Some(3000));
        s.secondary.used_percent = 40.;
        assert_eq!(evaluate(true, Some(&s), 1000).at, Some(2000));
        s.primary.reset_at = None;
        assert_eq!(evaluate(true, Some(&s), 1000).at, None);
    }
    #[test]
    fn stale_failed_invalid_and_elapsed_snapshots_are_not_recommended() {
        let mut s = snapshot(20., Some(4000));
        assert_eq!(
            evaluate(false, Some(&s), 1000).status,
            Availability::Invalid
        );
        assert_eq!(evaluate(true, None, 1000).status, Availability::Unknown);
        assert_eq!(evaluate(true, Some(&s), 1901).status, Availability::Unknown);
        s.error = "network".into();
        assert_eq!(evaluate(true, Some(&s), 1000).status, Availability::Unknown);
        s.error.clear();
        s.secondary.reset_at = Some(1000);
        assert_eq!(evaluate(true, Some(&s), 1000).reason, "order.elapsed");
    }
    #[test]
    fn unknown_times_sort_last_and_short_only_accounts_work() {
        let mut s = snapshot(0., None);
        s.primary = s.secondary.clone();
        s.primary.reset_at = Some(1800);
        s.secondary = UsageWindow::default();
        assert_eq!(evaluate(true, Some(&s), 1000).at, Some(1800));
        let items = vec![
            (
                "unknown".into(),
                evaluate(true, Some(&snapshot(10., None)), 1000),
            ),
            ("known".into(), evaluate(true, Some(&s), 1000)),
        ];
        assert_eq!(ranked(&items, Availability::Ready)[0].0, "known");
        let text = reset_summary(Some(&s), 1000, Language::ZhCn);
        assert!(text.contains("5 小时"));
        assert!(!text.contains("每周"));
        s.primary.reset_at = Some(999);
        assert!(reset_summary(Some(&s), 1000, Language::ZhCn).contains("待刷新"));
    }
}
