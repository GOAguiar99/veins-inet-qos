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


def _plot_delay(frame: pd.DataFrame, simulation_label: str):
    melted = frame.melt(
        id_vars=["config", "run"],
        value_vars=["be_delay_ms", "vo_delay_ms"],
        var_name="metric",
        value_name="delay_ms",
    )
    melted["metric"] = melted["metric"].map(
        {
            "be_delay_ms": "BE Delay (ms)",
            "vo_delay_ms": "VO Delay (ms)",
        }
    )
    fig = px.bar(
        melted,
        x="config",
        y="delay_ms",
        color="metric",
        barmode="group",
        hover_data=["run"],
        title=f"End-to-End Delay ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Delay (ms)", xaxis_title="Config")
    return fig


def _plot_counts(frame: pd.DataFrame, simulation_label: str):
    melted = frame.melt(
        id_vars=["config", "run"],
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
        hover_data=["run"],
        title=f"TX / RX Packet Counts ({simulation_label})",
    )
    fig.update_layout(yaxis_title="Packet Count", xaxis_title="Config")
    return fig


def _plot_tradeoff(frame: pd.DataFrame, simulation_label: str):
    fig = px.scatter(
        frame,
        x="be_delay_ms",
        y="vo_tx_count",
        color="config",
        hover_name="run",
        title=f"Trade-Off View ({simulation_label}): BE Delay vs VO TX Count",
    )
    fig.update_layout(xaxis_title="BE Delay (ms)", yaxis_title="VO TX Count")
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
            html.Div(id="status", style={"marginTop": "12px", "marginBottom": "12px"}),
            dash_table.DataTable(
                id="summary-table",
                page_size=20,
                style_table={"overflowX": "auto"},
                style_cell={"textAlign": "left", "padding": "6px"},
            ),
            dcc.Graph(id="delay-plot"),
            dcc.Graph(id="counts-plot"),
            dcc.Graph(id="tradeoff-plot"),
        ],
        style={"maxWidth": "1200px", "margin": "0 auto", "padding": "20px"},
    )

    @app.callback(
        Output("simulation-label", "children"),
        Output("results-source", "children"),
        Output("status", "children"),
        Output("summary-table", "data"),
        Output("summary-table", "columns"),
        Output("delay-plot", "figure"),
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
                empty,
                empty,
                empty,
            )

        display = frame.copy()
        for col in ("be_delay_ms", "vo_delay_ms", "be_delivery_ratio", "vo_delivery_ratio"):
            display[col] = display[col].round(6)

        columns = [{"name": col, "id": col} for col in display.columns]
        status = f"Loaded {len(display)} run(s) from {results_path}"

        return (
            f"Simulation: {simulation_label}",
            f"Results source: {results_path}",
            status,
            display.to_dict("records"),
            columns,
            _plot_delay(frame, simulation_label),
            _plot_counts(frame, simulation_label),
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
