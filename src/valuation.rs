//! 官方价格快照与 Token 总量情景估值；缺失日期保留为未知。
use std::collections::BTreeMap;
use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::usage::TokenUsageProfile;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Assumptions {
    pub model: String,
    pub input_percent: f64,
    pub cached_input_percent: f64,
    pub input_price: f64,
    pub cached_input_price: f64,
    pub output_price: f64,
    pub price_date: NaiveDate,
    #[serde(default)]
    pub speed: i32,
    #[serde(default)]
    pub long_context: bool,
}

impl Assumptions {
    pub fn validate(&self) -> bool {
        (0..=2).contains(&self.speed)
            && !self.model.trim().is_empty()
            && self.model.chars().count() <= 80
            && [self.input_percent, self.cached_input_percent]
                .iter()
                .all(|v| v.is_finite() && (0.0..=100.0).contains(v))
            && [self.input_price, self.cached_input_price, self.output_price]
                .iter()
                .all(|v| v.is_finite() && (0.0..=1_000_000.0).contains(v))
    }

    // 缓存占比是输入的一部分，不与普通输入重复计费。推理 Token 已包含在输出假设中。
    pub fn usd(&self, tokens: u64) -> Option<f64> {
        if !self.validate() {
            return None;
        }
        let rates = if OFFICIAL_PRICES.iter().any(|p| p.0 == self.model) {
            scenario_prices(&self.model, self.speed, self.long_context)?
        } else if self.speed == 0 && !self.long_context {
            (self.input_price, self.cached_input_price, self.output_price)
        } else {
            return None;
        };
        let input = self.input_percent / 100.0;
        let cached = self.cached_input_percent / 100.0;
        let blended =
            input * ((1.0 - cached) * rates.0 + cached * rates.1) + (1.0 - input) * rates.2;
        Some(tokens as f64 / 1_000_000.0 * blended)
    }
}

// 来源：https://developers.openai.com/api/docs/pricing
// 2026-11-14 核验，Standard / Short context；旧 gpt-5.3-codex 保留历史快照。
// 不含缓存写入、长上下文、Fast mode 和地域附加费用。
pub const OFFICIAL_PRICES: &[(&str, f64, f64, f64)] = &[
    ("gpt-5.6-sol", 4.0, 0.4, 20.0),
    ("gpt-6-astra", 10.0, 1.0, 50.0),
    ("gpt-5.6-terra", 2.0, 0.2, 12.0),
    ("gpt-5.6-luna", 0.2, 0.02, 1.2),
    ("gpt-5.3-codex", 1.75, 0.175, 14.0),
    // https://developers.openai.com/api/docs/models/gpt-5.5
    ("gpt-5.5", 5.0, 0.5, 30.0),
    // 以下三项于 2026-10-10 核验各模型官方页面，仍为标准短上下文价格。
    ("gpt-6.1-sol", 2.0, 0.1, 10.0),
    ("gpt-6-sol", 2.0, 0.2, 10.0),
    ("gpt-6-luna", 0.1, 0.01, 0.5),
    ("gpt-6.1-terra", 1.0, 0.05, 5.0),
];

pub fn price_date(model: &str) -> NaiveDate {
    if model == "gpt-5.3-codex" {
        NaiveDate::from_ymd_opt(2026, 9, 16).unwrap()
    } else {
        NaiveDate::from_ymd_opt(2026, 11, 14).unwrap()
    }
}

// 仅对官方已核验的组合折算。模型能力、套餐权限与 API 价格是独立事项。
pub fn scenario_prices(model: &str, speed: i32, long: bool) -> Option<(f64, f64, f64)> {
    let &(_, input, cached, output) = OFFICIAL_PRICES.iter().find(|p| p.0 == model)?;
    if model == "gpt-5.3-codex" {
        return (speed == 0 && !long).then_some((input, cached, output));
    }
    let multiplier = match speed {
        0 => 1.,
        1 if model == "gpt-5.5" && !long => 2.5,
        1 if model != "gpt-5.5" => 2.,
        2 if matches!(model, "gpt-6-astra" | "gpt-6.1-sol") => 6.,
        _ => return None,
    };
    Some((
        input * multiplier * if long { 2. } else { 1. },
        cached * multiplier * if long { 2. } else { 1. },
        output * multiplier * if long { 1.5 } else { 1. },
    ))
}
pub fn scenario_label(a: &Assumptions) -> String {
    format!(
        "{} / {} · 手动折算基准，并非已检测到的实际速度或上下文",
        match a.speed {
            1 => "Fast",
            2 => "Ultrafast",
            _ => "Standard",
        },
        if a.long_context {
            "长上下文 >272K"
        } else {
            "短上下文 ≤272K"
        }
    )
}
pub fn price_reference(a: &Assumptions) -> String {
    let Some((i, c, o)) = scenario_prices(&a.model, a.speed, a.long_context) else {
        return "当前模型与速度／上下文组合未提供已核验价格，列为未计价".into();
    };
    format!(
        "{} · 每百万 Token：输入 ${i} / 缓存读取 ${c} / 输出 ${o}\n{} · 核验 {}\n{}",
        a.model,
        scenario_label(a),
        price_date(&a.model),
        if matches!(a.model.as_str(), "gpt-5.3-codex" | "gpt-5.5") {
            "该快照未提供缓存写入价；未计入工具费、区域加价与历史价格变化".into()
        } else {
            format!("缓存写入参考：30 分钟 ${} / 12 小时 ${}；缺少写入分类 Token，当前折算未计入写入费用。未含工具费与区域加价。",i*1.25,i*2.5)
        }
    )
}

