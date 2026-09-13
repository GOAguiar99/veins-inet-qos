//! SVG plot helpers, export writers, and shared drawing utilities.

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use super::style::{
    Figure, FigureStyle, CDF_DASHES, CDF_LEGEND_HORIZONTAL_GAP, FONT_SERIF, GRID_COLOR, INK_COLOR,
    MIN_BAR_HEIGHT, MUTED_COLOR,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlotArea {
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) bottom: f64,
    pub(crate) inner_w: f64,
    pub(crate) inner_h: f64,
}

impl PlotArea {
    pub(crate) fn new(style: FigureStyle) -> Self {
        let footer = footer_band_height(style);
        Self::from_margins(style, footer)
    }

    pub(crate) fn new_cdf(style: FigureStyle) -> Self {
        Self::from_margins(style, cdf_footer_band_height(style))
    }

    fn from_margins(style: FigureStyle, footer: f64) -> Self {
        Self {
            left: style.margin_left,
            top: style.margin_top,
            bottom: style.height as f64 - style.margin_bottom - footer,
            inner_w: style.width as f64 - style.margin_left - style.margin_right,
            inner_h: style.height as f64 - style.margin_top - style.margin_bottom - footer,
        }
    }

    pub(crate) fn scale_y(&self, value: f64, max_value: f64) -> f64 {
        self.bottom - self.inner_h * value / padded_max(max_value)
    }

    pub(crate) fn scale_x_range(&self, value: f64, min_value: f64, max_value: f64) -> f64 {
        let span = padded_range(min_value, max_value);
        self.left + self.inner_w * (value - min_value) / span
    }

    pub(crate) fn scale_y_range(&self, value: f64, min_value: f64, max_value: f64) -> f64 {
        let span = padded_range(min_value, max_value);
        self.bottom - self.inner_h * (value - min_value) / span
    }

    pub(crate) fn scale_x_exact(&self, value: f64, min_value: f64, max_value: f64) -> f64 {
        let span = (max_value - min_value).max(1e-9);
        self.left + self.inner_w * (value - min_value) / span
    }

    pub(crate) fn scale_y_exact(&self, value: f64, min_value: f64, max_value: f64) -> f64 {
        let span = (max_value - min_value).max(1e-9);
        self.bottom - self.inner_h * (value - min_value) / span
    }
}

