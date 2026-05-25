use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use clap::Parser;
use kpi_dashboard::{default_results_dir, rebuild_raw_dataset, ConfigSummary, DashboardDataset};

const STRATEGIES: &[&str] = &["plain", "edca_only", "stable", "guarded", "emergency"];
const WORKLOADS: &[&str] = &["low", "medium", "high"];
const COLORS: &[&str] = &["#4c78a8", "#f58518", "#54a24b", "#b279a2", "#e45756"];
const BE_COLOR: &str = "#4c78a8";
const VO_COLOR: &str = "#e45756";
const INK_COLOR: &str = "#111827";
const MUTED_COLOR: &str = "#6b7280";
const GRID_COLOR: &str = "#e5e7eb";
const FONT_SERIF: &str = r#""Times New Roman", Times, serif"#;
/// Dash patterns for CDF curves (grayscale-friendly when printed).
const CDF_DASHES: &[&str] = &["", "7,3", "3,3", "9,4,2,4", "2,2"];
const DEFAULT_IEEE_WIDTH: u32 = 252;
const DEFAULT_IEEE_HEIGHT: u32 = 216;

/// Layout tuned for `\includegraphics[width=\linewidth]` in IEEE two-column figures
/// (252 px wide ≈ 3.5 in at 72 pt/in; fonts are sized to stay legible at column width).
#[derive(Debug, Clone, Copy)]
struct FigureStyle {
    width: u32,
    height: u32,
    margin_left: f64,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    font_axis: u32,
    font_tick: u32,
    font_category: u32,
    font_legend: u32,
    font_heatmap: u32,
    show_header: bool,
    axis_stroke: f64,
    line_stroke: f64,
    tick_count: u32,
}

#[derive(Debug, Clone, Copy)]
enum LegendCorner {
    TopRight,
    BottomRight,
}

impl FigureStyle {
    fn ieee_column() -> Self {
        Self {
            width: DEFAULT_IEEE_WIDTH,
            height: DEFAULT_IEEE_HEIGHT,
            margin_left: 48.0,
            margin_top: 10.0,
            margin_right: 10.0,
            margin_bottom: 38.0,
            font_axis: 11,
            font_tick: 10,
            font_category: 10,
            font_legend: 9,
            font_heatmap: 10,
            show_header: false,
            axis_stroke: 1.15,
            line_stroke: 2.0,
            tick_count: 4,
        }
    }

    fn screen() -> Self {
        Self {
            width: 1400,
            height: 900,
            margin_left: 150.0,
            margin_top: 145.0,
            margin_right: 60.0,
            margin_bottom: 100.0,
            font_axis: 16,
            font_tick: 13,
            font_category: 14,
            font_legend: 13,
            font_heatmap: 14,
            show_header: true,
            axis_stroke: 1.4,
            line_stroke: 2.4,
            tick_count: 5,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Export publication-oriented figures from Veins QoS KPI results"
)]
struct Cli {
    #[arg(long)]
    results: Vec<PathBuf>,

    #[arg(long, default_value = "publication_figures")]
    output: PathBuf,

    #[arg(long, default_value = "svg,pdf")]
    formats: String,

    #[arg(long, default_value_t = 300)]
    dpi: u32,

    /// IEEE single-column layout (3.5 in wide, tight margins, no in-figure titles).
    #[arg(long = "ieee", action = clap::ArgAction::SetTrue, default_value_t = true)]
    #[arg(long = "no-ieee", action = clap::ArgAction::SetFalse)]
    ieee: bool,

    #[arg(long, default_value_t = DEFAULT_IEEE_WIDTH)]
    width: u32,

    #[arg(long, default_value_t = DEFAULT_IEEE_HEIGHT)]
    height: u32,