pub fn official_assumption(index: usize) -> Option<Assumptions> {
    let &(model, input_price, cached_input_price, output_price) = OFFICIAL_PRICES.get(index)?;
    Some(Assumptions {
        model: model.into(),
        input_percent: 80.0,
        cached_input_percent: 50.0,
        input_price,
        cached_input_price,
        output_price,
        price_date: price_date(model),
        speed: 0,
        long_context: false,
    })
}

// 旧配置只保留比例；公开价格统一从官方快照读取，避免残留自定义单价。
pub fn effective_assumption(saved: Option<&Assumptions>) -> Assumptions {
    let index = saved
        .and_then(|a| OFFICIAL_PRICES.iter().position(|p| p.0 == a.model))
        .unwrap_or(0);
    let mut a = official_assumption(index).unwrap();
    if let Some(saved) = saved {
        a.input_percent = saved.input_percent;
        a.cached_input_percent = saved.cached_input_percent;
        a.speed = saved.speed;
        a.long_context = saved.long_context;
    }
    a
}

pub type Settings = BTreeMap<String, Assumptions>;

pub fn read_settings(path: &Path) -> AppResult<Settings> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::new()),
        Err(e) => return Err(e.into()),
    };
    let settings: Settings = serde_json::from_slice(&bytes)?;
    if settings.values().any(|a| !a.validate()) {
        return Err(AppError::msg("估值设置含无效数值"));
    }
    Ok(settings)
}

pub fn save_setting(path: &Path, account: &str, assumption: Assumptions) -> AppResult<()> {
    if account.is_empty() || !assumption.validate() {
        return Err(AppError::msg("估值设置无效"));
    }
    // 写入前重读，保留其他账号配置；损坏的文件不静默覆盖。
    let mut settings = read_settings(path)?;
    settings.insert(account.into(), assumption);
    crate::profile::atomic_write(path, &serde_json::to_vec_pretty(&settings)?)
}

pub fn settings_path() -> std::path::PathBuf {
    crate::paths::app_home().join("valuation.json")
}

#[derive(Debug, PartialEq)]
pub struct TokenSeries {
    pub daily: Vec<Option<u64>>,
    pub total: Option<u64>,
    pub covered: usize,
}

pub fn token_series(profile: &TokenUsageProfile, dates: &[NaiveDate]) -> TokenSeries {
    let mut returned = BTreeMap::new();
    for bucket in profile.daily_usage_buckets.iter().flatten() {
        let Ok(date) = NaiveDate::parse_from_str(&bucket.start_date, "%Y-%m-%d") else {
            continue;
        };
        let value = u64::try_from(bucket.tokens).ok();
        returned
            .entry(date)
            .and_modify(|old| {
                // 同一天重复且冲突时，不能猜测哪条正确，更不能重复累计。
                if *old != value {
                    *old = None;
                }
            })
            .or_insert(value);
    }
    let daily: Vec<_> = dates
        .iter()
        .map(|d| returned.get(d).copied().flatten())
        .collect();
    let covered = daily.iter().filter(|d| d.is_some()).count();
    let total = if covered == 0 {
        None
    } else {
        daily
            .iter()
            .flatten()
            .try_fold(0u64, |sum, value| sum.checked_add(*value))
    };
    TokenSeries {
        daily,
        total,
        covered,
    }
}

