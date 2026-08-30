//! Cost- and latency-aware combo member ordering (`Cheapest` / `Fastest`).
//!
//! Both orderings read the same pricing table the usage/cost engine uses
//! (`AppDb::pricing`, exposed to the dashboard as `/api/pricing`), so a model
//! priced in the UI immediately affects routing order.

use serde_json::Value;

use crate::core::model::resolve_provider_alias;
use crate::core::usage::{parse_model_pricing, CostModel};
use crate::types::PricingTable;

/// Resolve the pricing row for a combo entry (`prefix/model`).
///
/// The pricing table may be keyed by provider alias (`cc`) or canonical id
/// (`claude`), and free tiers register a single provider-wide `all` row
/// (`kiro`, `opencode`, `vertex`), so all four combinations are tried.
fn pricing_entry<'a>(model: &'a str, pricing: &'a PricingTable) -> Option<(&'a Value, &'a str)> {
    let (prefix, name) = model.split_once('/')?;
    let resolved = resolve_provider_alias(prefix);
    for provider in [prefix, resolved.as_str()] {
        if let Some(models) = pricing.get(provider) {
            if let Some(entry) = models.get(name).or_else(|| models.get("all")) {
                return Some((entry, name));
            }
        }
    }
    None
}

/// Representative per-request cost of a combo member (USD per 1M in + out).
///
/// A missing provider/model row, an unpriced row, or a non-per-token cost
/// model (free / flat-monthly subscription / prepaid credits) all yield `0.0`,
/// so free-tier members sort ahead of metered ones under `Cheapest`.
fn model_cost(model: &str, pricing: &PricingTable) -> f64 {
    let Some((entry, name)) = pricing_entry(model, pricing) else {
        return 0.0;
    };
    let provider = model.split('/').next().unwrap_or_default();
    let parsed = parse_model_pricing(provider, name, entry);
    match parsed.cost_model {
        CostModel::PerToken => parsed.input_price_per_million + parsed.output_price_per_million,
        CostModel::Free | CostModel::FlatMonthly | CostModel::Credits => 0.0,
    }
}

/// Sort combo members ascending by cost (free `$0` first).
///
/// Stable: equal-cost members keep their configured relative order, so the
/// operator's declared priority still decides ties.
pub fn sort_models_by_cost(models: &[String], pricing: &PricingTable) -> Vec<String> {
    sort_by_key(models, |model| Some(model_cost(model, pricing)))
}

/// Latency hint (ms) for a combo member, when the pricing row carries one.
fn model_latency(model: &str, pricing: &PricingTable) -> Option<f64> {
    let (entry, _) = pricing_entry(model, pricing)?;
    let obj = entry.as_object()?;
    ["latency", "latencyMs", "latency_ms"]
        .iter()
        .find_map(|field| obj.get(*field).and_then(Value::as_f64))
}

/// Sort combo members ascending by latency hint (fastest first).
///
/// Members without a hint keep their configured relative order and are placed
/// after every hinted member, so `Fastest` degrades to the declared priority
/// when no latency data exists.
pub fn sort_models_by_latency(models: &[String], pricing: &PricingTable) -> Vec<String> {
    sort_by_key(models, |model| model_latency(model, pricing))
}

const RANK_SCALE: f64 = 31.0;

fn tier_value(label: &str) -> f64 {
    match label.trim().to_lowercase().as_str() {
        "frontier" => 4.0,
        "large" => 3.0,
        "medium" => 2.0,
        "small" => 1.0,
        _ => 0.0,
    }
}

fn intelligence_composite(size_label: &str, rank: u32) -> f64 {
    tier_value(size_label) * 1000.0 - (rank.max(1) as f64).sqrt() * RANK_SCALE
}

fn model_intelligence_composite(model: &str, pricing: &PricingTable) -> Option<f64> {
    let (entry, _) = pricing_entry(model, pricing)?;
    let obj = entry.as_object()?;
    // Direct intelligenceScore overrides composite when present
    if let Some(v) = obj
        .get("intelligenceScore")
        .or_else(|| obj.get("intelligence_score"))
        .and_then(Value::as_f64)
    {
        return Some(v);
    }
    let size_label = obj
        .get("sizeLabel")
        .or_else(|| obj.get("size_label"))
        .or_else(|| obj.get("size"))
        .and_then(Value::as_str)
        .unwrap_or("Medium");
    let rank = obj
        .get("intelligenceRank")
        .or_else(|| obj.get("intelligence_rank"))
        .or_else(|| obj.get("rank"))
        .and_then(Value::as_u64)
        .unwrap_or(500) as u32;
    Some(intelligence_composite(size_label, rank))
}