pub fn write_svg(path: &Path, style: FigureStyle, figure: &Figure) -> Result<()> {
    let width = style.width;
    let height = style.height;
    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    writeln!(
        file,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    )?;
    writeln!(file, r##"<rect width="100%" height="100%" fill="#ffffff"/>"##)?;
    writeln!(
        file,
        r#"<style>text {{ font-family: {FONT_SERIF}; }}</style>"#
    )?;
    if style.show_header {
        writeln!(
            file,
            "{}",
            text(8.0, 18.0, figure.title, 11, "start", INK_COLOR)
        )?;
        writeln!(
            file,
            "{}",
            text(8.0, 32.0, figure.question, 9, "start", MUTED_COLOR)
        )?;
    }
    writeln!(file, "{}", figure.body)?;
    writeln!(file, "</svg>")?;
    Ok(())
}

pub(crate) fn axis_label_offset_cdf(style: FigureStyle) -> f64 {
    tick_label_offset(style) + f64::from(style.font_axis) + 8.0
}

pub(crate) fn axis_frame_cdf(plot: &PlotArea, style: FigureStyle, x_label: &str, y_label: &str) -> String {
    let mut svg = String::new();
    let stroke = style.axis_stroke;
    svg.push_str(&format!(
        r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{INK_COLOR}" stroke-width="{stroke}"/>"##,
        plot.left,
        plot.bottom,
        plot.left + plot.inner_w,
        plot.bottom
    ));
    svg.push_str(&format!(
        r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{INK_COLOR}" stroke-width="{stroke}"/>"##,
        plot.left, plot.top, plot.left, plot.bottom
    ));
    svg.push_str(&text(
        plot.left + plot.inner_w / 2.0,
        plot.bottom + axis_label_offset_cdf(style),
        x_label,
        style.font_axis,
        "middle",
        INK_COLOR,
    ));
    let y_axis_x = plot.left - y_axis_label_offset(style);
    svg.push_str(&format!(
        r##"<text x="{y_axis_x:.2}" y="{:.2}" text-anchor="middle" font-size="{}" fill="{INK_COLOR}" transform="rotate(-90 {y_axis_x:.2} {:.2})">{}</text>"##,
        plot.top + plot.inner_h / 2.0,
        style.font_axis,
        plot.top + plot.inner_h / 2.0,
        escape(y_label)
    ));
    svg
}

pub(crate) fn axis_frame(plot: &PlotArea, style: FigureStyle, x_label: &str, y_label: &str) -> String {
    let mut svg = String::new();
    let stroke = style.axis_stroke;
    svg.push_str(&format!(
        r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{INK_COLOR}" stroke-width="{stroke}"/>"##,
        plot.left,
        plot.bottom,
        plot.left + plot.inner_w,
        plot.bottom
    ));
    svg.push_str(&format!(
        r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{INK_COLOR}" stroke-width="{stroke}"/>"##,
        plot.left, plot.top, plot.left, plot.bottom
    ));
    svg.push_str(&text(
        plot.left + plot.inner_w / 2.0,
        plot.bottom + axis_label_offset(style),
        x_label,
        style.font_axis,
        "middle",
        INK_COLOR,
    ));
    let y_axis_x = plot.left - y_axis_label_offset(style);
    svg.push_str(&format!(
        r##"<text x="{y_axis_x:.2}" y="{:.2}" text-anchor="middle" font-size="{}" fill="{INK_COLOR}" transform="rotate(-90 {y_axis_x:.2} {:.2})">{}</text>"##,
        plot.top + plot.inner_h / 2.0,
        style.font_axis,
        plot.top + plot.inner_h / 2.0,
        escape(y_label)
    ));
    svg
}

pub(crate) fn draw_y_ticks(svg: &mut String, plot: &PlotArea, style: FigureStyle, max_value: f64) {
    draw_y_ticks_ranged(svg, plot, style, 0.0, max_value);
}

pub(crate) fn draw_y_ticks_ranged(
    svg: &mut String,
    plot: &PlotArea,
    style: FigureStyle,
    min_value: f64,
    max_value: f64,
) {
    let ticks = tick_values_ranged(min_value, max_value, style.tick_count);
    for (index, value) in ticks.iter().enumerate() {
        let y = plot.scale_y_range(*value, min_value, max_value);
        if index > 0 && index < ticks.len() - 1 {
            svg.push_str(&format!(
                r##"<line x1="{:.2}" y1="{y:.2}" x2="{:.2}" y2="{y:.2}" stroke="{GRID_COLOR}" stroke-width="0.6"/>"##,
                plot.left,
                plot.left + plot.inner_w
            ));
        }
        svg.push_str(&text(
            plot.left - tick_label_pad(style),
            y + 3.0,
            &format_tick(*value, max_value - min_value),
            style.font_tick,
            "end",
            MUTED_COLOR,
        ));
    }
}

pub(crate) fn draw_x_ticks_ranged(
    svg: &mut String,
    plot: &PlotArea,
    style: FigureStyle,
    min_value: f64,
    max_value: f64,
) {
    let ticks = tick_values_ranged(min_value, max_value, style.tick_count);
    for (index, value) in ticks.iter().enumerate() {
        let x = plot.scale_x_range(*value, min_value, max_value);
        if index > 0 && index < ticks.len() - 1 {
            svg.push_str(&format!(
                r##"<line x1="{x:.2}" y1="{:.2}" x2="{x:.2}" y2="{:.2}" stroke="{GRID_COLOR}" stroke-width="0.6"/>"##,
                plot.top,
                plot.bottom
            ));
        }
        svg.push_str(&text(
            x,
            plot.bottom + tick_label_offset(style),
            &format_tick(*value, max_value - min_value),
            style.font_tick,
            "middle",
            MUTED_COLOR,
        ));
    }
}

pub(crate) fn draw_x_ticks_exact(
    svg: &mut String,
    plot: &PlotArea,
    style: FigureStyle,
    min_value: f64,
    max_value: f64,
    ticks: &[f64],
) {
    for (index, value) in ticks.iter().enumerate() {
        let x = plot.scale_x_exact(*value, min_value, max_value);
        if index > 0 && index < ticks.len() - 1 {
            svg.push_str(&format!(
                r##"<line x1="{x:.2}" y1="{:.2}" x2="{x:.2}" y2="{:.2}" stroke="{GRID_COLOR}" stroke-width="0.6"/>"##,
                plot.top,
                plot.bottom
            ));
        }
        svg.push_str(&text(
            x,
            plot.bottom + tick_label_offset(style),
            &format_round_tick(*value),
            style.font_tick,
            "middle",
            MUTED_COLOR,
        ));
    }
}

pub(crate) fn draw_y_ticks_exact(
    svg: &mut String,
    plot: &PlotArea,
    style: FigureStyle,
    min_value: f64,
    max_value: f64,
    ticks: &[f64],
) {
    for (index, value) in ticks.iter().enumerate() {
        let y = plot.scale_y_exact(*value, min_value, max_value);
        if index > 0 && index < ticks.len() - 1 {
            svg.push_str(&format!(
                r##"<line x1="{:.2}" y1="{y:.2}" x2="{:.2}" y2="{y:.2}" stroke="{GRID_COLOR}" stroke-width="0.6"/>"##,
                plot.left,
                plot.left + plot.inner_w
            ));
        }
        svg.push_str(&text(
            plot.left - tick_label_pad(style),
            y + 3.0,
            &format_round_tick(*value),
            style.font_tick,
            "end",
            MUTED_COLOR,
        ));
    }
}

pub(crate) fn category_offset(style: FigureStyle) -> f64 {
    if style.show_header {
        28.0
    } else {
        f64::from(style.font_category) + 8.0
    }
}

pub(crate) fn axis_label_offset(style: FigureStyle) -> f64 {
    if style.show_header {
        68.0
    } else {
        category_offset(style) + f64::from(style.font_axis) + 6.0
    }
}

/// Space below the plot reserved for category labels, axis title, and legend.
pub(crate) fn footer_band_height(style: FigureStyle) -> f64 {
    if style.show_header {
        0.0
    } else {
        category_offset(style) + f64::from(style.font_axis) + 16.0 + legend_box_size(style) * 2.0 + 14.0
    }
}

/// Footer for line plots without category labels (e.g. VO delay CDF).
pub(crate) fn cdf_footer_band_height(style: FigureStyle) -> f64 {
    tick_label_offset(style) + f64::from(style.font_axis) + 12.0 + legend_box_size(style) + 10.0
}

pub(crate) fn footer_legend_y(style: FigureStyle) -> f64 {
    style.height as f64 - style.margin_bottom - legend_box_size(style) - 6.0
}

pub(crate) fn y_axis_label_offset(style: FigureStyle) -> f64 {
    if style.show_header {
        92.0
    } else if style.font_axis >= 14 {
        tick_label_pad(style) + f64::from(style.font_axis) + 14.0
    } else {
        tick_label_pad(style) + f64::from(style.font_axis) + 6.0
    }
}

pub(crate) fn heatmap_y_axis_label_offset(style: FigureStyle) -> f64 {
    if style.show_header {
        110.0
    } else {
        heatmap_row_label_gap(style) + f64::from(style.font_axis) + 4.0
    }
}

pub(crate) fn heatmap_row_label_gap(style: FigureStyle) -> f64 {
    if style.show_header {
        10.0
    } else if style.font_category >= 13 {
        14.0
    } else {
        8.0
    }
}

pub(crate) fn tick_label_offset(style: FigureStyle) -> f64 {
    if style.show_header {
        24.0
    } else {
        f64::from(style.font_tick) + 4.0
    }
}

pub(crate) fn tick_label_pad(style: FigureStyle) -> f64 {
    if style.show_header {
        8.0
    } else if style.font_tick >= 13 {
        10.0
    } else {
        6.0
    }
}

fn tick_values_ranged(min_value: f64, max_value: f64, tick_count: u32) -> Vec<f64> {
    let span = padded_range(min_value, max_value);
    (0..=tick_count)
        .map(|index| min_value + span * index as f64 / tick_count as f64)
        .collect()
}



pub(crate) fn legend_box_size(style: FigureStyle) -> f64 {
    if style.show_header {
        14.0
    } else {
        f64::from(style.font_legend) + 1.0
    }
}

pub(crate) fn legend_symbol_width(label: &str, style: FigureStyle) -> f64 {
    legend_box_size(style) + 3.0 + label.len() as f64 * f64::from(style.font_legend) * 0.52
}

pub(crate) fn legend_horizontal_width(items: &[(&str, &str)], style: FigureStyle) -> f64 {
    legend_horizontal_width_spaced(items, style, legend_horizontal_gap(style))
}

pub(crate) fn legend_horizontal_width_spaced(items: &[(&str, &str)], style: FigureStyle, gap: f64) -> f64 {
    let mut width = 0.0;
    for (index, (label, _)) in items.iter().enumerate() {
        if index > 0 {
            width += gap;
        }
        width += legend_symbol_width(label, style);
    }
    width
}

pub(crate) fn legend_horizontal_gap(style: FigureStyle) -> f64 {
    if style.show_header { 18.0 } else { 10.0 }
}

pub(crate) fn legend_top_right(plot: &PlotArea, items: &[(&str, &str)], style: FigureStyle) -> (f64, f64) {
    let pad = if style.show_header { 12.0 } else { 6.0 };
    let w = legend_horizontal_width(items, style);
    (plot.left + plot.inner_w - w - pad, plot.top + pad)
}

pub(crate) fn heatmap_plot_area(style: FigureStyle) -> PlotArea {
    let extra_left = if style.show_header { 0.0 } else { 52.0 };
    let footer = footer_band_height(style);
    PlotArea {
        left: style.margin_left + extra_left,
        top: style.margin_top,
        bottom: style.height as f64 - style.margin_bottom - footer,
        inner_w: style.width as f64
            - style.margin_left
            - style.margin_right
            - extra_left,
        inner_h: style.height as f64 - style.margin_top - style.margin_bottom - footer,
    }
}


fn grouped_bar_width_factor(category_count: usize, series_count: usize) -> f64 {
    let base = match category_count {
        0..=3 => 0.34,
        4..=5 => 0.30,
        _ => 0.26,
    };
    if series_count <= 1 {
        base * 1.35
    } else {
        base
    }
}

pub(crate) fn draw_grouped_bar_panel(
    svg: &mut String,
    plot: &PlotArea,
    style: FigureStyle,
    categories: &[&str],
    series: &[(&str, &str, &[f64])],
    max_value: f64,
    show_category_labels: bool,
) {
    if categories.is_empty() || series.is_empty() {
        return;
    }
    let group_width = plot.inner_w / categories.len() as f64;
    let bar_width = group_width * grouped_bar_width_factor(categories.len(), series.len());
    let offsets: Vec<f64> = if series.len() == 1 {
        vec![0.0]
    } else {
        let spread = bar_width * 0.65;
        series
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let center = (series.len() - 1) as f64 / 2.0;
                (index as f64 - center) * spread
            })
            .collect()
    };
    for (series_index, (_label, color, values)) in series.iter().enumerate() {
        for (category_index, value) in values.iter().enumerate().take(categories.len()) {
            let center = plot.left + group_width * (category_index as f64 + 0.5) + offsets[series_index];
            draw_bar(
                svg,
                plot,
                center,
                bar_width,
                *value,
                max_value,
                color,
            );
        }
    }
    if show_category_labels {
        for (index, label) in categories.iter().enumerate() {
            let center = plot.left + group_width * (index as f64 + 0.5);
            svg.push_str(&text(
                center,
                plot.bottom + category_offset(style),
                label,
                style.font_category,
                "middle",
                MUTED_COLOR,
            ));
        }
    }
}

