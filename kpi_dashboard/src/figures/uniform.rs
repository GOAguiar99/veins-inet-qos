//! Uniform netload figure builders (figs 01–03, 05–07).

use std::collections::BTreeMap;

use crate::ConfigSummary;

use super::data::{parse_config, strategy_label, Overlay, VoDelaySample};
use super::style::{
    Figure, FigureStyle, BE_COLOR, CDF_DASHES, CDF_MAX_POINTS, COLORS, INK_COLOR, MUTED_COLOR,
    STRATEGIES, VO_COLOR, VO_DELAY_CDF_X_MAX_MS, V2X_STRATEGIES, WORKLOADS,
};
use super::svg::{
    append_bottom_color_legend, append_cdf_bottom_line_legend, axis_frame, axis_frame_cdf,
    axis_label_offset, category_offset, cdf_footer_band_height, decimate_sorted, draw_grouped_bar_panel,
    draw_x_ticks_exact, draw_y_ticks, draw_y_ticks_exact, footer_band_height, format_metric,
    heat_color, heatmap_plot_area, heatmap_row_label_gap, heatmap_y_axis_label_offset,
    panel_axis_frame, rect, rect_fill, text, workload_label, PlotArea,
};

pub(crate) fn p95_priority_gap_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    let rows: Vec<_> = STRATEGIES
        .iter()
        .filter_map(|strategy| {
            let summary = matrix.get(&(strategy.to_string(), "high".to_string()))?;
            let be = summary.metrics.be_delay_p95_ms?;
            let vo = summary.metrics.vo_delay_p95_ms?;
            Some((strategy_label(strategy), be, vo))
        })
        .collect();
    if rows.is_empty() {
        return None;
    }
    let fig_style = if style.show_header {
        style
    } else {
        style.tall_variant()
    };
    let footer = footer_band_height(fig_style);
    let panel_gap = 22.0;
    let panel_area_bottom = fig_style.height as f64 - fig_style.margin_bottom - footer;
    let panel_area_top = fig_style.margin_top;
    let panel_h = (panel_area_bottom - panel_area_top - panel_gap) / 2.0;
    let top_plot = PlotArea {
        left: fig_style.margin_left,
        top: panel_area_top,
        bottom: panel_area_top + panel_h,
        inner_w: fig_style.width as f64 - fig_style.margin_left - fig_style.margin_right,
        inner_h: panel_h,
    };
    let bottom_plot = PlotArea {
        left: fig_style.margin_left,
        top: top_plot.bottom + panel_gap,
        bottom: panel_area_bottom,
        inner_w: top_plot.inner_w,
        inner_h: panel_area_bottom - top_plot.bottom - panel_gap,
    };
    let categories: Vec<&str> = rows.iter().map(|(label, _, _)| *label).collect();
    let vo_values: Vec<f64> = rows.iter().map(|(_, _, vo)| *vo).collect();
    let be_values: Vec<f64> = rows.iter().map(|(_, be, _)| *be).collect();
    let max_vo = vo_values.iter().copied().fold(0.0, f64::max);
    let max_be = be_values.iter().copied().fold(0.0, f64::max);
    let mut svg = String::new();
    svg.push_str(&panel_axis_frame(
        &top_plot,
        fig_style,
        "VO P95 delay (ms)",
        true,
    ));
    draw_y_ticks(&mut svg, &top_plot, fig_style, max_vo);
    draw_grouped_bar_panel(
        &mut svg,
        &top_plot,
        fig_style,
        &categories,
        &[("VO P95", VO_COLOR, &vo_values)],
        max_vo,
        false,
    );
    svg.push_str(&panel_axis_frame(
        &bottom_plot,
        fig_style,
        "BE P95 delay (ms)",
        true,
    ));
    draw_y_ticks(&mut svg, &bottom_plot, fig_style, max_be);
    draw_grouped_bar_panel(
        &mut svg,
        &bottom_plot,
        fig_style,
        &categories,
        &[("BE P95", BE_COLOR, &be_values)],
        max_be,
        true,
    );
    svg.push_str(&text(
        bottom_plot.left + bottom_plot.inner_w / 2.0,
        bottom_plot.bottom + axis_label_offset(fig_style),
        "Strategy",
        fig_style.font_axis,
        "middle",
        INK_COLOR,
    ));
    append_bottom_color_legend(
        &mut svg,
        &bottom_plot,
        fig_style,
        &[("VO P95", VO_COLOR), ("BE P95", BE_COLOR)],
    );
    Some(Figure {
        slug: "p95_delay_priority_gap",
        title: "Voice Tail Invariance and Best-Effort Cost Under High Load",
        question:
            "Does crash VO keep a low P95 while BE absorbs the prioritization cost under contention?",
        body: svg,
        style: Some(fig_style),
    })
}