    #[arg(long)]
    threads: Option<usize>,
}

#[derive(Debug, Clone)]
struct Figure {
    slug: &'static str,
    title: &'static str,
    question: &'static str,
    body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AccessCategory {
    Be,
    Vo,
}

#[derive(Debug, Clone)]
struct DelaySample {
    config: String,
    ac: AccessCategory,
    value_ms: f64,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let results_dirs = if args.results.is_empty() {
        vec![default_results_dir()]
    } else {
        args.results.clone()
    };
    let formats = ExportFormats::parse(&args.formats);
    let style = if args.ieee {
        let mut ieee = FigureStyle::ieee_column();
        ieee.width = args.width;
        ieee.height = args.height;
        ieee
    } else {
        let mut screen = FigureStyle::screen();
        screen.width = args.width;
        screen.height = args.height;
        screen
    };
    fs::create_dir_all(&args.output)
        .with_context(|| format!("failed to create {}", args.output.display()))?;

    for results_dir in results_dirs {
        let density = density_label(&results_dir);
        let dataset = rebuild_raw_dataset(&results_dir, args.threads).with_context(|| {
            format!("failed to rebuild KPI data from {}", results_dir.display())
        })?;
        let samples = load_delay_samples(&results_dir).unwrap_or_else(|error| {
            eprintln!(
                "warning: could not load delay samples from {}: {error:#}",
                results_dir.display()
            );
            Vec::new()
        });
        let figures = build_figures(&dataset, &samples, style);

        for figure in figures {
            let Some(order) = figure_order(figure.slug) else {
                eprintln!("warning: unknown figure slug {}, skipping", figure.slug);
                continue;
            };
            let base_name = format!("fig_{:02}_{}_{}", order, figure.slug, density);
            let svg_path = args.output.join(format!("{base_name}.svg"));
            write_svg(&svg_path, style, &figure)?;
            if formats.png {
                convert_svg(
                    &svg_path,
                    &args.output.join(format!("{base_name}.png")),
                    "png",
                    args.dpi,
                );
            }
            if formats.pdf {
                convert_svg(
                    &svg_path,
                    &args.output.join(format!("{base_name}.pdf")),
                    "pdf",
                    args.dpi,
                );
            }
            println!("{}", svg_path.display());
        }
    }