pub(crate) fn panel_axis_frame(
    plot: &PlotArea,
    style: FigureStyle,
    y_label: &str,
    draw_bottom_axis: bool,
) -> String {
    let mut svg = String::new();
    let stroke = style.axis_stroke;
    if draw_bottom_axis {
        svg.push_str(&format!(
            r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{INK_COLOR}" stroke-width="{stroke}"/>"##,
            plot.left,
            plot.bottom,
            plot.left + plot.inner_w,
            plot.bottom
        ));
    }
    svg.push_str(&format!(
        r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{INK_COLOR}" stroke-width="{stroke}"/>"##,
        plot.left, plot.top, plot.left, plot.bottom
    ));
    let y_axis_x = plot.left - y_axis_label_offset(style);
    svg.push_str(&format!(
        r##"<text x="{y_axis_x:.2}" y="{:.2}" text-anchor="middle" font-size="{}" fill="{INK_COLOR}" transform="rotate(-90 {y_axis_x:.2} {:.2})">{}</text>"##,
        plot.top + plot.inner_h / 2.0,
        style.font_axis,
        plot.top + plot.inner_h / 2.0,
        escape(y_label)
    ));
    svg
}

pub(crate) fn append_bottom_color_legend(
    svg: &mut String,
    plot: &PlotArea,
    style: FigureStyle,
    items: &[(&str, &str)],
) {
    if style.show_header {
        let (lx, ly) = legend_top_right(plot, items, style);
        svg.push_str(&legend_horizontal(lx, ly, items, style));
        return;
    }
    let w = legend_horizontal_width(items, style);
    let x = plot.left + (plot.inner_w - w) / 2.0;
    let y = footer_legend_y(style);
    svg.push_str(&legend_horizontal(x, y, items, style));
}

