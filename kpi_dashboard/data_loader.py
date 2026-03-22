from __future__ import annotations

import math
import re
from pathlib import Path
from typing import Dict, List

import pandas as pd


SCALAR_RE = re.compile(r"^scalar\s+(\S+)\s+(\S+)\s+(\S+)\s*$")
VECTOR_HEADER_RE = re.compile(r"^vector\s+(\d+)\s+(\S+)\s+(\S+)\s+\S+\s*$")
APP0_RE = re.compile(r"^Scenario\.node\[\d+\]\.app\[0\]$")
APP1_RE = re.compile(r"^Scenario\.node\[\d+\]\.app\[1\]$")


def _to_float(raw: str) -> float:
    try:
        return float(raw)
    except ValueError:
        return math.nan


def _safe_weighted_mean(weighted_sum: float, count_sum: float) -> float:
    if count_sum <= 0:
        return math.nan
    return weighted_sum / count_sum


def _safe_min(values: List[float]) -> float:
    return min(values) if values else math.nan


def _safe_max(values: List[float]) -> float:
    return max(values) if values else math.nan


def _percentile(values: List[float], quantile: float) -> float:
    if not values:
        return math.nan
    if len(values) == 1:
        return values[0]

    ordered = sorted(values)
    position = (len(ordered) - 1) * quantile
    lower_index = int(math.floor(position))
    upper_index = int(math.ceil(position))
    if lower_index == upper_index:
        return ordered[lower_index]

    lower_value = ordered[lower_index]
    upper_value = ordered[upper_index]
    fraction = position - lower_index
    return lower_value + (upper_value - lower_value) * fraction


def _compute_jitter(values: List[float]) -> tuple[float, int]:
    if len(values) < 2:
        return 0.0, 0

    jitter_sum = 0.0
    for previous, current in zip(values, values[1:]):
        jitter_sum += abs(current - previous)
    return jitter_sum, len(values) - 1


def parse_vec_file(path: Path) -> Dict[str, float]:
    if not path.exists():
        return {}

    metric_by_vector_id: Dict[str, str] = {}
    values_by_metric: Dict[str, List[float]] = {"be": [], "vo": []}
    jitter_sum_by_metric: Dict[str, float] = {"be": 0.0, "vo": 0.0}
    jitter_count_by_metric: Dict[str, int] = {"be": 0, "vo": 0}
    values_by_vector_id: Dict[str, List[float]] = {}

    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            header_match = VECTOR_HEADER_RE.match(line)
            if header_match:
                vector_id, module, metric = header_match.groups()
                if APP0_RE.match(module):
                    if metric == "beEndToEndDelay:vector":
                        metric_by_vector_id[vector_id] = "be"
                        values_by_vector_id[vector_id] = []
                    elif metric == "voEndToEndDelay:vector":
                        metric_by_vector_id[vector_id] = "vo"
                        values_by_vector_id[vector_id] = []
                continue

            parts = line.split()
            if len(parts) < 4:
                continue

            vector_id = parts[0]
            metric_name = metric_by_vector_id.get(vector_id)
            if metric_name is None:
                continue

            value = _to_float(parts[3])
            if math.isnan(value):
                continue

            values_by_vector_id[vector_id].append(value)
            values_by_metric[metric_name].append(value)

    for vector_id, metric_name in metric_by_vector_id.items():
        jitter_sum, jitter_count = _compute_jitter(values_by_vector_id[vector_id])
        jitter_sum_by_metric[metric_name] += jitter_sum
        jitter_count_by_metric[metric_name] += jitter_count

    result: Dict[str, float] = {}
    for metric_name in ("be", "vo"):
        metric_values = values_by_metric[metric_name]
        jitter_count = jitter_count_by_metric[metric_name]
        jitter_s = (jitter_sum_by_metric[metric_name] / jitter_count) if jitter_count > 0 else math.nan

        result[f"{metric_name}_delay_p50_s"] = _percentile(metric_values, 0.50)
        result[f"{metric_name}_delay_p95_s"] = _percentile(metric_values, 0.95)
        result[f"{metric_name}_delay_p99_s"] = _percentile(metric_values, 0.99)
        result[f"{metric_name}_jitter_s"] = jitter_s

    return result


