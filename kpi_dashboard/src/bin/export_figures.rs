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
const MUTED_COLOR: &str = "#6b7280";

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

    #[arg(long, default_value = "svg,png,pdf")]
    formats: String,

    #[arg(long, default_value_t = 300)]
    dpi: u32,

    #[arg(long, default_value_t = 1400)]
    width: u32,

    #[arg(long, default_value_t = 900)]
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
        let figures = build_figures(&dataset, &samples, args.width, args.height);

        for (index, figure) in figures.into_iter().enumerate() {
            let base_name = format!("fig_{:02}_{}_{}", index + 1, figure.slug, density);
            let svg_path = args.output.join(format!("{base_name}.svg"));
            write_svg(&svg_path, args.width, args.height, &figure)?;
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
    width: u32,
    height: u32,
) -> Vec<Figure> {
    let mut figures = Vec::new();
    let matrix = summary_matrix(&dataset.config_summary);
    if let Some(figure) = p95_priority_gap_figure(&matrix, width, height) {
        figures.push(figure);
    }
    if let Some(figure) = drop_rate_heatmap_figure(&matrix, width, height) {
        figures.push(figure);
    }
    if let Some(figure) = vo_reception_heatmap_figure(&matrix, width, height) {
        figures.push(figure);
    }
    if let Some(figure) = jitter_tradeoff_figure(&matrix, width, height) {
        figures.push(figure);
    }
    if let Some(figure) = drop_attribution_figure(&matrix, width, height) {
        figures.push(figure);
    }
    if let Some(figure) = vo_delay_cdf_figure(samples, width, height) {
        figures.push(figure);
    }
    if let Some(figure) = control_actions_figure(&matrix, width, height) {
        figures.push(figure);
    }
    figures
}

fn p95_priority_gap_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    width: u32,
    height: u32,
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
    let plot = PlotArea::new(width, height);
    let mut svg = axis_frame(&plot, "P95 delay (ms)", "MAC strategy");
    draw_y_ticks(&mut svg, &plot, max_value);
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
            plot.bottom + 28.0,
            label,
            13,
            "middle",
            MUTED_COLOR,
        ));
    }
    svg.push_str(&legend(
        plot.left + 10.0,
        plot.top + 8.0,
        &[("BE P95", BE_COLOR), ("VO P95", VO_COLOR)],
    ));
    Some(Figure {
        slug: "p95_delay_priority_gap",
        title: "Tail Delay Priority Gap Under High Load",
        question: "Does crash VO traffic obtain lower tail delay than ordinary BE traffic under contention?",
        body: svg,
    })
}

fn drop_rate_heatmap_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    width: u32,
    height: u32,
) -> Option<Figure> {
    heatmap_figure(
        matrix,
        width,
        height,
        "mac_drop_per_tx",
        "mac_drop_rate_by_strategy_load",
        "MAC Drop Rate Across Strategy and Load",
        "How quickly does contention translate into normalized packet loss as offered load grows?",
        "MAC drops / app TX",
    )
}

fn vo_reception_heatmap_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    width: u32,
    height: u32,
) -> Option<Figure> {
    heatmap_figure(
        matrix,
        width,
        height,
        "vo_rx_per_tx",
        "vo_reception_by_strategy_load",
        "VO Reception Across Strategy and Load",
        "Which MAC strategy preserves crash-message reception as load increases?",
        "VO RX / logical TX",
    )
}

fn jitter_tradeoff_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    width: u32,
    height: u32,
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
    let plot = PlotArea::new(width, height);
    let mut svg = axis_frame(&plot, "Mean delay (ms)", "Mean absolute jitter (ms)");
    draw_x_ticks(&mut svg, &plot, max_x);
    draw_y_ticks(&mut svg, &plot, max_y);
    for (strategy, workload, ac, delay, jitter, color) in points {
        let x = plot.scale_x(delay, max_x);
        let y = plot.scale_y(jitter, max_y);
        let radius = match workload {
            "low" => 4.0,
            "medium" => 6.0,
            "high" => 8.0,
            _ => 5.0,
        };
        svg.push_str(&format!(
            r#"<circle cx="{x:.2}" cy="{y:.2}" r="{radius:.2}" fill="{color}" fill-opacity="0.78"/>"#
        ));
        svg.push_str(&text(
            x + 7.0,
            y - 7.0,
            &format!("{strategy} {workload} {ac}"),
            10,
            "start",
            MUTED_COLOR,
        ));
    }
    svg.push_str(&legend(
        plot.left + 10.0,
        plot.top + 8.0,
        &[("BE", BE_COLOR), ("VO", VO_COLOR)],
    ));
    Some(Figure {
        slug: "latency_jitter_tradeoff",
        title: "Latency and Jitter Tradeoff",
        question: "Does prioritization reduce crash-message latency without introducing unstable delay variation?",
        body: svg,
    })
}

