use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CACHE_SCHEMA_VERSION: u32 = 3;
const PARSER_VERSION: &str = "rust-kpi-dashboard-0.2.0";
const CACHE_DIR_NAME: &str = ".kpi_cache_rs";
const AC_INDEX_BE: u8 = 1;
const AC_INDEX_VO: u8 = 3;

macro_rules! define_metrics {
    ($($field:ident),+ $(,)?) => {
        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        pub struct NumericMetrics {
            $(pub $field: Option<f64>,)+
        }

        impl NumericMetrics {
            fn get(&self, name: &str) -> Option<f64> {
                match name {
                    $(stringify!($field) => self.$field,)+
                    _ => None,
                }
            }

            fn add_to_accumulator(&self, accumulator: &mut MetricAccumulator) {
                $(accumulator.$field.add(self.$field);)+
            }
        }

        #[derive(Debug, Clone, Default)]
        struct MetricAccumulator {
            $($field: AverageAccumulator,)+
        }

        impl MetricAccumulator {
            fn finish(&self) -> NumericMetrics {
                NumericMetrics {
                    $($field: self.$field.mean(),)+
                }
            }
        }
    };
}

define_metrics!(
    be_delay_ms,
    be_delay_min_ms,
    be_delay_max_ms,
    be_delay_p95_ms,
    be_jitter_ms,
    vo_delay_ms,
    vo_delay_min_ms,
    vo_delay_max_ms,
    vo_delay_p95_ms,
    vo_jitter_ms,
    be_tx_count,
    be_rx_count,
    vo_tx_count,
    vo_physical_tx_count,
    vo_rx_count,
    be_rx_per_tx,
    vo_rx_per_tx,
    mac_drop_sum_count,
    mac_drop_queue_overflow_count,
    mac_drop_retry_limit_count,
    mac_drop_be_count,
    mac_drop_vo_count,
    mac_drop_unclassified_count,
    mac_drop_be_incorrect_rx_count,
    mac_drop_vo_incorrect_rx_count,
    mac_drop_be_queue_overflow_count,
    mac_drop_vo_queue_overflow_count,
    mac_drop_be_retry_limit_count,
    mac_drop_vo_retry_limit_count,
    mac_drop_be_congestion_count,
    mac_drop_vo_congestion_count,
    mac_drop_vo_per_vo_tx,
    mac_drop_be_per_be_tx,
    mac_drop_vo_queue_overflow_per_vo_tx,
    mac_drop_vo_incorrect_rx_per_vo_tx,
    mac_drop_be_queue_overflow_per_be_tx,
    mac_drop_be_incorrect_rx_per_be_tx,
    mac_drop_per_tx,
    be_dropped_while_blocked_count,
    be_grant_suppressed_while_blocked_count,
    vo_protection_activation_count,
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRow {
    pub config: String,
    pub run: String,
    pub source_file: String,
    #[serde(flatten)]
    pub metrics: NumericMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSummary {
    pub config: String,
    pub runs: usize,
    #[serde(flatten)]
    pub metrics: NumericMetrics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSignature {
    pub name: String,
    pub size: u64,
    pub mtime_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMeta {
    pub schema_version: u32,
    pub parser_version: String,
    pub built_at_unix_secs: u64,
    pub source_files: Vec<SourceSignature>,
    pub run_count: usize,
    pub config_count: usize,
    pub source_kind: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub results_dir: String,
    pub cache_dir: String,
    pub cache_state: String,
    pub source_kind: String,
    pub run_count: usize,
    pub config_count: usize,
    pub built_at_unix_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardDataset {
    pub run_rows: Vec<RunRow>,
    pub config_summary: Vec<ConfigSummary>,
    pub cache_info: CacheInfo,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table<T> {
    pub columns: Vec<Column>,
    pub rows: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildStatus {
    pub running: bool,
    pub message: Option<String>,
}

impl RebuildStatus {
    pub fn idle() -> Self {
        Self {
            running: false,
            message: None,
        }
    }

    pub fn running(message: impl Into<String>) -> Self {
        Self {
            running: true,
            message: Some(message.into()),
        }
    }

    pub fn finished(message: impl Into<String>) -> Self {
        Self {
            running: false,
            message: Some(message.into()),
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            running: false,
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DensityOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub cache_info: CacheInfo,
    pub warnings: Vec<String>,
    pub rebuild: RebuildStatus,
    pub density: String,
    pub density_options: Vec<DensityOption>,
    pub baseline: Option<String>,
    pub baseline_options: Vec<String>,
    pub comparison: Table<BTreeMap<String, Value>>,
    pub config_summary: Table<ConfigSummary>,
    pub run_details: Table<RunRow>,
    pub v2x_matrix: Table<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone)]
pub struct ResultsPackage {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct StartupDataset {
    pub dataset: DashboardDataset,
    pub spawn_rebuild: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct AverageAccumulator {
    sum: f64,
    count: usize,
}

impl AverageAccumulator {
    fn add(&mut self, value: Option<f64>) {
        if let Some(value) = value.filter(|value| value.is_finite()) {
            self.sum += value;
            self.count += 1;
        }
    }

    fn mean(&self) -> Option<f64> {
        (self.count > 0).then_some(self.sum / self.count as f64)
    }
}

#[derive(Debug, Clone, Default)]
struct VecMetrics {
    be_delay_p95_s: Option<f64>,
    be_jitter_s: Option<f64>,
    vo_delay_p95_s: Option<f64>,
    vo_jitter_s: Option<f64>,
    be_headers: usize,
    vo_headers: usize,
    be_samples: usize,
    vo_samples: usize,
}

#[derive(Debug, Clone, Default)]
struct RunParseWarnings {
    messages: Vec<String>,
}

impl RunParseWarnings {
    fn push(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }
}

pub fn simulations_results_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("veins_qos")
        .join("simulations")
}

pub fn density_id_from_path(path: &Path) -> String {
    for component in path.components().rev() {
        let name = component.as_os_str().to_string_lossy();
        if name.contains("highway_light") {
            return "highway_light".to_string();
        }
        if name.contains("highway_heavy") {
            return "highway_heavy".to_string();
        }
    }
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "custom".to_string())
}

pub fn density_label(id: &str) -> String {
    match id {
        "highway_light" => "Highway light (10 vehicles)".to_string(),
        "highway_heavy" => "Highway heavy (100 vehicles)".to_string(),
        other => format!("Custom ({other})"),
    }
}

/// Discover highway result packages. When `explicit` is set, only that directory is used.
pub fn discover_results_packages(explicit: Option<PathBuf>) -> Vec<ResultsPackage> {
    if let Some(path) = explicit {
        let id = density_id_from_path(&path);
        return vec![ResultsPackage {
            label: density_label(&id),
            id,
            path,
        }];
    }

    let simulations_dir = simulations_results_root();
    let candidates = [
        (
            "veins_inet_highway_light",
            "highway_light",
            "Highway light (10 vehicles)",
        ),
        (
            "veins_inet_highway_heavy",
            "highway_heavy",
            "Highway heavy (100 vehicles)",
        ),
    ];
    let mut packages = Vec::new();
    for (subdir, id, label) in candidates {
        let path = simulations_dir.join(subdir).join("results");
        if path.exists() {
            packages.push(ResultsPackage {
                id: id.to_string(),
                label: label.to_string(),
                path,
            });
        }
    }
    packages
}

pub fn default_results_dir() -> PathBuf {
    discover_results_packages(None)
        .first()
        .map(|package| package.path.clone())
        .unwrap_or_else(|| {
            simulations_results_root()
                .join("veins_inet_highway_heavy")
                .join("results")
        })
}

pub fn load_startup_dataset(
    results_dir: &Path,
    rebuild: bool,
    threads: Option<usize>,
) -> Result<StartupDataset> {
    if rebuild {
        return Ok(StartupDataset {
            dataset: rebuild_raw_dataset(results_dir, threads)?,
            spawn_rebuild: false,
        });
    }

    if let Some(dataset) = try_load_valid_rust_cache(results_dir)? {
        return Ok(StartupDataset {
            dataset,
            spawn_rebuild: false,
        });
    }

    Ok(StartupDataset {
        dataset: rebuild_raw_dataset(results_dir, threads)?,
        spawn_rebuild: false,
    })
}

pub fn rebuild_raw_dataset(results_dir: &Path, threads: Option<usize>) -> Result<DashboardDataset> {
    let (run_rows, warnings) = build_run_rows_from_raw(results_dir, threads)?;
    let config_summary = build_config_summary(&run_rows);
    let meta = write_rust_cache(results_dir, &run_rows, &config_summary, "raw", &warnings)?;
    Ok(dataset_from_parts(
        results_dir,
        "cache rebuilt",
        meta,
        run_rows,
        config_summary,
        warnings,
    ))
}

pub fn build_dashboard_response(
    dataset: &DashboardDataset,
    density: &str,
    density_options: &[DensityOption],
    requested_baseline: &str,
    rebuild: RebuildStatus,
) -> DashboardResponse {
    let mut warnings = dataset.warnings.clone();
    if let Some(message) = rebuild.message.clone() {
        warnings.push(message);
    }

    let baseline_options = baseline_option_values(&dataset.config_summary);
    let (comparison_rows, baseline) =
        build_comparison_rows(&dataset.config_summary, requested_baseline);
    let v2x_rows = build_v2x_variant_matrix(&dataset.config_summary, baseline.as_deref());
    let v2x_columns = v2x_matrix_columns(&v2x_rows, baseline.as_deref());

    DashboardResponse {
        cache_info: dataset.cache_info.clone(),
        warnings,
        rebuild,
        density: density.to_string(),
        density_options: density_options.to_vec(),
        baseline,
        baseline_options,
        comparison: Table {
            columns: visible_columns(&comparison_rows, COMPARISON_COLUMNS, &ALWAYS_VISIBLE),
            rows: comparison_rows,
        },
        config_summary: Table {
            columns: visible_columns(
                &high_load_only_or_all(&dataset.config_summary),
                CONFIG_SUMMARY_TABLE_COLUMNS,
                &ALWAYS_VISIBLE,
            ),
            rows: high_load_only_or_all(&dataset.config_summary),
        },
        run_details: Table {
            columns: visible_columns(&dataset.run_rows, RUN_ROW_COLUMNS, &ALWAYS_VISIBLE),
            rows: sorted_run_rows(&dataset.run_rows),
        },
        v2x_matrix: Table {
            columns: v2x_columns,
            rows: v2x_rows,
        },
    }
}

pub fn try_load_valid_rust_cache(results_dir: &Path) -> Result<Option<DashboardDataset>> {
    let meta_path = rust_cache_dir(results_dir).join("meta.json");
    let run_rows_path = rust_cache_dir(results_dir).join("run_rows.json");
    let config_summary_path = rust_cache_dir(results_dir).join("config_summary.json");
    if !meta_path.exists() || !run_rows_path.exists() || !config_summary_path.exists() {
        return Ok(None);
    }

    let meta: CacheMeta = read_json(&meta_path)?;
    if meta.schema_version != CACHE_SCHEMA_VERSION
        || meta.parser_version != PARSER_VERSION
        || meta.source_files != source_signatures(results_dir)?
    {
        return Ok(None);
    }

    let run_rows: Vec<RunRow> = read_json(&run_rows_path)?;
    let config_summary: Vec<ConfigSummary> = read_json(&config_summary_path)?;
    let warnings = meta.warnings.clone();

    Ok(Some(dataset_from_parts(
        results_dir,
        "cache hit",
        meta,
        run_rows,
        config_summary,
        warnings,
    )))
}

fn build_run_rows_from_raw(
    results_dir: &Path,
    threads: Option<usize>,
) -> Result<(Vec<RunRow>, Vec<String>)> {
    if !results_dir.exists() {
        return Err(anyhow!(
            "results directory not found: {}",
            results_dir.display()
        ));
    }

    let mut sca_files = files_with_extension(results_dir, "sca")?;
    if sca_files.is_empty() {
        return Err(anyhow!("no .sca files found in {}", results_dir.display()));
    }
    sca_files.sort();

    let parse_all = || -> Result<Vec<(RunRow, RunParseWarnings)>> {
        sca_files
            .par_iter()
            .map(|path| parse_run_results(path))
            .collect::<Result<Vec<_>>>()
    };

    let parsed = if let Some(threads) = threads.filter(|threads| *threads > 0) {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .context("failed to build Rayon thread pool")?
            .install(parse_all)?
    } else {
        parse_all()?
    };

    let mut rows = Vec::with_capacity(parsed.len());
    let mut warnings = Vec::new();
    for (row, row_warnings) in parsed {
        rows.push(row);
        warnings.extend(row_warnings.messages);
    }
    rows.sort_by(compare_run_row_values);
    Ok((rows, cap_warnings(warnings)))
}

fn parse_run_results(sca_path: &Path) -> Result<(RunRow, RunParseWarnings)> {
    let mut warnings = RunParseWarnings::default();
    let vec_path = sca_path.with_extension("vec");
    let vec_metrics = if vec_path.exists() {
        parse_vec_metrics(&vec_path)?
    } else {
        warnings.push(format!(
            "{} has no matching .vec file; P95/jitter will be N/A",
            path_file_name(sca_path)
        ));
        VecMetrics::default()
    };

    if vec_metrics.be_headers > 0 && vec_metrics.be_samples == 0 {
        warnings.push(format!(
            "{} contains BE delay vector headers but no samples",
            path_file_name(&vec_path)
        ));
    }
    if vec_metrics.vo_headers > 0 && vec_metrics.vo_samples == 0 {
        warnings.push(format!(
            "{} contains VO delay vector headers but no samples",
            path_file_name(&vec_path)
        ));
    }

    Ok((parse_sca_file(sca_path, &vec_metrics)?, warnings))
}

fn parse_vec_metrics(path: &Path) -> Result<VecMetrics> {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum DelayMetric {
        Be,
        Vo,
    }

    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut metric_by_vector_id: HashMap<String, DelayMetric> = HashMap::new();
    let mut last_value_by_vector_id: HashMap<String, f64> = HashMap::new();
    let mut be_values = Vec::new();
    let mut vo_values = Vec::new();
    let mut be_jitter_sum = 0.0;
    let mut vo_jitter_sum = 0.0;
    let mut be_jitter_count = 0usize;
    let mut vo_jitter_count = 0usize;
    let mut metrics = VecMetrics::default();

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
                metric_by_vector_id.insert(vector_id.to_string(), DelayMetric::Be);
                metrics.be_headers += 1;
            } else if is_node_app(module, 0) && metric == "voEndToEndDelay:vector" {
                metric_by_vector_id.insert(vector_id.to_string(), DelayMetric::Vo);
                metrics.vo_headers += 1;
            }
            continue;
        }

        let Some(metric) = metric_by_vector_id.get(first).copied() else {
            continue;
        };
        let _event = parts.next();
        let _time = parts.next();
        let Some(value_raw) = parts.next() else {
            continue;
        };
        let Some(value) = parse_finite(value_raw) else {
            continue;
        };

        if let Some(previous) = last_value_by_vector_id.insert(first.to_string(), value) {
            match metric {
                DelayMetric::Be => {
                    be_jitter_sum += (value - previous).abs();
                    be_jitter_count += 1;
                }
                DelayMetric::Vo => {
                    vo_jitter_sum += (value - previous).abs();
                    vo_jitter_count += 1;
                }
            }
        }

        match metric {
            DelayMetric::Be => {
                metrics.be_samples += 1;
                be_values.push(value);
            }
            DelayMetric::Vo => {
                metrics.vo_samples += 1;
                vo_values.push(value);
            }
        }
    }

    metrics.be_delay_p95_s = percentile(&mut be_values, 0.95);
    metrics.vo_delay_p95_s = percentile(&mut vo_values, 0.95);
    metrics.be_jitter_s = (be_jitter_count > 0).then_some(be_jitter_sum / be_jitter_count as f64);
    metrics.vo_jitter_s = (vo_jitter_count > 0).then_some(vo_jitter_sum / vo_jitter_count as f64);
    Ok(metrics)
}

fn parse_sca_file(path: &Path, vec_metrics: &VecMetrics) -> Result<RunRow> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut config = path_file_stem(path);
    let mut run = config.clone();

    let mut be_tx_total = 0.0;
    let mut be_rx_total = 0.0;
    let mut vo_logical_tx_total = 0.0;
    let mut vo_physical_tx_total = 0.0;
    let mut vo_rx_total = 0.0;

    let mut be_delay_count_by_module: HashMap<String, f64> = HashMap::new();
    let mut be_delay_mean_by_module: HashMap<String, f64> = HashMap::new();
    let mut be_delay_min_values = Vec::new();
    let mut be_delay_max_values = Vec::new();
    let mut vo_delay_count_by_module: HashMap<String, f64> = HashMap::new();
    let mut vo_delay_mean_by_module: HashMap<String, f64> = HashMap::new();
    let mut vo_delay_min_values = Vec::new();
    let mut vo_delay_max_values = Vec::new();

    let mut mac_drop_total = 0.0;
    let mut mac_drop_queue_overflow_total = 0.0;
    let mut mac_drop_retry_limit_total = 0.0;
    let mut mac_drop_be_queue_overflow_total = 0.0;
    let mut mac_drop_be_retry_limit_total = 0.0;
    let mut mac_drop_vo_queue_overflow_total = 0.0;
    let mut mac_drop_vo_retry_limit_total = 0.0;
    let mut mac_drop_be_total_from_mac = 0.0;
    let mut mac_drop_vo_total_from_mac = 0.0;
    let mut mac_drop_unclassified_total = 0.0;
    let mut mac_drop_be_incorrect_rx_total = 0.0;
    let mut mac_drop_vo_incorrect_rx_total = 0.0;
    let mut mac_drop_be_queue_overflow_from_mac = 0.0;
    let mut mac_drop_vo_queue_overflow_from_mac = 0.0;
    let mut mac_drop_be_retry_limit_from_mac = 0.0;
    let mut mac_drop_vo_retry_limit_from_mac = 0.0;
    let mut mac_drop_be_congestion_total = 0.0;
    let mut mac_drop_vo_congestion_total = 0.0;
    let mut saw_be_ac_metrics_from_mac = false;
    let mut saw_vo_ac_metrics_from_mac = false;
    let mut saw_unclassified_ac_metrics = false;
    let mut saw_be_incorrect_rx_metrics = false;
    let mut saw_vo_incorrect_rx_metrics = false;
    let mut saw_be_queue_overflow_metrics_from_mac = false;
    let mut saw_vo_queue_overflow_metrics_from_mac = false;
    let mut saw_be_retry_limit_metrics_from_mac = false;
    let mut saw_vo_retry_limit_metrics_from_mac = false;
    let mut saw_be_congestion_metrics = false;
    let mut saw_vo_congestion_metrics = false;
    let mut saw_be_ac_metrics = false;
    let mut saw_vo_ac_metrics = false;

    let mut be_dropped_while_blocked_stat_total = 0.0;
    let mut be_grant_suppressed_while_blocked_stat_total = 0.0;
    let mut vo_protection_activation_stat_total = 0.0;
    let mut be_dropped_while_blocked_scalar_total = 0.0;
    let mut be_grant_suppressed_while_blocked_scalar_total = 0.0;
    let mut vo_protection_activation_scalar_total = 0.0;
    let mut saw_be_dropped_while_blocked_stat = false;
    let mut saw_be_grant_suppressed_while_blocked_stat = false;
    let mut saw_vo_protection_activation_stat = false;
    let mut saw_be_dropped_while_blocked_scalar = false;
    let mut saw_be_grant_suppressed_while_blocked_scalar = false;
    let mut saw_vo_protection_activation_scalar = false;

    for line in reader.lines() {
        let line = line?;
        if let Some(rest) = line.strip_prefix("attr configname ") {
            config = rest.trim().to_string();
            continue;
        }
        if let Some(rest) = line.strip_prefix("run ") {
            run = rest.trim().to_string();
            continue;
        }

        let mut parts = line.split_whitespace();
        if parts.next() != Some("scalar") {
            continue;
        }
        let Some(module) = parts.next() else {
            continue;
        };
        let Some(metric) = parts.next() else {
            continue;
        };
        let Some(value_raw) = parts.next() else {
            continue;
        };
        let Some(value) = parse_finite(value_raw) else {
            continue;
        };

        if is_node_app(module, 0) {
            match metric {
                "beTxPackets:count" => be_tx_total += value,
                "beRxPackets:count" => be_rx_total += value,
                "voRxPackets:count" => vo_rx_total += value,
                "beEndToEndDelay:count" => {
                    be_delay_count_by_module.insert(module.to_string(), value);
                }
                "beEndToEndDelay:mean" => {
                    be_delay_mean_by_module.insert(module.to_string(), value);
                }
                "beEndToEndDelay:min" => be_delay_min_values.push(value),
                "beEndToEndDelay:max" => be_delay_max_values.push(value),
                "voEndToEndDelay:count" => {
                    vo_delay_count_by_module.insert(module.to_string(), value);
                }
                "voEndToEndDelay:mean" => {
                    vo_delay_mean_by_module.insert(module.to_string(), value);
                }
                "voEndToEndDelay:min" => vo_delay_min_values.push(value),
                "voEndToEndDelay:max" => vo_delay_max_values.push(value),
                _ => {}
            }
        } else if is_node_app(module, 1) {
            match metric {
                "voTxPackets:count" => vo_physical_tx_total += value,
                "voLogicalTxPackets:count" => vo_logical_tx_total += value,
                _ => {}
            }
        } else if is_node_mac(module) {
            match metric {
                "packetDrop:count" => mac_drop_total += value,
                "packetDropQueueOverflow:count" => mac_drop_queue_overflow_total += value,
                "packetDropRetryLimitReached:count" => mac_drop_retry_limit_total += value,
                "packetDropAcBeReasonIncorrectlyReceivedCount" => {
                    saw_be_incorrect_rx_metrics = true;
                    mac_drop_be_incorrect_rx_total += value;
                }
                "packetDropAcVoReasonIncorrectlyReceivedCount" => {
                    saw_vo_incorrect_rx_metrics = true;
                    mac_drop_vo_incorrect_rx_total += value;
                }
                "packetDropAcBeReasonQueueOverflowCount" => {
                    saw_be_queue_overflow_metrics_from_mac = true;
                    mac_drop_be_queue_overflow_from_mac += value;
                }
                "packetDropAcVoReasonQueueOverflowCount" => {
                    saw_vo_queue_overflow_metrics_from_mac = true;
                    mac_drop_vo_queue_overflow_from_mac += value;
                }
                "packetDropAcBeReasonRetryLimitReachedCount" => {
                    saw_be_retry_limit_metrics_from_mac = true;
                    mac_drop_be_retry_limit_from_mac += value;
                }
                "packetDropAcVoReasonRetryLimitReachedCount" => {
                    saw_vo_retry_limit_metrics_from_mac = true;
                    mac_drop_vo_retry_limit_from_mac += value;
                }
                "packetDropAcBeReasonCongestionCount" => {
                    saw_be_congestion_metrics = true;
                    mac_drop_be_congestion_total += value;
                }
                "packetDropAcVoReasonCongestionCount" => {
                    saw_vo_congestion_metrics = true;
                    mac_drop_vo_congestion_total += value;
                }
                _ => {
                    if let Some(ac) = parse_packet_drop_ac(metric) {
                        match ac {
                            "Be" => {
                                saw_be_ac_metrics_from_mac = true;
                                mac_drop_be_total_from_mac += value;
                            }
                            "Vo" => {
                                saw_vo_ac_metrics_from_mac = true;
                                mac_drop_vo_total_from_mac += value;
                            }
                            "Unclassified" => {
                                saw_unclassified_ac_metrics = true;
                                mac_drop_unclassified_total += value;
                            }
                            _ => {}
                        }
                    }
                }
            }
        } else if is_node_hcf(module) {
            match metric {
                "beDroppedWhileBlocked:count" => {
                    saw_be_dropped_while_blocked_stat = true;
                    be_dropped_while_blocked_stat_total += value;
                }
                "beGrantSuppressedWhileBlocked:count" => {
                    saw_be_grant_suppressed_while_blocked_stat = true;
                    be_grant_suppressed_while_blocked_stat_total += value;
                }
                "voProtectionActivation:count" => {
                    saw_vo_protection_activation_stat = true;
                    vo_protection_activation_stat_total += value;
                }
                "beDroppedWhileBlockedCount" => {
                    saw_be_dropped_while_blocked_scalar = true;
                    be_dropped_while_blocked_scalar_total += value;
                }
                "beGrantSuppressedWhileBlockedCount" => {
                    saw_be_grant_suppressed_while_blocked_scalar = true;
                    be_grant_suppressed_while_blocked_scalar_total += value;
                }
                "voProtectionActivationCount" => {
                    saw_vo_protection_activation_scalar = true;
                    vo_protection_activation_scalar_total += value;
                }
                _ => {}
            }
        } else if let Some(ac_index) = edcaf_index(module, "pendingQueue") {
            if metric == "droppedPacketsQueueOverflow:count" {
                if ac_index == AC_INDEX_BE {
                    saw_be_ac_metrics = true;
                    mac_drop_be_queue_overflow_total += value;
                } else if ac_index == AC_INDEX_VO {
                    saw_vo_ac_metrics = true;
                    mac_drop_vo_queue_overflow_total += value;
                }
            }
        } else if let Some(ac_index) = edcaf_index(module, "recoveryProcedure") {
            if metric == "retryLimitReached:count" {
                if ac_index == AC_INDEX_BE {
                    saw_be_ac_metrics = true;
                    mac_drop_be_retry_limit_total += value;
                } else if ac_index == AC_INDEX_VO {
                    saw_vo_ac_metrics = true;
                    mac_drop_vo_retry_limit_total += value;
                }
            }
        }
    }

    let be_delay_s = weighted_mean(&be_delay_count_by_module, &be_delay_mean_by_module);
    let vo_delay_s = weighted_mean(&vo_delay_count_by_module, &vo_delay_mean_by_module);

    let mac_drop_be_fallback = mac_drop_be_queue_overflow_total + mac_drop_be_retry_limit_total;
    let mac_drop_vo_fallback = mac_drop_vo_queue_overflow_total + mac_drop_vo_retry_limit_total;
    let mut mac_drop_be_value = if saw_be_ac_metrics_from_mac {
        Some(mac_drop_be_total_from_mac)
    } else if saw_be_ac_metrics {
        Some(mac_drop_be_fallback)
    } else {
        None
    };
    let mut mac_drop_vo_value = if saw_vo_ac_metrics_from_mac {
        Some(mac_drop_vo_total_from_mac)
    } else if saw_vo_ac_metrics {
        Some(mac_drop_vo_fallback)
    } else {
        None
    };
    let mut mac_drop_unclassified_value =
        saw_unclassified_ac_metrics.then_some(mac_drop_unclassified_total);

    let attributed_drop_sum: f64 = [
        mac_drop_be_value,
        mac_drop_vo_value,
        mac_drop_unclassified_value,
    ]
    .into_iter()
    .flatten()
    .sum();
    let mut drop_attribution_scale = 1.0;
    if mac_drop_total > 0.0 && attributed_drop_sum > 0.0 {
        let ratio = attributed_drop_sum / mac_drop_total;
        if (1.9..=2.1).contains(&ratio) {
            drop_attribution_scale = 0.5;
            mac_drop_be_value = mac_drop_be_value.map(|value| value / 2.0);
            mac_drop_vo_value = mac_drop_vo_value.map(|value| value / 2.0);
            mac_drop_unclassified_value = mac_drop_unclassified_value.map(|value| value / 2.0);
        }
    }

    let mac_drop_be_count = mac_drop_be_value.map(|value| value.round());
    let mac_drop_vo_count = mac_drop_vo_value.map(|value| value.round());
    let mac_drop_unclassified_count = if let Some(value) = mac_drop_unclassified_value {
        Some(value.round())
    } else {
        let no_be_attribution = mac_drop_be_count.unwrap_or(0.0) == 0.0;
        let no_vo_attribution = mac_drop_vo_count.unwrap_or(0.0) == 0.0;
        if mac_drop_total > 0.0 && no_be_attribution && no_vo_attribution {
            Some(mac_drop_total.round())
        } else {
            Some(0.0)
        }
    };

    let mac_drop_be_incorrect_rx_count = saw_be_incorrect_rx_metrics
        .then_some((mac_drop_be_incorrect_rx_total * drop_attribution_scale).round());
    let mac_drop_vo_incorrect_rx_count = saw_vo_incorrect_rx_metrics
        .then_some((mac_drop_vo_incorrect_rx_total * drop_attribution_scale).round());
    let mac_drop_be_queue_overflow_count = if saw_be_queue_overflow_metrics_from_mac {
        Some((mac_drop_be_queue_overflow_from_mac * drop_attribution_scale).round())
    } else if saw_be_ac_metrics {
        Some(mac_drop_be_queue_overflow_total.round())
    } else {
        None
    };
    let mac_drop_vo_queue_overflow_count = if saw_vo_queue_overflow_metrics_from_mac {
        Some((mac_drop_vo_queue_overflow_from_mac * drop_attribution_scale).round())
    } else if saw_vo_ac_metrics {
        Some(mac_drop_vo_queue_overflow_total.round())
    } else {
        None
    };
    let mac_drop_be_retry_limit_count = if saw_be_retry_limit_metrics_from_mac {
        Some((mac_drop_be_retry_limit_from_mac * drop_attribution_scale).round())
    } else if saw_be_ac_metrics {
        Some(mac_drop_be_retry_limit_total.round())
    } else {
        None
    };
    let mac_drop_vo_retry_limit_count = if saw_vo_retry_limit_metrics_from_mac {
        Some((mac_drop_vo_retry_limit_from_mac * drop_attribution_scale).round())
    } else if saw_vo_ac_metrics {
        Some(mac_drop_vo_retry_limit_total.round())
    } else {
        None
    };
    let mac_drop_be_congestion_count = saw_be_congestion_metrics
        .then_some((mac_drop_be_congestion_total * drop_attribution_scale).round());
    let mac_drop_vo_congestion_count = saw_vo_congestion_metrics
        .then_some((mac_drop_vo_congestion_total * drop_attribution_scale).round());

    let vo_tx_total = if vo_logical_tx_total > 0.0 {
        vo_logical_tx_total
    } else {
        vo_physical_tx_total
    };
    let app_tx_total = be_tx_total + vo_physical_tx_total;
    let be_dropped_while_blocked_total = if saw_be_dropped_while_blocked_stat {
        Some(be_dropped_while_blocked_stat_total)
    } else if saw_be_dropped_while_blocked_scalar {
        Some(be_dropped_while_blocked_scalar_total)
    } else {
        None
    };
    let be_grant_suppressed_while_blocked_total = if saw_be_grant_suppressed_while_blocked_stat {
        Some(be_grant_suppressed_while_blocked_stat_total)
    } else if saw_be_grant_suppressed_while_blocked_scalar {
        Some(be_grant_suppressed_while_blocked_scalar_total)
    } else {
        None
    };
    let vo_protection_activation_total = if saw_vo_protection_activation_stat {
        Some(vo_protection_activation_stat_total)
    } else if saw_vo_protection_activation_scalar {
        Some(vo_protection_activation_scalar_total)
    } else {
        None
    };

    Ok(RunRow {
        config,
        run,
        source_file: path_file_name(path),
        metrics: NumericMetrics {
            be_delay_ms: be_delay_s.map(seconds_to_ms),
            be_delay_min_ms: finite_min(&be_delay_min_values).map(seconds_to_ms),
            be_delay_max_ms: finite_max(&be_delay_max_values).map(seconds_to_ms),
            be_delay_p95_ms: vec_metrics.be_delay_p95_s.map(seconds_to_ms),
            be_jitter_ms: vec_metrics.be_jitter_s.map(seconds_to_ms),
            vo_delay_ms: vo_delay_s.map(seconds_to_ms),
            vo_delay_min_ms: finite_min(&vo_delay_min_values).map(seconds_to_ms),
            vo_delay_max_ms: finite_max(&vo_delay_max_values).map(seconds_to_ms),
            vo_delay_p95_ms: vec_metrics.vo_delay_p95_s.map(seconds_to_ms),
            vo_jitter_ms: vec_metrics.vo_jitter_s.map(seconds_to_ms),
            be_tx_count: Some(be_tx_total.round()),
            be_rx_count: Some(be_rx_total.round()),
            vo_tx_count: Some(vo_tx_total.round()),
            vo_physical_tx_count: Some(vo_physical_tx_total.round()),
            vo_rx_count: Some(vo_rx_total.round()),
            be_rx_per_tx: ratio(be_rx_total, be_tx_total),
            vo_rx_per_tx: ratio(vo_rx_total, vo_tx_total),
            mac_drop_sum_count: Some(mac_drop_total.round()),
            mac_drop_queue_overflow_count: Some(mac_drop_queue_overflow_total.round()),
            mac_drop_retry_limit_count: Some(mac_drop_retry_limit_total.round()),
            mac_drop_be_count,
            mac_drop_vo_count,
            mac_drop_unclassified_count,
            mac_drop_be_incorrect_rx_count,
            mac_drop_vo_incorrect_rx_count,
            mac_drop_be_queue_overflow_count,
            mac_drop_vo_queue_overflow_count,
            mac_drop_be_retry_limit_count,
            mac_drop_vo_retry_limit_count,
            mac_drop_be_congestion_count,
            mac_drop_vo_congestion_count,
            mac_drop_vo_per_vo_tx: ratio_optional(mac_drop_vo_count, vo_physical_tx_total),
            mac_drop_be_per_be_tx: ratio_optional(mac_drop_be_count, be_tx_total),
            mac_drop_vo_queue_overflow_per_vo_tx: ratio_optional(
                mac_drop_vo_queue_overflow_count,
                vo_physical_tx_total,
            ),
            mac_drop_vo_incorrect_rx_per_vo_tx: ratio_optional(
                mac_drop_vo_incorrect_rx_count,
                vo_physical_tx_total,
            ),
            mac_drop_be_queue_overflow_per_be_tx: ratio_optional(
                mac_drop_be_queue_overflow_count,
                be_tx_total,
            ),
            mac_drop_be_incorrect_rx_per_be_tx: ratio_optional(
                mac_drop_be_incorrect_rx_count,
                be_tx_total,
            ),
            mac_drop_per_tx: ratio(mac_drop_total, app_tx_total),
            be_dropped_while_blocked_count: be_dropped_while_blocked_total
                .map(|value| value.round()),
            be_grant_suppressed_while_blocked_count: be_grant_suppressed_while_blocked_total
                .map(|value| value.round()),
            vo_protection_activation_count: vo_protection_activation_total
                .map(|value| value.round()),
        },
    })
}

fn build_config_summary(rows: &[RunRow]) -> Vec<ConfigSummary> {
    let mut by_config: BTreeMap<String, (usize, MetricAccumulator)> = BTreeMap::new();
    for row in rows {
        let entry = by_config
            .entry(row.config.clone())
            .or_insert_with(|| (0, MetricAccumulator::default()));
        entry.0 += 1;
        row.metrics.add_to_accumulator(&mut entry.1);
    }

    let mut summaries: Vec<ConfigSummary> = by_config
        .into_iter()
        .map(|(config, (runs, accumulator))| ConfigSummary {
            config,
            runs,
            metrics: accumulator.finish(),
        })
        .collect();
    summaries.sort_by(|left, right| compare_configs(&left.config, &right.config));
    summaries
}

fn build_comparison_rows(
    config_summary: &[ConfigSummary],
    requested_baseline: &str,
) -> (Vec<BTreeMap<String, Value>>, Option<String>) {
    let comparison_source = high_load_only_or_all(config_summary);
    if comparison_source.is_empty() {
        return (Vec::new(), None);
    }

    let baseline = if comparison_source
        .iter()
        .any(|summary| summary.config == requested_baseline)
    {
        requested_baseline.to_string()
    } else {
        preferred_baseline(&comparison_source).unwrap_or_default()
    };
    if baseline.is_empty() {
        return (Vec::new(), None);
    }
    let Some(base) = comparison_source
        .iter()
        .find(|summary| summary.config == baseline)
    else {
        return (Vec::new(), None);
    };
    let base_metrics = base.metrics.clone();

    let mut rows = Vec::new();
    for summary in &comparison_source {
        let mut row = BTreeMap::new();
        insert_string(&mut row, "config", &summary.config);
        insert_number(&mut row, "runs", Some(summary.runs as f64));
        insert_string(&mut row, "baseline", &baseline);
        add_comparison_metric(
            &mut row,
            "vo_delay_ms",
            "vo_delay_delta_ms",
            "vo_delay_delta_pct",
            summary.metrics.vo_delay_ms,
            base_metrics.vo_delay_ms,
        );
        add_comparison_metric(
            &mut row,
            "vo_delay_p95_ms",
            "vo_delay_p95_delta_ms",
            "vo_delay_p95_delta_pct",
            summary.metrics.vo_delay_p95_ms,
            base_metrics.vo_delay_p95_ms,
        );
        add_comparison_metric(
            &mut row,
            "vo_jitter_ms",
            "vo_jitter_delta_ms",
            "vo_jitter_delta_pct",
            summary.metrics.vo_jitter_ms,
            base_metrics.vo_jitter_ms,
        );
        add_comparison_metric(
            &mut row,
            "vo_rx_per_tx",
            "vo_rx_per_tx_delta",
            "vo_rx_per_tx_delta_pct",
            summary.metrics.vo_rx_per_tx,
            base_metrics.vo_rx_per_tx,
        );
        add_comparison_metric(
            &mut row,
            "be_delay_ms",
            "be_delay_delta_ms",
            "be_delay_delta_pct",
            summary.metrics.be_delay_ms,
            base_metrics.be_delay_ms,
        );
        add_comparison_metric(
            &mut row,
            "be_delay_p95_ms",
            "be_delay_p95_delta_ms",
            "be_delay_p95_delta_pct",
            summary.metrics.be_delay_p95_ms,
            base_metrics.be_delay_p95_ms,
        );
        add_comparison_metric(
            &mut row,
            "be_jitter_ms",
            "be_jitter_delta_ms",
            "be_jitter_delta_pct",
            summary.metrics.be_jitter_ms,
            base_metrics.be_jitter_ms,
        );
        add_comparison_metric(
            &mut row,
            "be_rx_per_tx",
            "be_rx_per_tx_delta",
            "be_rx_per_tx_delta_pct",
            summary.metrics.be_rx_per_tx,
            base_metrics.be_rx_per_tx,
        );
        add_comparison_metric(
            &mut row,
            "mac_drop_sum_count",
            "mac_drop_delta_count",
            "mac_drop_delta_pct",
            summary.metrics.mac_drop_sum_count,
            base_metrics.mac_drop_sum_count,
        );
        add_comparison_metric(
            &mut row,
            "mac_drop_vo_incorrect_rx_count",
            "mac_drop_vo_incorrect_rx_delta_count",
            "mac_drop_vo_incorrect_rx_delta_pct",
            summary.metrics.mac_drop_vo_incorrect_rx_count,
            base_metrics.mac_drop_vo_incorrect_rx_count,
        );
        add_comparison_metric(
            &mut row,
            "mac_drop_vo_queue_overflow_count",
            "mac_drop_vo_queue_overflow_delta_count",
            "mac_drop_vo_queue_overflow_delta_pct",
            summary.metrics.mac_drop_vo_queue_overflow_count,
            base_metrics.mac_drop_vo_queue_overflow_count,
        );
        add_comparison_metric(
            &mut row,
            "mac_drop_be_incorrect_rx_count",
            "mac_drop_be_incorrect_rx_delta_count",
            "mac_drop_be_incorrect_rx_delta_pct",
            summary.metrics.mac_drop_be_incorrect_rx_count,
            base_metrics.mac_drop_be_incorrect_rx_count,
        );
        add_comparison_metric(
            &mut row,
            "mac_drop_be_queue_overflow_count",
            "mac_drop_be_queue_overflow_delta_count",
            "mac_drop_be_queue_overflow_delta_pct",
            summary.metrics.mac_drop_be_queue_overflow_count,
            base_metrics.mac_drop_be_queue_overflow_count,
        );
        add_comparison_metric(
            &mut row,
            "mac_drop_per_tx",
            "mac_drop_per_tx_delta",
            "mac_drop_per_tx_delta_pct",
            summary.metrics.mac_drop_per_tx,
            base_metrics.mac_drop_per_tx,
        );
        add_comparison_metric(
            &mut row,
            "be_dropped_while_blocked_count",
            "be_dropped_while_blocked_delta_count",
            "be_dropped_while_blocked_delta_pct",
            summary.metrics.be_dropped_while_blocked_count,
            base_metrics.be_dropped_while_blocked_count,
        );
        add_comparison_metric(
            &mut row,
            "be_grant_suppressed_while_blocked_count",
            "be_grant_suppressed_while_blocked_delta_count",
            "be_grant_suppressed_while_blocked_delta_pct",
            summary.metrics.be_grant_suppressed_while_blocked_count,
            base_metrics.be_grant_suppressed_while_blocked_count,
        );
        add_comparison_metric(
            &mut row,
            "vo_protection_activation_count",
            "vo_protection_activation_delta_count",
            "vo_protection_activation_delta_pct",
            summary.metrics.vo_protection_activation_count,
            base_metrics.vo_protection_activation_count,
        );
        rows.push(row);
    }
    rows.sort_by(|left, right| {
        let left = left
            .get("config")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let right = right
            .get("config")
            .and_then(Value::as_str)
            .unwrap_or_default();
        compare_configs(left, right)
    });
    (rows, Some(baseline))
}

fn build_v2x_variant_matrix(
    config_summary: &[ConfigSummary],
    selected_baseline: Option<&str>,
) -> Vec<BTreeMap<String, Value>> {
    const METRICS: &[(&str, &str, &str)] = &[
        ("Run", "Runs", "runs"),
        ("VO", "P95 delay (ms)", "vo_delay_p95_ms"),
        ("VO", "Mean delay (ms)", "vo_delay_ms"),
        ("VO", "Jitter (ms)", "vo_jitter_ms"),
        ("VO", "RX / logical TX", "vo_rx_per_tx"),
        ("VO", "Incorrect RX drops", "mac_drop_vo_incorrect_rx_count"),
        (
            "VO",
            "Incorrect RX / physical TX",
            "mac_drop_vo_incorrect_rx_per_vo_tx",
        ),
        (
            "VO",
            "Queue overflow drops",
            "mac_drop_vo_queue_overflow_count",
        ),
        (
            "VO",
            "Queue overflow / physical TX",
            "mac_drop_vo_queue_overflow_per_vo_tx",
        ),
        ("BE", "P95 delay (ms)", "be_delay_p95_ms"),
        ("BE", "Mean delay (ms)", "be_delay_ms"),
        ("BE", "Jitter (ms)", "be_jitter_ms"),
        ("BE", "RX / TX", "be_rx_per_tx"),
        ("BE", "Incorrect RX drops", "mac_drop_be_incorrect_rx_count"),
        (
            "BE",
            "Queue overflow drops",
            "mac_drop_be_queue_overflow_count",
        ),
        ("MAC", "Total drops", "mac_drop_sum_count"),
        ("MAC", "Drops / app TX", "mac_drop_per_tx"),
        (
            "Control",
            "BE dropped while blocked",
            "be_dropped_while_blocked_count",
        ),
        (
            "Control",
            "BE grants suppressed",
            "be_grant_suppressed_while_blocked_count",
        ),
        (
            "Control",
            "VO protection activations",
            "vo_protection_activation_count",
        ),
    ];

    let mut by_workload: BTreeMap<String, BTreeMap<String, ConfigSummary>> = BTreeMap::new();
    for summary in config_summary {
        if let Some((variant, workload)) = extract_matrix_variant_and_workload(&summary.config) {
            by_workload
                .entry(workload)
                .or_default()
                .insert(variant, summary.clone());
        }
    }
    let baseline_variant = selected_baseline
        .and_then(|config| extract_matrix_variant_and_workload(config).map(|(variant, _)| variant))
        .unwrap_or_else(|| "edca_only".to_string());

    let mut workloads: Vec<String> = by_workload.keys().cloned().collect();
    workloads.sort_by_key(|workload| workload_rank(workload));
    let mut rows = Vec::new();
    for workload in workloads {
        let variants = by_workload.get(&workload).expect("workload exists");
        for &(group, label, metric_key) in METRICS {
            let plain = variant_metric(variants.get("plain"), metric_key);
            let edca_only = variant_metric(variants.get("edca_only"), metric_key);
            let stable = variant_metric(variants.get("stable"), metric_key);
            let guarded = variant_metric(variants.get("guarded"), metric_key);
            let emergency = variant_metric(variants.get("emergency"), metric_key);
            let baseline_value = variant_metric(variants.get(&baseline_variant), metric_key);

            if metric_key != "runs"
                && plain.is_none()
                && edca_only.is_none()
                && stable.is_none()
                && guarded.is_none()
                && emergency.is_none()
            {
                continue;
            }

            let mut row = BTreeMap::new();
            insert_string(&mut row, "workload", &workload);
            insert_string(&mut row, "group", group);
            insert_string(&mut row, "metric", label);
            insert_string(
                &mut row,
                "plain",
                &format_v2x_matrix_value(metric_key, plain, false),
            );
            insert_string(
                &mut row,
                "edca_only",
                &format_v2x_matrix_value(metric_key, edca_only, false),
            );
            insert_string(
                &mut row,
                "stable",
                &format_v2x_matrix_value(metric_key, stable, false),
            );
            insert_string(
                &mut row,
                "guarded",
                &format_v2x_matrix_value(metric_key, guarded, false),
            );
            insert_string(
                &mut row,
                "emergency",
                &format_v2x_matrix_value(metric_key, emergency, false),
            );
            insert_string(
                &mut row,
                "stable_delta_vs_baseline",
                &format_v2x_matrix_value(
                    metric_key,
                    (metric_key != "runs")
                        .then(|| delta(stable, baseline_value))
                        .flatten(),
                    true,
                ),
            );
            insert_string(
                &mut row,
                "guarded_delta_vs_baseline",
                &format_v2x_matrix_value(
                    metric_key,
                    (metric_key != "runs")
                        .then(|| delta(guarded, baseline_value))
                        .flatten(),
                    true,
                ),
            );
            insert_string(
                &mut row,
                "emergency_delta_vs_baseline",
                &format_v2x_matrix_value(
                    metric_key,
                    (metric_key != "runs")
                        .then(|| delta(emergency, baseline_value))
                        .flatten(),
                    true,
                ),
            );
            rows.push(row);
        }
    }
    rows
}

fn format_v2x_matrix_value(metric_key: &str, value: Option<f64>, is_delta: bool) -> String {
    let Some(value) = value.filter(|value| value.is_finite()) else {
        return "N/A".to_string();
    };

    let decimals = if metric_key == "runs" || metric_key.ends_with("_count") {
        0
    } else if metric_key.ends_with("_ms") {
        3
    } else {
        3
    };
    let formatted = format_number_pt(value, decimals);

    if is_delta && value > 0.0 {
        format!("+{formatted}")
    } else {
        formatted
    }
}

fn format_number_pt(value: f64, decimals: usize) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let raw = format!("{:.*}", decimals, value.abs());
    let (integer, fraction) = raw.split_once('.').unwrap_or((&raw, ""));
    let mut grouped_reversed = String::new();
    for (index, ch) in integer.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped_reversed.push('.');
        }
        grouped_reversed.push(ch);
    }
    let grouped: String = grouped_reversed.chars().rev().collect();

    if decimals == 0 {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped},{fraction}")
    }
}

