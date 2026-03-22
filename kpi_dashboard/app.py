from __future__ import annotations

import argparse
from pathlib import Path

import pandas as pd
import plotly.express as px
from dash import Dash, Input, Output, dash_table, dcc, html

try:
    from .data_loader import load_results
except ImportError:
    from data_loader import load_results


RUN_TABLE_COLUMNS = [
    "config",
    "run",
    "source_file",
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
]

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
]


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
    return display[column_order]


def _build_config_summary(frame: pd.DataFrame) -> pd.DataFrame:
    numeric_columns = [column for column in CONFIG_SUMMARY_COLUMNS if column not in {"config", "runs"}]
    summary = (
        frame.groupby("config", as_index=False)[numeric_columns]
        .mean(numeric_only=True)
        .sort_values("config")
        .reset_index(drop=True)
    )
    run_counts = frame.groupby("config").size().rename("runs").reset_index()
    return run_counts.merge(summary, on="config")[CONFIG_SUMMARY_COLUMNS]


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
            html.Div(id="simulation-label", style={"marginBottom": "6px", "fontWeight": "bold"}),
            html.Div(id="results-source", style={"marginBottom": "12px"}),
            html.Button("Reload Results", id="reload-button", n_clicks=0),
            html.Div(id="status", style={"marginTop": "12px", "marginBottom": "8px"}),
            html.Div(
                "Jitter is computed as the mean absolute change between consecutive packet delays per receiver. "
                "RX per TX is used instead of delivery ratio because these runs use multicast.",
                style={"marginBottom": "16px", "color": "#4a5568"},
            ),
            html.H3("Config Summary"),
            dash_table.DataTable(
                id="config-summary-table",
                page_size=20,
                style_table={"overflowX": "auto"},
                style_cell={"textAlign": "left", "padding": "6px"},
            ),
            html.H3("Per-Run Detail"),
            dash_table.DataTable(
                id="summary-table",
                page_size=20,
                style_table={"overflowX": "auto"},
                style_cell={"textAlign": "left", "padding": "6px"},
            ),
            dcc.Graph(id="latency-profile-plot"),
            dcc.Graph(id="jitter-plot"),
            dcc.Graph(id="reception-plot"),
            dcc.Graph(id="counts-plot"),
            dcc.Graph(id="tradeoff-plot"),
        ],
        style={"maxWidth": "1400px", "margin": "0 auto", "padding": "20px"},
    )

    @app.callback(
        Output("simulation-label", "children"),
        Output("results-source", "children"),
        Output("status", "children"),
        Output("config-summary-table", "data"),
        Output("config-summary-table", "columns"),
        Output("summary-table", "data"),
        Output("summary-table", "columns"),
        Output("latency-profile-plot", "figure"),
        Output("jitter-plot", "figure"),
        Output("reception-plot", "figure"),
        Output("counts-plot", "figure"),
        Output("tradeoff-plot", "figure"),
        Input("reload-button", "n_clicks"),
        Input("simulation-select", "value"),
    )
    def refresh(_n_clicks: int, results_path_raw: str):
        results_path = Path(results_path_raw)
        simulation_label = _infer_simulation_label(results_path)
        try:
            frame = load_results(results_path)
        except Exception as exc:
            empty = px.scatter(title="No data")
            status = f"Failed to load results: {exc}"
            return (
                f"Simulation: {simulation_label}",
                f"Results source: {results_path}",
                status,
                [],
                [],
                [],
                [],
                empty,
                empty,
                empty,
                empty,
                empty,
            )

        config_summary = _build_config_summary(frame)
        run_display = _display_frame(frame, RUN_TABLE_COLUMNS)
        config_display = _display_frame(config_summary, CONFIG_SUMMARY_COLUMNS)
        status = f"Loaded {len(frame)} run(s) across {frame['config'].nunique()} config(s) from {results_path}"

        return (
            f"Simulation: {simulation_label}",
            f"Results source: {results_path}",
            status,
            config_display.to_dict("records"),
            _table_columns(CONFIG_SUMMARY_COLUMNS),
            run_display.to_dict("records"),
            _table_columns(RUN_TABLE_COLUMNS),
            _plot_latency_profile(config_summary, simulation_label),
            _plot_jitter(config_summary, simulation_label),
            _plot_reception_efficiency(config_summary, simulation_label),
            _plot_counts(config_summary, simulation_label),
            _plot_tradeoff(frame, simulation_label),
        )

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
