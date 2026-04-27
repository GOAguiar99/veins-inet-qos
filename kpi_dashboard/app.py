from __future__ import annotations

import argparse
import json
import math
from datetime import datetime, timezone
from pathlib import Path

import pandas as pd
import plotly.express as px
from dash import Dash, Input, Output, State, ctx, dash_table, dcc, html

try:
    from .data_loader import (
        DEFAULT_TIMELINE_BIN_SIZE_S,
        load_cached_run_rows,
        load_cached_timeline_rows,
        load_dashboard_dataset,
    )
except ImportError:
    from data_loader import (
        DEFAULT_TIMELINE_BIN_SIZE_S,
        load_cached_run_rows,
        load_cached_timeline_rows,
        load_dashboard_dataset,
    )


CONFIG_SUMMARY_COLUMNS = [
    "config",
    "runs",
    "be_delay_ms",
    "be_delay_min_ms",
    "be_delay_p95_ms",
    "be_delay_max_ms",
    "be_jitter_ms",
    "be_rx_per_tx",
    "be_tx_count",
    "be_rx_count",
    "vo_delay_ms",
    "vo_delay_min_ms",
    "vo_delay_p95_ms",
    "vo_delay_max_ms",
    "vo_jitter_ms",
    "vo_rx_per_tx",
    "vo_tx_count",
    "vo_rx_count",
    "mac_drop_sum_count",
    "mac_drop_be_count",
    "mac_drop_vo_count",
    "mac_drop_unclassified_count",
    "mac_drop_queue_overflow_count",
    "mac_drop_retry_limit_count",
    "mac_drop_be_per_be_tx",
    "mac_drop_vo_per_vo_tx",
    "mac_drop_per_tx",
]

CONFIG_SUMMARY_TABLE_COLUMNS = [
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
    "vo_rx_count",
    "mac_drop_sum_count",
    "mac_drop_be_count",
    "mac_drop_vo_count",
    "mac_drop_unclassified_count",
    "mac_drop_per_tx",
]

COMPARISON_COLUMNS = [
    "config",
    "runs",
    "baseline",
    "vo_delay_p95_delta_ms",
    "vo_delay_p95_delta_pct",
    "be_delay_p95_delta_ms",
    "be_delay_p95_delta_pct",
    "vo_delay_delta_ms",
    "be_delay_delta_ms",
    "vo_jitter_delta_ms",
    "be_jitter_delta_ms",
    "vo_rx_per_tx_delta",
    "be_rx_per_tx_delta",
    "mac_drop_delta_count",
    "mac_drop_be_delta_count",
    "mac_drop_vo_delta_count",
    "mac_drop_unclassified_delta_count",
    "mac_drop_per_tx_delta",
]

V2X_WORKLOAD_COMPARISON_COLUMNS = [
    "metric",
    "low_stable",
    "low_guarded",
    "low_delta",
    "medium_stable",
    "medium_guarded",
    "medium_delta",
    "high_stable",
    "high_guarded",
    "high_delta",
]

LATENCY_PROFILE_LABELS = {
    "be_delay_min_ms": ("BE", "Min"),
    "be_delay_ms": ("BE", "Mean"),
    "be_delay_p95_ms": ("BE", "P95"),
    "be_delay_max_ms": ("BE", "Max"),
    "vo_delay_min_ms": ("VO", "Min"),
    "vo_delay_ms": ("VO", "Mean"),
    "vo_delay_p95_ms": ("VO", "P95"),
    "vo_delay_max_ms": ("VO", "Max"),
}

DISPLAY_LABELS = {
    "config": "Config",
    "run": "Run",
    "runs": "Runs",
    "source_file": "Source File",
    "time_s": "Time (s)",
    "be_delay_ms": "BE Mean Delay (ms)",
    "be_delay_min_ms": "BE Min Delay (ms)",
    "be_delay_p95_ms": "BE P95 Delay (ms)",
    "be_delay_max_ms": "BE Max Delay (ms)",
    "be_jitter_ms": "BE Jitter (ms)",
    "be_rx_per_tx": "BE RX per TX",
    "be_tx_count": "BE TX",
    "be_rx_count": "BE RX",
    "vo_delay_ms": "VO Mean Delay (ms)",
    "vo_delay_min_ms": "VO Min Delay (ms)",
    "vo_delay_p95_ms": "VO P95 Delay (ms)",
    "vo_delay_max_ms": "VO Max Delay (ms)",
    "vo_jitter_ms": "VO Jitter (ms)",
    "vo_rx_per_tx": "VO RX per TX",
    "vo_tx_count": "VO TX",
    "vo_rx_count": "VO RX",
    "throughput_kbps": "Total Throughput (kbps)",
    "throughput_be_kbps": "BE Throughput (kbps)",
    "throughput_vo_kbps": "VO Throughput (kbps)",
    "active_tx_nodes": "Active TX Nodes",
    "listening_nodes": "Nodes in LISTENING",
    "blocking_nodes": "Nodes in BLOCKING",
    "sending_nodes": "Nodes in SENDING",
    "mac_drop_sum_count": "MAC Sum of All Drops",
    "mac_drop_be_count": "MAC BE Drop Count",
    "mac_drop_vo_count": "MAC VO Drop Count",
    "mac_drop_unclassified_count": "MAC Unclassified Drop Count",
    "mac_drop_queue_overflow_count": "MAC Drop Queue Overflow",
    "mac_drop_retry_limit_count": "MAC Drop Retry Limit",
    "mac_drop_be_per_be_tx": "MAC BE Drops per BE TX",
    "mac_drop_vo_per_vo_tx": "MAC VO Drops per VO TX",
    "mac_drop_per_tx": "MAC Drops per App TX",
    "baseline": "Baseline",
    "vo_delay_p95_delta_ms": "VO P95 Delta (ms)",
    "vo_delay_p95_delta_pct": "VO P95 Delta (%)",
    "be_delay_p95_delta_ms": "BE P95 Delta (ms)",
    "be_delay_p95_delta_pct": "BE P95 Delta (%)",
    "vo_delay_delta_ms": "VO Mean Delta (ms)",
    "be_delay_delta_ms": "BE Mean Delta (ms)",
    "vo_jitter_delta_ms": "VO Jitter Delta (ms)",
    "be_jitter_delta_ms": "BE Jitter Delta (ms)",
    "vo_rx_per_tx_delta": "VO RX per TX Delta",
    "be_rx_per_tx_delta": "BE RX per TX Delta",
    "mac_drop_delta_count": "MAC Drop Delta (count)",
    "mac_drop_be_delta_count": "MAC BE Drop Delta (count)",
    "mac_drop_vo_delta_count": "MAC VO Drop Delta (count)",
    "mac_drop_unclassified_delta_count": "MAC Unclassified Drop Delta (count)",
    "mac_drop_per_tx_delta": "MAC Drops per App TX Delta",
    "metric": "Metric",
    "low_stable": "Low Stable",
    "low_guarded": "Low Guarded",
    "low_delta": "Low Delta",
    "medium_stable": "Medium Stable",
    "medium_guarded": "Medium Guarded",
    "medium_delta": "Medium Delta",
    "high_stable": "High Stable",
    "high_guarded": "High Guarded",
    "high_delta": "High Delta",
}

