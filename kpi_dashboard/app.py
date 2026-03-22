from __future__ import annotations

import argparse
import json
import math
from datetime import datetime, timezone
from pathlib import Path

import pandas as pd
import plotly.express as px
from dash import Dash, Input, Output, State, dash_table, dcc, html

try:
    from .data_loader import load_results, load_timeseries
except ImportError:
    from data_loader import load_results, load_timeseries


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
    "mac_drop_count",
    "mac_drop_queue_overflow_count",
    "mac_drop_retry_limit_count",
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
    "mac_drop_per_tx_delta",
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
    "mac_drop_count": "MAC Packet Drop Count",
    "mac_drop_queue_overflow_count": "MAC Drop Queue Overflow",
    "mac_drop_retry_limit_count": "MAC Drop Retry Limit",
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
    "mac_drop_per_tx_delta": "MAC Drops per App TX Delta",
}

ROUND_COLUMNS = [
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
    "mac_drop_delta_count",
    "mac_drop_per_tx_delta",
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
    "mac_drop_count",
    "mac_drop_per_tx",
]


def _records_for_json(frame: pd.DataFrame) -> list[dict]:
    if frame.empty:
        return []
    safe = frame.replace({float("inf"): None, float("-inf"): None}).where(pd.notnull(frame), None)
    return safe.to_dict("records")


def _build_feedback_snapshot(payload: dict | None, baseline_config: str | None) -> dict:
    if not payload:
        return {}

    rows = payload.get("rows", [])
    simulation_label = payload.get("simulation_label", "Custom")
    if not rows:
        return {
            "generated_at_utc": datetime.now(timezone.utc).isoformat(),
            "simulation_label": simulation_label,
            "message": "No result rows loaded.",
        }

    frame = pd.DataFrame(rows)
    config_summary = _build_config_summary(frame)
    comparison_summary, baseline_used = _build_comparison_summary(config_summary, baseline_config)
    config_list = _ordered_configs(frame["config"].astype(str).unique().tolist())
    run_columns = [column for column in RUN_EXPORT_COLUMNS if column in frame.columns]
    if run_columns:
        sort_by = [column for column in ("config", "run") if column in run_columns] or [run_columns[0]]
        run_level = _display_frame(frame, run_columns).sort_values(sort_by).reset_index(drop=True)
    else:
        run_level = pd.DataFrame()

    return {
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "simulation_label": simulation_label,
        "selected_baseline": baseline_config,
        "baseline_used": baseline_used,
        "run_count": int(len(frame)),
        "config_count": int(frame["config"].nunique()),
        "configs": config_list,
        "run_level_metrics": _records_for_json(run_level),
        "config_summary": _records_for_json(_display_frame(config_summary, CONFIG_SUMMARY_COLUMNS)),
        "comparison_vs_baseline": _records_for_json(_display_frame(comparison_summary, COMPARISON_COLUMNS)),
    }


def _default_results_dir() -> Path:
    simulations_dir = (Path(__file__).resolve().parent / ".." / "veins_qos" / "simulations").resolve()
    candidates = [
        simulations_dir / "veins_inet_highway" / "results",
        simulations_dir / "veins_inet_square" / "results",
        simulations_dir / "veins_inet_light" / "results",
        simulations_dir / "veins_inet" / "results",
    ]
    for candidate in candidates:
        if candidate.exists():
            return candidate
    return candidates[0]


def _scenario_options() -> list[tuple[str, Path]]:
    simulations_dir = (Path(__file__).resolve().parent / ".." / "veins_qos" / "simulations").resolve()
    return [
        ("Highway", simulations_dir / "veins_inet_highway" / "results"),
        ("Square", simulations_dir / "veins_inet_square" / "results"),
        ("Light", simulations_dir / "veins_inet_light" / "results"),
        ("Legacy Mixed", simulations_dir / "veins_inet" / "results"),
    ]


