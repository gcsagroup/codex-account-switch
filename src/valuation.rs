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
}

impl Assumptions {
    pub fn validate(&self) -> bool {
        !self.model.trim().is_empty()
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
        let input = self.input_percent / 100.0;
        let cached = self.cached_input_percent / 100.0;
        let blended = input
            * ((1.0 - cached) * self.input_price + cached * self.cached_input_price)
            + (1.0 - input) * self.output_price;
        Some(tokens as f64 / 1_000_000.0 * blended)
    }
}

// 来源：https://developers.openai.com/api/docs/pricing
// 2026-09-13 核验，Standard / Short context；不是实时同步价格。
// 不含缓存写入、长上下文、Fast mode 和地域附加费用。
pub const OFFICIAL_PRICES: &[(&str, f64, f64, f64)] = &[
    ("gpt-5.6-sol", 4.0, 0.4, 20.0),
    ("gpt-6-astra", 10.0, 1.0, 50.0),
    ("gpt-5.6-terra", 2.0, 0.2, 12.0),
    ("gpt-5.6-luna", 0.2, 0.02, 1.2),
    ("gpt-5.3-codex", 1.75, 0.175, 14.0),
];

pub fn official_assumption(index: usize) -> Option<Assumptions> {
    let &(model, input_price, cached_input_price, output_price) = OFFICIAL_PRICES.get(index)?;
    Some(Assumptions {
        model: model.into(),
        input_percent: 80.0,
        cached_input_percent: 50.0,
        input_price,
        cached_input_price,
        output_price,
        price_date: NaiveDate::from_ymd_opt(2026, 9, 13).unwrap(),
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