ROUND_COLUMNS = [
    "time_s",
    "be_delay_ms",
    "be_delay_min_ms",
    "be_delay_p95_ms",
    "be_delay_max_ms",
    "be_jitter_ms",
    "be_rx_per_tx",
    "vo_delay_ms",
    "vo_delay_min_ms",
    "vo_delay_p95_ms",
    "vo_delay_max_ms",
    "vo_jitter_ms",
    "vo_rx_per_tx",
    "throughput_kbps",
    "throughput_be_kbps",
    "throughput_vo_kbps",
    "active_tx_nodes",
    "listening_nodes",
    "blocking_nodes",
    "sending_nodes",
    "vo_delay_p95_delta_ms",
    "vo_delay_p95_delta_pct",
    "be_delay_p95_delta_ms",
    "be_delay_p95_delta_pct",
    "vo_delay_delta_ms",
    "be_delay_delta_ms",
    "vo_jitter_delta_ms",
    "be_jitter_delta_ms",
    "vo_rx_per_tx_delta",
    "be_rx_per_tx_delta",
    "mac_drop_per_tx",
    "mac_drop_be_per_be_tx",
    "mac_drop_vo_per_vo_tx",
    "mac_drop_delta_count",
    "mac_drop_be_delta_count",
    "mac_drop_vo_delta_count",
    "mac_drop_unclassified_delta_count",
    "mac_drop_per_tx_delta",
    "low_stable",
    "low_guarded",
    "low_delta",
    "medium_stable",
    "medium_guarded",
    "medium_delta",
    "high_stable",
    "high_guarded",
    "high_delta",
]

RUN_EXPORT_COLUMNS = [
    "config",
    "run",
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
    "vo_rx_count",
    "mac_drop_sum_count",
    "mac_drop_be_count",
    "mac_drop_vo_count",
    "mac_drop_unclassified_count",
    "mac_drop_queue_overflow_count",
    "mac_drop_retry_limit_count",
    "mac_drop_be_per_be_tx",
    "mac_drop_vo_per_vo_tx",
    "mac_drop_per_tx",
]


def _records_for_json(frame: pd.DataFrame) -> list[dict]:
    if frame.empty:
        return []
    safe = frame.replace({float("inf"): None, float("-inf"): None}).where(pd.notnull(frame), None)
    return safe.to_dict("records")


def _frame_from_records(records: list[dict] | None, columns: list[str]) -> pd.DataFrame:
    if not records:
        return pd.DataFrame(columns=columns)
    frame = pd.DataFrame(records)
    for column in columns:
        if column not in frame.columns:
            frame[column] = math.nan
    return frame[columns]


def _default_snapshot_text() -> str:
    return "Click Generate Snapshot to build a shareable JSON export from the cached dataset."


def _placeholder_figure(title: str, message: str):
    fig = px.scatter(title=title)
    fig.update_xaxes(visible=False)
    fig.update_yaxes(visible=False)
    fig.update_layout(showlegend=False)
    fig.add_annotation(
        text=message,
        x=0.5,
        y=0.5,
        xref="paper",
        yref="paper",
        showarrow=False,
    )
    return fig


def _build_feedback_snapshot(results_dir: Path, simulation_label: str, config_summary: pd.DataFrame, baseline_config: str | None) -> dict:
    run_frame = load_cached_run_rows(results_dir, bin_size_s=DEFAULT_TIMELINE_BIN_SIZE_S)
    timeline_frame = load_cached_timeline_rows(results_dir, bin_size_s=DEFAULT_TIMELINE_BIN_SIZE_S)
    config_summary_display = _high_load_only_or_all(config_summary)
    comparison_summary, baseline_used = _build_comparison_summary(config_summary, baseline_config)
    v2x_workload_comparison = _build_v2x_workload_comparison(config_summary)
    config_list = _ordered_configs(config_summary["config"].astype(str).unique().tolist()) if not config_summary.empty else []

    run_columns = [column for column in ("config", "run", "source_file") if column in run_frame.columns]
    run_columns += [column for column in run_frame.columns if column not in set(run_columns)]
    if run_columns:
        sort_by = [column for column in ("config", "run") if column in run_columns] or [run_columns[0]]
        run_level = _display_frame(run_frame, run_columns).sort_values(sort_by).reset_index(drop=True)
    else:
        run_level = pd.DataFrame()

    timeline_columns = [column for column in ("config", "time_s") if column in timeline_frame.columns]
    timeline_columns += [column for column in timeline_frame.columns if column not in set(timeline_columns)]
    if timeline_columns:
        timeline_level = _display_frame(timeline_frame, timeline_columns).sort_values(
            [column for column in ("config", "time_s") if column in timeline_frame.columns]
        ).reset_index(drop=True)
    else:
        timeline_level = pd.DataFrame()

    return {
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "simulation_label": simulation_label,
        "selected_baseline": baseline_config,
        "baseline_used": baseline_used,
        "run_count": int(len(run_frame)),
        "config_count": int(config_summary["config"].nunique()) if not config_summary.empty else 0,
        "configs": config_list,
        "run_level_columns": run_columns,
        "run_level_metrics": _records_for_json(run_level),
        "config_summary_columns": CONFIG_SUMMARY_COLUMNS,
        "config_summary": _records_for_json(_display_frame(config_summary_display, CONFIG_SUMMARY_COLUMNS)),
        "comparison_columns": COMPARISON_COLUMNS,
        "comparison_vs_baseline": _records_for_json(_display_frame(comparison_summary, COMPARISON_COLUMNS)),
        "v2x_workload_comparison_columns": V2X_WORKLOAD_COMPARISON_COLUMNS,
        "v2x_workload_comparison": _records_for_json(_display_frame(v2x_workload_comparison, V2X_WORKLOAD_COMPARISON_COLUMNS)),
        "timeline_columns": timeline_columns,
        "timeline_metrics": _records_for_json(timeline_level),
    }