fn variant_metric(summary: Option<&ConfigSummary>, metric_key: &str) -> Option<f64> {
    summary.and_then(|summary| {
        if metric_key == "runs" {
            Some(summary.runs as f64)
        } else {
            summary.metrics.get(metric_key)
        }
    })
}

fn write_rust_cache(
    results_dir: &Path,
    run_rows: &[RunRow],
    config_summary: &[ConfigSummary],
    source_kind: &str,
    warnings: &[String],
) -> Result<CacheMeta> {
    let cache_dir = rust_cache_dir(results_dir);
    fs::create_dir_all(&cache_dir)?;
    write_json(&cache_dir.join("run_rows.json"), run_rows)?;
    write_json(&cache_dir.join("config_summary.json"), config_summary)?;
    let meta = CacheMeta {
        schema_version: CACHE_SCHEMA_VERSION,
        parser_version: PARSER_VERSION.to_string(),
        built_at_unix_secs: unix_now_secs(),
        source_files: source_signatures(results_dir)?,
        run_count: run_rows.len(),
        config_count: config_summary.len(),
        source_kind: source_kind.to_string(),
        warnings: warnings.to_vec(),
    };
    write_json(&cache_dir.join("meta.json"), &meta)?;
    Ok(meta)
}

fn dataset_from_parts(
    results_dir: &Path,
    cache_state: &str,
    meta: CacheMeta,
    run_rows: Vec<RunRow>,
    config_summary: Vec<ConfigSummary>,
    warnings: Vec<String>,
) -> DashboardDataset {
    DashboardDataset {
        run_rows,
        config_summary,
        cache_info: CacheInfo {
            results_dir: results_dir.display().to_string(),
            cache_dir: rust_cache_dir(results_dir).display().to_string(),
            cache_state: cache_state.to_string(),
            source_kind: meta.source_kind,
            run_count: meta.run_count,
            config_count: meta.config_count,
            built_at_unix_secs: meta.built_at_unix_secs,
        },
        warnings,
    }
}

