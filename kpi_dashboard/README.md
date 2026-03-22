# Veins QoS KPI Dashboard

This dashboard reads OMNeT++ scalar files (`.sca`) and delay vectors (`.vec`) and plots the core KPIs for the MAC trade-off study:

- `BE` and `VO` mean delay
- `min`, `P95`, and `max` delay
- `jitter` for BE and VO
- `multicast reach` as receptions per transmission
- `TX/RX counts` for BE and VO

It is designed for fast comparisons such as `plain` vs `edca_v2x` inside one scenario package, or legacy pairs such as `highway_plain` vs `highway_edca_v2x`.

The dashboard is scenario-scoped:
- you select one simulation package at a time (`Highway`, `Square`, `Light`, or legacy mixed)
- the plots and table only show runs from that selected scenario's `results/` folder
- it is not intended to mix or compare different simulation packages in one view

## Expected Interpretation

For the adaptive MAC trade-off, the typical expectation is:

- VO path is protected:
- lower VO mean delay
- lower VO tail delay (`P95`, `max`)
- lower VO jitter
- stable or improved VO reach (`RX per TX`)
- BE may pay a penalty:
- higher BE tail delay
- higher BE jitter
- reduced BE reach
- TX/RX counts help detect whether changes are due to scheduling, losses, or traffic generation mismatches

Use this dashboard to check whether that behavior appears in each run pair.

## Setup

From repository root:

```bash
cd /home/goaguiar/master_veins
python3 -m venv .venv
source .venv/bin/activate
pip install -r kpi_dashboard/requirements.txt
```

## Run

Default results directory:
- auto-detects the first existing folder in this order:
- `veins_qos/simulations/veins_inet_highway/results`
- `veins_qos/simulations/veins_inet_square/results`
- `veins_qos/simulations/veins_inet_light/results`
- `veins_qos/simulations/veins_inet/results`

```bash
cd /home/goaguiar/master_veins/kpi_dashboard
python app.py
```

Inside the UI:
- use the `Simulation` dropdown to choose which scenario package to view
- the header shows `Simulation: ...` so it is always clear which package is loaded

Open:
- `http://127.0.0.1:8050`

Custom results directory:

```bash
python app.py --results /absolute/path/to/results
```

Examples:

```bash
python app.py --results /home/goaguiar/master_veins/veins_qos/simulations/veins_inet_highway/results
python app.py --results /home/goaguiar/master_veins/veins_qos/simulations/veins_inet_square/results
python app.py --results /home/goaguiar/master_veins/veins_qos/simulations/veins_inet_light/results
```

## Typical Workflow

1. Run baseline config (for example `plain` or `highway_plain`).
2. Run adaptive config (for example `edca_v2x` or `highway_edca_v2x`) with the same seed.
3. Open dashboard and compare:
   - confirm the selected `Simulation` label matches the scenario you want
   - config summary table
   - latency profile chart (`min`, `mean`, `P95`, `max`)
   - jitter chart
   - multicast reach chart (`RX per TX`)
   - BE/VO TX-RX count chart
   - protection-vs-cost scatter (`BE P95 delay` vs `VO P95 delay`, marker size = `VO RX per TX`)

## KPI Definitions Used

- `BE delay`: weighted mean of `beEndToEndDelay:mean` using `beEndToEndDelay:count` per node (`app[0]`)
- `VO delay`: weighted mean of `voEndToEndDelay:mean` using `voEndToEndDelay:count` per node (`app[0]`)
- `BE min/max delay`: global minimum and maximum from `beEndToEndDelay:min/max`
- `VO min/max delay`: global minimum and maximum from `voEndToEndDelay:min/max`
- `BE/VO P95 delay`: 95th percentile computed from per-packet delay vectors
- `BE/VO jitter`: mean absolute change between consecutive packet delays, computed per receiver stream and aggregated
- `BE TX/RX`: sum of `beTxPackets:count` and `beRxPackets:count` over nodes (`app[0]`)
- `VO TX`: sum of `voTxPackets:count` over nodes (`app[1]`)
- `VO RX`: sum of `voRxPackets:count` over nodes (`app[0]`)

For multicast runs, the dashboard uses `RX per TX` instead of calling this a delivery ratio:
- `be_rx_per_tx = BE_RX / BE_TX`
- `vo_rx_per_tx = VO_RX / VO_TX`

This is more honest for the current experiment because one transmission may be received by multiple vehicles.