/// Sort combo members descending by Balanced score: 0.5*reliability +0.25*speed +0.25*intelligence.
/// Intelligence is FreeLLMAPI composite normalized per-combo (min-max). Reliability/speed from pricing hints
/// (default 0.5 when missing). Higher overall first, stable on ties.
pub fn sort_models_by_balanced(models: &[String], pricing: &PricingTable) -> Vec<String> {
    if models.is_empty() {
        return Vec::new();
    }
    // Collect composites for per-combo min-max normalization
    let composites: Vec<Option<f64>> = models
        .iter()
        .map(|m| model_intelligence_composite(m, pricing))
        .collect();
    let present: Vec<f64> = composites.iter().filter_map(|v| *v).collect();
    let (min_c, max_c) = if present.is_empty() {
        (0.0, 0.0)
    } else {
        (
            present.iter().cloned().fold(f64::INFINITY, f64::min),
            present.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        )
    };
    // Max latency for speed normalization (like ScoringEngine)
    let max_latency = models
        .iter()
        .filter_map(|m| model_latency(m, pricing))
        .fold(0.0_f64, f64::max)
        .max(1.0);

    let mut scored: Vec<(usize, f64, &String)> = models
        .iter()
        .enumerate()
        .map(|(idx, model)| {
            // Reliability from pricing or default 0.5
            let reliability = pricing_entry(model, pricing)
                .and_then(|(e, _)| {
                    e.as_object()
                        .and_then(|o| {
                            o.get("reliabilityScore")
                                .or_else(|| o.get("reliability_score"))
                                .or_else(|| o.get("reliability"))
                        })
                        .and_then(Value::as_f64)
                })
                .unwrap_or(0.5)
                .clamp(0.0, 1.0);
            let latency = model_latency(model, pricing).unwrap_or(max_latency);
            let speed = (1.0 - (latency / max_latency)).clamp(0.0, 1.0);
            let intelligence = match composites[idx] {
                Some(c) if max_c > min_c => ((c - min_c) / (max_c - min_c)).clamp(0.0, 1.0),
                Some(_) => 1.0,
                None => 0.5,
            };
            let overall = reliability * 0.5 + speed * 0.25 + intelligence * 0.25;
            (idx, overall, model)
        })
        .collect();
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    scored.into_iter().map(|(_, _, m)| m.clone()).collect()
}