def _default_results_dir() -> Path:
    simulations_dir = (Path(__file__).resolve().parent / ".." / "veins_qos" / "simulations").resolve()
    candidates = [
        simulations_dir / "veins_inet_highway_heavy" / "results",
        simulations_dir / "veins_inet_highway_light" / "results",
    ]
    for candidate in candidates:
        if candidate.exists():
            return candidate
    return candidates[0]


def _scenario_options() -> list[tuple[str, Path]]:
    simulations_dir = (Path(__file__).resolve().parent / ".." / "veins_qos" / "simulations").resolve()
    return [
        ("Highway Heavy", simulations_dir / "veins_inet_highway_heavy" / "results"),
        ("Highway Light", simulations_dir / "veins_inet_highway_light" / "results"),
    ]


def _infer_simulation_label(results_dir: Path) -> str:
    mapping = {
        "veins_inet_highway_heavy": "Highway Heavy",
        "veins_inet_highway_light": "Highway Light",
    }
    parent_name = results_dir.parent.name
    return mapping.get(parent_name, parent_name or "Custom")


def _available_dropdown_options(selected_results_dir: Path) -> list[dict[str, str]]:
    options: list[dict[str, str]] = []
    seen_values: set[str] = set()

    for label, path in _scenario_options():
        if path.exists():
            value = str(path.resolve())
            options.append({"label": label, "value": value})
            seen_values.add(value)

    selected_value = str(selected_results_dir.resolve())
    if selected_value not in seen_values:
        options.append({"label": f"Custom: {_infer_simulation_label(selected_results_dir)}", "value": selected_value})

    return options


def _table_columns(column_names: list[str]) -> list[dict[str, str]]:
    return [{"name": DISPLAY_LABELS.get(name, name), "id": name} for name in column_names]


def _display_frame(frame: pd.DataFrame, column_order: list[str]) -> pd.DataFrame:
    display = frame.copy()
    for column in ROUND_COLUMNS:
        if column in display.columns:
            display[column] = display[column].round(6)
    selected_columns = [column for column in column_order if column in display.columns]
    return display[selected_columns]


def _config_sort_key(config: str) -> tuple[int, str]:
    lower = config.lower()
    if lower == "plain" or lower.endswith("_plain"):
        return (0, lower)
    if lower == "edca_only" or lower.endswith("_edca_only"):
        return (1, lower)
    if lower == "edca_v2x" or lower.endswith("_edca_v2x"):
        return (2, lower)
    return (3, lower)


def _ordered_configs(configs: list[str]) -> list[str]:
    return sorted(configs, key=_config_sort_key)


def _preferred_baseline(configs: list[str]) -> str | None:
    if not configs:
        return None
    preferred = [
        "plain",
        "highway_plain",
        "square_plain",
        "edca_only",
        "highway_edca_only",
    ]
    config_set = set(configs)
    for candidate in preferred:
        if candidate in config_set:
            return candidate
    ordered = _ordered_configs(configs)
    return ordered[0] if ordered else None


def _is_high_load_config(config: str) -> bool:
    return config.endswith("_netload_high")


def _high_load_only_or_all(config_summary: pd.DataFrame) -> pd.DataFrame:
    if config_summary.empty:
        return config_summary
    high_load_summary = config_summary[config_summary["config"].astype(str).map(_is_high_load_config)].copy()
    return high_load_summary if not high_load_summary.empty else config_summary


def _baseline_option_values(config_summary: pd.DataFrame) -> list[str]:
    comparison_source = _high_load_only_or_all(config_summary)
    return _ordered_configs(comparison_source["config"].astype(str).tolist()) if not comparison_source.empty else []


def _safe_pct_delta(value: float, baseline_value: float) -> float:
    if math.isnan(value) or math.isnan(baseline_value) or baseline_value == 0:
        return math.nan
    return ((value - baseline_value) / baseline_value) * 100.0


def _build_comparison_summary(config_summary: pd.DataFrame, baseline_config: str | None) -> tuple[pd.DataFrame, str | None]:
    if config_summary.empty:
        return pd.DataFrame(columns=COMPARISON_COLUMNS), None

    comparison_source = _high_load_only_or_all(config_summary)
    config_values = comparison_source["config"].astype(str).tolist()
    baseline = baseline_config if baseline_config in set(config_values) else _preferred_baseline(config_values)
    if baseline is None:
        return pd.DataFrame(columns=COMPARISON_COLUMNS), None

    baseline_row = comparison_source.loc[comparison_source["config"] == baseline]
    if baseline_row.empty:
        return pd.DataFrame(columns=COMPARISON_COLUMNS), None
    base = baseline_row.iloc[0]

    rows: list[dict[str, float | str]] = []
    for _, row in comparison_source.iterrows():
        comparison_row: dict[str, float | str] = {
            "config": str(row["config"]),
            "runs": int(row["runs"]),
            "baseline": baseline,
        }
        comparison_row["vo_delay_p95_delta_ms"] = row["vo_delay_p95_ms"] - base["vo_delay_p95_ms"]
        comparison_row["vo_delay_p95_delta_pct"] = _safe_pct_delta(row["vo_delay_p95_ms"], base["vo_delay_p95_ms"])
        comparison_row["be_delay_p95_delta_ms"] = row["be_delay_p95_ms"] - base["be_delay_p95_ms"]
        comparison_row["be_delay_p95_delta_pct"] = _safe_pct_delta(row["be_delay_p95_ms"], base["be_delay_p95_ms"])
        comparison_row["vo_delay_delta_ms"] = row["vo_delay_ms"] - base["vo_delay_ms"]
        comparison_row["be_delay_delta_ms"] = row["be_delay_ms"] - base["be_delay_ms"]
        comparison_row["vo_jitter_delta_ms"] = row["vo_jitter_ms"] - base["vo_jitter_ms"]
        comparison_row["be_jitter_delta_ms"] = row["be_jitter_ms"] - base["be_jitter_ms"]
        comparison_row["vo_rx_per_tx_delta"] = row["vo_rx_per_tx"] - base["vo_rx_per_tx"]
        comparison_row["be_rx_per_tx_delta"] = row["be_rx_per_tx"] - base["be_rx_per_tx"]
        comparison_row["mac_drop_delta_count"] = row["mac_drop_sum_count"] - base["mac_drop_sum_count"]
        comparison_row["mac_drop_be_delta_count"] = row["mac_drop_be_count"] - base["mac_drop_be_count"]
        comparison_row["mac_drop_vo_delta_count"] = row["mac_drop_vo_count"] - base["mac_drop_vo_count"]
        comparison_row["mac_drop_unclassified_delta_count"] = row["mac_drop_unclassified_count"] - base["mac_drop_unclassified_count"]
        comparison_row["mac_drop_per_tx_delta"] = row["mac_drop_per_tx"] - base["mac_drop_per_tx"]
        rows.append(comparison_row)

    comparison = pd.DataFrame(rows)
    order = _ordered_configs(comparison["config"].astype(str).tolist())
    comparison["config"] = pd.Categorical(comparison["config"], categories=order, ordered=True)
    comparison = comparison.sort_values("config").reset_index(drop=True)
    comparison["config"] = comparison["config"].astype(str)
    return comparison[COMPARISON_COLUMNS], baseline