    Ok(())
}

fn build_figures(
    dataset: &DashboardDataset,
    samples: &[DelaySample],
    style: FigureStyle,
) -> Vec<Figure> {
    let mut figures = Vec::new();
    let matrix = summary_matrix(&dataset.config_summary);
    if let Some(figure) = p95_priority_gap_figure(&matrix, style) {
        figures.push(figure);
    }
    if let Some(figure) = drop_rate_heatmap_figure(&matrix, style) {
        figures.push(figure);
    }
    if let Some(figure) = vo_reception_heatmap_figure(&matrix, style) {
        figures.push(figure);
    }
    if let Some(figure) = jitter_tradeoff_figure(&matrix, style) {
        figures.push(figure);
    }
    if let Some(figure) = drop_attribution_figure(&matrix, style) {
        figures.push(figure);
    }
    if let Some(figure) = vo_delay_cdf_figure(samples, style) {
        figures.push(figure);
    }
    if let Some(figure) = control_actions_figure(&matrix, style) {
        figures.push(figure);
    }
    figures
}

fn p95_priority_gap_figure(
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
    let max_value = rows
        .iter()
        .flat_map(|(_, be, vo)| [*be, *vo])
        .fold(0.0, f64::max);
    let plot = PlotArea::new(style);
    let mut svg = axis_frame(&plot, style, "Strategy", "P95 delay (ms)");
    draw_y_ticks(&mut svg, &plot, style, max_value);
    let group_width = plot.inner_w / rows.len() as f64;
    let bar_width = group_width * 0.26;
    for (index, (label, be, vo)) in rows.iter().enumerate() {
        let center = plot.left + group_width * (index as f64 + 0.5);
        draw_bar(
            &mut svg,
            &plot,
            center - bar_width * 0.65,
            bar_width,
            *be,
            max_value,
            BE_COLOR,
        );
        draw_bar(
            &mut svg,
            &plot,
            center + bar_width * 0.65,
            bar_width,
            *vo,
            max_value,
            VO_COLOR,
        );
        svg.push_str(&text(
            center,
            plot.bottom + category_offset(style),
            label,
            style.font_category,
            "middle",
            MUTED_COLOR,
        ));
    }
    let items = [("BE P95", BE_COLOR), ("VO P95", VO_COLOR)];
    let (lx, ly) = legend_origin_horizontal(&plot, LegendCorner::TopRight, &items, style);
    svg.push_str(&legend_horizontal(lx, ly, &items, style));
    Some(Figure {
        slug: "p95_delay_priority_gap",
        title: "Tail Delay Priority Gap Under High Load",
        question: "Does crash VO traffic obtain lower tail delay than ordinary BE traffic under contention?",
        body: svg,
    })
}

fn drop_rate_heatmap_figure(
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

fn vo_reception_heatmap_figure(
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

fn jitter_tradeoff_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    let mut points = Vec::new();
    for ((strategy, workload), summary) in matrix {
        if let (Some(delay), Some(jitter)) =
            (summary.metrics.be_delay_ms, summary.metrics.be_jitter_ms)
        {
            points.push((
                strategy_label(strategy),
                workload.as_str(),
                "BE",
                delay,
                jitter,
                BE_COLOR,
            ));
        }
        if let (Some(delay), Some(jitter)) =
            (summary.metrics.vo_delay_ms, summary.metrics.vo_jitter_ms)
        {
            points.push((
                strategy_label(strategy),
                workload.as_str(),
                "VO",
                delay,
                jitter,
                VO_COLOR,
            ));
        }
    }
    if points.is_empty() {
        return None;
    }
    let max_x = points.iter().map(|point| point.3).fold(0.0, f64::max);
    let max_y = points.iter().map(|point| point.4).fold(0.0, f64::max);
    let plot = PlotArea::new(style);
    let mut svg = axis_frame(&plot, style, "Mean delay (ms)", "Mean |jitter| (ms)");
    draw_x_ticks(&mut svg, &plot, style, max_x);
    draw_y_ticks(&mut svg, &plot, style, max_y);
    for (strategy, workload, _ac, delay, jitter, color) in points {
        let x = plot.scale_x(delay, max_x);
        let y = plot.scale_y(jitter, max_y);
        let radius = workload_marker_radius(workload, style);
        svg.push_str(&format!(
            r#"<circle cx="{x:.2}" cy="{y:.2}" r="{radius:.2}" fill="{color}" fill-opacity="0.9" stroke="{INK_COLOR}" stroke-width="0.6"/>"#
        ));
        let _ = strategy;
    }
    let color_items = [("BE", BE_COLOR), ("VO", VO_COLOR)];
    let (lx, ly) = legend_origin_horizontal(&plot, LegendCorner::TopRight, &color_items, style);
    svg.push_str(&legend_horizontal(lx, ly, &color_items, style));
    let marker_items = [("low", ""), ("med", ""), ("high", "")];
    let (mx, my) = legend_origin_vertical(&plot, LegendCorner::BottomRight, &marker_items, style);
    svg.push_str(&workload_marker_legend(mx, my, style));
    Some(Figure {
        slug: "latency_jitter_tradeoff",
        title: "Latency and Jitter Tradeoff",
        question: "Does prioritization reduce crash-message latency without introducing unstable delay variation?",
        body: svg,
    })
}

fn drop_attribution_figure(
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
    let (lx, ly) = legend_origin_horizontal(&plot, LegendCorner::TopRight, &items, style);
    svg.push_str(&legend_horizontal(lx, ly, &items, style));
    Some(Figure {
        slug: "mac_drop_attribution_high_load",
        title: "Packet-Drop Attribution Under High Load",
        question: "Are packet losses concentrated in BE traffic, VO traffic, or unclassified MAC behavior?",
        body: svg,
    })
}

fn vo_delay_cdf_figure(samples: &[DelaySample], style: FigureStyle) -> Option<Figure> {
    let mut by_strategy: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for sample in samples {
        if sample.ac == AccessCategory::Vo && sample.config.ends_with("_netload_high") {
            if let Some((strategy, _)) = config_parts(&sample.config) {
                by_strategy
                    .entry(strategy)
                    .or_default()
                    .push(sample.value_ms);
            }
        }
    }
    by_strategy.retain(|_, values| values.len() >= 2);
    if by_strategy.is_empty() {
        return None;
    }
    let max_x = by_strategy
        .values()
        .flat_map(|values| values.iter().copied())
        .fold(0.0, f64::max);
    let plot = PlotArea::new(style);
    let mut svg = axis_frame(&plot, style, "VO delay (ms)", "CDF");
    draw_x_ticks(&mut svg, &plot, style, max_x);
    draw_y_ticks(&mut svg, &plot, style, 1.0);
    let mut legend_items = Vec::new();
    for (index, (strategy, values)) in by_strategy.iter_mut().enumerate() {
        values.sort_by(f64::total_cmp);
        let color = COLORS[index % COLORS.len()];
        let mut points = String::new();
        for (rank, value) in values.iter().enumerate() {
            let x = plot.scale_x(*value, max_x);
            let y = plot.scale_y((rank + 1) as f64 / values.len() as f64, 1.0);
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
            style.line_stroke
        ));
        legend_items.push((strategy_label(strategy).to_string(), color.to_string()));
    }
    let legend_refs: Vec<_> = legend_items
        .iter()
        .map(|(label, color)| (label.as_str(), color.as_str()))
        .collect();
    let (lx, ly) = legend_origin_vertical(&plot, LegendCorner::TopRight, &legend_refs, style);
    svg.push_str(&legend_lines(lx, ly, &legend_refs, style));
    Some(Figure {
        slug: "vo_delay_cdf_high_load",
        title: "Crash VO Delay Distribution Under High Load",
        question:
            "Do prioritization strategies improve the full delay distribution, not only the mean?",
        body: svg,
    })
}

fn control_actions_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    style: FigureStyle,
) -> Option<Figure> {
    let rows: Vec<_> = ["stable", "guarded", "emergency"]
        .iter()
        .flat_map(|strategy| {
            WORKLOADS.iter().filter_map(move |workload| {
                let summary = matrix.get(&(strategy.to_string(), workload.to_string()))?;
                let protection = summary
                    .metrics
                    .vo_protection_activation_count
                    .unwrap_or(0.0);
                let suppressed = summary
                    .metrics
                    .be_grant_suppressed_while_blocked_count
                    .unwrap_or(0.0);
                ((protection + suppressed) > 0.0).then_some((
                    strategy_label(strategy),
                    *workload,
                    protection,
                    suppressed,
                ))
            })
        })
        .collect();
    if rows.is_empty() {
        return None;
    }
    let max_value = rows
        .iter()
        .flat_map(|(_, _, protection, suppressed)| [*protection, *suppressed])
        .fold(0.0, f64::max);
    let plot = PlotArea::new(style);
    let mut svg = axis_frame(&plot, style, "Config", "Control events");
    draw_y_ticks(&mut svg, &plot, style, max_value);
    let group_width = plot.inner_w / rows.len() as f64;
    let bar_width = group_width * 0.24;
    for (index, (strategy, workload, protection, suppressed)) in rows.iter().enumerate() {
        let center = plot.left + group_width * (index as f64 + 0.5);
        draw_bar(
            &mut svg,
            &plot,
            center - bar_width * 0.65,
            bar_width,
            *protection,
            max_value,
            VO_COLOR,
        );
        draw_bar(
            &mut svg,
            &plot,
            center + bar_width * 0.65,
            bar_width,
            *suppressed,
            max_value,
            BE_COLOR,
        );
        svg.push_str(&text(
            center,
            plot.bottom + category_offset(style),
            &compact_config_label(strategy, workload),
            style.font_category,
            "middle",
            MUTED_COLOR,
        ));
    }
    let items = [("VO prot.", VO_COLOR), ("BE supp.", BE_COLOR)];
    let (lx, ly) = legend_origin_horizontal(&plot, LegendCorner::TopRight, &items, style);
    svg.push_str(&legend_horizontal(lx, ly, &items, style));
    Some(Figure {
        slug: "v2x_control_actions_by_load",
        title: "Adaptive V2X Control Actions",
        question: "When tuned EDCA protects VO traffic, how often does it actively suppress BE contention?",
        body: svg,
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
    })
}

fn summary_matrix(summaries: &[ConfigSummary]) -> BTreeMap<(String, String), ConfigSummary> {
    let mut matrix = BTreeMap::new();
    for summary in summaries {
        if let Some((strategy, workload)) = config_parts(&summary.config) {
            matrix.insert((strategy, workload), summary.clone());
        }
    }
    matrix
}

fn config_parts(config: &str) -> Option<(String, String)> {
    if let Some(workload) = config.strip_prefix("plain_netload_") {
        return Some(("plain".to_string(), workload.to_string()));
    }
    if let Some(workload) = config.strip_prefix("edca_only_netload_") {
        return Some(("edca_only".to_string(), workload.to_string()));
    }
    let rest = config.strip_prefix("edca_v2x_vo_")?;
    let (variant, workload) = rest.split_once("_netload_")?;
    Some((variant.to_string(), workload.to_string()))
}

fn metric(summary: &ConfigSummary, key: &str) -> Option<f64> {
    match key {
        "mac_drop_per_tx" => summary.metrics.mac_drop_per_tx,
        "vo_rx_per_tx" => summary.metrics.vo_rx_per_tx,
        _ => None,
    }
}

fn load_delay_samples(results_dir: &Path) -> Result<Vec<DelaySample>> {
    let mut config_by_stem = HashMap::new();
    for sca_path in files_with_extension(results_dir, "sca")? {
        config_by_stem.insert(path_stem(&sca_path), config_from_sca(&sca_path)?);
    }

    let mut samples = Vec::new();
    for vec_path in files_with_extension(results_dir, "vec")? {
        let stem = path_stem(&vec_path);
        let config = config_by_stem
            .get(&stem)
            .cloned()
            .unwrap_or_else(|| stem.clone());
        samples.extend(delay_samples_from_vec(&vec_path, &config)?);
    }
    Ok(samples)
}

fn delay_samples_from_vec(path: &Path, config: &str) -> Result<Vec<DelaySample>> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut ac_by_vector_id: HashMap<String, AccessCategory> = HashMap::new();
    let mut samples = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let mut parts = line.split_whitespace();
        let Some(first) = parts.next() else {
            continue;
        };
        if first == "vector" {
            let Some(vector_id) = parts.next() else {
                continue;
            };
            let Some(module) = parts.next() else {
                continue;
            };
            let Some(metric) = parts.next() else {
                continue;
            };
            if is_node_app(module, 0) && metric == "beEndToEndDelay:vector" {
                ac_by_vector_id.insert(vector_id.to_string(), AccessCategory::Be);
            } else if is_node_app(module, 0) && metric == "voEndToEndDelay:vector" {
                ac_by_vector_id.insert(vector_id.to_string(), AccessCategory::Vo);
            }
            continue;
        }
        let Some(ac) = ac_by_vector_id.get(first).copied() else {
            continue;
        };
        let _event = parts.next();
        let _time = parts.next();
        let Some(value_raw) = parts.next() else {
            continue;
        };
        if let Ok(value_s) = value_raw.parse::<f64>() {
            if value_s.is_finite() {
                samples.push(DelaySample {
                    config: config.to_string(),
                    ac,
                    value_ms: value_s * 1000.0,
                });
            }
        }
    }
    Ok(samples)
}

