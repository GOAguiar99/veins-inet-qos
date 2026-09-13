//! Hotspot regime figure builders (figs 08–09).

use std::collections::BTreeMap;

use crate::ConfigSummary;

use super::data::strategy_label;
use super::style::{
    Figure, FigureStyle, BE_COLOR, STRATEGIES, VO_COLOR, WORKLOADS,
};
use super::svg::{
    append_bottom_color_legend, axis_frame, draw_grouped_bar_panel, draw_y_ticks, PlotArea,
};

pub(crate) fn hotspot_vo_delay_by_policy_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    let rows: Vec<_> = STRATEGIES
        .iter()
        .filter_map(|strategy| {
            let summary = matrix.get(&(strategy.to_string(), "high".to_string()))?;
            let mean = summary.metrics.vo_delay_ms?;
            let p95 = summary.metrics.vo_delay_p95_ms?;
            Some((strategy_label(strategy), mean, p95))
        })
        .collect();
    if rows.is_empty() {
        return None;
    }
    let categories: Vec<&str> = rows.iter().map(|(label, _, _)| *label).collect();
    let mean_values: Vec<f64> = rows.iter().map(|(_, mean, _)| *mean).collect();
    let p95_values: Vec<f64> = rows.iter().map(|(_, _, p95)| *p95).collect();
    let max_value = rows
        .iter()
        .flat_map(|(_, mean, p95)| [*mean, *p95])
        .fold(0.0, f64::max);
    let plot = PlotArea::new(style);
    let mut svg = axis_frame(&plot, style, "Policy", "Voice delay (ms)");
    draw_y_ticks(&mut svg, &plot, style, max_value);
    draw_grouped_bar_panel(
        &mut svg,
        &plot,
        style,
        &categories,
        &[
            ("Voice mean delay", BE_COLOR, &mean_values),
            ("Voice P95 delay", VO_COLOR, &p95_values),
        ],
        max_value,
        true,
    );
    append_bottom_color_legend(
        &mut svg,
        &plot,
        style,
        &[("Voice mean delay", BE_COLOR), ("Voice P95 delay", VO_COLOR)],
    );
    Some(Figure {
        slug: "hotspot_vo_delay_by_policy",
        title: "Hotspot Voice Delay by Policy",
        question: "Under hotspot_high, how do VO mean and P95 delay compare across MAC policies?",
        body: svg,
        style: None,
    })
}

pub(crate) fn hotspot_vo_p95_by_load_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    let categories: Vec<&str> = WORKLOADS
        .iter()
        .map(|w| match *w {
            "low" => "Low",
            "medium" => "Medium",
            "high" => "High",
            other => other,
        })
        .collect();
    let plain: Vec<f64> = WORKLOADS
        .iter()
        .map(|workload| {
            matrix
                .get(&("plain".to_string(), workload.to_string()))
                .and_then(|s| s.metrics.vo_delay_p95_ms)
                .unwrap_or(f64::NAN)
        })
        .collect();
    let emergency: Vec<f64> = WORKLOADS
        .iter()
        .map(|workload| {
            matrix
                .get(&("emergency".to_string(), workload.to_string()))
                .and_then(|s| s.metrics.vo_delay_p95_ms)
                .unwrap_or(f64::NAN)
        })
        .collect();
    if plain.iter().all(|v| !v.is_finite()) && emergency.iter().all(|v| !v.is_finite()) {
        return None;
    }
    let plain_plot: Vec<f64> = plain.iter().map(|v| if v.is_finite() { *v } else { 0.0 }).collect();
    let emergency_plot: Vec<f64> = emergency
        .iter()
        .map(|v| if v.is_finite() { *v } else { 0.0 })
        .collect();
    let max_value = plain_plot
        .iter()
        .chain(emergency_plot.iter())
        .copied()
        .fold(0.0, f64::max);
    if max_value <= 0.0 {
        return None;
    }
    let plot = PlotArea::new(style);
    let mut svg = axis_frame(&plot, style, "Offered load", "Voice P95 delay (ms)");
    draw_y_ticks(&mut svg, &plot, style, max_value);
    draw_grouped_bar_panel(
        &mut svg,
        &plot,
        style,
        &categories,
        &[
            ("Plain", BE_COLOR, &plain_plot),
            ("Emergency", VO_COLOR, &emergency_plot),
        ],
        max_value,
        true,
    );
    append_bottom_color_legend(
        &mut svg,
        &plot,
        style,
        &[("Plain", BE_COLOR), ("Emergency", VO_COLOR)],
    );
    Some(Figure {
        slug: "hotspot_vo_p95_by_load",
        title: "Hotspot Voice P95 Across Load",
        question: "How does Emergency VO P95 compare with DCF as hotspot offered load grows?",
        body: svg,
        style: None,
    })
}