def _extract_v2x_variant_and_workload(config: str) -> tuple[str | None, str | None]:
    prefix = "edca_v2x_vo_"
    marker = "_netload_"
    if not config.startswith(prefix) or marker not in config:
        return None, None

    variant_and_suffix = config[len(prefix):]
    variant, workload = variant_and_suffix.split(marker, 1)
    if variant not in {"stable", "guarded"}:
        return None, None
    return variant, workload


def _build_v2x_workload_comparison(config_summary: pd.DataFrame) -> pd.DataFrame:
    if config_summary.empty:
        return pd.DataFrame(columns=V2X_WORKLOAD_COMPARISON_COLUMNS)

    rows_by_workload: dict[str, dict[str, float | int | str]] = {}

    for _, row in config_summary.iterrows():
        config = str(row["config"])
        variant, workload = _extract_v2x_variant_and_workload(config)
        if variant is None or workload is None:
            continue

        entry = rows_by_workload.setdefault(
            workload,
            {
                "workload": workload,
                "stable_runs": math.nan,
                "guarded_runs": math.nan,
            },
        )

        entry[f"{variant}_runs"] = int(row["runs"])
        entry[f"{variant}_vo_delay_p95_ms"] = row["vo_delay_p95_ms"]
        entry[f"{variant}_vo_delay_ms"] = row["vo_delay_ms"]
        entry[f"{variant}_vo_jitter_ms"] = row["vo_jitter_ms"]
        entry[f"{variant}_vo_rx_per_tx"] = row["vo_rx_per_tx"]
        entry[f"{variant}_be_delay_p95_ms"] = row["be_delay_p95_ms"]
        entry[f"{variant}_be_delay_ms"] = row["be_delay_ms"]
        entry[f"{variant}_be_jitter_ms"] = row["be_jitter_ms"]
        entry[f"{variant}_be_rx_per_tx"] = row["be_rx_per_tx"]
        entry[f"{variant}_mac_drop_sum_count"] = row["mac_drop_sum_count"]
        entry[f"{variant}_mac_drop_be_count"] = row["mac_drop_be_count"]
        entry[f"{variant}_mac_drop_vo_count"] = row["mac_drop_vo_count"]
        entry[f"{variant}_mac_drop_per_tx"] = row["mac_drop_per_tx"]

    if not rows_by_workload:
        return pd.DataFrame(columns=V2X_WORKLOAD_COMPARISON_COLUMNS)

    metric_definitions = [
        ("Runs", "runs"),
        ("VO P95 Delay (ms)", "vo_delay_p95_ms"),
        ("VO Mean Delay (ms)", "vo_delay_ms"),
        ("VO Jitter (ms)", "vo_jitter_ms"),
        ("VO RX per TX", "vo_rx_per_tx"),
        ("BE P95 Delay (ms)", "be_delay_p95_ms"),
        ("BE Mean Delay (ms)", "be_delay_ms"),
        ("BE Jitter (ms)", "be_jitter_ms"),
        ("BE RX per TX", "be_rx_per_tx"),
        ("MAC Total Drops", "mac_drop_sum_count"),
        ("MAC BE Drops", "mac_drop_be_count"),
        ("MAC VO Drops", "mac_drop_vo_count"),
        ("MAC Drops per TX", "mac_drop_per_tx"),
    ]

    matrix_rows: list[dict[str, float | int | str]] = []
    ordered_workloads = [workload for workload in ["low", "medium", "high"] if workload in rows_by_workload]

    for metric_label, metric_key in metric_definitions:
        row: dict[str, float | int | str] = {"metric": metric_label}
        for workload in ordered_workloads:
            workload_entry = rows_by_workload[workload]
            stable_key = f"stable_{metric_key}"
            guarded_key = f"guarded_{metric_key}"
            stable_value = workload_entry.get(stable_key, math.nan)
            guarded_value = workload_entry.get(guarded_key, math.nan)
            delta_value = math.nan
            if metric_key != "runs" and pd.notna(guarded_value) and pd.notna(stable_value):
                delta_value = guarded_value - stable_value
            row[f"{workload}_stable"] = stable_value
            row[f"{workload}_guarded"] = guarded_value
            row[f"{workload}_delta"] = delta_value
        matrix_rows.append(row)

    comparison = pd.DataFrame(matrix_rows)
    return comparison.reindex(columns=V2X_WORKLOAD_COMPARISON_COLUMNS)


def _plot_latency_profile(frame: pd.DataFrame, simulation_label: str):
    if frame.empty:
        return _placeholder_figure(f"Latency Profile ({simulation_label})", "No summary data loaded.")

    melted = frame.melt(
        id_vars=["config"],
        value_vars=list(LATENCY_PROFILE_LABELS.keys()),
        var_name="metric",
        value_name="delay_ms",
    )
    melted["traffic_class"] = melted["metric"].map(lambda metric: LATENCY_PROFILE_LABELS[metric][0])
    melted["statistic"] = melted["metric"].map(lambda metric: LATENCY_PROFILE_LABELS[metric][1])

    fig = px.bar(
        melted,
        x="config",
        y="delay_ms",
        color="statistic",
        facet_row="traffic_class",
        barmode="group",
        title=f"Latency Profile ({simulation_label})",
    )
    fig.for_each_annotation(lambda annotation: annotation.update(text=annotation.text.split("=")[-1]))
    fig.update_layout(height=760, yaxis_title="Delay (ms)", xaxis_title="Config")
    return fig