pub(crate) fn append_cdf_bottom_line_legend(
    svg: &mut String,
    plot: &PlotArea,
    style: FigureStyle,
    items: &[(&str, &str)],
) {
    let w = legend_horizontal_width_spaced(items, style, CDF_LEGEND_HORIZONTAL_GAP);
    let x = plot.left + (plot.inner_w - w) / 2.0;
    let y = footer_legend_y(style);
    svg.push_str(&legend_lines_horizontal_spaced(
        x,
        y,
        items,
        style,
        CDF_LEGEND_HORIZONTAL_GAP,
    ));
}

pub(crate) fn legend_lines_horizontal_spaced(
    x: f64,
    y: f64,
    items: &[(&str, &str)],
    style: FigureStyle,
    gap: f64,
) -> String {
    let mut svg = String::new();
    let box_size = legend_box_size(style);
    let line_len = box_size * 1.4;
    let mut cursor = x;
    let row_y = y + box_size * 0.5;
    for (index, (label, color)) in items.iter().enumerate() {
        if index > 0 {
            cursor += gap;
        }
        let dash = CDF_DASHES[index % CDF_DASHES.len()];
        let dash_attr = if dash.is_empty() {
            String::new()
        } else {
            format!(r#" stroke-dasharray="{dash}""#)
        };
        svg.push_str(&format!(
            r#"<line x1="{cursor:.2}" y1="{row_y:.2}" x2="{:.2}" y2="{row_y:.2}" stroke="{color}" stroke-width="{:.2}"{dash_attr}/>"#,
            cursor + line_len,
            style.line_stroke
        ));
        svg.push_str(&text(
            cursor + line_len + 4.0,
            row_y + 3.0,
            label,
            style.font_legend,
            "start",
            INK_COLOR,
        ));
        cursor += legend_symbol_width(label, style);
    }
    svg
}

pub(crate) fn decimate_sorted(values: &[f64], max_points: usize) -> Vec<f64> {
    if values.len() <= max_points {
        return values.to_vec();
    }
    let step = (values.len() - 1) as f64 / (max_points - 1) as f64;
    (0..max_points)
        .map(|index| {
            let source_index = (index as f64 * step).round() as usize;
            values[source_index.min(values.len() - 1)]
        })
        .collect()
}

pub(crate) fn percentile(values: &[f64], ratio: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * ratio.clamp(0.0, 1.0)).round() as usize;
    sorted[index]
}

