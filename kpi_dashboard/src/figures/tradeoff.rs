//! Cross-regime tradeoff figure builders (figs 04 and 10).

use std::collections::BTreeMap;

use crate::ConfigSummary;

use super::data::strategy_label;
use super::style::{Figure, FigureStyle, COLORS, GRID_COLOR, INK_COLOR, STRATEGIES};
use super::svg::{
    append_bottom_color_legend, axis_frame, draw_grouped_bar_panel, draw_x_ticks_ranged,
    draw_y_ticks, draw_y_ticks_ranged, footer_band_height, text, PlotArea,
};

/// Inclusive axis range for signed deltas: pads the data span and keeps the origin when useful.
fn signed_delta_bounds(values: &[f64]) -> (f64, f64) {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !min.is_finite() || !max.is_finite() {
        return (-1.0, 1.0);
    }
    let span = (max - min).abs().max(1e-6);
    let pad = (span * 0.18).max(0.05);
    let mut lo = min - pad;
    let mut hi = max + pad;
    // Keep a readable span when points nearly coincide.
    if hi - lo < 0.2 {
        let mid = 0.5 * (lo + hi);
        lo = mid - 0.1;
        hi = mid + 0.1;
    }
    // Include the origin when the cloud is entirely on one side of zero.
    if lo > 0.0 {
        lo = 0.0;
    }
    if hi < 0.0 {
        hi = 0.0;
    }
    (lo, hi)
}

fn policy_marker(strategy: &str, cx: f64, cy: f64, color: &str) -> String {
    let r = 6.0;
    match strategy {
        "edca_only" => format!(
            r#"<circle cx="{cx:.2}" cy="{cy:.2}" r="{r}" fill="{color}" fill-opacity="0.92" stroke="{INK_COLOR}" stroke-width="0.7"/>"#
        ),
        "stable" => format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{color}" fill-opacity="0.92" stroke="{INK_COLOR}" stroke-width="0.7"/>"#,
            cx - r,
            cy - r,
            2.0 * r,
            2.0 * r
        ),
        "guarded" => format!(
            r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{color}" fill-opacity="0.92" stroke="{INK_COLOR}" stroke-width="0.7"/>"#,
            cx,
            cy - r * 1.15,
            cx - r * 1.05,
            cy + r * 0.85,
            cx + r * 1.05,
            cy + r * 0.85
        ),
        _ => {
            // diamond (emergency)
            format!(
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{color}" fill-opacity="0.92" stroke="{INK_COLOR}" stroke-width="0.7"/>"#,
                cx,
                cy - r * 1.2,
                cx + r * 1.05,
                cy,
                cx,
                cy + r * 1.2,
                cx - r * 1.05,
                cy
            )
        }
    }
}

fn draw_zero_guides(
    svg: &mut String,
    plot: &PlotArea,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
) {
    if min_x <= 0.0 && max_x >= 0.0 {
        let x0 = plot.scale_x_range(0.0, min_x, max_x);
        svg.push_str(&format!(
            r##"<line x1="{x0:.2}" y1="{:.2}" x2="{x0:.2}" y2="{:.2}" stroke="{GRID_COLOR}" stroke-width="1" stroke-dasharray="4,3"/>"##,
            plot.top, plot.bottom
        ));
    }
    if min_y <= 0.0 && max_y >= 0.0 {
        let y0 = plot.scale_y_range(0.0, min_y, max_y);
        svg.push_str(&format!(
            r##"<line x1="{:.2}" y1="{y0:.2}" x2="{:.2}" y2="{y0:.2}" stroke="{GRID_COLOR}" stroke-width="1" stroke-dasharray="4,3"/>"##,
            plot.left,
            plot.left + plot.inner_w
        ));
    }
}

fn collect_gain_cost_points(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
) -> Vec<(&'static str, &'static str, f64, f64, &'static str)> {
    let mut points = Vec::new();
    let Some(plain) = matrix.get(&("plain".to_string(), "high".to_string())) else {
        return points;
    };
    let Some(plain_vo) = plain.metrics.vo_delay_p95_ms else {
        return points;
    };
    let Some(plain_be) = plain.metrics.be_delay_p95_ms else {
        return points;
    };
    for (index, strategy) in STRATEGIES
        .iter()
        .copied()
        .filter(|s| *s != "plain")
        .enumerate()
    {
        let Some(summary) = matrix.get(&(strategy.to_string(), "high".to_string())) else {
            continue;
        };
        let Some(vo) = summary.metrics.vo_delay_p95_ms else {
            continue;
        };
        let Some(be) = summary.metrics.be_delay_p95_ms else {
            continue;
        };
        points.push((
            strategy,
            strategy_label(strategy),
            vo - plain_vo,
            be - plain_be,
            COLORS[index % COLORS.len()],
        ));
    }
    points
}

