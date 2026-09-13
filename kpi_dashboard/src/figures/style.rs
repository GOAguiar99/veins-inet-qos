//! Layout presets and figure metadata.

pub const BE_COLOR: &str = "#4c78a8";
pub const VO_COLOR: &str = "#e45756";
pub const INK_COLOR: &str = "#111827";
pub const MUTED_COLOR: &str = "#6b7280";
pub const GRID_COLOR: &str = "#e5e7eb";
pub const FONT_SERIF: &str = r#""Times New Roman", Times, serif"#;
pub const COLORS: &[&str] = &["#4c78a8", "#f58518", "#54a24b", "#b279a2", "#e45756"];
pub const CDF_DASHES: &[&str] = &["", "7,3", "3,3", "9,4,2,4", "2,2"];

pub const DEFAULT_IEEE_WIDTH: u32 = 252;
pub const DEFAULT_IEEE_HEIGHT: u32 = 216;
pub const DEFAULT_IEEE_HEIGHT_TALL: u32 = 320;
pub const DEFAULT_PUB_WIDTH: u32 = 720;
pub const DEFAULT_PUB_HEIGHT: u32 = 480;
pub const DEFAULT_PUB_HEIGHT_TALL: u32 = 640;
pub const CDF_MAX_POINTS: usize = 150;
pub const VO_DELAY_CDF_X_MAX_MS: f64 = 3.0;
pub const CDF_LEGEND_HORIZONTAL_GAP: f64 = 24.0;
pub const CDF_VEC_SAMPLE_CAP: usize = 4_000;
pub const MIN_BAR_HEIGHT: f64 = 1.0;

pub const STRATEGIES: &[&str] = &["plain", "edca_only", "stable", "guarded", "emergency"];
pub const WORKLOADS: &[&str] = &["low", "medium", "high"];
pub const V2X_STRATEGIES: &[&str] = &["stable", "guarded", "emergency"];

/// Layout presets for exported SVG/PDF figures.
#[derive(Debug, Clone, Copy)]
pub struct FigureStyle {
    pub width: u32,
    pub height: u32,
    pub margin_left: f64,
    pub margin_top: f64,
    pub margin_right: f64,
    pub margin_bottom: f64,
    pub font_axis: u32,
    pub font_tick: u32,
    pub font_category: u32,
    pub font_legend: u32,
    pub font_heatmap: u32,
    pub show_header: bool,
    pub axis_stroke: f64,
    pub line_stroke: f64,
    pub tick_count: u32,
}

impl FigureStyle {
    pub fn ieee_column() -> Self {
        Self {
            width: DEFAULT_IEEE_WIDTH,
            height: DEFAULT_IEEE_HEIGHT,
            margin_left: 48.0,
            margin_top: 10.0,
            margin_right: 10.0,
            margin_bottom: 44.0,
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

    pub fn ieee_column_tall() -> Self {
        let mut style = Self::ieee_column();
        style.height = DEFAULT_IEEE_HEIGHT_TALL;
        style
    }

    /// Default export preset: 720×480, readable labels, legend in a footer band.
    pub fn publication() -> Self {
        Self {
            width: DEFAULT_PUB_WIDTH,
            height: DEFAULT_PUB_HEIGHT,
            margin_left: 78.0,
            margin_top: 28.0,
            margin_right: 28.0,
            margin_bottom: 78.0,
            font_axis: 15,
            font_tick: 13,
            font_category: 13,
            font_legend: 12,
            font_heatmap: 13,
            show_header: false,
            axis_stroke: 1.25,
            line_stroke: 2.5,
            tick_count: 5,
        }
    }

    pub fn publication_tall() -> Self {
        let mut style = Self::publication();
        style.height = DEFAULT_PUB_HEIGHT_TALL;
        style
    }

    pub fn tall_variant(self) -> Self {
        if self.width <= DEFAULT_IEEE_WIDTH + 40 {
            Self::ieee_column_tall()
        } else {
            Self::publication_tall()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Figure {
    pub slug: &'static str,
    pub title: &'static str,
    pub question: &'static str,
    pub body: String,
    pub style: Option<FigureStyle>,
}