def _plot_jitter(frame: pd.DataFrame, simulation_label: str):
    if frame.empty:
        return _placeholder_figure(f"Delay Variation ({simulation_label})", "No summary data loaded.")

    melted = frame.melt(
        id_vars=["config"],
        value_vars=["be_jitter_ms", "vo_jitter_ms"],
        var_name="metric",
        value_name="jitter_ms",
    )
    melted["metric"] = melted["metric"].map(
        {
            "be_jitter_ms": "BE Jitter",
            "vo_jitter_ms": "VO Jitter",
        }
    )
    fig = px.bar(
        melted,
        x="config",
        y="jitter_ms",
        color="metric",
        barmode="group",
        title=f"Delay Variation ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Jitter (ms)", xaxis_title="Config")
    return fig


def _plot_reception_efficiency(frame: pd.DataFrame, simulation_label: str):
    if frame.empty:
        return _placeholder_figure(f"Multicast Reach ({simulation_label})", "No summary data loaded.")

    melted = frame.melt(
        id_vars=["config"],
        value_vars=["be_rx_per_tx", "vo_rx_per_tx"],
        var_name="metric",
        value_name="rx_per_tx",
    )
    melted["metric"] = melted["metric"].map(
        {
            "be_rx_per_tx": "BE RX per TX",
            "vo_rx_per_tx": "VO RX per TX",
        }
    )
    fig = px.bar(
        melted,
        x="config",
        y="rx_per_tx",
        color="metric",
        barmode="group",
        title=f"Multicast Reach ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Receptions per Transmission", xaxis_title="Config")
    return fig


def _plot_counts(frame: pd.DataFrame, simulation_label: str):
    if frame.empty:
        return _placeholder_figure(f"TX / RX Packet Counts ({simulation_label})", "No summary data loaded.")

    melted = frame.melt(
        id_vars=["config"],
        value_vars=["be_tx_count", "be_rx_count", "vo_tx_count", "vo_rx_count"],
        var_name="metric",
        value_name="count",
    )
    melted["metric"] = melted["metric"].map(
        {
            "be_tx_count": "BE TX",
            "be_rx_count": "BE RX",
            "vo_tx_count": "VO TX",
            "vo_rx_count": "VO RX",
        }
    )
    fig = px.bar(
        melted,
        x="config",
        y="count",
        color="metric",
        barmode="group",
        title=f"TX / RX Packet Counts ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Packet Count", xaxis_title="Config")
    return fig


def _plot_tradeoff(frame: pd.DataFrame, simulation_label: str):
    if frame.empty:
        return _placeholder_figure(f"Protection vs Cost ({simulation_label})", "No summary data loaded.")

    fig = px.scatter(
        frame,
        x="be_delay_p95_ms",
        y="vo_delay_p95_ms",
        size="vo_rx_per_tx",
        color="config",
        hover_name="config",
        hover_data={
            "be_delay_p95_ms": True,
            "vo_delay_p95_ms": True,
            "vo_rx_per_tx": True,
            "be_delay_ms": True,
            "vo_delay_ms": True,
        },
        title=f"Protection vs Cost ({simulation_label})",
    )
    fig.update_layout(
        xaxis_title="BE P95 Delay (ms)",
        yaxis_title="VO P95 Delay (ms)",
    )
    return fig


def _plot_delta_tradeoff(comparison_summary: pd.DataFrame, simulation_label: str, baseline_config: str | None):
    if comparison_summary.empty or baseline_config is None:
        return _placeholder_figure(
            "Delta Protection vs Cost",
            "Select a baseline and load summary data to compare high-load configs.",
        )

    fig = px.scatter(
        comparison_summary,
        x="be_delay_p95_delta_ms",
        y="vo_delay_p95_delta_ms",
        color="config",
        hover_name="config",
        title=f"Delta Protection vs Cost ({simulation_label}, baseline={baseline_config})",
    )
    fig.add_hline(y=0.0, line_dash="dot")
    fig.add_vline(x=0.0, line_dash="dot")
    fig.update_layout(
        xaxis_title="BE P95 Delta (ms) (+ means more BE delay)",
        yaxis_title="VO P95 Delta (ms) (- means better VO protection)",
    )
    return fig


def _plot_drop_reasons(frame: pd.DataFrame, simulation_label: str):
    candidate_columns = [
        "mac_drop_sum_count",
        "mac_drop_be_count",
        "mac_drop_vo_count",
        "mac_drop_unclassified_count",
        "mac_drop_queue_overflow_count",
        "mac_drop_retry_limit_count",
    ]
    available_columns = [column for column in candidate_columns if column in frame.columns]
    if frame.empty or not available_columns:
        return _placeholder_figure(f"MAC Drop Breakdown ({simulation_label})", "No summary data loaded.")

    melted = frame.melt(
        id_vars=["config"],
        value_vars=available_columns,
        var_name="metric",
        value_name="count",
    )
    melted["metric"] = melted["metric"].map(
        {
            "mac_drop_sum_count": "MAC Drop Total",
            "mac_drop_be_count": "MAC Drop BE (AC_BE)",
            "mac_drop_vo_count": "MAC Drop VO (AC_VO)",
            "mac_drop_unclassified_count": "MAC Drop Unclassified",
            "mac_drop_queue_overflow_count": "MAC Drop Queue Overflow",
            "mac_drop_retry_limit_count": "MAC Drop Retry Limit",
        }
    )
    fig = px.bar(
        melted,
        x="config",
        y="count",
        color="metric",
        barmode="group",
        title=f"MAC Drop Breakdown ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Drop Count", xaxis_title="Config")
    return fig


def _plot_drop_rates(frame: pd.DataFrame, simulation_label: str):
    candidate_columns = [
        "mac_drop_per_tx",
        "mac_drop_be_per_be_tx",
        "mac_drop_vo_per_vo_tx",
    ]
    available_columns = [column for column in candidate_columns if column in frame.columns]
    if frame.empty or not available_columns:
        return _placeholder_figure(f"MAC Drop Rates ({simulation_label})", "No summary data loaded.")

    melted = frame.melt(
        id_vars=["config"],
        value_vars=available_columns,
        var_name="metric",
        value_name="drop_rate",
    )
    melted["metric"] = melted["metric"].map(
        {
            "mac_drop_per_tx": "Overall Drops per App TX",
            "mac_drop_be_per_be_tx": "BE Drops per BE TX",
            "mac_drop_vo_per_vo_tx": "VO Drops per VO TX",
        }
    )

    fig = px.line(
        melted,
        x="config",
        y="drop_rate",
        color="metric",
        markers=True,
        title=f"Normalized Drop Rates ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Drops per TX", xaxis_title="Config")
    return fig