fn config_from_sca(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if let Some(config) = line.strip_prefix("attr configname ") {
            return Ok(config.trim().to_string());
        }
    }
    Ok(path_stem(path))
}

#[derive(Debug, Clone, Copy)]
struct PlotArea {
    left: f64,
    top: f64,
    bottom: f64,
    inner_w: f64,
    inner_h: f64,
}

impl PlotArea {
    fn new(style: FigureStyle) -> Self {
        Self {
            left: style.margin_left,
            top: style.margin_top,
            bottom: style.height as f64 - style.margin_bottom,
            inner_w: style.width as f64 - style.margin_left - style.margin_right,
            inner_h: style.height as f64 - style.margin_top - style.margin_bottom,
        }
    }

    fn scale_x(&self, value: f64, max_value: f64) -> f64 {
        self.left + self.inner_w * value / padded_max(max_value)
    }

    fn scale_y(&self, value: f64, max_value: f64) -> f64 {
        self.bottom - self.inner_h * value / padded_max(max_value)
    }
}

fn write_svg(path: &Path, style: FigureStyle, figure: &Figure) -> Result<()> {
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

fn axis_frame(plot: &PlotArea, style: FigureStyle, x_label: &str, y_label: &str) -> String {
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

fn draw_y_ticks(svg: &mut String, plot: &PlotArea, style: FigureStyle, max_value: f64) {
    let ticks = tick_values(max_value, style.tick_count);
    for (index, value) in ticks.iter().enumerate() {
        let y = plot.scale_y(*value, max_value);
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
            &format_tick(*value, max_value),
            style.font_tick,
            "end",
            MUTED_COLOR,
        ));
    }
}