fn drop_attribution_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    width: u32,
    height: u32,
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
    let plot = PlotArea::new(width, height);
    let mut svg = axis_frame(&plot, "MAC drops (count)", "MAC strategy");
    draw_y_ticks(&mut svg, &plot, max_value);
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
            plot.bottom + 28.0,
            label,
            13,
            "middle",
            MUTED_COLOR,
        ));
    }
    svg.push_str(&legend(
        plot.left + 10.0,
        plot.top + 8.0,
        &[
            ("BE", BE_COLOR),
            ("VO", VO_COLOR),
            ("Unclassified", "#9ca3af"),
        ],
    ));
    Some(Figure {
        slug: "mac_drop_attribution_high_load",
        title: "Packet-Drop Attribution Under High Load",
        question: "Are packet losses concentrated in BE traffic, VO traffic, or unclassified MAC behavior?",
        body: svg,
    })
}

fn vo_delay_cdf_figure(samples: &[DelaySample], width: u32, height: u32) -> Option<Figure> {
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
    let plot = PlotArea::new(width, height);
    let mut svg = axis_frame(&plot, "VO end-to-end delay (ms)", "Empirical CDF");
    draw_x_ticks(&mut svg, &plot, max_x);
    draw_y_ticks(&mut svg, &plot, 1.0);
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
        svg.push_str(&format!(
            r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2.4"/>"#,
            points.trim(),
            color
        ));
        legend_items.push((strategy_label(strategy).to_string(), color.to_string()));
    }
    let legend_refs: Vec<_> = legend_items
        .iter()
        .map(|(label, color)| (label.as_str(), color.as_str()))
        .collect();
    svg.push_str(&legend(plot.left + 10.0, plot.top + 8.0, &legend_refs));
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
    width: u32,
    height: u32,
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
    let plot = PlotArea::new(width, height);
    let mut svg = axis_frame(&plot, "Control events (count)", "Strategy/load");
    draw_y_ticks(&mut svg, &plot, max_value);
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
            plot.bottom + 28.0,
            &format!("{strategy}\n{workload}"),
            12,
            "middle",
            MUTED_COLOR,
        ));
    }
    svg.push_str(&legend(
        plot.left + 10.0,
        plot.top + 8.0,
        &[
            ("VO protection activations", VO_COLOR),
            ("BE grants suppressed", BE_COLOR),
        ],
    ));
    Some(Figure {
        slug: "v2x_control_actions_by_load",
        title: "Adaptive V2X Control Actions",
        question: "When tuned EDCA protects VO traffic, how often does it actively suppress BE contention?",
        body: svg,
    })
}