def _aggregate_timeline(frame: pd.DataFrame) -> pd.DataFrame:
    if frame.empty:
        return pd.DataFrame()

    metric_columns = [
        "throughput_kbps",
        "throughput_be_kbps",
        "throughput_vo_kbps",
        "active_tx_nodes",
        "listening_nodes",
        "blocking_nodes",
        "sending_nodes",
    ]
    available_columns = [column for column in metric_columns if column in frame.columns]
    if not available_columns:
        return pd.DataFrame()

    if "run" not in frame.columns and "source_file" not in frame.columns:
        selected = ["config", "time_s", *available_columns]
        return frame[selected].sort_values(["config", "time_s"]).reset_index(drop=True)

    return (
        frame.groupby(
            ["config", "time_s"],
            as_index=False,
        )[available_columns]
        .mean(numeric_only=True)
        .sort_values(["config", "time_s"])
        .reset_index(drop=True)
    )


def _plot_throughput_timeline(frame: pd.DataFrame, simulation_label: str):
    aggregated = _aggregate_timeline(frame)
    if aggregated.empty:
        return _placeholder_figure(
            f"Throughput Timeline ({simulation_label})",
            "Click Load Timelines to read the cached per-second timeline.",
        )

    throughput_columns = []
    for column in ["throughput_kbps", "throughput_be_kbps", "throughput_vo_kbps"]:
        if column in aggregated.columns and not aggregated[column].isna().all():
            throughput_columns.append(column)

    if not throughput_columns:
        return _placeholder_figure(f"Throughput Timeline ({simulation_label})", "No throughput timeline data available.")

    melted = aggregated.melt(
        id_vars=["config", "time_s"],
        value_vars=throughput_columns,
        var_name="metric",
        value_name="throughput_value_kbps",
    )
    melted["metric"] = melted["metric"].map(
        {
            "throughput_kbps": "Total Throughput",
            "throughput_be_kbps": "BE Throughput",
            "throughput_vo_kbps": "VO Throughput",
        }
    )

    fig = px.line(
        melted,
        x="time_s",
        y="throughput_value_kbps",
        color="config",
        facet_row="metric",
        title=f"Throughput Timeline ({simulation_label})",
    )
    fig.for_each_annotation(lambda annotation: annotation.update(text=annotation.text.split("=")[-1]))
    fig.update_layout(
        xaxis_title="Simulation Time (s)",
        yaxis_title="Throughput (kbps)",
    )
    return fig


def _plot_simulation_timeline(frame: pd.DataFrame, simulation_label: str):
    aggregated = _aggregate_timeline(frame)
    if aggregated.empty:
        return _placeholder_figure(
            f"State Timeline ({simulation_label})",
            "Click Load Timelines to read the cached state occupancy timeline.",
        )

    available_metrics = []
    for column in ["active_tx_nodes", "listening_nodes", "blocking_nodes", "sending_nodes"]:
        if column in aggregated.columns and not aggregated[column].isna().all():
            available_metrics.append(column)

    if not available_metrics:
        return _placeholder_figure(f"State Timeline ({simulation_label})", "No state timeline data available.")

    melted = aggregated.melt(
        id_vars=["config", "time_s"],
        value_vars=available_metrics,
        var_name="metric",
        value_name="value",
    )
    melted["metric"] = melted["metric"].map(
        {
            "active_tx_nodes": "Active TX Nodes (count)",
            "listening_nodes": "Nodes in LISTENING",
            "blocking_nodes": "Nodes in BLOCKING",
            "sending_nodes": "Nodes in SENDING",
        }
    )
    fig = px.line(
        melted,
        x="time_s",
        y="value",
        color="config",
        facet_row="metric",
        title=f"State Timeline ({simulation_label})",
    )
    fig.for_each_annotation(lambda annotation: annotation.update(text=annotation.text.split("=")[-1]))
    fig.update_layout(xaxis_title="Simulation Time (s)", yaxis_title="")
    return fig


def _cache_status_text(results_path: Path, cache_info: dict) -> str:
    cache_state = "cache hit" if cache_info.get("cache_hit") else "cache rebuilt"
    run_count = int(cache_info.get("run_count", 0))
    config_count = int(cache_info.get("config_count", 0))
    timeline_rows = int(cache_info.get("timeline_row_count", 0))
    cache_dir = cache_info.get("cache_dir", str(results_path / ".kpi_cache"))
    return (
        f"Loaded {run_count} run(s) across {config_count} config(s) from {results_path} "
        f"({cache_state}; cached timelines: {timeline_rows} row(s); cache dir: {cache_dir})"
    )