pub(crate) fn drop_rate_heatmap_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    heatmap_figure(
        matrix,
        style,
        "mac_drop_per_tx",
        "mac_drop_rate_by_strategy_load",
        "MAC Drop Rate Across Strategy and Load",
        "How quickly does contention translate into normalized packet loss as offered load grows?",
    )
}

pub(crate) fn vo_reception_heatmap_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    heatmap_figure(
        matrix,
        style,
        "vo_rx_per_tx",
        "vo_reception_by_strategy_load",
        "VO Reception Across Strategy and Load",
        "Which MAC strategy preserves crash-message reception as load increases?",
    )
}

pub(crate) fn drop_attribution_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    let rows: Vec<_> = STRATEGIES
        .iter()
        .filter_map(|strategy| {
            let summary = matrix.get(&(strategy.to_string(), "high".to_string()))?;
            let be = summary.metrics.mac_drop_be_count.unwrap_or(0.0);
            let vo = summary.metrics.mac_drop_vo_count.unwrap_or(0.0);
            let other = summary.metrics.mac_drop_unclassified_count.unwrap_or(0.0);
            ((be + vo + other) > 0.0).then_some((strategy_label(strategy), be, vo, other))
        })
        .collect();
    if rows.is_empty() {
        return None;
    }
    let max_value = rows
        .iter()
        .map(|(_, be, vo, other)| be + vo + other)
        .fold(0.0, f64::max);
    let plot = PlotArea::new(style);
    let mut svg = axis_frame(&plot, style, "Strategy", "MAC drops");
    draw_y_ticks(&mut svg, &plot, style, max_value);
    let group_width = plot.inner_w / rows.len() as f64;
    let bar_width = group_width * 0.42;
    for (index, (label, be, vo, other)) in rows.iter().enumerate() {
        let x = plot.left + group_width * (index as f64 + 0.5) - bar_width / 2.0;
        let mut base = 0.0;
        for (value, color) in [(*be, BE_COLOR), (*vo, VO_COLOR), (*other, "#9ca3af")] {
            let y0 = plot.scale_y(base, max_value);
            let y1 = plot.scale_y(base + value, max_value);
            svg.push_str(&rect(x, y1, bar_width, y0 - y1, color));
            base += value;
        }
        svg.push_str(&text(
            x + bar_width / 2.0,
            plot.bottom + category_offset(style),
            label,
            style.font_category,
            "middle",
            MUTED_COLOR,
        ));
    }
    let items = [("BE", BE_COLOR), ("VO", VO_COLOR), ("Other", "#9ca3af")];
    append_bottom_color_legend(&mut svg, &plot, style, &items);
    Some(Figure {
        slug: "mac_drop_attribution_high_load",
        title: "Packet-Drop Attribution Under High Load",
        question: "Are packet losses concentrated in BE traffic, VO traffic, or unclassified MAC behavior?",
        body: svg,
        style: None,
    })
}

