from __future__ import annotations

import math
import re
from pathlib import Path
from typing import Dict, List

import pandas as pd


SCALAR_RE = re.compile(r"^scalar\s+(\S+)\s+(\S+)\s+(\S+)\s*$")
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
    vo_delay_count_by_module: Dict[str, float] = {}
    vo_delay_mean_by_module: Dict[str, float] = {}

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
                elif metric == "voEndToEndDelay:count":
                    vo_delay_count_by_module[module] = value
                elif metric == "voEndToEndDelay:mean":
                    vo_delay_mean_by_module[module] = value
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

    return {
        "config": configname,
        "run": run_name,
        "source_file": path.name,
        "be_delay_s": be_delay_s,
        "vo_delay_s": vo_delay_s,
        "be_delay_ms": be_delay_s * 1000.0 if not math.isnan(be_delay_s) else math.nan,
        "vo_delay_ms": vo_delay_s * 1000.0 if not math.isnan(vo_delay_s) else math.nan,
        "be_tx_count": int(round(be_tx_total)),
        "be_rx_count": int(round(be_rx_total)),
        "vo_tx_count": int(round(vo_tx_total)),
        "vo_rx_count": int(round(vo_rx_total)),
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