fn heatmap_figure(
    matrix: &BTreeMap<(String, String), ConfigSummary>,
    width: u32,
    height: u32,
    metric_key: &str,
    slug: &'static str,
    title: &'static str,
    question: &'static str,
    color_label: &str,
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
    let plot = PlotArea::new(width, height);
    let cell_w = plot.inner_w / WORKLOADS.len() as f64;
    let cell_h = plot.inner_h / STRATEGIES.len() as f64;
    let mut svg = String::new();
    svg.push_str(&text(
        plot.left + plot.inner_w / 2.0,
        plot.top - 32.0,
        color_label,
        14,
        "middle",
        MUTED_COLOR,
    ));
    for (row, strategy) in STRATEGIES.iter().enumerate() {
        let y = plot.top + row as f64 * cell_h;
        svg.push_str(&text(
            plot.left - 12.0,
            y + cell_h / 2.0 + 4.0,
            strategy_label(strategy),
            13,
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
            svg.push_str(&rect(x + 1.0, y + 1.0, cell_w - 2.0, cell_h - 2.0, &color));
            svg.push_str(&text(
                x + cell_w / 2.0,
                y + cell_h / 2.0 + 4.0,
                &value
                    .map(format_metric)
                    .unwrap_or_else(|| "N/A".to_string()),
                14,
                "middle",
                "#111827",
            ));
        }
    }
    for (col, workload) in WORKLOADS.iter().enumerate() {
        let x = plot.left + col as f64 * cell_w + cell_w / 2.0;
        svg.push_str(&text(
            x,
            plot.bottom + 28.0,
            workload,
            13,
            "middle",
            MUTED_COLOR,
        ));
    }
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
    fn new(width: u32, height: u32) -> Self {
        let left = 150.0;
        let top = 145.0;
        let right = 60.0;
        let bottom_margin = 100.0;
        Self {
            left,
            top,
            bottom: height as f64 - bottom_margin,
            inner_w: width as f64 - left - right,
            inner_h: height as f64 - top - bottom_margin,
        }
    }

    fn scale_x(&self, value: f64, max_value: f64) -> f64 {
        self.left + self.inner_w * value / padded_max(max_value)
    }

    fn scale_y(&self, value: f64, max_value: f64) -> f64 {
        self.bottom - self.inner_h * value / padded_max(max_value)
    }
}

fn write_svg(path: &Path, width: u32, height: u32, figure: &Figure) -> Result<()> {
    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    writeln!(
        file,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    )?;
    writeln!(
        file,
        r##"<rect width="100%" height="100%" fill="#ffffff"/>"##
    )?;
    writeln!(
        file,
        r#"<style>text {{ font-family: "Arial", "Helvetica", sans-serif; }} .title {{ font-weight: 700; }} .question {{ font-style: italic; }}</style>"#
    )?;
    writeln!(
        file,
        "{}",
        text(40.0, 48.0, figure.title, 26, "start", "#111827")
    )?;
    writeln!(
        file,
        "{}",
        text(40.0, 82.0, figure.question, 16, "start", MUTED_COLOR)
    )?;
    writeln!(file, "{}", figure.body)?;
    writeln!(file, "</svg>")?;
    Ok(())
}

fn axis_frame(plot: &PlotArea, x_label: &str, y_label: &str) -> String {
    let mut svg = String::new();
    svg.push_str(&format!(
        r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#111827" stroke-width="1.4"/>"##,
        plot.left,
        plot.bottom,
        plot.left + plot.inner_w,
        plot.bottom
    ));
    svg.push_str(&format!(
        r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#111827" stroke-width="1.4"/>"##,
        plot.left, plot.top, plot.left, plot.bottom
    ));
    svg.push_str(&text(
        plot.left + plot.inner_w / 2.0,
        plot.bottom + 68.0,
        x_label,
        15,
        "middle",
        "#111827",
    ));
    svg.push_str(&format!(
        r##"<text x="{:.2}" y="{:.2}" text-anchor="middle" font-size="15" fill="#111827" transform="rotate(-90 {:.2} {:.2})">{}</text>"##,
        plot.left - 92.0,
        plot.top + plot.inner_h / 2.0,
        plot.left - 92.0,
        plot.top + plot.inner_h / 2.0,
        escape(y_label)
    ));
    svg
}

fn draw_y_ticks(svg: &mut String, plot: &PlotArea, max_value: f64) {
    for index in 0..=5 {
        let value = padded_max(max_value) * index as f64 / 5.0;
        let y = plot.scale_y(value, max_value);
        svg.push_str(&format!(
            r##"<line x1="{:.2}" y1="{y:.2}" x2="{:.2}" y2="{y:.2}" stroke="#e5e7eb"/>"##,
            plot.left,
            plot.left + plot.inner_w
        ));
        svg.push_str(&text(
            plot.left - 10.0,
            y + 4.0,
            &format_metric(value),
            12,
            "end",
            MUTED_COLOR,
        ));
    }
}

fn draw_x_ticks(svg: &mut String, plot: &PlotArea, max_value: f64) {
    for index in 0..=5 {
        let value = padded_max(max_value) * index as f64 / 5.0;
        let x = plot.scale_x(value, max_value);
        svg.push_str(&format!(
            r##"<line x1="{x:.2}" y1="{:.2}" x2="{x:.2}" y2="{:.2}" stroke="#e5e7eb"/>"##,
            plot.top, plot.bottom
        ));
        svg.push_str(&text(
            x,
            plot.bottom + 24.0,
            &format_metric(value),
            12,
            "middle",
            MUTED_COLOR,
        ));
    }
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

fn rect(x: f64, y: f64, width: f64, height: f64, color: &str) -> String {
    format!(
        r#"<rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{height:.2}" fill="{color}"/>"#
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

fn legend(x: f64, y: f64, items: &[(&str, &str)]) -> String {
    let mut svg = String::new();
    for (index, (label, color)) in items.iter().enumerate() {
        let y = y + index as f64 * 24.0;
        svg.push_str(&rect(x, y - 12.0, 16.0, 16.0, color));
        svg.push_str(&text(x + 24.0, y + 1.0, label, 13, "start", MUTED_COLOR));
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
    } else {
        format!("{value:.3}")
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