pub fn money(value: f64) -> String {
    if value > 0.0 && value < 0.01 {
        "< $0.01".into()
    } else {
        format!("${value:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::TokenUsageDailyBucket;
    fn assumption() -> Assumptions {
        Assumptions {
            model: "自定义参考模型".into(),
            input_percent: 80.,
            cached_input_percent: 50.,
            input_price: 2.,
            cached_input_price: 0.2,
            output_price: 10.,
            price_date: NaiveDate::from_ymd_opt(2026, 9, 13).unwrap(),
            speed: 0,
            long_context: false,
        }
    }
    #[test]
    fn official_prices_default_and_legacy_settings() {
        let a = effective_assumption(None);
        assert_eq!(a.model, "gpt-5.6-sol");
        assert!((a.usd(1_000_000).unwrap() - 5.76).abs() < 1e-9);
        let mut legacy = assumption();
        legacy.model = "gpt-6-astra".into();
        legacy.input_percent = 60.;
        legacy.input_price = 999.;
        let migrated = effective_assumption(Some(&legacy));
        assert_eq!(migrated.input_price, 10.);
        assert_eq!(migrated.input_percent, 60.);
        assert!(official_assumption(usize::MAX).is_none());
        for i in 0..OFFICIAL_PRICES.len() {
            assert!(official_assumption(i).unwrap().validate());
        }
    }
    #[test]
    fn verified_scenarios_do_not_invent_unsupported_prices() {
        assert_eq!(
            scenario_prices("gpt-6.1-terra", 0, false),
            Some((1., 0.05, 5.))
        );
        assert_eq!(
            scenario_prices("gpt-6.1-sol", 1, true),
            Some((8., 0.4, 30.))
        );
        assert_eq!(
            scenario_prices("gpt-6-astra", 2, false),
            Some((60., 6., 300.))
        );
        assert!(scenario_prices("gpt-6.1-terra", 2, false).is_none());
        assert!(scenario_prices("gpt-5.5", 1, true).is_none());
        assert!(scenario_prices("new-model", 0, false).is_none());
        let mut a = official_assumption(6).unwrap();
        let base = a.usd(1_000_000).unwrap();
        a.speed = 1;
        assert_eq!(a.usd(1_000_000), Some(base * 2.));
        a.long_context = true;
        assert!(a.usd(1_000_000).unwrap() > base * 2.);
    }
    #[test]
    fn representative_estimate_and_extreme_ratios() {
        let mut a = assumption();
        assert!((a.usd(12_000_000).unwrap() - 34.56).abs() < 1e-9);
        assert!((a.usd(48_000_000).unwrap() - 138.24).abs() < 1e-9);
        a.input_percent = 100.;
        a.cached_input_percent = 100.;
        assert_eq!(a.usd(1_000_000), Some(0.2));
        a.input_percent = 0.;
        assert_eq!(a.usd(1_000_000), Some(10.));
        assert_eq!(a.usd(0), Some(0.));
        assert_eq!(money(0.0001), "< $0.01");
    }
    #[test]
    fn reject_invalid_prices_and_percentages() {
        let mut a = assumption();
        for v in [-1., 101., f64::NAN, f64::INFINITY] {
            a.input_percent = v;
            assert_eq!(a.usd(100), None);
        }
        a = assumption();
        a.output_price = -1.;
        assert!(!a.validate());
        a = assumption();
        a.model = " ".into();
        assert!(!a.validate());
    }
    #[test]
    fn missing_zero_duplicates_and_range_are_distinct() {
        let dates = crate::analytics::dates(NaiveDate::from_ymd_opt(2026, 9, 13).unwrap(), 7);
        let mut p = TokenUsageProfile::default();
        assert_eq!(token_series(&p, &dates).total, None);
        p.daily_usage_buckets = Some(vec![
            TokenUsageDailyBucket {
                start_date: "2026-09-07".into(),
                tokens: 0,
            },
            TokenUsageDailyBucket {
                start_date: "2026-09-08".into(),
                tokens: 12_000_000,
            },
            TokenUsageDailyBucket {
                start_date: "2026-09-08".into(),
                tokens: 12_000_000,
            },
            TokenUsageDailyBucket {
                start_date: "2026-09-09".into(),
                tokens: -1,
            },
            TokenUsageDailyBucket {
                start_date: "2026-08-01".into(),
                tokens: 999,
            },
        ]);
        let s = token_series(&p, &dates);
        assert_eq!(s.covered, 2);
        assert_eq!(s.total, Some(12_000_000));
        assert_eq!(s.daily[0], Some(0));
        assert_eq!(s.daily[2], None);
        p.daily_usage_buckets
            .as_mut()
            .unwrap()
            .push(TokenUsageDailyBucket {
                start_date: "2026-09-08".into(),
                tokens: 4,
            });
        assert_eq!(token_series(&p, &dates).total, Some(0));
    }
    #[test]
    fn settings_survive_restart_and_isolate_accounts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("valuation.json");
        save_setting(&path, "a", assumption()).unwrap();
        let mut b = assumption();
        b.output_price = 20.;
        save_setting(&path, "b", b.clone()).unwrap();
        let settings = read_settings(&path).unwrap();
        assert_eq!(settings["a"], assumption());
        assert_eq!(settings["b"], b);
        std::fs::write(&path, b"broken").unwrap();
        assert!(save_setting(&path, "a", assumption()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"broken");
    }
}