fn draw_x_ticks(svg: &mut String, plot: &PlotArea, style: FigureStyle, max_value: f64) {
    let ticks = tick_values(max_value, style.tick_count);
    for (index, value) in ticks.iter().enumerate() {
        let x = plot.scale_x(*value, max_value);
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
            &format_tick(*value, max_value),
            style.font_tick,
            "middle",
            MUTED_COLOR,
        ));
    }
}

fn category_offset(style: FigureStyle) -> f64 {
    if style.show_header {
        28.0
    } else {
        f64::from(style.font_category) + 6.0
    }
}

fn axis_label_offset(style: FigureStyle) -> f64 {
    if style.show_header {
        68.0
    } else {
        category_offset(style) + f64::from(style.font_axis) + 2.0
    }
}

fn y_axis_label_offset(style: FigureStyle) -> f64 {
    if style.show_header {
        92.0
    } else {
        tick_label_pad(style) + f64::from(style.font_axis) + 6.0
    }
}

fn heatmap_y_axis_label_offset(style: FigureStyle) -> f64 {
    if style.show_header {
        110.0
    } else {
        heatmap_row_label_gap(style) + f64::from(style.font_axis) + 4.0
    }
}

fn heatmap_row_label_gap(style: FigureStyle) -> f64 {
    if style.show_header { 10.0 } else { 8.0 }
}