fn source_signatures(results_dir: &Path) -> Result<Vec<SourceSignature>> {
    let mut paths = Vec::new();
    paths.extend(files_with_extension(results_dir, "sca")?);
    paths.extend(files_with_extension(results_dir, "vec")?);
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let metadata = path.metadata()?;
            Ok(SourceSignature {
                name: path_file_name(&path),
                size: metadata.len(),
                mtime_ns: metadata_mtime_ns(&metadata)?,
            })
        })
        .collect()
}

fn files_with_extension(results_dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    if !results_dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(results_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some(extension) {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn metadata_mtime_ns(metadata: &fs::Metadata) -> Result<u64> {
    let modified = metadata.modified()?;
    let duration = modified.duration_since(UNIX_EPOCH).unwrap_or_default();
    Ok(duration
        .as_secs()
        .saturating_mul(1_000_000_000)
        .saturating_add(duration.subsec_nanos() as u64))
}

fn rust_cache_dir(results_dir: &Path) -> PathBuf {
    results_dir.join(CACHE_DIR_NAME)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let file = File::open(path)?;
    serde_json::from_reader(file).with_context(|| format!("invalid JSON in {}", path.display()))
}

fn write_json<T: Serialize + ?Sized>(path: &Path, payload: &T) -> Result<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, payload)?;
    Ok(())
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn parse_finite(raw: &str) -> Option<f64> {
    raw.parse::<f64>().ok().filter(|value| value.is_finite())
}

fn is_node_app(module: &str, app_index: u8) -> bool {
    module.starts_with("Scenario.node[") && module.ends_with(&format!(".app[{app_index}]"))
}

fn is_node_mac(module: &str) -> bool {
    module.starts_with("Scenario.node[") && module.contains("].wlan[") && module.ends_with("].mac")
}

fn is_node_hcf(module: &str) -> bool {
    module.starts_with("Scenario.node[")
        && module.contains("].wlan[")
        && module.ends_with("].mac.hcf")
}

fn edcaf_index(module: &str, suffix: &str) -> Option<u8> {
    let marker = ".mac.hcf.edca.edcaf[";
    let start = module.find(marker)? + marker.len();
    let end = module[start..].find(']')? + start;
    if !module.ends_with(&format!("].{suffix}")) {
        return None;
    }
    module[start..end].parse::<u8>().ok()
}

fn parse_packet_drop_ac(metric: &str) -> Option<&str> {
    let inner = metric.strip_prefix("packetDropAc")?.strip_suffix("Count")?;
    if inner.contains("Reason") {
        None
    } else {
        Some(inner)
    }
}

fn weighted_mean(
    counts_by_module: &HashMap<String, f64>,
    means_by_module: &HashMap<String, f64>,
) -> Option<f64> {
    let mut count_sum = 0.0;
    let mut weighted_sum = 0.0;
    for (module, count) in counts_by_module {
        if *count <= 0.0 {
            continue;
        }
        if let Some(mean) = means_by_module.get(module).filter(|mean| mean.is_finite()) {
            count_sum += count;
            weighted_sum += count * mean;
        }
    }
    (count_sum > 0.0).then_some(weighted_sum / count_sum)
}

fn finite_min(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .reduce(f64::min)
}

fn finite_max(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .reduce(f64::max)
}

fn percentile(values: &mut [f64], quantile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    if values.len() == 1 {
        return Some(values[0]);
    }
    let position = (values.len() - 1) as f64 * quantile;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    if lower_index == upper_index {
        return Some(values[lower_index]);
    }
    let fraction = position - lower_index as f64;
    Some(values[lower_index] + (values[upper_index] - values[lower_index]) * fraction)
}

fn seconds_to_ms(value: f64) -> f64 {
    value * 1000.0
}

fn ratio(numerator: f64, denominator: f64) -> Option<f64> {
    (denominator > 0.0).then_some(numerator / denominator)
}

fn ratio_optional(numerator: Option<f64>, denominator: f64) -> Option<f64> {
    numerator.and_then(|numerator| ratio(numerator, denominator))
}

fn delta(value: Option<f64>, baseline: Option<f64>) -> Option<f64> {
    Some(value? - baseline?)
}

fn pct_delta(value: Option<f64>, baseline: Option<f64>) -> Option<f64> {
    let value = value?;
    let baseline = baseline?;
    if baseline == 0.0 {
        None
    } else {
        Some(((value - baseline) / baseline) * 100.0)
    }
}

fn add_comparison_metric(
    row: &mut BTreeMap<String, Value>,
    value_id: &str,
    delta_id: &str,
    pct_id: &str,
    value: Option<f64>,
    baseline: Option<f64>,
) {
    insert_number(row, value_id, value);
    insert_number(row, delta_id, delta(value, baseline));
    insert_number(row, pct_id, pct_delta(value, baseline));
}

fn insert_string(row: &mut BTreeMap<String, Value>, key: &str, value: &str) {
    row.insert(key.to_string(), Value::String(value.to_string()));
}

fn insert_number(row: &mut BTreeMap<String, Value>, key: &str, value: Option<f64>) {
    row.insert(
        key.to_string(),
        value
            .filter(|value| value.is_finite())
            .map(Value::from)
            .unwrap_or(Value::Null),
    );
}

fn compare_run_row_values(left: &RunRow, right: &RunRow) -> std::cmp::Ordering {
    compare_configs(&left.config, &right.config)
        .then_with(|| left.run.cmp(&right.run))
        .then_with(|| left.source_file.cmp(&right.source_file))
}

fn sorted_run_rows(rows: &[RunRow]) -> Vec<RunRow> {
    let mut rows = rows.to_vec();
    rows.sort_by(compare_run_row_values);
    rows
}

fn config_sort_tuple(config: &str) -> (usize, usize, String) {
    let lower = config.to_lowercase();
    let base_rank = if lower.starts_with("plain") {
        0
    } else if lower.starts_with("edca_only") {
        1
    } else if lower.starts_with("edca_v2x_vo_stable") {
        2
    } else if lower.starts_with("edca_v2x_vo_guarded") {
        3
    } else if lower.starts_with("edca_v2x_vo_emergency") {
        4
    } else if lower.starts_with("edca_v2x") {
        5
    } else {
        99
    };
    (base_rank, workload_rank(config_workload(&lower)), lower)
}

fn compare_configs(left: &str, right: &str) -> std::cmp::Ordering {
    config_sort_tuple(left).cmp(&config_sort_tuple(right))
}

fn config_workload(config: &str) -> &str {
    config
        .rsplit_once("_netload_")
        .map(|(_, workload)| workload)
        .unwrap_or("medium")
}

fn workload_rank(workload: &str) -> usize {
    match workload {
        "low" => 0,
        "medium" => 1,
        "high" => 2,
        _ => 99,
    }
}

fn is_high_load_config(config: &str) -> bool {
    config.ends_with("_netload_high")
}

fn high_load_only_or_all(config_summary: &[ConfigSummary]) -> Vec<ConfigSummary> {
    let high_load: Vec<ConfigSummary> = config_summary
        .iter()
        .filter(|summary| is_high_load_config(&summary.config))
        .cloned()
        .collect();
    if high_load.is_empty() {
        config_summary.to_vec()
    } else {
        high_load
    }
}

fn baseline_option_values(config_summary: &[ConfigSummary]) -> Vec<String> {
    let mut values: Vec<String> = high_load_only_or_all(config_summary)
        .into_iter()
        .map(|summary| summary.config)
        .collect();
    values.sort_by(|left, right| compare_configs(left, right));
    values
}

fn preferred_baseline(config_summary: &[ConfigSummary]) -> Option<String> {
    let config_set: Vec<&str> = config_summary
        .iter()
        .map(|summary| summary.config.as_str())
        .collect();
    for candidate in [
        "plain_netload_high",
        "plain",
        "highway_plain",
        "square_plain",
        "edca_only_netload_high",
        "edca_only",
        "highway_edca_only",
    ] {
        if config_set.contains(&candidate) {
            return Some(candidate.to_string());
        }
    }
    let mut values: Vec<String> = config_summary
        .iter()
        .map(|summary| summary.config.clone())
        .collect();
    values.sort_by(|left, right| compare_configs(left, right));
    values.into_iter().next()
}

fn extract_matrix_variant_and_workload(config: &str) -> Option<(String, String)> {
    if let Some(workload) = config.strip_prefix("plain_netload_") {
        return Some(("plain".to_string(), workload.to_string()));
    }
    if let Some(workload) = config.strip_prefix("edca_only_netload_") {
        return Some(("edca_only".to_string(), workload.to_string()));
    }

    let rest = config.strip_prefix("edca_v2x_vo_")?;
    let (variant, workload) = rest.split_once("_netload_")?;
    matches!(variant, "stable" | "guarded" | "emergency")
        .then(|| (variant.to_string(), workload.to_string()))
}

fn visible_columns<T: Serialize>(rows: &[T], preferred: &[&str], always: &[&str]) -> Vec<Column> {
    let values: Vec<Value> = rows
        .iter()
        .filter_map(|row| serde_json::to_value(row).ok())
        .collect();
    preferred
        .iter()
        .filter_map(|id| {
            let visible = always.contains(id)
                || values
                    .iter()
                    .any(|row| row.get(*id).map(|value| !value.is_null()).unwrap_or(false));
            visible.then(|| Column {
                id: (*id).to_string(),
                label: label_for(id).to_string(),
            })
        })
        .collect()
}

fn cap_warnings(warnings: Vec<String>) -> Vec<String> {
    const MAX_WARNINGS: usize = 24;
    if warnings.len() <= MAX_WARNINGS {
        return warnings;
    }
    let total = warnings.len();
    let mut capped: Vec<String> = warnings.into_iter().take(MAX_WARNINGS).collect();
    capped.push(format!(
        "{} additional warning(s) hidden",
        total - MAX_WARNINGS
    ));
    capped
}

const ALWAYS_VISIBLE: [&str; 8] = [
    "config",
    "run",
    "runs",
    "source_file",
    "baseline",
    "workload",
    "metric",
    "mac_drop_sum_count",
];

const CONFIG_SUMMARY_TABLE_COLUMNS: &[&str] = &[
    "config",
    "runs",
    "be_delay_ms",
    "be_delay_p95_ms",
    "be_jitter_ms",
    "be_rx_per_tx",
    "be_tx_count",
    "be_rx_count",
    "vo_delay_ms",
    "vo_delay_p95_ms",
    "vo_jitter_ms",
    "vo_rx_per_tx",
    "vo_tx_count",
    "vo_physical_tx_count",
    "vo_rx_count",
    "mac_drop_sum_count",
    "mac_drop_be_count",
    "mac_drop_vo_count",
    "mac_drop_unclassified_count",
    "mac_drop_be_incorrect_rx_count",
    "mac_drop_vo_incorrect_rx_count",
    "mac_drop_be_queue_overflow_count",
    "mac_drop_vo_queue_overflow_count",
    "mac_drop_be_retry_limit_count",
    "mac_drop_vo_retry_limit_count",
    "mac_drop_be_congestion_count",
    "mac_drop_vo_congestion_count",
    "mac_drop_vo_queue_overflow_per_vo_tx",
    "mac_drop_vo_incorrect_rx_per_vo_tx",
    "mac_drop_be_queue_overflow_per_be_tx",
    "mac_drop_be_incorrect_rx_per_be_tx",
    "mac_drop_per_tx",
    "be_dropped_while_blocked_count",
    "be_grant_suppressed_while_blocked_count",
    "vo_protection_activation_count",
];

const RUN_ROW_COLUMNS: &[&str] = &[
    "config",
    "run",
    "source_file",
    "be_delay_ms",
    "be_delay_min_ms",
    "be_delay_max_ms",
    "be_delay_p95_ms",
    "be_jitter_ms",
    "vo_delay_ms",
    "vo_delay_min_ms",
    "vo_delay_max_ms",
    "vo_delay_p95_ms",
    "vo_jitter_ms",
    "be_tx_count",
    "be_rx_count",
    "vo_tx_count",
    "vo_physical_tx_count",
    "vo_rx_count",
    "be_rx_per_tx",
    "vo_rx_per_tx",
    "mac_drop_sum_count",
    "mac_drop_queue_overflow_count",
    "mac_drop_retry_limit_count",
    "mac_drop_be_count",
    "mac_drop_vo_count",
    "mac_drop_unclassified_count",
    "mac_drop_be_incorrect_rx_count",
    "mac_drop_vo_incorrect_rx_count",
    "mac_drop_be_queue_overflow_count",
    "mac_drop_vo_queue_overflow_count",
    "mac_drop_be_retry_limit_count",
    "mac_drop_vo_retry_limit_count",
    "mac_drop_be_congestion_count",
    "mac_drop_vo_congestion_count",
    "mac_drop_vo_per_vo_tx",
    "mac_drop_be_per_be_tx",
    "mac_drop_vo_queue_overflow_per_vo_tx",
    "mac_drop_vo_incorrect_rx_per_vo_tx",
    "mac_drop_be_queue_overflow_per_be_tx",
    "mac_drop_be_incorrect_rx_per_be_tx",
    "mac_drop_per_tx",
    "be_dropped_while_blocked_count",
    "be_grant_suppressed_while_blocked_count",
    "vo_protection_activation_count",
];

const COMPARISON_COLUMNS: &[&str] = &[
    "config",
    "runs",
    "baseline",
    "vo_delay_ms",
    "vo_delay_delta_ms",
    "vo_delay_delta_pct",
    "vo_delay_p95_ms",
    "vo_delay_p95_delta_ms",
    "vo_delay_p95_delta_pct",
    "vo_jitter_ms",
    "vo_jitter_delta_ms",
    "vo_jitter_delta_pct",
    "vo_rx_per_tx",
    "vo_rx_per_tx_delta",
    "vo_rx_per_tx_delta_pct",
    "be_delay_ms",
    "be_delay_delta_ms",
    "be_delay_delta_pct",
    "be_delay_p95_ms",
    "be_delay_p95_delta_ms",
    "be_delay_p95_delta_pct",
    "be_jitter_ms",
    "be_jitter_delta_ms",
    "be_jitter_delta_pct",
    "be_rx_per_tx",
    "be_rx_per_tx_delta",
    "be_rx_per_tx_delta_pct",
    "mac_drop_sum_count",
    "mac_drop_delta_count",
    "mac_drop_delta_pct",
    "mac_drop_vo_incorrect_rx_count",
    "mac_drop_vo_incorrect_rx_delta_count",
    "mac_drop_vo_incorrect_rx_delta_pct",
    "mac_drop_vo_queue_overflow_count",
    "mac_drop_vo_queue_overflow_delta_count",
    "mac_drop_vo_queue_overflow_delta_pct",
    "mac_drop_be_incorrect_rx_count",
    "mac_drop_be_incorrect_rx_delta_count",
    "mac_drop_be_incorrect_rx_delta_pct",
    "mac_drop_be_queue_overflow_count",
    "mac_drop_be_queue_overflow_delta_count",
    "mac_drop_be_queue_overflow_delta_pct",
    "mac_drop_per_tx",
    "mac_drop_per_tx_delta",
    "mac_drop_per_tx_delta_pct",
    "be_dropped_while_blocked_count",
    "be_dropped_while_blocked_delta_count",
    "be_dropped_while_blocked_delta_pct",
    "be_grant_suppressed_while_blocked_count",
    "be_grant_suppressed_while_blocked_delta_count",
    "be_grant_suppressed_while_blocked_delta_pct",
    "vo_protection_activation_count",
    "vo_protection_activation_delta_count",
    "vo_protection_activation_delta_pct",
];

const V2X_VARIANT_MATRIX_COLUMNS: &[&str] = &[
    "workload",
    "group",
    "metric",
    "plain",
    "edca_only",
    "stable",
    "guarded",
    "emergency",
    "stable_delta_vs_baseline",
    "guarded_delta_vs_baseline",
    "emergency_delta_vs_baseline",
];

fn v2x_matrix_columns(
    rows: &[BTreeMap<String, Value>],
    selected_baseline: Option<&str>,
) -> Vec<Column> {
    let baseline_label = selected_baseline
        .and_then(|config| extract_matrix_variant_and_workload(config).map(|(variant, _)| variant))
        .map(|variant| matrix_variant_label(&variant))
        .unwrap_or("EDCA Only");
    visible_columns(rows, V2X_VARIANT_MATRIX_COLUMNS, &ALWAYS_VISIBLE)
        .into_iter()
        .map(|mut column| {
            column.label = match column.id.as_str() {
                "stable_delta_vs_baseline" => format!("Stable Delta vs {baseline_label}"),
                "guarded_delta_vs_baseline" => format!("Guarded Delta vs {baseline_label}"),
                "emergency_delta_vs_baseline" => format!("Emergency Delta vs {baseline_label}"),
                _ => column.label,
            };
            column
        })
        .collect()
}

fn matrix_variant_label(variant: &str) -> &'static str {
    match variant {
        "plain" => "Plain",
        "edca_only" => "EDCA Only",
        "stable" => "Stable",
        "guarded" => "Guarded",
        "emergency" => "Emergency",
        _ => "Selected",
    }
}

fn label_for(id: &str) -> &'static str {
    match id {
        "config" => "Config",
        "run" => "Run",
        "runs" => "Runs",
        "source_file" => "Source File",
        "baseline" => "Baseline",
        "workload" => "Workload",
        "group" => "Group",
        "metric" => "Metric",
        "be_delay_ms" => "BE Mean Delay (ms)",
        "be_delay_min_ms" => "BE Min Delay (ms)",
        "be_delay_max_ms" => "BE Max Delay (ms)",
        "be_delay_p95_ms" => "BE P95 Delay (ms)",
        "be_jitter_ms" => "BE Jitter (ms)",
        "be_rx_per_tx" => "BE RX per TX",
        "be_tx_count" => "BE TX",
        "be_rx_count" => "BE RX",
        "vo_delay_ms" => "VO Mean Delay (ms)",
        "vo_delay_min_ms" => "VO Min Delay (ms)",
        "vo_delay_max_ms" => "VO Max Delay (ms)",
        "vo_delay_p95_ms" => "VO P95 Delay (ms)",
        "vo_jitter_ms" => "VO Jitter (ms)",
        "vo_rx_per_tx" => "VO RX per Logical TX",
        "vo_tx_count" => "VO Logical TX",
        "vo_physical_tx_count" => "VO Physical TX",
        "vo_rx_count" => "VO RX",
        "mac_drop_sum_count" => "MAC Total Drops",
        "mac_drop_queue_overflow_count" => "MAC Queue Overflow Drops",
        "mac_drop_retry_limit_count" => "MAC Retry Limit Drops",
        "mac_drop_be_count" => "MAC BE Drops",
        "mac_drop_vo_count" => "MAC VO Drops",
        "mac_drop_unclassified_count" => "MAC Unclassified Drops",
        "mac_drop_be_incorrect_rx_count" => "MAC BE Incorrect RX Drops",
        "mac_drop_vo_incorrect_rx_count" => "MAC VO Incorrect RX Drops",
        "mac_drop_be_queue_overflow_count" => "MAC BE Queue Overflow Drops",
        "mac_drop_vo_queue_overflow_count" => "MAC VO Queue Overflow Drops",
        "mac_drop_be_retry_limit_count" => "MAC BE Retry Limit Drops",
        "mac_drop_vo_retry_limit_count" => "MAC VO Retry Limit Drops",
        "mac_drop_be_congestion_count" => "MAC BE Congestion Drops",
        "mac_drop_vo_congestion_count" => "MAC VO Congestion Drops",
        "mac_drop_vo_per_vo_tx" => "MAC VO Drops per Physical VO TX",
        "mac_drop_be_per_be_tx" => "MAC BE Drops per BE TX",
        "mac_drop_vo_queue_overflow_per_vo_tx" => "MAC VO Queue Overflow per Physical VO TX",
        "mac_drop_vo_incorrect_rx_per_vo_tx" => "MAC VO Incorrect RX per Physical VO TX",
        "mac_drop_be_queue_overflow_per_be_tx" => "MAC BE Queue Overflow per BE TX",
        "mac_drop_be_incorrect_rx_per_be_tx" => "MAC BE Incorrect RX per BE TX",
        "mac_drop_per_tx" => "MAC Drops per App TX",
        "be_dropped_while_blocked_count" => "BE Dropped While Blocked",
        "be_grant_suppressed_while_blocked_count" => "BE Grants Suppressed While Blocked",
        "vo_protection_activation_count" => "VO Protection Activations",
        "vo_delay_delta_ms" => "VO Mean Delta (ms)",
        "vo_delay_delta_pct" => "VO Mean Delta (%)",
        "vo_delay_p95_delta_ms" => "VO P95 Delta (ms)",
        "vo_delay_p95_delta_pct" => "VO P95 Delta (%)",
        "vo_jitter_delta_ms" => "VO Jitter Delta (ms)",
        "vo_jitter_delta_pct" => "VO Jitter Delta (%)",
        "vo_rx_per_tx_delta" => "VO RX per TX Delta",
        "vo_rx_per_tx_delta_pct" => "VO RX per TX Delta (%)",
        "be_delay_delta_ms" => "BE Mean Delta (ms)",
        "be_delay_delta_pct" => "BE Mean Delta (%)",
        "be_delay_p95_delta_ms" => "BE P95 Delta (ms)",
        "be_delay_p95_delta_pct" => "BE P95 Delta (%)",
        "be_jitter_delta_ms" => "BE Jitter Delta (ms)",
        "be_jitter_delta_pct" => "BE Jitter Delta (%)",
        "be_rx_per_tx_delta" => "BE RX per TX Delta",
        "be_rx_per_tx_delta_pct" => "BE RX per TX Delta (%)",
        "mac_drop_delta_count" => "MAC Drop Delta",
        "mac_drop_delta_pct" => "MAC Drop Delta (%)",
        "mac_drop_vo_incorrect_rx_delta_count" => "MAC VO Incorrect RX Delta",
        "mac_drop_vo_incorrect_rx_delta_pct" => "MAC VO Incorrect RX Delta (%)",
        "mac_drop_vo_queue_overflow_delta_count" => "MAC VO Queue Overflow Delta",
        "mac_drop_vo_queue_overflow_delta_pct" => "MAC VO Queue Overflow Delta (%)",
        "mac_drop_be_incorrect_rx_delta_count" => "MAC BE Incorrect RX Delta",
        "mac_drop_be_incorrect_rx_delta_pct" => "MAC BE Incorrect RX Delta (%)",
        "mac_drop_be_queue_overflow_delta_count" => "MAC BE Queue Overflow Delta",
        "mac_drop_be_queue_overflow_delta_pct" => "MAC BE Queue Overflow Delta (%)",
        "mac_drop_per_tx_delta" => "MAC Drops per TX Delta",
        "mac_drop_per_tx_delta_pct" => "MAC Drops per TX Delta (%)",
        "be_dropped_while_blocked_delta_count" => "BE Dropped While Blocked Delta",
        "be_dropped_while_blocked_delta_pct" => "BE Dropped While Blocked Delta (%)",
        "be_grant_suppressed_while_blocked_delta_count" => "BE Grants Suppressed Delta",
        "be_grant_suppressed_while_blocked_delta_pct" => "BE Grants Suppressed Delta (%)",
        "vo_protection_activation_delta_count" => "VO Protection Activation Delta",
        "vo_protection_activation_delta_pct" => "VO Protection Activation Delta (%)",
        "stable" => "Stable",
        "guarded" => "Guarded",
        "emergency" => "Emergency",
        "plain" => "Plain",
        "edca_only" => "EDCA Only",
        "stable_delta_vs_baseline" => "Stable Delta",
        "guarded_delta_vs_baseline" => "Guarded Delta",
        "emergency_delta_vs_baseline" => "Emergency Delta",
        _ => "Metric",
    }
}