pub(crate) fn scatter_axis_bounds(delays: &[f64], jitters: &[f64]) -> (f64, f64, f64, f64) {
    let max_x = delays.iter().copied().fold(0.0, f64::max);
    let max_y = jitters.iter().copied().fold(0.0, f64::max);
    if delays.is_empty() {
        return (0.0, max_x.max(1.0), 0.0, max_y.max(1.0));
    }
    let threshold_x = max_x * 0.1;
    let threshold_y = max_y * 0.1;
    let clustered = delays
        .iter()
        .zip(jitters.iter())
        .filter(|(delay, jitter)| **delay <= threshold_x && **jitter <= threshold_y)
        .count();
    let clustered_ratio = clustered as f64 / delays.len() as f64;
    if clustered_ratio > 0.8 && max_x > 0.0 && max_y > 0.0 {
        let min_x = (percentile(delays, 0.05) * 0.85).max(0.0);
        let min_y = (percentile(jitters, 0.05) * 0.85).max(0.0);
        (min_x, max_x, min_y, max_y)
    } else {
        (0.0, max_x, 0.0, max_y)
    }
}



pub(crate) fn workload_label(workload: &str) -> String {
    match workload {
        "low" => "Low".to_string(),
        "medium" => "Med".to_string(),
        "high" => "High".to_string(),
        other => other.to_string(),
    }
}