fn tick_label_offset(style: FigureStyle) -> f64 {
    if style.show_header {
        24.0
    } else {
        f64::from(style.font_tick) + 4.0
    }
}

fn tick_label_pad(style: FigureStyle) -> f64 {
    if style.show_header { 8.0 } else { 6.0 }
}

fn tick_values(max_value: f64, tick_count: u32) -> Vec<f64> {
    let max = padded_max(max_value);
    (0..=tick_count)
        .map(|index| max * index as f64 / tick_count as f64)
        .collect()
}

fn legend_item_stride(style: FigureStyle) -> f64 {
    f64::from(style.font_legend) + 4.0
}

fn legend_box_size(style: FigureStyle) -> f64 {
    if style.show_header {
        14.0
    } else {
        f64::from(style.font_legend) + 1.0
    }
}

fn legend_symbol_width(label: &str, style: FigureStyle) -> f64 {
    legend_box_size(style) + 3.0 + label.len() as f64 * f64::from(style.font_legend) * 0.52
}

fn legend_horizontal_width(items: &[(&str, &str)], style: FigureStyle) -> f64 {
    let mut width = 0.0;
    for (index, (label, _)) in items.iter().enumerate() {
        if index > 0 {
            width += legend_horizontal_gap(style);
        }
        width += legend_symbol_width(label, style);
    }
    width
}

fn legend_vertical_height(rows: usize, style: FigureStyle) -> f64 {
    if rows == 0 {
        0.0
    } else {
        legend_item_stride(style) * (rows - 1) as f64 + legend_box_size(style)
    }
}

fn legend_horizontal_gap(style: FigureStyle) -> f64 {
    if style.show_header { 18.0 } else { 10.0 }
}

fn legend_vertical_width(items: &[(&str, &str)], style: FigureStyle) -> f64 {
    items
        .iter()
        .map(|(label, _)| legend_symbol_width(label, style))
        .fold(0.0, f64::max)
}

