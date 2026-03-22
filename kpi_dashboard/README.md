# Veins QoS KPI Dashboard

This dashboard reads OMNeT++ scalar files (`.sca`) and plots the core KPIs for the MAC trade-off study:

- `BE delay` (ms)
- `VO delay` (ms)
- `TX/RX counts` for BE and VO

It is designed for fast comparisons such as `highway_plain` vs `highway_edca_v2x`.

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
- `veins_qos/simulations/veins_inet/results`

```bash
cd /home/goaguiar/master_veins/kpi_dashboard
python app.py
```

Open:
- `http://127.0.0.1:8050`

Custom results directory:

```bash
python app.py --results /absolute/path/to/results
```

## KPI Definitions Used

- `BE delay`: weighted mean of `beEndToEndDelay:mean` using `beEndToEndDelay:count` per node (`app[0]`)
- `VO delay`: weighted mean of `voEndToEndDelay:mean` using `voEndToEndDelay:count` per node (`app[0]`)
- `BE TX/RX`: sum of `beTxPackets:count` and `beRxPackets:count` over nodes (`app[0]`)
- `VO TX`: sum of `voTxPackets:count` over nodes (`app[1]`)
- `VO RX`: sum of `voRxPackets:count` over nodes (`app[0]`)

The app also shows delivery ratios:
- `be_delivery_ratio = BE_RX / BE_TX`
- `vo_delivery_ratio = VO_RX / VO_TX`