def _infer_simulation_label(results_dir: Path) -> str:
    mapping = {
        "veins_inet_highway": "Highway",
        "veins_inet_square": "Square",
        "veins_inet_light": "Light",
        "veins_inet": "Legacy Mixed",
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


def _safe_pct_delta(value: float, baseline_value: float) -> float:
    if math.isnan(value) or math.isnan(baseline_value) or baseline_value == 0:
        return math.nan
    return ((value - baseline_value) / baseline_value) * 100.0


def _build_config_summary(frame: pd.DataFrame) -> pd.DataFrame:
    numeric_columns = [column for column in CONFIG_SUMMARY_COLUMNS if column not in {"config", "runs"}]
    summary = (
        frame.groupby("config", as_index=False)[numeric_columns]
        .mean(numeric_only=True)
        .sort_values("config")
        .reset_index(drop=True)
    )
    run_counts = frame.groupby("config").size().rename("runs").reset_index()
    merged = run_counts.merge(summary, on="config")
    order = _ordered_configs(merged["config"].astype(str).tolist())
    merged["config"] = pd.Categorical(merged["config"], categories=order, ordered=True)
    merged = merged.sort_values("config").reset_index(drop=True)
    merged["config"] = merged["config"].astype(str)
    return merged[CONFIG_SUMMARY_COLUMNS]


def _build_comparison_summary(config_summary: pd.DataFrame, baseline_config: str | None) -> tuple[pd.DataFrame, str | None]:
    if config_summary.empty:
        return pd.DataFrame(columns=COMPARISON_COLUMNS), None

    config_values = config_summary["config"].astype(str).tolist()
    baseline = baseline_config if baseline_config in set(config_values) else _preferred_baseline(config_values)
    if baseline is None:
        return pd.DataFrame(columns=COMPARISON_COLUMNS), None

    baseline_row = config_summary.loc[config_summary["config"] == baseline]
    if baseline_row.empty:
        return pd.DataFrame(columns=COMPARISON_COLUMNS), None
    base = baseline_row.iloc[0]

    rows: list[dict[str, float | str]] = []
    for _, row in config_summary.iterrows():
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
        comparison_row["mac_drop_delta_count"] = row["mac_drop_count"] - base["mac_drop_count"]
        comparison_row["mac_drop_per_tx_delta"] = row["mac_drop_per_tx"] - base["mac_drop_per_tx"]
        rows.append(comparison_row)

    comparison = pd.DataFrame(rows)
    order = _ordered_configs(comparison["config"].astype(str).tolist())
    comparison["config"] = pd.Categorical(comparison["config"], categories=order, ordered=True)
    comparison = comparison.sort_values("config").reset_index(drop=True)
    comparison["config"] = comparison["config"].astype(str)
    return comparison[COMPARISON_COLUMNS], baseline


def _plot_latency_profile(frame: pd.DataFrame, simulation_label: str):
    id_vars = [column for column in ("config", "run") if column in frame.columns]
    hover_data = ["run"] if "run" in frame.columns else None
    melted = frame.melt(
        id_vars=id_vars,
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
        hover_data=hover_data,
        title=f"Latency Profile ({simulation_label})",
    )
    fig.for_each_annotation(lambda annotation: annotation.update(text=annotation.text.split("=")[-1]))
    fig.update_layout(height=760, yaxis_title="Delay (ms)", xaxis_title="Config")
    return fig


def _plot_jitter(frame: pd.DataFrame, simulation_label: str):
    id_vars = [column for column in ("config", "run") if column in frame.columns]
    hover_data = ["run"] if "run" in frame.columns else None
    melted = frame.melt(
        id_vars=id_vars,
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
        hover_data=hover_data,
        title=f"Delay Variation ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Jitter (ms)", xaxis_title="Config")
    return fig


def _plot_reception_efficiency(frame: pd.DataFrame, simulation_label: str):
    id_vars = [column for column in ("config", "run") if column in frame.columns]
    hover_data = ["run"] if "run" in frame.columns else None
    melted = frame.melt(
        id_vars=id_vars,
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
        hover_data=hover_data,
        title=f"Multicast Reach ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Receptions per Transmission", xaxis_title="Config")
    return fig


def _plot_counts(frame: pd.DataFrame, simulation_label: str):
    id_vars = [column for column in ("config", "run") if column in frame.columns]
    hover_data = ["run"] if "run" in frame.columns else None
    melted = frame.melt(
        id_vars=id_vars,
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
        hover_data=hover_data,
        title=f"TX / RX Packet Counts ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Packet Count", xaxis_title="Config")
    return fig


def _plot_tradeoff(frame: pd.DataFrame, simulation_label: str):
    fig = px.scatter(
        frame,
        x="be_delay_p95_ms",
        y="vo_delay_p95_ms",
        size="vo_rx_per_tx",
        color="config",
        hover_name="run",
        title=f"Protection vs Cost ({simulation_label})",
    )
    fig.update_layout(
        xaxis_title="BE P95 Delay (ms)",
        yaxis_title="VO P95 Delay (ms)",
    )
    return fig


def _plot_delta_tradeoff(comparison_summary: pd.DataFrame, simulation_label: str, baseline_config: str | None):
    if comparison_summary.empty or baseline_config is None:
        return px.scatter(title="Delta Protection vs Cost (no baseline)")

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


def _plot_packet_loss(frame: pd.DataFrame, simulation_label: str):
    id_vars = [column for column in ("config", "run") if column in frame.columns]
    hover_data = ["run"] if "run" in frame.columns else None
    melted = frame.melt(
        id_vars=id_vars,
        value_vars=[
            "mac_drop_count",
            "mac_drop_queue_overflow_count",
            "mac_drop_retry_limit_count",
        ],
        var_name="metric",
        value_name="count",
    )
    melted["metric"] = melted["metric"].map(
        {
            "mac_drop_count": "MAC Drop Total",
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
        hover_data=hover_data,
        title=f"Packet Loss / Drops ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Drop Count", xaxis_title="Config")
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

    return (
        frame.groupby(
            ["config", "time_s"],
            as_index=False,
        )[available_columns]
        .mean(numeric_only=True)
        .sort_values(["config", "time_s"])
    )


def _plot_throughput_timeline(frame: pd.DataFrame, simulation_label: str):
    aggregated = _aggregate_timeline(frame)
    if aggregated.empty:
        return px.scatter(title="Throughput Timeline (no vector data)")

    throughput_columns = []
    for column in ["throughput_kbps", "throughput_be_kbps", "throughput_vo_kbps"]:
        if column in aggregated.columns and not aggregated[column].isna().all():
            throughput_columns.append(column)

    if not throughput_columns:
        return px.scatter(title=f"Throughput Timeline ({simulation_label})")

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
        return px.scatter(title="State Timeline (no vector data)")

    available_metrics = []
    for column in ["active_tx_nodes", "listening_nodes", "blocking_nodes", "sending_nodes"]:
        if column in aggregated.columns and not aggregated[column].isna().all():
            available_metrics.append(column)

    if not available_metrics:
        return px.scatter(title=f"State Timeline ({simulation_label})")

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
            dcc.Store(id="results-store"),
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
            html.H3("Share With AI"),
            html.Div(
                "Use this snapshot to share your current dashboard data in chat for feedback.",
                style={"marginBottom": "8px"},
            ),
            html.Div(
                [
                    dcc.Clipboard(
                        target_id="feedback-snapshot",
                        title="Copy snapshot",
                        style={"display": "inline-block", "fontSize": 20, "cursor": "pointer"},
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
                readOnly=True,
                style={
                    "width": "100%",
                    "height": "260px",
                    "fontFamily": "monospace",
                    "fontSize": "12px",
                },
            ),
            html.H3("Throughput Timeline"),
            dcc.Graph(id="throughput-timeline-plot"),
            html.H3("State Timeline"),
            dcc.Graph(id="simulation-timeline-plot"),
            dcc.Graph(id="latency-profile-plot"),
            dcc.Graph(id="jitter-plot"),
            dcc.Graph(id="reception-plot"),
            dcc.Graph(id="counts-plot"),
            dcc.Graph(id="loss-plot"),
            dcc.Graph(id="tradeoff-plot"),
            dcc.Graph(id="delta-tradeoff-plot"),
        ],
        style={"maxWidth": "1400px", "margin": "0 auto", "padding": "20px"},
    )

    @app.callback(
        Output("simulation-label", "children"),
        Output("results-source", "children"),
        Output("status", "children"),
        Output("results-store", "data"),
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
            frame = load_results(results_path)
            timeline_frame = load_timeseries(results_path, bin_size_s=1.0)
        except Exception as exc:
            status = f"Failed to load results: {exc}"
            return (
                f"Simulation: {simulation_label}",
                f"Results source: {results_path}",
                status,
                {"rows": [], "timeline_rows": [], "simulation_label": simulation_label},
                [],
                None,
            )

        config_values = _ordered_configs(frame["config"].astype(str).unique().tolist())
        baseline_options = [{"label": config, "value": config} for config in config_values]
        baseline_value = current_baseline if current_baseline in set(config_values) else _preferred_baseline(config_values)
        status = f"Loaded {len(frame)} run(s) across {frame['config'].nunique()} config(s) from {results_path}"
        store_payload = {
            "rows": frame.to_dict("records"),
            "timeline_rows": timeline_frame.to_dict("records"),
            "simulation_label": simulation_label,
        }

        return (
            f"Simulation: {simulation_label}",
            f"Results source: {results_path}",
            status,
            store_payload,
            baseline_options,
            baseline_value,
        )

    @app.callback(
        Output("config-summary-table", "data"),
        Output("config-summary-table", "columns"),
        Output("comparison-table", "data"),
        Output("comparison-table", "columns"),
        Output("throughput-timeline-plot", "figure"),
        Output("simulation-timeline-plot", "figure"),
        Output("latency-profile-plot", "figure"),
        Output("jitter-plot", "figure"),
        Output("reception-plot", "figure"),
        Output("counts-plot", "figure"),
        Output("loss-plot", "figure"),
        Output("tradeoff-plot", "figure"),
        Output("delta-tradeoff-plot", "figure"),
        Input("results-store", "data"),
        Input("baseline-select", "value"),
    )
    def refresh(payload: dict | None, baseline_config: str | None):
        if not payload:
            empty = px.scatter(title="No data")
            return (
                [],
                [],
                [],
                [],
                empty,
                empty,
                empty,
                empty,
                empty,
                empty,
                empty,
                empty,
                empty,
            )

        rows = payload.get("rows", [])
        simulation_label = payload.get("simulation_label", "Custom")
        if not rows:
            empty = px.scatter(title="No data")
            return (
                [],
                [],
                [],
                [],
                empty,
                empty,
                empty,
                empty,
                empty,
                empty,
                empty,
                empty,
                empty,
            )

        frame = pd.DataFrame(rows)
        timeline_rows = payload.get("timeline_rows", [])
        timeline_frame = (
            pd.DataFrame(timeline_rows)
            if timeline_rows
            else pd.DataFrame(
                columns=[
                    "config",
                    "time_s",
                    "throughput_kbps",
                    "throughput_be_kbps",
                    "throughput_vo_kbps",
                    "active_tx_nodes",
                    "listening_nodes",
                    "blocking_nodes",
                    "sending_nodes",
                ]
            )
        )
        config_summary = _build_config_summary(frame)
        comparison_summary, baseline_used = _build_comparison_summary(config_summary, baseline_config)
        config_display = _display_frame(config_summary, CONFIG_SUMMARY_COLUMNS)
        comparison_display = _display_frame(comparison_summary, COMPARISON_COLUMNS)

        return (
            config_display.to_dict("records"),
            _table_columns(CONFIG_SUMMARY_COLUMNS),
            comparison_display.to_dict("records"),
            _table_columns(COMPARISON_COLUMNS),
            _plot_throughput_timeline(timeline_frame, simulation_label),
            _plot_simulation_timeline(timeline_frame, simulation_label),
            _plot_latency_profile(config_summary, simulation_label),
            _plot_jitter(config_summary, simulation_label),
            _plot_reception_efficiency(config_summary, simulation_label),
            _plot_counts(config_summary, simulation_label),
            _plot_packet_loss(config_summary, simulation_label),
            _plot_tradeoff(frame, simulation_label),
            _plot_delta_tradeoff(comparison_summary, simulation_label, baseline_used),
        )

    @app.callback(
        Output("feedback-snapshot", "value"),
        Output("feedback-export-store", "data"),
        Input("results-store", "data"),
        Input("baseline-select", "value"),
    )
    def refresh_feedback_snapshot(payload: dict | None, baseline_config: str | None):
        snapshot = _build_feedback_snapshot(payload, baseline_config)
        if not snapshot:
            return "{}", {}
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