def build_app(results_dir: Path) -> Dash:
    app = Dash(__name__)
    app.title = "Veins QoS KPI Dashboard"
    dropdown_options = _available_dropdown_options(results_dir)
    selected_value = str(results_dir.resolve())

    app.layout = html.Div(
        [
            html.H2("Veins QoS KPI Dashboard"),
            html.Div(
                [
                    html.Label("Simulation", htmlFor="simulation-select"),
                    dcc.Dropdown(
                        id="simulation-select",
                        options=dropdown_options,
                        value=selected_value,
                        clearable=False,
                        style={"maxWidth": "360px"},
                    ),
                ],
                style={"marginBottom": "12px"},
            ),
            html.Div(
                [
                    html.Label("Baseline config", htmlFor="baseline-select"),
                    dcc.Dropdown(
                        id="baseline-select",
                        options=[],
                        value=None,
                        clearable=False,
                        placeholder="Select baseline config",
                        style={"maxWidth": "360px"},
                    ),
                ],
                style={"marginBottom": "12px"},
            ),
            html.Div(id="simulation-label", style={"marginBottom": "6px", "fontWeight": "bold"}),
            html.Div(id="results-source", style={"marginBottom": "12px"}),
            html.Button("Reload Results", id="reload-button", n_clicks=0),
            html.Div(id="status", style={"marginTop": "12px", "marginBottom": "8px"}),
            html.Div(
                "Jitter is computed as the mean absolute change between consecutive packet delays per receiver. "
                "RX per TX is used instead of delivery ratio because these runs use multicast.",
                style={"marginBottom": "16px", "color": "#4a5568"},
            ),
            dcc.Store(id="dataset-meta-store"),
            dcc.Store(id="config-summary-store"),
            dcc.Store(id="timeline-meta-store"),
            dcc.Store(id="feedback-export-store"),
            html.H3("Config Summary"),
            dash_table.DataTable(
                id="config-summary-table",
                page_size=20,
                style_table={"overflowX": "auto"},
                style_cell={"textAlign": "left", "padding": "6px"},
            ),
            html.H3("Comparison vs Baseline"),
            dash_table.DataTable(
                id="comparison-table",
                page_size=20,
                style_table={"overflowX": "auto"},
                style_cell={"textAlign": "left", "padding": "6px"},
            ),
            html.H3("V2X Workload Comparison"),
            html.Div(
                "Compares only the EDCA V2X variants in a workload matrix: each row is one KPI, and each load shows Stable, Guarded, and Delta (Guarded - Stable) side by side.",
                style={"marginBottom": "8px", "color": "#4a5568"},
            ),
            dash_table.DataTable(
                id="v2x-workload-comparison-table",
                page_size=14,
                style_table={"overflowX": "auto"},
                style_cell={"textAlign": "left", "padding": "6px"},
            ),
            html.H3("Share With AI"),
            html.Div(
                "Generate a JSON snapshot from the cached dataset only when you need to share it.",
                style={"marginBottom": "8px"},
            ),
            html.Div(
                [
                    html.Button(
                        "Generate Snapshot",
                        id="generate-feedback-button",
                        n_clicks=0,
                    ),
                    dcc.Clipboard(
                        target_id="feedback-snapshot",
                        title="Copy snapshot",
                        style={"display": "inline-block", "fontSize": 20, "cursor": "pointer", "marginLeft": "12px"},
                    ),
                    html.Button(
                        "Download Snapshot JSON",
                        id="download-feedback-button",
                        n_clicks=0,
                        style={"marginLeft": "12px"},
                    ),
                    dcc.Download(id="download-feedback-json"),
                ],
                style={"marginBottom": "8px"},
            ),
            dcc.Textarea(
                id="feedback-snapshot",
                value=_default_snapshot_text(),
                readOnly=True,
                style={
                    "width": "100%",
                    "height": "260px",
                    "fontFamily": "monospace",
                    "fontSize": "12px",
                },
            ),
            html.H3("Throughput Timeline"),
            html.Div(
                [
                    html.Button("Load Timelines", id="load-timelines-button", n_clicks=0),
                    html.Div(
                        id="timeline-status",
                        children="Timelines load on demand from the cache to keep the dashboard responsive.",
                        style={"marginTop": "8px", "marginBottom": "8px", "color": "#4a5568"},
                    ),
                ],
                style={"marginBottom": "8px"},
            ),
            dcc.Graph(id="throughput-timeline-plot"),
            html.H3("State Timeline"),
            dcc.Graph(id="simulation-timeline-plot"),
            html.H3("Latency Profile"),
            dcc.Graph(id="latency-profile-plot"),
            html.H3("Jitter"),
            dcc.Graph(id="jitter-plot"),
            html.H3("Multicast Reach"),
            dcc.Graph(id="reception-plot"),
            html.H3("TX / RX Counts"),
            dcc.Graph(id="counts-plot"),
            html.H3("MAC Drop Breakdown"),
            dcc.Graph(id="drop-reasons-plot"),
            html.H3("Normalized Drop Rates"),
            dcc.Graph(id="drop-rates-plot"),
            html.H3("Protection vs Cost"),
            dcc.Graph(id="tradeoff-plot"),
            html.H3("Delta Protection vs Cost"),
            dcc.Graph(id="delta-tradeoff-plot"),
        ],
        style={"maxWidth": "1400px", "margin": "0 auto", "padding": "20px"},
    )

    @app.callback(
        Output("simulation-label", "children"),
        Output("results-source", "children"),
        Output("status", "children"),
        Output("dataset-meta-store", "data"),
        Output("config-summary-store", "data"),
        Output("timeline-meta-store", "data"),
        Output("baseline-select", "options"),
        Output("baseline-select", "value"),
        Input("reload-button", "n_clicks"),
        Input("simulation-select", "value"),
        State("baseline-select", "value"),
    )
    def load_data(_n_clicks: int, results_path_raw: str, current_baseline: str | None):
        results_path = Path(results_path_raw)
        simulation_label = _infer_simulation_label(results_path)
        try:
            dataset = load_dashboard_dataset(results_path, bin_size_s=DEFAULT_TIMELINE_BIN_SIZE_S)
        except Exception as exc:
            status = f"Failed to load results: {exc}"
            return (
                f"Simulation: {simulation_label}",
                f"Results source: {results_path}",
                status,
                {},
                {"rows": []},
                {"available": False},
                [],
                None,
            )

        config_summary = dataset["config_summary"]
        cache_info = dataset["cache_info"]
        config_values = _baseline_option_values(config_summary)
        baseline_options = [{"label": config, "value": config} for config in config_values]
        baseline_value = current_baseline if current_baseline in set(config_values) else _preferred_baseline(config_values)

        dataset_meta = {
            "results_path": str(results_path.resolve()),
            "simulation_label": simulation_label,
            "cache_dir": cache_info.get("cache_dir"),
            "cache_hit": bool(cache_info.get("cache_hit")),
            "bin_size_s": float(cache_info.get("bin_size_s", DEFAULT_TIMELINE_BIN_SIZE_S)),
            "run_count": int(cache_info.get("run_count", 0)),
            "config_count": int(cache_info.get("config_count", 0)),
            "timeline_row_count": int(cache_info.get("timeline_row_count", 0)),
            "built_at_utc": cache_info.get("built_at_utc"),
        }
        config_store = {"rows": _records_for_json(config_summary)}
        timeline_meta = {
            "available": int(cache_info.get("timeline_row_count", 0)) > 0,
            "row_count": int(cache_info.get("timeline_row_count", 0)),
        }

        return (
            f"Simulation: {simulation_label}",
            f"Results source: {results_path}",
            _cache_status_text(results_path, cache_info),
            dataset_meta,
            config_store,
            timeline_meta,
            baseline_options,
            baseline_value,
        )

    @app.callback(
        Output("config-summary-table", "data"),
        Output("config-summary-table", "columns"),
        Output("v2x-workload-comparison-table", "data"),
        Output("v2x-workload-comparison-table", "columns"),
        Output("latency-profile-plot", "figure"),
        Output("jitter-plot", "figure"),
        Output("reception-plot", "figure"),
        Output("counts-plot", "figure"),
        Output("drop-reasons-plot", "figure"),
        Output("drop-rates-plot", "figure"),
        Output("tradeoff-plot", "figure"),
        Input("config-summary-store", "data"),
        Input("dataset-meta-store", "data"),
    )
    def refresh_static(config_summary_payload: dict | None, dataset_meta: dict | None):
        simulation_label = (dataset_meta or {}).get("simulation_label", "Custom")
        config_summary = _frame_from_records((config_summary_payload or {}).get("rows"), CONFIG_SUMMARY_COLUMNS)
        if config_summary.empty:
            empty = _placeholder_figure("No data", "Load a scenario to populate the dashboard.")
            return [], [], [], [], empty, empty, empty, empty, empty, empty, empty

        config_summary_display = _high_load_only_or_all(config_summary)
        v2x_workload_comparison = _build_v2x_workload_comparison(config_summary)
        config_display = _display_frame(config_summary_display, CONFIG_SUMMARY_TABLE_COLUMNS)
        v2x_workload_display = _display_frame(v2x_workload_comparison, V2X_WORKLOAD_COMPARISON_COLUMNS)

        return (
            config_display.to_dict("records"),
            _table_columns(CONFIG_SUMMARY_TABLE_COLUMNS),
            v2x_workload_display.to_dict("records"),
            _table_columns(V2X_WORKLOAD_COMPARISON_COLUMNS),
            _plot_latency_profile(config_summary_display, simulation_label),
            _plot_jitter(config_summary_display, simulation_label),
            _plot_reception_efficiency(config_summary_display, simulation_label),
            _plot_counts(config_summary_display, simulation_label),
            _plot_drop_reasons(config_summary_display, simulation_label),
            _plot_drop_rates(config_summary_display, simulation_label),
            _plot_tradeoff(config_summary_display, simulation_label),
        )

    @app.callback(
        Output("comparison-table", "data"),
        Output("comparison-table", "columns"),
        Output("delta-tradeoff-plot", "figure"),
        Input("config-summary-store", "data"),
        Input("dataset-meta-store", "data"),
        Input("baseline-select", "value"),
    )
    def refresh_baseline(config_summary_payload: dict | None, dataset_meta: dict | None, baseline_config: str | None):
        simulation_label = (dataset_meta or {}).get("simulation_label", "Custom")
        config_summary = _frame_from_records((config_summary_payload or {}).get("rows"), CONFIG_SUMMARY_COLUMNS)
        if config_summary.empty:
            empty = _placeholder_figure("Comparison vs Baseline", "Load a scenario to compare high-load configs.")
            return [], [], empty

        comparison_summary, baseline_used = _build_comparison_summary(config_summary, baseline_config)
        comparison_display = _display_frame(comparison_summary, COMPARISON_COLUMNS)

        return (
            comparison_display.to_dict("records"),
            _table_columns(COMPARISON_COLUMNS),
            _plot_delta_tradeoff(comparison_summary, simulation_label, baseline_used),
        )

    @app.callback(
        Output("throughput-timeline-plot", "figure"),
        Output("simulation-timeline-plot", "figure"),
        Output("timeline-status", "children"),
        Input("load-timelines-button", "n_clicks"),
        Input("dataset-meta-store", "data"),
        Input("timeline-meta-store", "data"),
    )
    def refresh_timelines(_n_clicks: int, dataset_meta: dict | None, timeline_meta: dict | None):
        simulation_label = (dataset_meta or {}).get("simulation_label", "Custom")
        waiting_throughput = _placeholder_figure(
            f"Throughput Timeline ({simulation_label})",
            "Click Load Timelines to read the cached per-second timeline.",
        )
        waiting_state = _placeholder_figure(
            f"State Timeline ({simulation_label})",
            "Click Load Timelines to read the cached state occupancy timeline.",
        )

        if not dataset_meta:
            return waiting_throughput, waiting_state, "Load a scenario before requesting timelines."

        if ctx.triggered_id != "load-timelines-button":
            return waiting_throughput, waiting_state, "Timelines load on demand from the cache to keep the dashboard responsive."

        if not (timeline_meta or {}).get("available"):
            return waiting_throughput, waiting_state, "No cached timeline data is available for this scenario."

        try:
            timeline_frame = load_cached_timeline_rows(
                Path(dataset_meta["results_path"]),
                bin_size_s=float(dataset_meta.get("bin_size_s", DEFAULT_TIMELINE_BIN_SIZE_S)),
            )
        except Exception as exc:
            error_fig = _placeholder_figure(
                f"Throughput Timeline ({simulation_label})",
                f"Failed to load cached timeline data: {exc}",
            )
            return error_fig, waiting_state, f"Failed to load cached timeline data: {exc}"

        return (
            _plot_throughput_timeline(timeline_frame, simulation_label),
            _plot_simulation_timeline(timeline_frame, simulation_label),
            f"Loaded {len(timeline_frame)} cached timeline row(s) from {dataset_meta['results_path']}.",
        )

    @app.callback(
        Output("feedback-snapshot", "value"),
        Output("feedback-export-store", "data"),
        Input("generate-feedback-button", "n_clicks"),
        Input("dataset-meta-store", "data"),
        Input("config-summary-store", "data"),
        Input("baseline-select", "value"),
    )
    def refresh_feedback_snapshot(
        _n_clicks: int,
        dataset_meta: dict | None,
        config_summary_payload: dict | None,
        baseline_config: str | None,
    ):
        if not dataset_meta:
            return _default_snapshot_text(), {}

        if ctx.triggered_id != "generate-feedback-button":
            return _default_snapshot_text(), {}

        config_summary = _frame_from_records((config_summary_payload or {}).get("rows"), CONFIG_SUMMARY_COLUMNS)
        try:
            snapshot = _build_feedback_snapshot(
                Path(dataset_meta["results_path"]),
                str(dataset_meta.get("simulation_label", "Custom")),
                config_summary,
                baseline_config,
            )
        except Exception as exc:
            return f"Failed to generate snapshot: {exc}", {}

        return json.dumps(snapshot, indent=2), snapshot

    @app.callback(
        Output("download-feedback-json", "data"),
        Input("download-feedback-button", "n_clicks"),
        State("feedback-export-store", "data"),
        prevent_initial_call=True,
    )
    def download_feedback_snapshot(_n_clicks: int, snapshot: dict | None):
        if not snapshot:
            return {"content": "{}", "filename": "dashboard_snapshot.json"}
        simulation = str(snapshot.get("simulation_label", "custom")).strip().lower().replace(" ", "_")
        filename = f"dashboard_snapshot_{simulation}.json"
        return {"content": json.dumps(snapshot, indent=2), "filename": filename}

    return app


def main():
    parser = argparse.ArgumentParser(description="Run Veins QoS KPI dashboard.")
    parser.add_argument(
        "--results",
        type=Path,
        default=_default_results_dir(),
        help="Directory containing OMNeT++ .sca result files.",
    )
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8050)
    parser.add_argument("--debug", action="store_true")
    args = parser.parse_args()

    app = build_app(args.results.resolve())
    app.run(host=args.host, port=args.port, debug=args.debug)


if __name__ == "__main__":
    main()