pub(crate) fn draw_bar(
    svg: &mut String,
    plot: &PlotArea,
    center: f64,
    width: f64,
    value: f64,
    max_value: f64,
    color: &str,
) {
    let mut y = plot.scale_y(value, max_value);
    let mut height = plot.bottom - y;
    if value > 0.0 && height < MIN_BAR_HEIGHT {
        height = MIN_BAR_HEIGHT;
        y = plot.bottom - height;
    }
    svg.push_str(&rect(
        center - width / 2.0,
        y,
        width,
        height,
        color,
    ));
}

pub(crate) fn rect_fill(x: f64, y: f64, width: f64, height: f64, color: &str) -> String {
    format!(
        r#"<rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{height:.2}" fill="{color}"/>"#
    )
}

pub(crate) fn rect(x: f64, y: f64, width: f64, height: f64, color: &str) -> String {
    format!(
        r#"<rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{height:.2}" fill="{color}" stroke="{INK_COLOR}" stroke-width="0.3" stroke-opacity="0.4"/>"#
    )
}

pub(crate) fn text(x: f64, y: f64, value: &str, size: u32, anchor: &str, color: &str) -> String {
    let lines: Vec<&str> = value.split('\n').collect();
    if lines.len() == 1 {
        return format!(
            r#"<text x="{x:.2}" y="{y:.2}" text-anchor="{anchor}" font-size="{size}" fill="{color}">{}</text>"#,
            escape(value)
        );
    }
    let mut output = format!(
        r#"<text x="{x:.2}" y="{y:.2}" text-anchor="{anchor}" font-size="{size}" fill="{color}">"#
    );
    for (index, line) in lines.iter().enumerate() {
        let dy = if index == 0 { 0 } else { size + 2 };
        output.push_str(&format!(
            r#"<tspan x="{x:.2}" dy="{dy}">{}</tspan>"#,
            escape(line)
        ));
    }
    output.push_str("</text>");
    output
}

pub(crate) fn legend_horizontal(x: f64, y: f64, items: &[(&str, &str)], style: FigureStyle) -> String {
    let mut svg = String::new();
    let box_size = legend_box_size(style);
    let mut cursor = x;
    let row_y = y + box_size;
    for (index, (label, color)) in items.iter().enumerate() {
        if index > 0 {
            cursor += legend_horizontal_gap(style);
        }
        svg.push_str(&rect(
            cursor,
            row_y - box_size,
            box_size,
            box_size,
            color,
        ));
        svg.push_str(&text(
            cursor + box_size + 3.0,
            row_y - 1.0,
            label,
            style.font_legend,
            "start",
            INK_COLOR,
        ));
        cursor += legend_symbol_width(label, style);
    }
    svg
}