fn label_offsets(strategy: &str, panel_index: usize) -> (f64, f64) {
    // Nudge labels so near-coincident points remain readable.
    match (panel_index, strategy) {
        (0, "edca_only") => (10.0, -14.0),
        (0, "stable") => (10.0, -4.0),
        (0, "guarded") => (10.0, 6.0),
        (0, "emergency") => (-10.0, 16.0),
        (1, "emergency") => (10.0, -4.0),
        _ => (8.0, -4.0),
    }
}

fn points_are_clustered(points: &[(&str, &str, f64, f64, &str)], threshold: f64) -> bool {
    if points.len() < 2 {
        return false;
    }
    let mut max_dist: f64 = 0.0;
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let dx = points[i].2 - points[j].2;
            let dy = points[i].3 - points[j].3;
            max_dist = max_dist.max((dx * dx + dy * dy).sqrt());
        }
    }
    max_dist < threshold
}

fn draw_scatter_panel(
    svg: &mut String,
    plot: &PlotArea,
    style: FigureStyle,
    title: &str,
    y_label: &str,
    points: &[(&'static str, &'static str, f64, f64, &'static str)],
    panel_index: usize,
    cluster_threshold: f64,
) {
    let xs: Vec<f64> = points.iter().map(|p| p.2).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.3).collect();
    let (min_x, max_x) = signed_delta_bounds(&xs);
    let (min_y, max_y) = signed_delta_bounds(&ys);
    svg.push_str(&axis_frame(plot, style, "Δ VO P95 vs DCF (ms)", y_label));
    svg.push_str(&text(
        plot.left + plot.inner_w / 2.0,
        plot.top - 6.0,
        title,
        style.font_category,
        "middle",
        INK_COLOR,
    ));
    draw_x_ticks_ranged(svg, plot, style, min_x, max_x);
    draw_y_ticks_ranged(svg, plot, style, min_y, max_y);
    draw_zero_guides(svg, plot, min_x, max_x, min_y, max_y);

    // On hotspot, EDCA/Stable/Guarded sit almost on top of each other; label the
    // pack once and keep Emergency as the only individually labeled point.
    let (winners, pack): (Vec<_>, Vec<_>) = points.iter().partition(|p| p.0 == "emergency");
    let cluster_pack = panel_index == 1 && points_are_clustered(&pack, cluster_threshold);

    for (strategy, label, dx, dy, color) in points {
        let x = plot.scale_x_range(*dx, min_x, max_x);
        let y = plot.scale_y_range(*dy, min_y, max_y);
        svg.push_str(&policy_marker(strategy, x, y, color));
        if cluster_pack && *strategy != "emergency" {
            continue;
        }
        let (ox, oy) = label_offsets(strategy, panel_index);
        let anchor = if ox < 0.0 { "end" } else { "start" };
        svg.push_str(&text(
            x + ox,
            y + oy,
            label,
            style.font_tick.saturating_sub(1).max(9),
            anchor,
            INK_COLOR,
        ));
    }

    if cluster_pack && !pack.is_empty() {
        let cx = pack.iter().map(|p| p.2).sum::<f64>() / pack.len() as f64;
        let cy = pack.iter().map(|p| p.3).sum::<f64>() / pack.len() as f64;
        let x = plot.scale_x_range(cx, min_x, max_x);
        let y = plot.scale_y_range(cy, min_y, max_y);
        svg.push_str(&text(
            x - 10.0,
            y - 16.0,
            "EDCA / Stable / Guarded",
            style.font_tick.saturating_sub(1).max(9),
            "end",
            INK_COLOR,
        ));
        let _ = winners; // Emergency already labeled above
    }
}

pub(crate) fn vo_gain_vs_be_cost_figure(
    netload: &BTreeMap<(String, String), ConfigSummary>,
    hotspot: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    let panels = [
        ("Uniform netload_high", collect_gain_cost_points(netload)),
        ("Hotspot_high", collect_gain_cost_points(hotspot)),
    ];
    if panels.iter().all(|(_, pts)| pts.is_empty()) {
        return None;
    }

    // Side-by-side panels need independent Y scales: hotspot BE Δ is ~25× netload Stable.
    let fig_style = if style.show_header {
        style
    } else {
        style.tall_variant()
    };
    let footer = footer_band_height(fig_style);
    let panel_gap = 36.0;
    let panel_area_left = fig_style.margin_left;
    let panel_area_right = fig_style.width as f64 - fig_style.margin_right;
    let panel_area_top = fig_style.margin_top + 18.0;
    let panel_area_bottom = fig_style.height as f64 - fig_style.margin_bottom - footer;
    let panel_w = (panel_area_right - panel_area_left - panel_gap) / 2.0;
    let panel_h = panel_area_bottom - panel_area_top;

    let mut svg = String::new();
    let mut legend_items: Vec<(&str, &str)> = Vec::new();

    for (panel_index, (title, points)) in panels.iter().enumerate() {
        if points.is_empty() {
            continue;
        }
        let left = panel_area_left + panel_index as f64 * (panel_w + panel_gap);
        let plot = PlotArea {
            left,
            top: panel_area_top,
            bottom: panel_area_bottom,
            inner_w: panel_w,
            inner_h: panel_h,
        };
        let y_label = if panel_index == 0 {
            "Δ BE P95 vs DCF (ms)"
        } else {
            ""
        };
        // Hotspot pack spans ~1 ms in ΔVO and ~5 ms in ΔBE against a ~770 ms Y range.
        draw_scatter_panel(
            &mut svg,
            &plot,
            fig_style,
            title,
            y_label,
            points,
            panel_index,
            20.0,
        );
        if panel_index == 0 {
            for (_strategy, label, _dx, _dy, color) in points {
                if !legend_items.iter().any(|(l, _)| *l == *label) {
                    legend_items.push((*label, *color));
                }
            }
        }
    }

    let legend_plot = PlotArea {
        left: panel_area_left,
        top: panel_area_top,
        bottom: panel_area_bottom,
        inner_w: panel_area_right - panel_area_left,
        inner_h: panel_h,
    };
    if !legend_items.is_empty() {
        append_bottom_color_legend(&mut svg, &legend_plot, fig_style, &legend_items);
    }

    Some(Figure {
        slug: "vo_gain_vs_be_cost",
        title: "Voice Gain Versus Best-Effort Cost",
        question:
            "Relative to DCF, how much VO P95 improves for each unit of BE P95 cost across regimes?",
        body: svg,
        style: Some(fig_style),
    })
}

pub(crate) fn regime_vo_p95_contrast_figure(
    netload: &BTreeMap<(String, String), ConfigSummary>,
    hotspot: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    let rows: Vec<_> = STRATEGIES
        .iter()
        .filter_map(|strategy| {
            let net = netload
                .get(&(strategy.to_string(), "high".to_string()))
                .and_then(|s| s.metrics.vo_delay_p95_ms)?;
            let hot = hotspot
                .get(&(strategy.to_string(), "high".to_string()))
                .and_then(|s| s.metrics.vo_delay_p95_ms)?;
            Some((strategy_label(strategy), net, hot))
        })
        .collect();
    if rows.is_empty() {
        return None;
    }
    let categories: Vec<&str> = rows.iter().map(|(label, _, _)| *label).collect();
    let net_values: Vec<f64> = rows.iter().map(|(_, net, _)| *net).collect();
    let hot_values: Vec<f64> = rows.iter().map(|(_, _, hot)| *hot).collect();
    let max_value = rows
        .iter()
        .flat_map(|(_, net, hot)| [*net, *hot])
        .fold(0.0, f64::max);
    let plot = PlotArea::new(style);
    let mut svg = axis_frame(&plot, style, "Policy", "VO P95 delay (ms)");
    draw_y_ticks(&mut svg, &plot, style, max_value);
    draw_grouped_bar_panel(
        &mut svg,
        &plot,
        style,
        &categories,
        &[
            ("Netload high", COLORS[0], &net_values),
            ("Hotspot high", COLORS[1], &hot_values),
        ],
        max_value,
        true,
    );
    append_bottom_color_legend(
        &mut svg,
        &plot,
        style,
        &[("Netload high", COLORS[0]), ("Hotspot high", COLORS[1])],
    );
    Some(Figure {
        slug: "regime_vo_p95_contrast",
        title: "Regime Contrast for Voice P95",
        question: "How does high-load VO P95 differ between netload and hotspot regimes by policy?",
        body: svg,
        style: None,
    })
}