fn legend_origin_horizontal(
    plot: &PlotArea,
    corner: LegendCorner,
    items: &[(&str, &str)],
    style: FigureStyle,
) -> (f64, f64) {
    let pad = if style.show_header { 12.0 } else { 6.0 };
    let w = legend_horizontal_width(items, style);
    let h = legend_box_size(style);
    match corner {
        LegendCorner::TopRight => (
            plot.left + plot.inner_w - w - pad,
            plot.top + pad,
        ),
        LegendCorner::BottomRight => (
            plot.left + plot.inner_w - w - pad,
            plot.bottom - h - pad - tick_label_offset(style),
        ),
    }
}

fn legend_origin_vertical(
    plot: &PlotArea,
    corner: LegendCorner,
    items: &[(&str, &str)],
    style: FigureStyle,
) -> (f64, f64) {
    let pad = if style.show_header { 12.0 } else { 6.0 };
    let w = legend_vertical_width(items, style);
    let h = legend_vertical_height(items.len(), style);
    match corner {
        LegendCorner::TopRight => (
            plot.left + plot.inner_w - w - pad,
            plot.top + pad,
        ),
        LegendCorner::BottomRight => (
            plot.left + plot.inner_w - w - pad,
            plot.bottom - h - pad - tick_label_offset(style),
        ),
    }
}

fn heatmap_plot_area(style: FigureStyle) -> PlotArea {
    let extra_left = if style.show_header { 0.0 } else { 34.0 };
    PlotArea {
        left: style.margin_left + extra_left,
        top: style.margin_top,
        bottom: style.height as f64 - style.margin_bottom,
        inner_w: style.width as f64
            - style.margin_left
            - style.margin_right
            - extra_left,
        inner_h: style.height as f64 - style.margin_top - style.margin_bottom,
    }
}

fn compact_config_label(strategy: &str, workload: &str) -> String {
    let workload_short = match workload {
        "low" => "lo",
        "medium" => "med",
        "high" => "hi",
        other => other,
    };
    format!("{strategy},{workload_short}")
}

fn workload_marker_radius(workload: &str, style: FigureStyle) -> f64 {
    let scale = if style.show_header { 1.0 } else { 0.72 };
    match workload {
        "low" => 2.5 * scale,
        "medium" => 3.5 * scale,
        "high" => 4.5 * scale,
        _ => 3.0 * scale,
    }
}

fn workload_label(workload: &str) -> String {
    match workload {
        "low" => "low".to_string(),
        "medium" => "med".to_string(),
        "high" => "high".to_string(),
        other => other.to_string(),
    }
}

fn workload_marker_legend(x: f64, y: f64, style: FigureStyle) -> String {
    let mut svg = String::new();
    let entries = [("low", 2.5), ("med", 3.5), ("high", 4.5)];
    let scale = if style.show_header { 1.0 } else { 0.72 };
    let box_size = legend_box_size(style);
    for (index, (label, radius)) in entries.iter().enumerate() {
        let row_y = y + index as f64 * legend_item_stride(style) + box_size * 0.5;
        let cx = x + 6.0 * scale;
        let r = radius * scale;
        svg.push_str(&format!(
            r#"<circle cx="{cx:.2}" cy="{row_y:.2}" r="{r:.2}" fill="{MUTED_COLOR}" fill-opacity="0.65" stroke="{INK_COLOR}" stroke-width="0.5"/>"#
        ));
        svg.push_str(&text(
            x + 16.0 * scale,
            row_y + 3.0,
            label,
            style.font_legend,
            "start",
            INK_COLOR,
        ));
    }
    svg
}

fn draw_bar(
    svg: &mut String,
    plot: &PlotArea,
    center: f64,
    width: f64,
    value: f64,
    max_value: f64,
    color: &str,
) {
    let y = plot.scale_y(value, max_value);
    svg.push_str(&rect(
        center - width / 2.0,
        y,
        width,
        plot.bottom - y,
        color,
    ));
}

fn rect_fill(x: f64, y: f64, width: f64, height: f64, color: &str) -> String {
    format!(
        r#"<rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{height:.2}" fill="{color}"/>"#
    )
}

fn rect(x: f64, y: f64, width: f64, height: f64, color: &str) -> String {
    format!(
        r#"<rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{height:.2}" fill="{color}" stroke="{INK_COLOR}" stroke-width="0.3" stroke-opacity="0.4"/>"#
    )
}

