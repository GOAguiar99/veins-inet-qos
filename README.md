# Crash-Aware EDCA Traffic for Veins

This repository contains a master's-project workspace focused on one question:

Can crash-critical packets (VO) get better service than normal traffic (BE) in Veins/INET Wi-Fi simulations, and what is the BE penalty?

The active project is `veins_qos/`.

## Repository Structure

- `veins_qos/`: active code, scenarios, and configs.
- `kpi_dashboard/`: Dash app for KPI visualization from `.sca` files.
- `uppaal/`: offline timed-automata model + queries for MAC-policy verification.
- `inet/`, `veins/`, `omnetpp-6.1/`: framework/dependency trees.

For AI/dev context details, see [`veins_qos/AI_CONTEXT.md`](veins_qos/AI_CONTEXT.md).

## Current Experiment Matrix

Active OMNeT++ configs (`veins_qos/simulations/veins_inet/omnetpp.ini`):

- square: `plain`, `edca_only`, `edca_v2x`
- highway: `highway_plain`, `highway_edca_only`, `highway_edca_v2x`

## Quick Start

1. Start Veins launch daemon:

```bash
cd /home/goaguiar/master_veins/veins/bin
./veins_launchd -vv
```

2. Run one simulation (new terminal):

```bash
cd /home/goaguiar/master_veins/veins_qos/simulations/veins_inet
./run -u Cmdenv -c highway_edca_v2x
```

3. Results are written to:

- `veins_qos/simulations/veins_inet/results/`

4. Open KPI dashboard:

```bash
cd /home/goaguiar/master_veins
python3 -m venv .venv
source .venv/bin/activate
pip install -r kpi_dashboard/requirements.txt
python kpi_dashboard/app.py
```

Then open `http://127.0.0.1:8050`.
