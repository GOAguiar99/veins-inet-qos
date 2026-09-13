//! Config parsing, matrices, and VO delay sample loading.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crate::ConfigSummary;

use super::style::CDF_VEC_SAMPLE_CAP;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Overlay {
    Netload,
    Hotspot,
}

#[derive(Debug, Clone)]
pub struct VoDelaySample {
    pub config: String,
    pub value_ms: f64,
}

pub fn parse_config(config: &str) -> Option<(String, String, Overlay)> {
    for (overlay, marker) in [(Overlay::Netload, "_netload_"), (Overlay::Hotspot, "_hotspot_")] {
        if let Some(workload) = config.strip_prefix("plain").and_then(|rest| {
            rest.strip_prefix(marker)
        }) {
            return Some(("plain".to_string(), workload.to_string(), overlay));
        }
        if let Some(workload) = config.strip_prefix("edca_only").and_then(|rest| {
            rest.strip_prefix(marker)
        }) {
            return Some(("edca_only".to_string(), workload.to_string(), overlay));
        }
        if let Some(rest) = config.strip_prefix("edca_v2x_vo_") {
            if let Some((variant, workload)) = rest.split_once(marker) {
                return Some((variant.to_string(), workload.to_string(), overlay));
            }
        }
    }
    None
}

/// Strategy + workload for a single overlay (netload or hotspot).
pub fn summary_matrix(
    summaries: &[ConfigSummary],
    overlay: Overlay,
) -> BTreeMap<(String, String), ConfigSummary> {
    let mut matrix = BTreeMap::new();
    for summary in summaries {
        if let Some((strategy, workload, kind)) = parse_config(&summary.config) {
            if kind == overlay {
                matrix.insert((strategy, workload), summary.clone());
            }
        }
    }
    matrix
}

pub fn merge_summaries(datasets: &[&[ConfigSummary]]) -> Vec<ConfigSummary> {
    let mut out = Vec::new();
    for rows in datasets {
        out.extend_from_slice(rows);
    }
    out
}

pub fn load_config_summary_json(path: &Path) -> Result<Vec<ConfigSummary>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read summary JSON {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse summary JSON {}", path.display()))
}

pub fn strategy_label(strategy: &str) -> &'static str {
    match strategy {
        "plain" => "DCF",
        "edca_only" => "EDCA",
        "stable" => "Stable",
        "guarded" => "Guarded",
        "emergency" => "Emergency",
        _ => "Other",
    }
}

pub fn density_label(path: &Path) -> String {
    let raw = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .find(|component| {
            component.contains("highway_light")
                || component.contains("highway_heavy")
                || component.contains("hotspot")
        })
        .map(|component| {
            if component.contains("hotspot") {
                "highway_heavy_hotspot".to_string()
            } else if component.contains("light") {
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

pub fn slugify(value: &str) -> String {
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

pub fn load_vo_delay_cdf_samples(results_dir: &Path) -> Result<Vec<VoDelaySample>> {
    eprintln!("loading VO delay samples (high-load netload configs only)…");
    let config_by_stem = load_config_by_stem(results_dir)?;
    let vec_paths: Vec<_> = files_with_extension(results_dir, "vec")?
        .into_iter()
        .filter(|path| {
            let stem = path_stem(path);
            let config = config_by_stem.get(&stem).map(String::as_str).unwrap_or(&stem);
            config.ends_with("_netload_high")
        })
        .collect();
    let total = vec_paths.len();
    let mut samples = Vec::new();
    for (index, vec_path) in vec_paths.iter().enumerate() {
        let stem = path_stem(vec_path);
        let config = config_by_stem
            .get(&stem)
            .cloned()
            .unwrap_or_else(|| stem.clone());
        let name = vec_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("?");
        eprintln!("  [{}/{}] {name}", index + 1, total);
        samples.extend(vo_delay_samples_from_vec(
            vec_path,
            &config,
            CDF_VEC_SAMPLE_CAP,
        )?);
    }
    eprintln!("  {} VO delay samples loaded", samples.len());
    Ok(samples)
}

fn load_config_by_stem(results_dir: &Path) -> Result<HashMap<String, String>> {
    let mut config_by_stem = HashMap::new();
    for sca_path in files_with_extension(results_dir, "sca")? {
        config_by_stem.insert(path_stem(&sca_path), config_from_sca(&sca_path)?);
    }
    Ok(config_by_stem)
}

fn vo_delay_samples_from_vec(
    path: &Path,
    config: &str,
    cap: usize,
) -> Result<Vec<VoDelaySample>> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut vo_vector_ids = HashSet::new();
    let mut reservoir = Vec::with_capacity(cap);
    let mut seen = 0usize;
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
            if is_node_app(module, 0) && metric == "voEndToEndDelay:vector" {
                vo_vector_ids.insert(vector_id.to_string());
            }
            continue;
        }
        if !vo_vector_ids.contains(first) {
            continue;
        }
        let _event = parts.next();
        let _time = parts.next();
        let Some(value_raw) = parts.next() else {
            continue;
        };
        let Ok(value_s) = value_raw.parse::<f64>() else {
            continue;
        };
        if !value_s.is_finite() {
            continue;
        }
        let value_ms = value_s * 1000.0;
        seen += 1;
        if reservoir.len() < cap {
            reservoir.push(VoDelaySample {
                config: config.to_string(),
                value_ms,
            });
        } else {
            let slot = seen.wrapping_mul(0x9E37_79B9) % cap;
            reservoir[slot] = VoDelaySample {
                config: config.to_string(),
                value_ms,
            };
        }
    }
    Ok(reservoir)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_netload_and_hotspot() {
        assert_eq!(
            parse_config("plain_netload_high"),
            Some(("plain".into(), "high".into(), Overlay::Netload))
        );
        assert_eq!(
            parse_config("edca_v2x_vo_emergency_hotspot_medium"),
            Some(("emergency".into(), "medium".into(), Overlay::Hotspot))
        );
    }
}
