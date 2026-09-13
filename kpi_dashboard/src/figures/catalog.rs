//! Figure catalog: default (thesis) vs audit-only selectors.

use std::collections::HashSet;

use anyhow::{Context, Result};

/// Full catalog: `(slug, id, audit_only)`.
pub const FIGURE_CATALOG: &[(&str, u8, bool)] = &[
    ("p95_delay_priority_gap", 1, false),
    ("mac_drop_rate_by_strategy_load", 2, true),
    ("vo_reception_by_strategy_load", 3, true),
    ("vo_gain_vs_be_cost", 4, false),
    ("mac_drop_attribution_high_load", 5, false),
    ("vo_delay_cdf_high_load", 6, false),
    ("v2x_control_actions_by_load", 7, false),
    ("hotspot_vo_delay_by_policy", 8, false),
    ("hotspot_vo_p95_by_load", 9, false),
    ("regime_vo_p95_contrast", 10, false),
];

/// Default export set (thesis narrative). Audit heatmaps (02/03) excluded.
pub fn default_figure_ids() -> HashSet<u8> {
    FIGURE_CATALOG
        .iter()
        .filter(|(_, _, audit)| !*audit)
        .map(|(_, id, _)| *id)
        .collect()
}

pub fn figure_order(slug: &str) -> Option<u8> {
    FIGURE_CATALOG
        .iter()
        .find(|(name, _, _)| *name == slug)
        .map(|(_, id, _)| *id)
}

pub fn slug_for_figure_id(id: u8) -> Option<&'static str> {
    FIGURE_CATALOG
        .iter()
        .find(|(_, figure_id, _)| *figure_id == id)
        .map(|(slug, _, _)| *slug)
}

pub fn print_figure_catalog() {
    println!("Available figures (--figures accepts id, fig_NN, or slug):");
    println!("  Default export omits audit-only heatmaps (02, 03); pass them explicitly if needed.");
    for (slug, id, audit) in FIGURE_CATALOG {
        let tag = if *audit { " [audit]" } else { "" };
        println!("  {id:02}  fig_{id:02}  {slug}{tag}");
    }
}

pub fn resolve_figure_selectors(inputs: &[String]) -> Result<HashSet<u8>> {
    if inputs.is_empty() {
        return Ok(default_figure_ids());
    }
    let mut selected = HashSet::new();
    for input in inputs {
        let id = parse_figure_selector(input).with_context(|| {
            format!("unknown figure {input:?}; use --list-figures to see ids and slugs")
        })?;
        selected.insert(id);
    }
    Ok(selected)
}

pub fn parse_figure_selector(input: &str) -> Option<u8> {
    let token = input.trim().to_lowercase();
    let token = token.strip_prefix("fig_").unwrap_or(&token);
    if let Ok(id) = token.parse::<u8>() {
        return slug_for_figure_id(id).and_then(figure_order);
    }
    match token.as_ref() {
        "cdf" | "vo_delay_cdf" => Some(6),
        "p95" | "p95_gap" => Some(1),
        "drops" | "drop_attribution" => Some(5),
        "v2x" | "v2x_control" => Some(7),
        "tradeoff" | "gain_cost" => Some(4),
        "hotspot" | "hotspot_policy" => Some(8),
        "dose" | "hotspot_dose" => Some(9),
        "regime" | "contrast" => Some(10),
        _ => figure_order(&token),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_omits_audit_heatmaps() {
        let ids = default_figure_ids();
        assert!(ids.contains(&1));
        assert!(!ids.contains(&2));
        assert!(!ids.contains(&3));
        assert!(ids.contains(&4));
        assert!(ids.contains(&8));
        assert!(ids.contains(&10));
    }

    #[test]
    fn figure_order_is_stable_by_slug() {
        assert_eq!(figure_order("p95_delay_priority_gap"), Some(1));
        assert_eq!(figure_order("vo_gain_vs_be_cost"), Some(4));
        assert_eq!(figure_order("hotspot_vo_delay_by_policy"), Some(8));
        assert_eq!(figure_order("regime_vo_p95_contrast"), Some(10));
        assert_eq!(figure_order("latency_jitter_tradeoff"), None);
    }
}
