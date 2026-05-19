# Crash-Aware Vehicular QoS Workspace

**Status:** Active research workspace (master's project)

## Research Question

Can crash-critical traffic obtain better service than ordinary traffic under Wi-Fi contention, and what performance cost does that impose on best-effort (BE) traffic?

The implementation deliberately uses a **two-class** model (BE vs crash VO) for clarity and reproducibility.

## Repository Layout

| Path | Role |
|------|------|
| [`veins_qos/`](veins_qos/) | OMNeT++ simulations, traffic apps, QoS classifier, V2X MAC |
| [`kpi_dashboard/`](kpi_dashboard/) | Rust KPI tables and publication figure export |
| [`inet/`](inet/), [`veins/`](veins/) | Framework submodules (read-only) |
| [`docs/`](docs/) | README standard, publication checklist, deprecated modes |
| [`AUDIT_REPORT.md`](AUDIT_REPORT.md), [`REPOSITORY_MAP.md`](REPOSITORY_MAP.md) | Maintainer audit artifacts |

## Active Experiments (Paper Baseline)

**Scenario packages** (vehicle density):

- [`veins_qos/simulations/veins_inet_highway_light/`](veins_qos/simulations/veins_inet_highway_light/) — 10 vehicles
- [`veins_qos/simulations/veins_inet_highway_heavy/`](veins_qos/simulations/veins_inet_highway_heavy/) — 100 vehicles

**MAC policies** (× network load `low` / `medium` / `high`):

| Config prefix | Label | Behavior |
|---------------|-------|----------|
| `plain_netload_*` | DCF | Non-QoS baseline |
| `edca_only_netload_*` | EDCA | DSCP → AC via `QosClassifier` |
| `edca_v2x_vo_stable_netload_*` | Stable | Mild adaptive BE suppression around VO |
| `edca_v2x_vo_guarded_netload_*` | Guarded | Shorter blocking windows |
| `edca_v2x_vo_emergency_netload_*` | Emergency | BE drop/suppression during VO protection |

**Legacy (not for current paper runs):** `veins_inet_square`, `veins_inet_highway` (includes deprecated `edca_v2x_be_friendly`, `edca_v2x_vo_protect`), `veins_inet_light` (smoke tests).

## Prerequisites

- OMNeT++ 6.1, INET, Veins (see framework docs in submodule trees)
- SUMO via `veins_launchd`
- Rust toolchain for `kpi_dashboard`

Build the simulation:

```bash
cd veins_qos
make makefiles && make
```

## Reproduce Simulations

**1. Start TraCI**

```bash
cd veins/bin
./veins_launchd -vv
```

**2. Run experiments** (example: light density, core high-load matrix)

```bash
cd veins_qos/simulations/veins_inet_highway_light
RUNS=0 UI=Cmdenv ./run_matrix.sh
```

| Profile | Use when |
|---------|----------|
| `core` (default) | Five high-load configs, one run index |
| `full` | All 15 policy × load combinations |
| `quick` | Plain + guarded high-load only |

Full sweep with repeated seeds:

```bash
cd veins_qos/simulations/veins_inet_highway_light
PROFILE=full RUNS=0..9 UI=Cmdenv EXTRA_ARGS="--repeat=10" ./run_matrix.sh
```

**Results hygiene:** Archive or delete old `results/` before mixing incompatible traffic profiles.

## Packet and QoS Flow

1. **BE** — `CritPacketSender` (app 0), DSCP `0`, UDP multicast `224.0.0.1`
2. **Crash VO** — `CrashBurstApp` (app 1) on `targetNodeIndex` 0: TraCI stop + DSCP `46` bursts (30s–60s in 70s runs)
3. **Classifier** — DSCP `46` → Wi-Fi voice priority; else BE
4. **MAC** — DCF, EDCA, or `V2xHcf` (stable / guarded / emergency)

See [`veins_qos/AI_CONTEXT.md`](veins_qos/AI_CONTEXT.md) for module-level detail.

## Analyze Results

**Interactive tables**

```bash
cd kpi_dashboard
cargo run --release -- --rebuild
```

Open `http://127.0.0.1:8050` (default baseline: `plain_netload_high`).

**Publication figures**

```bash
cd kpi_dashboard
cargo run --release --bin export_figures -- \
  --results ../veins_qos/simulations/veins_inet_highway_light/results \
  --results ../veins_qos/simulations/veins_inet_highway_heavy/results \
  --output publication_figures \
  --formats svg,png,pdf \
  --dpi 300
```

## Documentation Index

- Implementation context: [`veins_qos/AI_CONTEXT.md`](veins_qos/AI_CONTEXT.md)
- Dashboard & metrics: [`kpi_dashboard/README.md`](kpi_dashboard/README.md)
- Figure strategy: [`kpi_dashboard/SCIENTIFIC_DASHBOARD.md`](kpi_dashboard/SCIENTIFIC_DASHBOARD.md)
- README template: [`docs/README_STANDARD.md`](docs/README_STANDARD.md)
- Pre-submission: [`docs/PUBLICATION_CHECKLIST.md`](docs/PUBLICATION_CHECKLIST.md)
- Legacy modes: [`docs/DEPRECATED_MODES.md`](docs/DEPRECATED_MODES.md)
- Scenario READMEs under `veins_qos/simulations/*/README`

## Framework Documentation

- INET: [`inet/README.md`](inet/README.md)
- Veins: [`veins/README.txt`](veins/README.txt)
- OMNeT++: [`omnetpp-6.1/README`](omnetpp-6.1/README)
