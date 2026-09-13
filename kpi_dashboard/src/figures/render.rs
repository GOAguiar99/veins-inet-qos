//! Figure builder orchestration and SVG export re-exports.

use std::collections::HashSet;

use crate::ConfigSummary;

use super::data::{summary_matrix, Overlay, VoDelaySample};
use super::hotspot::{hotspot_vo_delay_by_policy_figure, hotspot_vo_p95_by_load_figure};
use super::style::{Figure, FigureStyle};
use super::tradeoff::{regime_vo_p95_contrast_figure, vo_gain_vs_be_cost_figure};
use super::uniform::{
    control_actions_figure, drop_attribution_figure, drop_rate_heatmap_figure,
    p95_priority_gap_figure, vo_delay_cdf_figure, vo_reception_heatmap_figure,
};

pub use super::svg::{convert_svg, write_svg, ExportFormats};

/// Build selected figures from merged summaries and optional VO delay samples.
pub fn build_all_figures(
    summaries: &[ConfigSummary],
    samples: &[VoDelaySample],
    style: FigureStyle,
    selected: &HashSet<u8>,
    density: &str,
    defaults_active: bool,
) -> Vec<Figure> {
    let want = |id: u8| selected.contains(&id);
    let mut figures = Vec::new();
    let netload = summary_matrix(summaries, Overlay::Netload);
    let hotspot = summary_matrix(summaries, Overlay::Hotspot);

    if want(1) {
        if let Some(figure) = p95_priority_gap_figure(&netload, style) {
            figures.push(figure);
        }
    }
    if want(2) {
        if let Some(figure) = drop_rate_heatmap_figure(&netload, style) {
            figures.push(figure);
        }
    }
    if want(3) {
        if let Some(figure) = vo_reception_heatmap_figure(&netload, style) {
            figures.push(figure);
        }
    }
    if want(4) {
        if let Some(figure) = vo_gain_vs_be_cost_figure(&netload, &hotspot, style) {
            figures.push(figure);
        }
    }
    if want(5) {
        if let Some(figure) = drop_attribution_figure(&netload, style) {
            figures.push(figure);
        }
    }
    if want(6) {
        let skip_light = defaults_active && density.contains("light");
        if !skip_light {
            if let Some(figure) = vo_delay_cdf_figure(samples, style) {
                figures.push(figure);
            }
        }
    }
    if want(7) {
        if let Some(figure) = control_actions_figure(&netload, style) {
            figures.push(figure);
        }
    }
    if want(8) {
        if let Some(figure) = hotspot_vo_delay_by_policy_figure(&hotspot, style) {
            figures.push(figure);
        }
    }
    if want(9) {
        if let Some(figure) = hotspot_vo_p95_by_load_figure(&hotspot, style) {
            figures.push(figure);
        }
    }
    if want(10) {
        if let Some(figure) = regime_vo_p95_contrast_figure(&netload, &hotspot, style) {
            figures.push(figure);
        }
    }
    figures
}
