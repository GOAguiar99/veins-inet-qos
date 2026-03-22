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
    return (Path(__file__).resolve().parent / ".." / "veins_qos" / "simulations" / "veins_inet" / "results").resolve()


def _plot_delay(frame: pd.DataFrame):
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
        title="End-to-End Delay",
    )
    fig.update_layout(yaxis_title="Delay (ms)", xaxis_title="Config")
    return fig


def _plot_counts(frame: pd.DataFrame):
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
        title="TX / RX Packet Counts",
    )
    fig.update_layout(yaxis_title="Packet Count", xaxis_title="Config")
    return fig


def _plot_tradeoff(frame: pd.DataFrame):
    fig = px.scatter(
        frame,
        x="be_delay_ms",
        y="vo_tx_count",
        color="config",
        hover_name="run",
        title="Trade-Off View: BE Delay vs VO TX Count",
    )
    fig.update_layout(xaxis_title="BE Delay (ms)", yaxis_title="VO TX Count")
    return fig


def build_app(results_dir: Path) -> Dash:
    app = Dash(__name__)
    app.title = "Veins QoS KPI Dashboard"

    app.layout = html.Div(
        [
            html.H2("Veins QoS KPI Dashboard"),
            html.Div(f"Results source: {results_dir}", style={"marginBottom": "12px"}),
            html.Button("Reload Results", id="reload-button", n_clicks=0),
            dcc.Store(id="results-path", data=str(results_dir)),
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
        Output("status", "children"),
        Output("summary-table", "data"),
        Output("summary-table", "columns"),
        Output("delay-plot", "figure"),
        Output("counts-plot", "figure"),
        Output("tradeoff-plot", "figure"),
        Input("reload-button", "n_clicks"),
        Input("results-path", "data"),
    )
    def refresh(_n_clicks: int, results_path_raw: str):
        results_path = Path(results_path_raw)
        try:
            frame = load_results(results_path)
        except Exception as exc:
            empty = px.scatter(title="No data")
            status = f"Failed to load results: {exc}"
            return status, [], [], empty, empty, empty

        display = frame.copy()
        for col in ("be_delay_ms", "vo_delay_ms", "be_delivery_ratio", "vo_delivery_ratio"):
            display[col] = display[col].round(6)

        columns = [{"name": col, "id": col} for col in display.columns]
        status = f"Loaded {len(display)} run(s) from {results_path}"

        return (
            status,
            display.to_dict("records"),
            columns,
            _plot_delay(frame),
            _plot_counts(frame),
            _plot_tradeoff(frame),
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