fn text(x: f64, y: f64, value: &str, size: u32, anchor: &str, color: &str) -> String {
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

fn legend_horizontal(x: f64, y: f64, items: &[(&str, &str)], style: FigureStyle) -> String {
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

fn legend_lines(x: f64, y: f64, items: &[(&str, &str)], style: FigureStyle) -> String {
    let mut svg = String::new();
    let box_size = legend_box_size(style);
    let line_len = box_size * 1.4;
    for (index, (label, color)) in items.iter().enumerate() {
        let row_y = y + index as f64 * legend_item_stride(style) + box_size * 0.5;
        let dash = CDF_DASHES[index % CDF_DASHES.len()];
        let dash_attr = if dash.is_empty() {
            String::new()
        } else {
            format!(r#" stroke-dasharray="{dash}""#)
        };
        svg.push_str(&format!(
            r#"<line x1="{x:.2}" y1="{row_y:.2}" x2="{:.2}" y2="{row_y:.2}" stroke="{color}" stroke-width="{:.2}"{dash_attr}/>"#,
            x + line_len,
            style.line_stroke
        ));
        svg.push_str(&text(
            x + line_len + 4.0,
            row_y + 3.0,
            label,
            style.font_legend,
            "start",
            INK_COLOR,
        ));
    }
    svg
}

fn heat_color(value: f64, max_value: f64) -> String {
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

fn padded_max(value: f64) -> f64 {
    if value <= 0.0 || !value.is_finite() {
        1.0
    } else {
        value * 1.08
    }
}

fn format_metric(value: f64) -> String {
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

fn format_tick(value: f64, max_value: f64) -> String {
    if max_value <= 1.05 && max_value > 0.0 {
        format!("{value:.1}")
    } else {
        format_metric(value)
    }
}

fn figure_order(slug: &str) -> Option<u8> {
    match slug {
        "p95_delay_priority_gap" => Some(1),
        "mac_drop_rate_by_strategy_load" => Some(2),
        "vo_reception_by_strategy_load" => Some(3),
        "latency_jitter_tradeoff" => Some(4),
        "mac_drop_attribution_high_load" => Some(5),
        "vo_delay_cdf_high_load" => Some(6),
        "v2x_control_actions_by_load" => Some(7),
        _ => None,
    }
}

fn strategy_label(strategy: &str) -> &'static str {
    match strategy {
        "plain" => "DCF",
        "edca_only" => "EDCA",
        "stable" => "Stable",
        "guarded" => "Guarded",
        "emergency" => "Emergency",
        _ => "Other",
    }
}

fn density_label(path: &Path) -> String {
    let raw = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .find(|component| {
            component.contains("highway_light") || component.contains("highway_heavy")
        })
        .map(|component| {
            if component.contains("light") {
                "highway_light".to_string()
            } else {
                "highway_heavy".to_string()
            }
        })
        .unwrap_or_else(|| {
            path.parent()
                .and_then(Path::file_name)
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| "results".to_string())
        });
    slugify(&raw)
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn files_with_extension(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    if !dir.exists() {
        return Ok(paths);
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some(extension) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn path_stem(path: &Path) -> String {
    path.file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn is_node_app(module: &str, app_index: u8) -> bool {
    module.starts_with("Scenario.node[") && module.ends_with(&format!(".app[{app_index}]"))
}

fn convert_svg(svg_path: &Path, output_path: &Path, format: &str, dpi: u32) {
    let converted = if command_exists("rsvg-convert") {
        Command::new("rsvg-convert")
            .args([
                "-f",
                format,
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
struct ExportFormats {
    png: bool,
    pdf: bool,
}

impl ExportFormats {
    fn parse(raw: &str) -> Self {
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
    use super::figure_order;

    #[test]
    fn figure_order_is_stable_by_slug() {
        assert_eq!(figure_order("p95_delay_priority_gap"), Some(1));
        assert_eq!(figure_order("v2x_control_actions_by_load"), Some(7));
        assert_eq!(figure_order("unknown_slug"), None);
    }
}