def parse_sca_file(path: Path) -> Dict[str, float]:
    configname = path.stem
    run_name = path.stem

    be_tx_total = 0.0
    be_rx_total = 0.0
    vo_tx_total = 0.0
    vo_rx_total = 0.0

    be_delay_weighted_sum = 0.0
    be_delay_count_sum = 0.0
    vo_delay_weighted_sum = 0.0
    vo_delay_count_sum = 0.0

    be_delay_count_by_module: Dict[str, float] = {}
    be_delay_mean_by_module: Dict[str, float] = {}
    be_delay_min_values: List[float] = []
    be_delay_max_values: List[float] = []
    vo_delay_count_by_module: Dict[str, float] = {}
    vo_delay_mean_by_module: Dict[str, float] = {}
    vo_delay_min_values: List[float] = []
    vo_delay_max_values: List[float] = []

    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("attr configname "):
                configname = line.split(" ", 2)[2].strip()
                continue
            if line.startswith("run "):
                run_name = line.split(" ", 1)[1].strip()
                continue

            match = SCALAR_RE.match(line)
            if not match:
                continue

            module, metric, value_raw = match.groups()
            value = _to_float(value_raw)
            if math.isnan(value):
                continue

            if APP0_RE.match(module):
                if metric == "beTxPackets:count":
                    be_tx_total += value
                elif metric == "beRxPackets:count":
                    be_rx_total += value
                elif metric == "voRxPackets:count":
                    vo_rx_total += value
                elif metric == "beEndToEndDelay:count":
                    be_delay_count_by_module[module] = value
                elif metric == "beEndToEndDelay:mean":
                    be_delay_mean_by_module[module] = value
                elif metric == "beEndToEndDelay:min":
                    be_delay_min_values.append(value)
                elif metric == "beEndToEndDelay:max":
                    be_delay_max_values.append(value)
                elif metric == "voEndToEndDelay:count":
                    vo_delay_count_by_module[module] = value
                elif metric == "voEndToEndDelay:mean":
                    vo_delay_mean_by_module[module] = value
                elif metric == "voEndToEndDelay:min":
                    vo_delay_min_values.append(value)
                elif metric == "voEndToEndDelay:max":
                    vo_delay_max_values.append(value)
            elif APP1_RE.match(module):
                if metric == "voTxPackets:count":
                    vo_tx_total += value

    for module, count in be_delay_count_by_module.items():
        mean = be_delay_mean_by_module.get(module, math.nan)
        if count > 0 and not math.isnan(mean):
            be_delay_count_sum += count
            be_delay_weighted_sum += count * mean

    for module, count in vo_delay_count_by_module.items():
        mean = vo_delay_mean_by_module.get(module, math.nan)
        if count > 0 and not math.isnan(mean):
            vo_delay_count_sum += count
            vo_delay_weighted_sum += count * mean

    be_delay_s = _safe_weighted_mean(be_delay_weighted_sum, be_delay_count_sum)
    vo_delay_s = _safe_weighted_mean(vo_delay_weighted_sum, vo_delay_count_sum)
    be_delay_min_s = _safe_min(be_delay_min_values)
    be_delay_max_s = _safe_max(be_delay_max_values)
    vo_delay_min_s = _safe_min(vo_delay_min_values)
    vo_delay_max_s = _safe_max(vo_delay_max_values)

    vec_stats = parse_vec_file(path.with_suffix(".vec"))
    be_delay_p50_s = vec_stats.get("be_delay_p50_s", math.nan)
    be_delay_p95_s = vec_stats.get("be_delay_p95_s", math.nan)
    be_delay_p99_s = vec_stats.get("be_delay_p99_s", math.nan)
    be_jitter_s = vec_stats.get("be_jitter_s", math.nan)
    vo_delay_p50_s = vec_stats.get("vo_delay_p50_s", math.nan)
    vo_delay_p95_s = vec_stats.get("vo_delay_p95_s", math.nan)
    vo_delay_p99_s = vec_stats.get("vo_delay_p99_s", math.nan)
    vo_jitter_s = vec_stats.get("vo_jitter_s", math.nan)

    return {
        "config": configname,
        "run": run_name,
        "source_file": path.name,
        "be_delay_s": be_delay_s,
        "be_delay_min_s": be_delay_min_s,
        "be_delay_max_s": be_delay_max_s,
        "be_delay_p50_s": be_delay_p50_s,
        "be_delay_p95_s": be_delay_p95_s,
        "be_delay_p99_s": be_delay_p99_s,
        "be_jitter_s": be_jitter_s,
        "vo_delay_s": vo_delay_s,
        "vo_delay_min_s": vo_delay_min_s,
        "vo_delay_max_s": vo_delay_max_s,
        "vo_delay_p50_s": vo_delay_p50_s,
        "vo_delay_p95_s": vo_delay_p95_s,
        "vo_delay_p99_s": vo_delay_p99_s,
        "vo_jitter_s": vo_jitter_s,
        "be_delay_ms": be_delay_s * 1000.0 if not math.isnan(be_delay_s) else math.nan,
        "be_delay_min_ms": be_delay_min_s * 1000.0 if not math.isnan(be_delay_min_s) else math.nan,
        "be_delay_max_ms": be_delay_max_s * 1000.0 if not math.isnan(be_delay_max_s) else math.nan,
        "be_delay_p50_ms": be_delay_p50_s * 1000.0 if not math.isnan(be_delay_p50_s) else math.nan,
        "be_delay_p95_ms": be_delay_p95_s * 1000.0 if not math.isnan(be_delay_p95_s) else math.nan,
        "be_delay_p99_ms": be_delay_p99_s * 1000.0 if not math.isnan(be_delay_p99_s) else math.nan,
        "be_jitter_ms": be_jitter_s * 1000.0 if not math.isnan(be_jitter_s) else math.nan,
        "vo_delay_ms": vo_delay_s * 1000.0 if not math.isnan(vo_delay_s) else math.nan,
        "vo_delay_min_ms": vo_delay_min_s * 1000.0 if not math.isnan(vo_delay_min_s) else math.nan,
        "vo_delay_max_ms": vo_delay_max_s * 1000.0 if not math.isnan(vo_delay_max_s) else math.nan,
        "vo_delay_p50_ms": vo_delay_p50_s * 1000.0 if not math.isnan(vo_delay_p50_s) else math.nan,
        "vo_delay_p95_ms": vo_delay_p95_s * 1000.0 if not math.isnan(vo_delay_p95_s) else math.nan,
        "vo_delay_p99_ms": vo_delay_p99_s * 1000.0 if not math.isnan(vo_delay_p99_s) else math.nan,
        "vo_jitter_ms": vo_jitter_s * 1000.0 if not math.isnan(vo_jitter_s) else math.nan,
        "be_tx_count": int(round(be_tx_total)),
        "be_rx_count": int(round(be_rx_total)),
        "vo_tx_count": int(round(vo_tx_total)),
        "vo_rx_count": int(round(vo_rx_total)),
        "be_rx_per_tx": (be_rx_total / be_tx_total) if be_tx_total > 0 else math.nan,
        "vo_rx_per_tx": (vo_rx_total / vo_tx_total) if vo_tx_total > 0 else math.nan,
        "be_delivery_ratio": (be_rx_total / be_tx_total) if be_tx_total > 0 else math.nan,
        "vo_delivery_ratio": (vo_rx_total / vo_tx_total) if vo_tx_total > 0 else math.nan,
    }


def load_results(results_dir: Path) -> pd.DataFrame:
    if not results_dir.exists():
        raise FileNotFoundError(f"Results directory not found: {results_dir}")

    sca_files = sorted(results_dir.glob("*.sca"))
    if not sca_files:
        raise FileNotFoundError(f"No .sca files found in: {results_dir}")

    rows: List[Dict[str, float]] = [parse_sca_file(path) for path in sca_files]
    frame = pd.DataFrame(rows)
    frame = frame.sort_values(["config", "run", "source_file"]).reset_index(drop=True)
    return frame