pub(crate) fn vo_delay_cdf_figure(samples: &[VoDelaySample], style: FigureStyle) -> Option<Figure> {
    let mut by_strategy: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for sample in samples {
        if let Some((strategy, _, Overlay::Netload)) = parse_config(&sample.config) {
            by_strategy
                .entry(strategy)
                .or_default()
                .push(sample.value_ms);
        }
    }
    by_strategy.retain(|_, values| values.len() >= 2);
    if by_strategy.is_empty() {
        return None;
    }
    let fig_style = vo_delay_cdf_style(style);
    let max_x = VO_DELAY_CDF_X_MAX_MS;
    let max_y = 1.0;
    let plot = PlotArea::new_cdf(fig_style);
    let mut svg = axis_frame_cdf(&plot, fig_style, "VO delay (ms)", "CDF");
    draw_x_ticks_exact(
        &mut svg,
        &plot,
        fig_style,
        0.0,
        max_x,
        &[0.0, 1.0, 2.0, 3.0],
    );
    draw_y_ticks_exact(
        &mut svg,
        &plot,
        fig_style,
        0.0,
        max_y,
        &[0.0, 0.2, 0.4, 0.6, 0.8, 1.0],
    );
    let mut legend_items = Vec::new();
    for (index, (strategy, values)) in by_strategy.iter_mut().enumerate() {
        values.sort_by(f64::total_cmp);
        let decimated = decimate_sorted(values, CDF_MAX_POINTS);
        let color = COLORS[index % COLORS.len()];
        let mut points = String::new();
        for (rank, value) in decimated.iter().enumerate() {
            let x = plot.scale_x_exact(value.min(max_x), 0.0, max_x);
            let y = plot.scale_y_exact((rank + 1) as f64 / decimated.len() as f64, 0.0, max_y);
            points.push_str(&format!("{x:.2},{y:.2} "));
        }
        let dash = CDF_DASHES[index % CDF_DASHES.len()];
        let dash_attr = if dash.is_empty() {
            String::new()
        } else {
            format!(r#" stroke-dasharray="{dash}""#)
        };
        svg.push_str(&format!(
            r#"<polyline points="{}" fill="none" stroke="{color}" stroke-width="{:.2}"{dash_attr}/>"#,
            points.trim(),
            fig_style.line_stroke
        ));
        legend_items.push((strategy_label(strategy).to_string(), color.to_string()));
    }
    let legend_refs: Vec<_> = legend_items
        .iter()
        .map(|(label, color)| (label.as_str(), color.as_str()))
        .collect();
    append_cdf_bottom_line_legend(&mut svg, &plot, fig_style, &legend_refs);
    Some(Figure {
        slug: "vo_delay_cdf_high_load",
        title: "Crash VO Delay Distribution Under High Load",
        question:
            "Do prioritization strategies improve the full delay distribution, not only the mean?",
        body: svg,
        style: Some(fig_style),
    })
}

fn vo_delay_cdf_style(mut style: FigureStyle) -> FigureStyle {
    style.font_axis = style.font_axis.saturating_add(3);
    style.font_tick = style.font_tick.saturating_add(3);
    style.font_legend = style.font_legend.saturating_add(2);
    style.margin_bottom = 18.0;
    let footer = cdf_footer_band_height(style);
    style.height = (style.margin_top + 276.0 + footer + style.margin_bottom).round() as u32;
    style
}

pub(crate) fn control_actions_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    let mut has_any = false;
    for strategy in V2X_STRATEGIES {
        for workload in WORKLOADS {
            if let Some(summary) = matrix.get(&(strategy.to_string(), workload.to_string())) {
                let protection = summary.metrics.vo_protection_activation_count.unwrap_or(0.0);
                let suppressed = summary
                    .metrics
                    .be_grant_suppressed_while_blocked_count
                    .unwrap_or(0.0);
                let dropped = summary
                    .metrics
                    .be_dropped_while_blocked_count
                    .unwrap_or(0.0);
                if protection + suppressed + dropped > 0.0 {
                    has_any = true;
                }
            }
        }
    }
    if !has_any {
        return None;
    }

    let mut fig_style = style.tall_variant();
    fig_style.height = fig_style.height.saturating_add(120);
    let footer = footer_band_height(fig_style);
    let panel_gap = 18.0;
    let panel_area_bottom = fig_style.height as f64 - fig_style.margin_bottom - footer;
    let panel_area_top = fig_style.margin_top;
    let panel_h = (panel_area_bottom - panel_area_top - 2.0 * panel_gap) / 3.0;
    let make_panel = |index: usize| -> PlotArea {
        let top = panel_area_top + index as f64 * (panel_h + panel_gap);
        PlotArea {
            left: fig_style.margin_left,
            top,
            bottom: top + panel_h,
            inner_w: fig_style.width as f64 - fig_style.margin_left - fig_style.margin_right,
            inner_h: panel_h,
        }
    };
    let top_plot = make_panel(0);
    let mid_plot = make_panel(1);
    let bottom_plot = make_panel(2);

    let categories: Vec<&str> = WORKLOADS
        .iter()
        .map(|w| match *w {
            "low" => "Low",
            "medium" => "Med",
            "high" => "High",
            other => other,
        })
        .collect();

    let series_for = |metric: fn(&ConfigSummary) -> Option<f64>| -> Vec<(&str, &str, Vec<f64>)> {
        V2X_STRATEGIES
            .iter()
            .enumerate()
            .map(|(idx, strategy)| {
                let values: Vec<f64> = WORKLOADS
                    .iter()
                    .map(|workload| {
                        matrix
                            .get(&(strategy.to_string(), workload.to_string()))
                            .and_then(metric)
                            .unwrap_or(0.0)
                    })
                    .collect();
                (
                    strategy_label(strategy),
                    COLORS[idx % COLORS.len()],
                    values,
                )
            })
            .collect()
    };

    let protection_series = series_for(|s| s.metrics.vo_protection_activation_count);
    let suppressed_series = series_for(|s| s.metrics.be_grant_suppressed_while_blocked_count);
    let dropped_series = series_for(|s| s.metrics.be_dropped_while_blocked_count);

    let max_of = |series: &[(&str, &str, Vec<f64>)]| {
        series
            .iter()
            .flat_map(|(_, _, values)| values.iter().copied())
            .fold(0.0, f64::max)
    };

    let mut svg = String::new();
    let draw_panel = |svg: &mut String,
                      plot: &PlotArea,
                      y_label: &str,
                      series: &[(&str, &str, Vec<f64>)],
                      show_categories: bool| {
        let max_value = max_of(series).max(1.0);
        svg.push_str(&panel_axis_frame(plot, fig_style, y_label, true));
        draw_y_ticks(svg, plot, fig_style, max_value);
        let refs: Vec<(&str, &str, &[f64])> = series
            .iter()
            .map(|(label, color, values)| (*label, *color, values.as_slice()))
            .collect();
        draw_grouped_bar_panel(
            svg,
            plot,
            fig_style,
            &categories,
            &refs,
            max_value,
            show_categories,
        );
    };

    draw_panel(
        &mut svg,
        &top_plot,
        "VO protection activations",
        &protection_series,
        false,
    );
    draw_panel(
        &mut svg,
        &mid_plot,
        "BE grants suppressed",
        &suppressed_series,
        false,
    );
    draw_panel(
        &mut svg,
        &bottom_plot,
        "BE dropped while blocked",
        &dropped_series,
        true,
    );
    svg.push_str(&text(
        bottom_plot.left + bottom_plot.inner_w / 2.0,
        bottom_plot.bottom + axis_label_offset(fig_style),
        "Offered load",
        fig_style.font_axis,
        "middle",
        INK_COLOR,
    ));
    let legend: Vec<(&str, &str)> = V2X_STRATEGIES
        .iter()
        .enumerate()
        .map(|(idx, strategy)| (strategy_label(strategy), COLORS[idx % COLORS.len()]))
        .collect();
    append_bottom_color_legend(&mut svg, &bottom_plot, fig_style, &legend);
    Some(Figure {
        slug: "v2x_control_actions_by_load",
        title: "Adaptive V2X Control Actions Across Load",
        question:
            "How do VO protection activations and BE blocking counters scale from low to high load?",
        body: svg,
        style: Some(fig_style),
    })
}
fn heatmap_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
    metric_key: &str,
    slug: &'static str,
    title: &'static str,
    question: &'static str,
) -> Option<Figure> {
    let mut values = Vec::new();
    for strategy in STRATEGIES {
        for workload in WORKLOADS {
            if let Some(summary) = matrix.get(&(strategy.to_string(), workload.to_string())) {
                if let Some(value) = metric(summary, metric_key) {
                    values.push((strategy_label(strategy), *workload, value));
                }
            }
        }
    }
    if values.is_empty() {
        return None;
    }
    let max_value = values
        .iter()
        .map(|(_, _, value)| *value)
        .fold(0.0, f64::max);
    let plot = heatmap_plot_area(style);
    let cell_w = plot.inner_w / WORKLOADS.len() as f64;
    let cell_h = plot.inner_h / STRATEGIES.len() as f64;
    let mut svg = String::new();
    for (row, strategy) in STRATEGIES.iter().enumerate() {
        let y = plot.top + row as f64 * cell_h;
        svg.push_str(&text(
            plot.left - heatmap_row_label_gap(style),
            y + cell_h / 2.0 + 3.0,
            strategy_label(strategy),
            style.font_category,
            "end",
            MUTED_COLOR,
        ));
        for (col, workload) in WORKLOADS.iter().enumerate() {
            let x = plot.left + col as f64 * cell_w;
            let value = matrix
                .get(&(strategy.to_string(), workload.to_string()))
                .and_then(|summary| metric(summary, metric_key));
            let color = value
                .map(|value| heat_color(value, max_value))
                .unwrap_or_else(|| "#f3f4f6".to_string());
            svg.push_str(&rect_fill(x + 1.0, y + 1.0, cell_w - 2.0, cell_h - 2.0, &color));
            svg.push_str(&text(
                x + cell_w / 2.0,
                y + cell_h / 2.0 + 3.0,
                &value
                    .map(format_metric)
                    .unwrap_or_else(|| "—".to_string()),
                style.font_heatmap,
                "middle",
                INK_COLOR,
            ));
        }
    }
    for (col, workload) in WORKLOADS.iter().enumerate() {
        let x = plot.left + col as f64 * cell_w + cell_w / 2.0;
        svg.push_str(&text(
            x,
            plot.bottom + category_offset(style),
            &workload_label(workload),
            style.font_category,
            "middle",
            MUTED_COLOR,
        ));
    }
    svg.push_str(&text(
        plot.left + plot.inner_w / 2.0,
        plot.bottom + axis_label_offset(style),
        "Load",
        style.font_axis,
        "middle",
        INK_COLOR,
    ));
    let y_axis_x = plot.left - heatmap_y_axis_label_offset(style);
    svg.push_str(&format!(
        r##"<text x="{y_axis_x:.2}" y="{:.2}" text-anchor="middle" font-size="{}" fill="{INK_COLOR}" transform="rotate(-90 {y_axis_x:.2} {:.2})">Strategy</text>"##,
        plot.top + plot.inner_h / 2.0,
        style.font_axis,
        plot.top + plot.inner_h / 2.0,
    ));
    Some(Figure {
        slug,
        title,
        question,
        body: svg,
        style: None,
    })
}

fn metric(summary: &ConfigSummary, key: &str) -> Option<f64> {
    match key {
        "mac_drop_per_tx" => summary.metrics.mac_drop_per_tx,
        "vo_rx_per_tx" => summary.metrics.vo_rx_per_tx,
        _ => None,
    }
}

