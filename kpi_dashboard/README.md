# Veins QoS KPI Dashboard

Rust local dashboard and publication-figure exporter for OMNeT++ KPI analysis. The dashboard keeps the run tables auditable, while `export_figures` produces deterministic comparative figures for paper writing.

## Setup

Install Rust with `cargo` available on `PATH`.

From this directory:

```bash
cd /home/goaguiar/master/master_veins/kpi_dashboard
cargo test
```

## Run

Default result lookup checks these folders in order:

- `../veins_qos/simulations/veins_inet_highway_heavy/results`
- `../veins_qos/simulations/veins_inet_highway_light/results`

```bash
cd /home/goaguiar/master/master_veins/kpi_dashboard
cargo run --release
```

Open:

```text
http://127.0.0.1:8050
```

Custom result directory:

```bash
cargo run --release -- \
  --results /home/goaguiar/master/master_veins/veins_qos/simulations/veins_inet_highway_heavy/results
```

Useful options:

```text
--baseline plain_netload_high
--rebuild
--threads 4
--host 127.0.0.1
--port 8050
```

## Publication Figures

The scientific dashboard strategy and figure rationale are documented in [`SCIENTIFIC_DASHBOARD.md`](SCIENTIFIC_DASHBOARD.md).

Export reproducible figures from collected simulation results:

```bash
cd /home/goaguiar/master/master_veins/kpi_dashboard
cargo run --release --bin export_figures -- \
  --results ../veins_qos/simulations/veins_inet_highway_light/results \
  --results ../veins_qos/simulations/veins_inet_highway_heavy/results \
  --output publication_figures \
  --formats svg,png,pdf \
  --dpi 300
```

SVG is always generated. PNG/PDF export is attempted through `rsvg-convert` or `inkscape` when either converter is installed.

## Cache

The Rust cache is written under each selected result directory:

- `results/.kpi_cache_rs/meta.json`
- `results/.kpi_cache_rs/run_rows.json`
- `results/.kpi_cache_rs/config_summary.json`

When no valid Rust cache exists, the dashboard builds it from raw `.sca` and `.vec` files. Use `--rebuild` to force a fresh parse.

## Tables

- `Comparison vs Baseline`: primary table with absolute values plus delta and percent delta against the baseline.
- `Config Summary`: averaged row per config.
- `Run Details`: one row per OMNeT++ run/result file.
- `V2X Mode Matrix`: stable, guarded, and emergency V2X modes side by side when those configs exist.

Missing values are emitted as JSON `null` and displayed as `N/A`. The dashboard does not fabricate P95 or jitter values; those require vector samples in the `.vec` files.

## KPI Definitions

- `BE/VO mean delay`: weighted mean from OMNeT++ scalar delay means and counts.
- `BE/VO P95 delay`: exact 95th percentile from recorded delay vector samples.
- `BE/VO jitter`: mean absolute change between consecutive packet delays per receiver stream.
- `RX per TX`: receptions per transmission, used because these runs are multicast.
- `VO logical TX`: deduplicated crash-event transmissions when available.
- `VO physical TX`: repeated physical VO transmissions.
- `MAC drops`: total MAC drops plus BE, VO, unclassified, queue-overflow, retry-limit, and normalized drop-rate views when the scalars are present.
- `BE dropped while blocked`, `BE grants suppressed`, and `VO protection activations`: V2X HCF instrumentation counters when available.

## Exported Figure Set

- `fig_01_p95_delay_priority_gap_<density>`: BE vs VO P95 delay under high load.
- `fig_02_mac_drop_rate_by_strategy_load_<density>`: normalized MAC drop heatmap.
- `fig_03_vo_reception_by_strategy_load_<density>`: VO reception heatmap.
- `fig_04_latency_jitter_tradeoff_<density>`: delay/jitter scatter by access category.
- `fig_05_mac_drop_attribution_high_load_<density>`: high-load drop attribution.
- `fig_06_vo_delay_cdf_high_load_<density>`: high-load empirical VO delay CDF when vector samples exist.
- `fig_07_v2x_control_actions_by_load_<density>`: adaptive-control counters when V2X HCF metrics exist.
