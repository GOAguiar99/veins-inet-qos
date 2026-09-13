//! Publication figure catalog, data helpers, styling, and SVG renderers.

pub mod catalog;
pub mod data;
pub mod style;
mod svg;
mod uniform;
mod hotspot;
mod tradeoff;
mod render;

pub use catalog::{figure_order, print_figure_catalog, resolve_figure_selectors};
pub use data::{
    density_label, load_config_summary_json, load_vo_delay_cdf_samples, merge_summaries, Overlay,
};
pub use render::{build_all_figures, convert_svg, write_svg, ExportFormats};
pub use style::{FigureStyle, DEFAULT_PUB_HEIGHT, DEFAULT_PUB_WIDTH};
