//! Thin CLI for publication figure export.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use kpi_dashboard::figures::{
    build_all_figures, convert_svg, density_label, figure_order, load_config_summary_json,
    load_vo_delay_cdf_samples, merge_summaries, print_figure_catalog, resolve_figure_selectors,
    write_svg, ExportFormats, FigureStyle, DEFAULT_PUB_HEIGHT, DEFAULT_PUB_WIDTH,
};
use kpi_dashboard::{
    default_results_dir, rebuild_raw_dataset, try_load_valid_rust_cache, ConfigSummary,
};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Export publication-oriented figures from Veins QoS KPI results"
)]
struct Cli {
    #[arg(long)]
    results: Vec<PathBuf>,

    /// Merge ConfigSummary JSON archives (e.g. hotspot without local .sca).
    #[arg(long = "summary-json")]
    summary_json: Vec<PathBuf>,

    #[arg(long, default_value = "publication_figures")]
    output: PathBuf,

    #[arg(long, default_value = "svg,pdf")]
    formats: String,

    #[arg(long, default_value_t = 300)]
    dpi: u32,

    /// Compact IEEE single-column layout (252 px wide). Use `--ieee` explicitly.
    #[arg(long = "ieee", action = clap::ArgAction::SetTrue, default_value_t = false)]
    #[arg(long = "no-ieee", action = clap::ArgAction::SetFalse)]
    ieee: bool,

    #[arg(long, default_value_t = DEFAULT_PUB_WIDTH)]
    width: u32,

    #[arg(long, default_value_t = DEFAULT_PUB_HEIGHT)]
    height: u32,

    #[arg(long)]
    threads: Option<usize>,

    /// Export only these figures (repeatable). Accepts id (`06`), slug, or `fig_06`.
    /// Omit to export the default thesis set (audit heatmaps 02/03 excluded).
    #[arg(long = "figures", value_name = "FIG", num_args = 1..)]
    figures: Vec<String>,

    /// Print available figure ids and slugs, then exit.
    #[arg(long = "list-figures", action = clap::ArgAction::SetTrue)]
    list_figures: bool,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    if args.list_figures {
        print_figure_catalog();
        return Ok(());
    }

    let defaults_active = args.figures.is_empty();
    let selected = resolve_figure_selectors(&args.figures)?;
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
        let mut publication = FigureStyle::publication();
        publication.width = args.width;
        publication.height = args.height;
        publication
    };
    fs::create_dir_all(&args.output)
        .with_context(|| format!("failed to create {}", args.output.display()))?;

    let mut archive_summaries = Vec::new();
    for path in &args.summary_json {
        let rows = load_config_summary_json(path)
            .with_context(|| format!("failed to load summary JSON {}", path.display()))?;
        eprintln!(
            "loaded {} config summary row(s) from {}",
            rows.len(),
            path.display()
        );
        archive_summaries.push(rows);
    }

    let density_specific: HashSet<u8> = [1u8, 2, 3, 5, 6, 7].into_iter().collect();
    let cross_regime: HashSet<u8> = [4u8, 8, 9, 10].into_iter().collect();

    let mut all_summaries: Vec<ConfigSummary> = Vec::new();
    for rows in &archive_summaries {
        all_summaries.extend(rows.iter().cloned());
    }

    for results_dir in &results_dirs {
        let density = density_label(results_dir);
        let need_dataset = selected.iter().any(|id| density_specific.contains(id));
        let need_samples = selected.contains(&6)
            && !(defaults_active && density.contains("light"));

        let dataset = if need_dataset {
            match try_load_valid_rust_cache(results_dir)? {
                Some(cached) => {
                    eprintln!("using KPI cache for {}", results_dir.display());
                    Some(cached)
                }
                None => Some(rebuild_raw_dataset(results_dir, args.threads).with_context(
                    || format!("failed to rebuild KPI data from {}", results_dir.display()),
                )?),
            }
        } else {
            None
        };

        if let Some(dataset) = dataset.as_ref() {
            all_summaries.extend(dataset.config_summary.iter().cloned());
        }

        let samples = if need_samples {
            load_vo_delay_cdf_samples(results_dir).unwrap_or_else(|error| {
                eprintln!(
                    "warning: could not load VO delay samples from {}: {error:#}",
                    results_dir.display()
                );
                Vec::new()
            })
        } else {
            Vec::new()
        };

        let per_density: HashSet<u8> = selected
            .iter()
            .copied()
            .filter(|id| density_specific.contains(id))
            .collect();
        if per_density.is_empty() {
            continue;
        }

        let summaries = dataset
            .as_ref()
            .map(|data| data.config_summary.as_slice())
            .unwrap_or(&[]);
        let figures = build_all_figures(
            summaries,
            &samples,
            style,
            &per_density,
            &density,
            defaults_active,
        );
        if figures.is_empty() {
            eprintln!(
                "warning: no density-specific figures for {} (selected: {:?})",
                results_dir.display(),
                per_density
            );
        }
        write_figures(&figures, &args.output, &density, style, &formats, args.dpi)?;
    }

    let cross_selected: HashSet<u8> = selected
        .iter()
        .copied()
        .filter(|id| cross_regime.contains(id))
        .collect();
    if !cross_selected.is_empty() {
        let merged = merge_summaries(&[&all_summaries]);
        let density = "highway_heavy";
        let figures = build_all_figures(
            &merged,
            &[],
            style,
            &cross_selected,
            density,
            defaults_active,
        );
        if figures.is_empty() {
            eprintln!(
                "warning: no cross-regime figures produced (selected: {:?})",
                cross_selected
            );
        }
        write_figures(&figures, &args.output, density, style, &formats, args.dpi)?;
    }

    Ok(())
}

fn write_figures(
    figures: &[kpi_dashboard::figures::style::Figure],
    output: &std::path::Path,
    density: &str,
    style: FigureStyle,
    formats: &ExportFormats,
    dpi: u32,
) -> Result<()> {
    for figure in figures {
        let Some(order) = figure_order(figure.slug) else {
            eprintln!("warning: unknown figure slug {}, skipping", figure.slug);
            continue;
        };
        let base_name = format!("fig_{:02}_{}_{}", order, figure.slug, density);
        let svg_path = output.join(format!("{base_name}.svg"));
        let figure_style = figure.style.unwrap_or(style);
        write_svg(&svg_path, figure_style, figure)?;
        if formats.png {
            convert_svg(
                &svg_path,
                &output.join(format!("{base_name}.png")),
                "png",
                dpi,
            );
        }
        if formats.pdf {
            convert_svg(
                &svg_path,
                &output.join(format!("{base_name}.pdf")),
                "pdf",
                dpi,
            );
        }
        println!("{}", svg_path.display());
    }
    Ok(())
}