/// Stable ascending sort on an optional numeric key; `None` sorts last.
fn sort_by_key(models: &[String], key: impl Fn(&str) -> Option<f64>) -> Vec<String> {
    let mut indexed: Vec<(usize, Option<f64>, &String)> = models
        .iter()
        .enumerate()
        .map(|(index, model)| (index, key(model), model))
        .collect();
    indexed.sort_by(|a, b| match (a.1, b.1) {
        (Some(x), Some(y)) => x
            .partial_cmp(&y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.0.cmp(&b.0),
    });
    indexed
        .into_iter()
        .map(|(_, _, model)| model.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn table(rows: &[(&str, &str, Value)]) -> PricingTable {
        let mut pricing = PricingTable::new();
        for (provider, model, value) in rows {
            pricing
                .entry((*provider).to_string())
                .or_default()
                .insert((*model).to_string(), value.clone());
        }
        pricing
    }

    #[test]
    fn cheapest_sorts_by_api_style_pricing() {
        let models = vec![
            "openai/gpt-4o".to_string(),
            "openrouter/llama-3.1-free".to_string(),
            "anthropic/claude-3-haiku".to_string(),
        ];
        let pricing = table(&[
            ("openai", "gpt-4o", json!({ "input": 2.5, "output": 10.0 })),
            (
                "openrouter",
                "llama-3.1-free",
                json!({ "input": 0.0, "output": 0.0 }),
            ),
            (
                "anthropic",
                "claude-3-haiku",
                json!({ "input": 0.25, "output": 1.25 }),
            ),
        ]);
        assert_eq!(
            sort_models_by_cost(&models, &pricing),
            vec![
                "openrouter/llama-3.1-free".to_string(),
                "anthropic/claude-3-haiku".to_string(),
                "openai/gpt-4o".to_string(),
            ]
        );
    }

    #[test]
    fn cheapest_treats_missing_pricing_as_free_and_is_stable() {
        let models = vec![
            "openai/gpt-4o".to_string(),
            "nvidia/llama-3.1".to_string(),
            "oc/grok-code".to_string(),
        ];
        let pricing = table(&[("openai", "gpt-4o", json!({ "input": 2.5, "output": 10.0 }))]);
        assert_eq!(
            sort_models_by_cost(&models, &pricing),
            vec![
                "nvidia/llama-3.1".to_string(),
                "oc/grok-code".to_string(),
                "openai/gpt-4o".to_string(),
            ],
            "unpriced members are free and keep their configured order"
        );
    }

    #[test]
    fn cheapest_reads_alias_keyed_and_provider_wide_rows() {
        let models = vec![
            "cc/claude-opus-4".to_string(),
            "kr/claude-sonnet".to_string(),
        ];
        let pricing = table(&[
            (
                "claude",
                "claude-opus-4",
                json!({ "input": 15.0, "output": 75.0 }),
            ),
            ("kiro", "all", json!({ "costModel": "free" })),
        ]);
        assert_eq!(
            sort_models_by_cost(&models, &pricing),
            vec![
                "kr/claude-sonnet".to_string(),
                "cc/claude-opus-4".to_string(),
            ],
            "alias cc→claude resolves and kiro's free `all` row wins"
        );
    }

    #[test]
    fn cheapest_ignores_non_metered_cost_models() {
        let models = vec!["kimi/kimi-k2.5".to_string(), "glm/glm-4.7".to_string()];
        let pricing = table(&[
            (
                "kimi",
                "kimi-k2.5",
                json!({ "costModel": "flat_monthly", "flatMonthlyPrice": 9.0 }),
            ),
            ("glm", "glm-4.7", json!({ "pricePerMillion": 0.6 })),
        ]);
        assert_eq!(
            sort_models_by_cost(&models, &pricing),
            vec!["kimi/kimi-k2.5".to_string(), "glm/glm-4.7".to_string()],
            "a flat-monthly subscription has no marginal per-request cost"
        );
    }

    #[test]
    fn fastest_sorts_by_latency_hint() {
        let models = vec![
            "openai/gpt-4o".to_string(),
            "nvidia/llama-3.1".to_string(),
            "anthropic/claude-3-haiku".to_string(),
        ];
        let pricing = table(&[
            ("openai", "gpt-4o", json!({ "latency": 800.0 })),
            ("nvidia", "llama-3.1", json!({ "latencyMs": 200 })),
            ("anthropic", "claude-3-haiku", json!({ "latency": 400.0 })),
        ]);
        assert_eq!(
            sort_models_by_latency(&models, &pricing),
            vec![
                "nvidia/llama-3.1".to_string(),
                "anthropic/claude-3-haiku".to_string(),
                "openai/gpt-4o".to_string(),
            ]
        );
    }

    #[test]
    fn balanced_orders_by_weighted_score() {
        // Balanced = 0.5*reliability +0.25*speed +0.25*intelligence
        // Models: A Frontier rank1 high intel but slow/high cost reliable, B Medium rank8 mid, C Small rank500 low intel but fast
        let models = vec![
            "openai/gpt-4o".to_string(), // Frontier rank1, latency 800, reliability 0.9
            "anthropic/claude-3-haiku".to_string(), // Medium rank8, latency 400, reliability 0.6
            "nvidia/llama-3.1".to_string(), // Small rank500, latency 200, reliability 0.5
        ];
        let pricing = table(&[
            (
                "openai",
                "gpt-4o",
                serde_json::json!({ "latency": 800, "reliabilityScore": 0.9, "sizeLabel": "Frontier", "intelligenceRank": 1 }),
            ),
            (
                "anthropic",
                "claude-3-haiku",
                serde_json::json!({ "latency": 400, "reliabilityScore": 0.6, "sizeLabel": "Medium", "intelligenceRank": 8 }),
            ),
            (
                "nvidia",
                "llama-3.1",
                serde_json::json!({ "latency": 200, "reliabilityScore": 0.5, "sizeLabel": "Small", "intelligenceRank": 500 }),
            ),
        ]);
        let ordered = sort_models_by_balanced(&models, &pricing);
        // With per-combo min-max intel (Small 0, Frontier 1), Frontier should win despite slow latency due to high reliability+intelligence
        assert_eq!(ordered[0], "openai/gpt-4o");
        // Ensure stable and all present
        assert_eq!(ordered.len(), 3);
    }

    #[test]
    fn balanced_intelligence_composite_tier_dominance() {
        let models = vec!["a/m1".to_string(), "b/m2".to_string()];
        let pricing = table(&[
            (
                "a",
                "m1",
                serde_json::json!({ "sizeLabel": "Large", "intelligenceRank": 100 }),
            ),
            (
                "b",
                "m2",
                serde_json::json!({ "sizeLabel": "Medium", "intelligenceRank": 1 }),
            ),
        ]);
        // Large tier*1000 -31*sqrt(100)=3000-310=2690 vs Medium 2000-31=1969 => Large wins even with worst rank
        let ordered = sort_models_by_balanced(&models, &pricing);
        assert_eq!(ordered[0], "a/m1");
    }

    #[test]
    fn fastest_keeps_configured_order_without_hints() {
        let models = vec![
            "openai/gpt-4o".to_string(),
            "nvidia/llama-3.1".to_string(),
            "anthropic/claude-3-haiku".to_string(),
        ];
        let pricing = table(&[("nvidia", "llama-3.1", json!({ "latency": 200.0 }))]);
        assert_eq!(
            sort_models_by_latency(&models, &pricing),
            vec![
                "nvidia/llama-3.1".to_string(),
                "openai/gpt-4o".to_string(),
                "anthropic/claude-3-haiku".to_string(),
            ],
            "hinted member first, unhinted members keep configured order"
        );
    }
}