fn path_file_name(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn path_file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Veins QoS KPI Tables</title>
  <style>
    :root {
      color-scheme: light;
      --ink: #16202a;
      --muted: #5c6670;
      --line: #d7dde4;
      --panel: #f7f9fb;
      --accent: #0f766e;
      --warn: #9a3412;
      --warn-bg: #fff7ed;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      color: var(--ink);
      font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: #ffffff;
    }
    main {
      width: min(1600px, calc(100vw - 32px));
      margin: 0 auto;
      padding: 18px 0 28px;
    }
    header {
      display: flex;
      align-items: end;
      justify-content: space-between;
      gap: 16px;
      margin-bottom: 14px;
      border-bottom: 1px solid var(--line);
      padding-bottom: 12px;
    }
    h1 {
      margin: 0;
      font-size: 24px;
      letter-spacing: 0;
      line-height: 1.2;
    }
    h2 {
      margin: 22px 0 8px;
      font-size: 17px;
      letter-spacing: 0;
    }
    .controls {
      display: flex;
      align-items: end;
      flex-wrap: wrap;
      gap: 10px;
    }
    label {
      display: grid;
      gap: 4px;
      color: var(--muted);
      font-size: 12px;
      font-weight: 650;
    }
    select, button {
      height: 36px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: #fff;
      color: var(--ink);
      font: inherit;
      font-size: 14px;
      padding: 0 10px;
    }
    button {
      cursor: pointer;
      border-color: var(--accent);
      color: var(--accent);
      font-weight: 700;
    }
    .status {
      display: grid;
      gap: 4px;
      padding: 10px 12px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: var(--panel);
      color: var(--muted);
      font-size: 13px;
    }
    .warnings {
      margin-top: 10px;
      display: grid;
      gap: 6px;
    }
    .warning {
      border: 1px solid #fed7aa;
      border-radius: 6px;
      background: var(--warn-bg);
      color: var(--warn);
      padding: 8px 10px;
      font-size: 13px;
    }
    .table-wrap {
      overflow: auto;
      border: 1px solid var(--line);
      border-radius: 6px;
    }
    .table-tools {
      display: flex;
      align-items: center;
      justify-content: flex-end;
      gap: 8px;
      margin-bottom: 8px;
    }
    .copy-status {
      color: var(--muted);
      font-size: 12px;
      min-width: 56px;
      text-align: right;
    }
    table {
      width: 100%;
      min-width: 980px;
      border-collapse: collapse;
      font-size: 13px;
    }
    th, td {
      border-bottom: 1px solid var(--line);
      padding: 7px 8px;
      text-align: left;
      vertical-align: top;
      white-space: nowrap;
    }
    th {
      position: sticky;
      top: 0;
      z-index: 1;
      background: #eef3f7;
      font-size: 12px;
      color: #25313d;
    }
    tbody tr:nth-child(even) td { background: #fbfcfd; }
    td.num { text-align: right; font-variant-numeric: tabular-nums; }
    .empty {
      color: var(--muted);
      padding: 16px 0;
      font-size: 13px;
    }
    @media (max-width: 720px) {
      main { width: min(100vw - 20px, 1600px); }
      header { align-items: stretch; }
      h1 { font-size: 20px; }
      .controls { width: 100%; }
      label, select, button { width: 100%; }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <h1>Veins QoS KPI Tables</h1>
      <div class="controls">
        <label>Scenario
          <select id="density"></select>
        </label>
        <label>Baseline
          <select id="baseline"></select>
        </label>
        <button id="reload" type="button">Reload</button>
      </div>
    </header>
    <section id="status" class="status"></section>
    <section id="warnings" class="warnings"></section>
    <section>
      <h2>Comparison vs Baseline</h2>
      <div id="comparison"></div>
    </section>
    <section>
      <h2>Config Summary</h2>
      <div id="config_summary"></div>
    </section>
    <section>
      <h2>Run Details</h2>
      <div id="run_details"></div>
    </section>
    <section id="v2x-section">
      <h2>V2X Mode Matrix</h2>
      <div id="v2x_matrix"></div>
    </section>
  </main>
  <script>
    const density = document.getElementById('density');
    const baseline = document.getElementById('baseline');
    const reloadButton = document.getElementById('reload');
    const nf = new Intl.NumberFormat('pt-BR', { maximumFractionDigits: 3 });
    let lastDensity = '';
    let lastBaseline = '';

    reloadButton.addEventListener('click', loadDashboard);
    density.addEventListener('change', () => {
      lastDensity = density.value;
      loadDashboard();
    });
    baseline.addEventListener('change', () => {
      lastBaseline = baseline.value;
      loadDashboard();
    });

    function renderStatus(data) {
      const info = data.cache_info;
      const rebuild = data.rebuild.running ? 'rebuilding' : 'ready';
      document.getElementById('status').innerHTML = `
        <div><strong>${escapeHtml(info.cache_state)}</strong> · ${escapeHtml(info.source_kind)} · ${rebuild}</div>
        <div>${escapeHtml(info.run_count)} run(s), ${escapeHtml(info.config_count)} config(s)</div>
        <div>${escapeHtml(info.results_dir)}</div>
      `;
      const warnings = document.getElementById('warnings');
      warnings.innerHTML = (data.warnings || []).map((warning) =>
        `<div class="warning">${escapeHtml(warning)}</div>`
      ).join('');
    }

    function syncDensityOptions(data) {
      const selected = data.density || lastDensity || '';
      density.innerHTML = (data.density_options || []).map((option) => {
        const isSelected = option.id === selected ? ' selected' : '';
        return `<option value="${escapeHtml(option.id)}"${isSelected}>${escapeHtml(option.label)}</option>`;
      }).join('');
      if (selected && density.value !== selected) density.value = selected;
      lastDensity = density.value || selected;
    }

    function syncBaselineOptions(data) {
      const selected = data.baseline || lastBaseline || '';
      baseline.innerHTML = (data.baseline_options || []).map((option) => {
        const isSelected = option === selected ? ' selected' : '';
        return `<option value="${escapeHtml(option)}"${isSelected}>${escapeHtml(option)}</option>`;
      }).join('');
      if (selected && baseline.value !== selected) baseline.value = selected;
      lastBaseline = baseline.value || selected;
    }

    function renderTable(targetId, table) {
      const target = document.getElementById(targetId);
      if (!table || !table.rows || table.rows.length === 0) {
        target.innerHTML = '<div class="empty">No rows</div>';
        return;
      }
      const headers = table.columns.map((column) => `<th>${escapeHtml(column.label)}</th>`).join('');
      const body = table.rows.map((row) => {
        const cells = table.columns.map((column) => {
          const value = row[column.id];
          const isNumber = typeof value === 'number';
          return `<td class="${isNumber ? 'num' : ''}">${formatValue(value)}</td>`;
        }).join('');
        return `<tr>${cells}</tr>`;
      }).join('');
      target.innerHTML = `
        <div class="table-tools">
          <button type="button" data-copy-table="${escapeHtml(targetId)}">Copy table</button>
          <span class="copy-status" id="${escapeHtml(targetId)}_copy_status"></span>
        </div>
        <div class="table-wrap"><table><thead><tr>${headers}</tr></thead><tbody>${body}</tbody></table></div>
      `;
      const copyButton = target.querySelector('[data-copy-table]');
      copyButton.addEventListener('click', () => copyTable(targetId, table));
    }

    async function copyTable(targetId, table) {
      const status = document.getElementById(`${targetId}_copy_status`);
      const text = tableToTsv(table);
      try {
        await navigator.clipboard.writeText(text);
        status.textContent = 'Copied';
      } catch (_error) {
        fallbackCopyText(text);
        status.textContent = 'Copied';
      }
      window.setTimeout(() => { status.textContent = ''; }, 1800);
    }

    function tableToTsv(table) {
      const header = table.columns.map((column) => tsvCell(column.label)).join('\t');
      const rows = table.rows.map((row) =>
        table.columns.map((column) => tsvCell(rawValue(row[column.id]))).join('\t')
      );
      return [header, ...rows].join('\n');
    }

    function rawValue(value) {
      if (value === null || value === undefined) return 'N/A';
      if (typeof value === 'number') return nf.format(value);
      return String(value);
    }

    function tsvCell(value) {
      return String(value).replaceAll('\t', ' ').replaceAll('\r', ' ').replaceAll('\n', ' ');
    }

    function fallbackCopyText(text) {
      const textarea = document.createElement('textarea');
      textarea.value = text;
      textarea.setAttribute('readonly', '');
      textarea.style.position = 'fixed';
      textarea.style.left = '-9999px';
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      textarea.remove();
    }

    function formatValue(value) {
      if (value === null || value === undefined) return 'N/A';
      if (typeof value === 'number') return nf.format(value);
      return escapeHtml(String(value));
    }

    function escapeHtml(value) {
      return String(value)
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;')
        .replaceAll("'", '&#039;');
    }

    async function loadDashboard() {
      const params = new URLSearchParams();
      if (lastDensity) params.set('density', lastDensity);
      if (lastBaseline) params.set('baseline', lastBaseline);
      const response = await fetch(`/api/dashboard?${params.toString()}`, { cache: 'no-store' });
      const data = await response.json();
      renderStatus(data);
      syncDensityOptions(data);
      syncBaselineOptions(data);
      renderTable('comparison', data.comparison);
      renderTable('config_summary', data.config_summary);
      renderTable('run_details', data.run_details);
      renderTable('v2x_matrix', data.v2x_matrix);
      document.getElementById('v2x-section').style.display =
        data.v2x_matrix && data.v2x_matrix.rows && data.v2x_matrix.rows.length ? '' : 'none';
      if (data.rebuild && data.rebuild.running) {
        window.setTimeout(loadDashboard, 5000);
      }
    }

    loadDashboard().catch((error) => {
      document.getElementById('status').textContent = `Failed to load dashboard: ${error}`;
    });
  </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_vec_p95_and_jitter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sample.vec");
        fs::write(
            &path,
            "\
vector 1 Scenario.node[0].app[0] beEndToEndDelay:vector ETV
vector 2 Scenario.node[0].app[0] voEndToEndDelay:vector ETV
1 10 0.1 0.001
1 11 0.2 0.003
1 12 0.3 0.005
2 20 0.1 0.002
2 21 0.2 0.006
",
        )
        .unwrap();

        let metrics = parse_vec_metrics(&path).unwrap();
        assert_eq!(metrics.be_samples, 3);
        assert_eq!(metrics.vo_samples, 2);
        assert!((metrics.be_delay_p95_s.unwrap() - 0.0048).abs() < 1e-12);
        assert!((metrics.be_jitter_s.unwrap() - 0.002).abs() < 1e-12);
        assert!((metrics.vo_jitter_s.unwrap() - 0.004).abs() < 1e-12);
    }

    #[test]
    fn detects_vector_headers_without_samples() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.vec");
        fs::write(
            &path,
            "vector 1 Scenario.node[0].app[0] beEndToEndDelay:vector ETV\n",
        )
        .unwrap();

        let metrics = parse_vec_metrics(&path).unwrap();
        assert_eq!(metrics.be_headers, 1);
        assert_eq!(metrics.be_samples, 0);
        assert_eq!(metrics.be_delay_p95_s, None);
    }

    #[test]
    fn parses_sca_scalars_and_uses_vec_metrics() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("plain_netload_high-#0.sca");
        fs::write(
            &path,
            "\
version 3
run plain_netload_high-0
attr configname plain_netload_high
scalar Scenario.node[0].app[0] beTxPackets:count 2
scalar Scenario.node[1].app[0] beRxPackets:count 4
scalar Scenario.node[1].app[0] beEndToEndDelay:count 2
scalar Scenario.node[1].app[0] beEndToEndDelay:mean 0.002
scalar Scenario.node[1].app[0] beEndToEndDelay:min 0.001
scalar Scenario.node[1].app[0] beEndToEndDelay:max 0.003
scalar Scenario.node[2].app[0] voRxPackets:count 3
scalar Scenario.node[2].app[0] voEndToEndDelay:count 3
scalar Scenario.node[2].app[0] voEndToEndDelay:mean 0.004
scalar Scenario.node[0].app[1] voLogicalTxPackets:count 1
scalar Scenario.node[0].app[1] voTxPackets:count 5
scalar Scenario.node[0].wlan[0].mac packetDrop:count 7
scalar Scenario.node[0].wlan[0].mac packetDropAcBeCount 4
scalar Scenario.node[0].wlan[0].mac packetDropAcVoCount 2
",
        )
        .unwrap();
        let vec_metrics = VecMetrics {
            be_delay_p95_s: Some(0.003),
            be_jitter_s: Some(0.001),
            vo_delay_p95_s: Some(0.005),
            vo_jitter_s: Some(0.002),
            ..Default::default()
        };

        let row = parse_sca_file(&path, &vec_metrics).unwrap();
        assert_eq!(row.config, "plain_netload_high");
        assert_eq!(row.metrics.be_tx_count, Some(2.0));
        assert_eq!(row.metrics.be_rx_count, Some(4.0));
        assert_eq!(row.metrics.be_delay_ms, Some(2.0));
        assert_eq!(row.metrics.be_delay_p95_ms, Some(3.0));
        assert_eq!(row.metrics.vo_tx_count, Some(1.0));
        assert_eq!(row.metrics.vo_physical_tx_count, Some(5.0));
        assert_eq!(row.metrics.mac_drop_be_count, Some(4.0));
    }

    #[test]
    fn density_id_from_path_detects_highway_packages() {
        let light = Path::new("/tmp/veins_inet_highway_light/results");
        let heavy = Path::new("/tmp/veins_inet_highway_heavy/results");
        assert_eq!(density_id_from_path(light), "highway_light");
        assert_eq!(density_id_from_path(heavy), "highway_heavy");
    }

    #[test]
    fn builds_cache_and_serializes_nulls() {
        let dir = tempdir().unwrap();
        let results = dir.path();
        fs::write(
            results.join("plain_netload_high-#0.sca"),
            "\
run plain_netload_high-0
attr configname plain_netload_high
scalar Scenario.node[0].app[0] beTxPackets:count 1
scalar Scenario.node[1].app[0] beRxPackets:count 2
scalar Scenario.node[1].app[0] beEndToEndDelay:count 1
scalar Scenario.node[1].app[0] beEndToEndDelay:mean 0.001
scalar Scenario.node[0].app[1] voTxPackets:count 1
scalar Scenario.node[0].wlan[0].mac packetDrop:count 0
",
        )
        .unwrap();
        fs::write(
            results.join("plain_netload_high-#0.vec"),
            "vector 1 Scenario.node[0].app[0] beEndToEndDelay:vector ETV\n",
        )
        .unwrap();

        let dataset = rebuild_raw_dataset(results, Some(1)).unwrap();
        assert_eq!(dataset.cache_info.run_count, 1);
        assert_eq!(dataset.cache_info.config_count, 1);
        assert!(dataset
            .warnings
            .iter()
            .any(|warning| warning.contains("headers but no samples")));
        let density_options = vec![DensityOption {
            id: "custom".to_string(),
            label: "Custom".to_string(),
        }];
        let response = build_dashboard_response(
            &dataset,
            "custom",
            &density_options,
            "plain_netload_high",
            RebuildStatus::idle(),
        );
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("null"));
        assert!(!json.contains("NaN"));
    }
}
