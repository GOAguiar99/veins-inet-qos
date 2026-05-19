# Veins QoS KPI Dashboard

**Status:** Active — Rust parser, local audit tables, and publication figure exporter.

## Purpose

Parse OMNeT++ `.sca` / `.vec` results from the highway density study, expose auditable comparison tables, and export deterministic SVG-first figures for the paper.

## Prerequisites

- Rust (`cargo` on `PATH`)
- Simulation results under `veins_qos/simulations/veins_inet_highway_*/results/`
- Optional: `rsvg-convert` or `inkscape` for PNG/PDF conversion

## Quick Start

```bash
cd kpi_dashboard
cargo test
cargo run --release -- --rebuild
```

For publication figures, see [Publication figures](#publication-figures) below (do not pass literal `...`).

Open `http://127.0.0.1:8050`.

## Inputs

| Input | Location |
|-------|----------|
| Scalar results | `results/*.sca` |
| Vector results | `results/*.vec` |
| Rust cache | `results/.kpi_cache_rs/` |

By default the server discovers **both** highway packages when present:

- `veins_inet_highway_light/results` (10 vehicles)
- `veins_inet_highway_heavy/results` (100 vehicles)

Use the **Scenario** dropdown in the UI to switch between them. Light is listed first and selected by default when both exist.

Pass `--results` to lock the dashboard to a single directory:

```bash
cargo run --release -- --results ../veins_qos/simulations/veins_inet_highway_light/results
```

The figure exporter accepts **multiple** `--results` paths in one command.

### CLI Options

| Flag | Default | Meaning |
|------|---------|---------|
| `--baseline` | `plain_netload_high` | Comparison reference config |
| `--rebuild` | off | Force re-parse from raw files |
| `--threads` | auto | Parser parallelism |
| `--host` / `--port` | `127.0.0.1:8050` | HTTP bind |

## Outputs

### Interactive tables

- **Comparison vs Baseline** — deltas vs selected baseline (high-load configs preferred)
- **Config Summary** — arithmetic mean per config over runs
- **Run Details** — one row per result file
- **V2X Mode Matrix** — stable / guarded / emergency × workload (when present)

### Publication figures

```bash
cargo run --release --bin export_figures -- \
  --results ../veins_qos/simulations/veins_inet_highway_light/results \
  --results ../veins_qos/simulations/veins_inet_highway_heavy/results \
  --output publication_figures \
  --formats svg,png,pdf \
  --dpi 300
```

Naming: `fig_{01..07}_{slug}_{highway_light|highway_heavy}.{ext}`

See [`SCIENTIFIC_DASHBOARD.md`](SCIENTIFIC_DASHBOARD.md) for figure rationale.

## Cache and Reproducibility

| File | Role |
|------|------|
| `meta.json` | `schema_version`, `parser_version` (`rust-kpi-dashboard-0.2.0`), file signatures |
| `run_rows.json` | Per-run metrics |
| `config_summary.json` | Per-config means |

Cache invalidates when parser version, schema, or source file name/size/mtime changes.

**Note:** `export_figures` always re-parses raw files (and refreshes cache); it does not skip parsing for speed.

Before analyzing a new experiment batch, clear or archive incompatible `results/` (see top-level [`README.md`](../README.md)).

## Metric Definitions

| Metric | Definition | Caveat |
|--------|------------|--------|
| BE/VO mean delay | Weighted mean from scalar `:mean` × `:count` | Aggregates all `app[0]` modules |
| BE/VO P95 delay | 95th percentile from delay vectors | **`Scenario.node[0].app[0]` only** |
| BE/VO jitter | Mean \|Δdelay\| on consecutive vector samples | Same node[0] scope as P95 |
| VO logical TX | `voLogicalTxPackets:count` if &gt; 0, else physical | Per crash burst sequence |
| VO physical TX | `voTxPackets:count` | Includes `repeatCount` replicas |
| VO / BE RX per TX | Receptions per transmission | Multicast semantics |
| MAC drops / per TX | MAC + normalized drop rate | Attribution heuristic when AC sums ≈ 2× total |
| V2X counters | Protection activations, BE suppressed/dropped | V2X configs only |

Missing data → JSON `null` / UI `N/A` (no fabrication).

## Figure Catalog

| ID | Slug | Content |
|----|------|---------|
| 01 | `p95_delay_priority_gap` | BE vs VO P95 at high load |
| 02 | `mac_drop_rate_by_strategy_load` | Drop rate heatmap |
| 03 | `vo_reception_by_strategy_load` | VO RX per logical TX heatmap |
| 04 | `latency_jitter_tradeoff` | Mean delay vs jitter scatter |
| 05 | `mac_drop_attribution_high_load` | BE/VO/unclassified drops |
| 06 | `vo_delay_cdf_high_load` | Empirical VO delay CDF |
| 07 | `v2x_control_actions_by_load` | V2X control counters |

Figures are **omitted** when required metrics or samples are absent.

## Troubleshooting

| Symptom | Action |
|---------|--------|
| P95 / jitter always N/A | Ensure `.vec` exists; vectors recorded in `omnetpp.ini` |
| Unexpected averages | Check for mixed old/new results in `results/` |
| Missing fig_07 on light density | V2X counters may be zero — expected if no V2X runs |
| PNG/PDF missing | Install `rsvg-convert` or `inkscape`; SVG remains canonical |

## See Also

- [`SCIENTIFIC_DASHBOARD.md`](SCIENTIFIC_DASHBOARD.md)
- [`../docs/PUBLICATION_CHECKLIST.md`](../docs/PUBLICATION_CHECKLIST.md)
- [`../AUDIT_REPORT.md`](../AUDIT_REPORT.md)
