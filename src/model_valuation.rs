//! 自动按模型计价。精确 Token 分类优先，个人套餐仅按同日轮次分摊并明确标为估算。
use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::Deserialize;

use crate::analytics::ActivityResponse;
use crate::valuation::{Assumptions, TokenSeries, OFFICIAL_PRICES};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct DetailedUsage {
    pub data: Vec<DetailedDay>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DetailedDay {
    pub date: NaiveDate,
    #[serde(default, alias = "groups")]
    pub models: Vec<TokenModel>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TokenModel {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub dimensions: BTreeMap<String, String>,
    #[serde(default)]
    pub is_other: bool,
    pub uncached_text_input_tokens: Option<u64>,
    pub cached_text_input_tokens: Option<u64>,
    pub text_output_tokens: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct ModelCost {
    pub model: String,
    pub tokens: f64,
    pub input: f64,
    pub cached: f64,
    pub output: f64,
    pub usd: Option<f64>,
    pub estimated: bool,
}

#[derive(Default, Debug)]
pub struct AutomaticCost {
    pub rows: Vec<ModelCost>,
    pub daily: Vec<Option<f64>>,
    pub exact_days: usize,
    pub estimated_days: usize,
    pub priced_tokens: f64,
    pub unpriced_tokens: f64,
    pub total: Option<f64>,
}

fn prices(name: &str) -> Option<(f64, f64, f64)> {
    // 不猜测内部别名、日期别名或未知模型的价目。
    OFFICIAL_PRICES
        .iter()
        .find(|p| p.0 == name)
        .map(|p| (p.1, p.2, p.3))
}

fn cost(row: &ModelCost) -> Option<f64> {
    let (input, cached, output) = prices(&row.model)?;
    Some((row.input * input + row.cached * cached + row.output * output) / 1_000_000.)
}

// 分类端点可独立返回；总量端点滞后或缺失时，至少计入已明确报告的 Token。
pub fn merge_reported_totals(
    series: &mut TokenSeries,
    dates: &[NaiveDate],
    details: Option<&DetailedUsage>,
) {
    for (i, date) in dates.iter().enumerate() {
        let days: Vec<_> = details
            .into_iter()
            .flat_map(|d| &d.data)
            .filter(|d| d.date == *date)
            .collect();
        if days.len() != 1 || days[0].models.is_empty() {
            continue;
        }
        let total = days[0].models.iter().try_fold(0u64, |sum, m| {
            sum.checked_add(m.uncached_text_input_tokens?)?
                .checked_add(m.cached_text_input_tokens?)?
                .checked_add(m.text_output_tokens?)
        });
        if let Some(total) = total {
            series.daily[i] = Some(series.daily[i].unwrap_or(0).max(total));
        }
    }
    series.covered = series.daily.iter().flatten().count();
    series.total = if series.covered == 0 {
        None
    } else {
        series
            .daily
            .iter()
            .flatten()
            .try_fold(0u64, |n, v| n.checked_add(*v))
    };
}

pub fn calculate(
    dates: &[NaiveDate],
    tokens: &TokenSeries,
    details: Option<&DetailedUsage>,
    activity: Option<&ActivityResponse>,
    assumptions: &Assumptions,
) -> AutomaticCost {
    let mut result = AutomaticCost::default();
    let mut combined: BTreeMap<String, ModelCost> = BTreeMap::new();
    for (index, date) in dates.iter().enumerate() {
        let exact: Vec<_> = details
            .into_iter()
            .flat_map(|d| &d.data)
            .filter(|d| d.date == *date)
            .collect();
        let mut rows = Vec::new();
        // 重复日期不合并，以免同一份数据计费两次。
        if exact.len() == 1 && !exact[0].models.is_empty() {
            let parsed: Option<Vec<ModelCost>> = exact[0]
                .models
                .iter()
                .map(|m| {
                    let input = m.uncached_text_input_tokens?;
                    let cached = m.cached_text_input_tokens?;
                    let output = m.text_output_tokens?;
                    let total = input.checked_add(cached)?.checked_add(output)?;
                    Some(ModelCost {
                        model: if m.is_other {
                            "other".into()
                        } else {
                            m.model
                                .clone()
                                .or_else(|| m.dimensions.get("model").cloned())
                                .unwrap_or_else(|| "unknown".into())
                        },
                        tokens: total as f64,
                        input: input as f64,
                        cached: cached as f64,
                        output: output as f64,
                        ..Default::default()
                    })
                })
                .collect();
            if let Some(parsed) = parsed {
                rows = parsed;
                result.exact_days += 1;
            }
        }
        if rows.is_empty() {
            let total = tokens.daily.get(index).copied().flatten();
            let days: Vec<_> = activity
                .into_iter()
                .flat_map(|a| &a.data)
                .filter(|d| d.date == *date)
                .collect();
            if let Some(total) = total {
                if total == 0 {
                    result.daily.push(Some(0.));
                    continue;
                }
                if days.len() == 1 && days[0].totals.turns > 0 {
                    let day = days[0];
                    let sum = day
                        .models
                        .iter()
                        .try_fold(0u64, |n, m| n.checked_add(m.turns));
                    if let Some(sum) = sum.filter(|s| *s <= day.totals.turns) {
                        for m in &day.models {
                            if m.turns == 0 {
                                continue;
                            }
                            let count = total as f64 * (m.turns as f64 / day.totals.turns as f64);
                            let input = count * assumptions.input_percent / 100.;
                            let cached = input * assumptions.cached_input_percent / 100.;
                            rows.push(ModelCost {
                                model: m.model.clone(),
                                tokens: count,
                                input: input - cached,
                                cached,
                                output: count - input,
                                estimated: true,
                                ..Default::default()
                            });
                        }
                        if sum < day.totals.turns {
                            rows.push(ModelCost {
                                model: "unknown".into(),
                                tokens: total as f64
                                    * ((day.totals.turns - sum) as f64 / day.totals.turns as f64),
                                estimated: true,
                                ..Default::default()
                            });
                        }
                        result.estimated_days += 1;
                    }
                }
                if rows.is_empty() {
                    result.unpriced_tokens += total as f64;
                }
            }
        }
        let mut day_cost = None;
        let row_tokens: f64 = rows.iter().map(|r| r.tokens).sum();
        if let Some(total) = tokens.daily.get(index).copied().flatten() {
            // 精确端点只覆盖部分用途时，不把剩余部分擅自套用文本价。
            if !rows.is_empty() {
                result.unpriced_tokens += (total as f64 - row_tokens).max(0.);
            }
        }
        for mut row in rows {
            row.usd = cost(&row);
            if let Some(usd) = row.usd {
                *day_cost.get_or_insert(0.) += usd;
                result.priced_tokens += row.tokens;
            } else {
                result.unpriced_tokens += row.tokens;
            }
            let entry = combined
                .entry(row.model.clone())
                .or_insert_with(|| ModelCost {
                    model: row.model.clone(),
                    ..Default::default()
                });
            entry.tokens += row.tokens;
            entry.input += row.input;
            entry.cached += row.cached;
            entry.output += row.output;
            entry.estimated |= row.estimated;
            if let Some(usd) = row.usd {
                *entry.usd.get_or_insert(0.) += usd;
            }
        }
        if let Some(usd) = day_cost {
            *result.total.get_or_insert(0.) += usd;
        }
        result.daily.push(day_cost);
    }
    // 所有真实零值仍应显示 0，而非缺失。
    if result.total.is_none() && result.daily.contains(&Some(0.)) && result.unpriced_tokens == 0. {
        result.total = Some(0.);
    }
    result.rows = combined.into_values().collect();
    result.rows.sort_by(|a, b| b.tokens.total_cmp(&a.tokens));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dates() -> Vec<NaiveDate> {
        vec![NaiveDate::from_ymd_opt(2026, 9, 16).unwrap()]
    }
    #[test]
    fn multiple_models_use_actual_token_types_without_double_counting() {
        let details: DetailedUsage = serde_json::from_value(serde_json::json!({"data":[{"date":"2026-09-16","models":[
            {"model":"gpt-6-astra","uncached_text_input_tokens":1000000,"cached_text_input_tokens":2000000,"text_output_tokens":100000},
            {"model":"gpt-5.6-sol","uncached_text_input_tokens":1000000,"cached_text_input_tokens":0,"text_output_tokens":100000}
        ]}]})).unwrap();
        let t = TokenSeries {
            daily: vec![Some(4_200_000)],
            total: Some(4_200_000),
            covered: 1,
        };
        let r = calculate(
            &dates(),
            &t,
            Some(&details),
            None,
            &crate::valuation::effective_assumption(None),
        );
        assert_eq!(r.total, Some(23.));
        assert_eq!(r.exact_days, 1);
        assert_eq!(r.estimated_days, 0);
        assert_eq!(r.unpriced_tokens, 0.);
    }
    #[test]
    fn estimated_split_unknown_models_and_missing_days_remain_visible() {
        let a: ActivityResponse=serde_json::from_value(serde_json::json!({"data":[{"date":"2026-09-16","totals":{"turns":4},"models":[{"model":"gpt-6-astra","turns":1},{"model":"gpt-5.6-sol","turns":2},{"model":"unpriced-model","turns":1}]}]})).unwrap();
        let t = TokenSeries {
            daily: vec![Some(4_000_000)],
            total: Some(4_000_000),
            covered: 1,
        };
        let r = calculate(
            &dates(),
            &t,
            None,
            Some(&a),
            &crate::valuation::effective_assumption(None),
        );
        assert!((r.total.unwrap() - 25.92).abs() < 1e-8);
        assert_eq!(r.unpriced_tokens, 1_000_000.);
        assert_eq!(r.priced_tokens, 3_000_000.);
        assert!(r.rows.iter().all(|r| r.estimated));
        let empty = calculate(
            &dates(),
            &t,
            None,
            None,
            &crate::valuation::effective_assumption(None),
        );
        assert_eq!(empty.total, None);
        assert_eq!(empty.unpriced_tokens, 4_000_000.);
    }
    #[test]
    fn details_fill_missing_totals_and_overflow_stays_missing() {
        let details: DetailedUsage = serde_json::from_value(serde_json::json!({"data":[{"date":"2026-09-16","groups":[{"dimensions":{"model":"gpt-5.6-sol"},"uncached_text_input_tokens":100,"cached_text_input_tokens":200,"text_output_tokens":50}]}]})).unwrap();
        let mut series = TokenSeries {
            daily: vec![None],
            total: None,
            covered: 0,
        };
        merge_reported_totals(&mut series, &dates(), Some(&details));
        assert_eq!(series.total, Some(350));
        assert_eq!(
            calculate(
                &dates(),
                &series,
                Some(&details),
                None,
                &crate::valuation::effective_assumption(None)
            )
            .rows[0]
                .model,
            "gpt-5.6-sol"
        );
        let mut invalid = details.clone();
        invalid.data[0].models[0].cached_text_input_tokens = Some(u64::MAX);
        let mut missing = TokenSeries {
            daily: vec![None],
            total: None,
            covered: 0,
        };
        merge_reported_totals(&mut missing, &dates(), Some(&invalid));
        assert_eq!(missing.total, None);
    }
    #[test]
    fn invalid_or_duplicate_counts_are_not_charged() {
        let a: ActivityResponse=serde_json::from_value(serde_json::json!({"data":[{"date":"2026-09-16","totals":{"turns":1},"models":[{"model":"gpt-6-astra","turns":2}]}]})).unwrap();
        let t = TokenSeries {
            daily: vec![Some(1_000)],
            total: Some(1_000),
            covered: 1,
        };
        assert_eq!(
            calculate(
                &dates(),
                &t,
                None,
                Some(&a),
                &crate::valuation::effective_assumption(None)
            )
            .total,
            None
        );
    }
}