pub(crate) fn heat_color(value: f64, max_value: f64) -> String {
    let ratio = if max_value <= 0.0 {
        0.0
    } else {
        (value / max_value).clamp(0.0, 1.0)
    };
    let r = (239.0 - 128.0 * ratio).round() as u8;
    let g = (246.0 - 118.0 * ratio).round() as u8;
    let b = (255.0 - 120.0 * ratio).round() as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

pub(crate) fn padded_max(value: f64) -> f64 {
    if value <= 0.0 || !value.is_finite() {
        1.0
    } else {
        value * 1.08
    }
}

pub(crate) fn padded_range(min_value: f64, max_value: f64) -> f64 {
    let span = max_value - min_value;
    if span <= 0.0 || !span.is_finite() {
        1.0
    } else {
        span * 1.08
    }
}

pub(crate) fn format_metric(value: f64) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else if value.abs() >= 1.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.2}")
    }
}

pub(crate) fn format_count(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if abs >= 1000.0 {
        format!("{:.0}k", value / 1000.0)
    } else if abs >= 1.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

pub(crate) fn format_round_tick(value: f64) -> String {
    if value.fract().abs() < 1e-6 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

pub(crate) fn format_tick(value: f64, span: f64) -> String {
    let abs = value.abs();
    if span <= 0.5 {
        // Small signed deltas (e.g. netload Δ VO P95): need hundredths.
        if abs < 1e-9 {
            "0".to_string()
        } else {
            format!("{value:.2}")
        }
    } else if span <= 1.05 {
        format!("{value:.2}")
    } else if span >= 1000.0 {
        format_count(value)
    } else {
        format_metric(value)
    }
}

pub(crate) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn convert_svg(svg_path: &Path, output_path: &Path, format: &str, dpi: u32) {
    let converted = if command_exists("rsvg-convert") {
        let rsvg_format = if format == "pdf" { "pdf1.4" } else { format };
        Command::new("rsvg-convert")
            .args([
                "-f",
                rsvg_format,
                "-d",
                &dpi.to_string(),
                "-p",
                &dpi.to_string(),
                "-o",
            ])
            .arg(output_path)
            .arg(svg_path)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    } else if command_exists("inkscape") {
        let export_type = format!("--export-type={format}");
        let export_dpi = format!("--export-dpi={dpi}");
        let export_file = format!("--export-filename={}", output_path.display());
        Command::new("inkscape")
            .arg(svg_path)
            .args([export_type, export_dpi, export_file])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    } else {
        false
    };
    if !converted {
        eprintln!(
            "warning: skipped {} export for {}; install rsvg-convert or inkscape",
            format,
            svg_path.display()
        );
    }
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy)]
pub struct ExportFormats {
    pub png: bool,
    pub pdf: bool,
}

impl ExportFormats {
    pub fn parse(raw: &str) -> Self {
        let mut formats = Self {
            png: false,
            pdf: false,
        };
        for format in raw.split(',').map(str::trim) {
            match format {
                "png" => formats.png = true,
                "pdf" => formats.pdf = true,
                "svg" | "" => {}
                other => eprintln!("warning: unsupported export format ignored: {other}"),
            }
        }
        formats
    }
}


#[cfg(test)]
mod tests {
    use super::{decimate_sorted, format_count};

    #[test]
    fn format_count_uses_k_suffix_for_large_values() {
        assert_eq!(format_count(523_459.0), "523k");
        assert_eq!(format_count(120_643.0), "121k");
    }

    #[test]
    fn decimate_sorted_preserves_endpoints() {
        let values: Vec<f64> = (0..1000).map(|value| value as f64).collect();
        let decimated = decimate_sorted(&values, 150);
        assert_eq!(decimated.len(), 150);
        assert_eq!(decimated.first().copied(), Some(0.0));
        assert_eq!(decimated.last().copied(), Some(999.0));
    }
}
