# Veins QoS KPI Dashboard

This dashboard reads OMNeT++ scalar files (`.sca`) and plots the core KPIs for the MAC trade-off study:

- `BE delay` (ms)
- `VO delay` (ms)
- `TX/RX counts` for BE and VO

It is designed for fast comparisons such as `plain` vs `edca_v2x` inside one scenario package, or legacy pairs such as `highway_plain` vs `highway_edca_v2x`.

The dashboard is scenario-scoped:
- you select one simulation package at a time (`Highway`, `Square`, `Light`, or legacy mixed)
- the plots and table only show runs from that selected scenario's `results/` folder
- it is not intended to mix or compare different simulation packages in one view

## Expected Interpretation

For the adaptive MAC trade-off, the typical expectation is:

- VO path is protected (stable or improved VO service)
- BE may pay a penalty (for example increased BE delay)
- TX/RX counts help detect whether changes are due to scheduling or packet loss

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
   - BE delay chart
   - VO delay chart
   - BE/VO TX-RX count chart
   - trade-off scatter (`BE delay` vs `VO TX count`)

## KPI Definitions Used

- `BE delay`: weighted mean of `beEndToEndDelay:mean` using `beEndToEndDelay:count` per node (`app[0]`)
- `VO delay`: weighted mean of `voEndToEndDelay:mean` using `voEndToEndDelay:count` per node (`app[0]`)
- `BE TX/RX`: sum of `beTxPackets:count` and `beRxPackets:count` over nodes (`app[0]`)
- `VO TX`: sum of `voTxPackets:count` over nodes (`app[1]`)
- `VO RX`: sum of `voRxPackets:count` over nodes (`app[0]`)

The app also shows delivery ratios:
- `be_delivery_ratio = BE_RX / BE_TX`
- `vo_delivery_ratio = VO_RX / VO_TX`
