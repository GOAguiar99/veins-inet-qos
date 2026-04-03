# Veins QoS KPI Dashboard

This dashboard reads OMNeT++ scalar files (`.sca`) and delay vectors (`.vec`) and plots the core KPIs for the MAC trade-off study:

- `BE` and `VO` mean delay
- `min`, `P95`, and `max` delay
- `jitter` for BE and VO
- `multicast reach` as receptions per transmission
- `TX/RX counts` for BE and VO
- MAC-level packet drops (`total`, `queue overflow`, `retry limit`)
- MAC-level packet drops by packet type (`BE` via `AC_BE`, `VO` via `AC_VO`)
- unclassified MAC drops (useful for plain DCF runs without AC attribution)
- normalized MAC drops per application TX (overall + BE + VO)
- baseline-aware KPI deltas across configs (`config - baseline`)
- simulation-time network throughput trend
- simulation-time node activity trend (`active TX nodes`)
- simulation-time V2X FSM state occupancy (`LISTENING`, `BLOCKING`, `SENDING`)

It is designed for fast comparisons such as `plain` vs `edca_v2x` inside one scenario package, or legacy pairs such as `highway_plain` vs `highway_edca_v2x`.

The dashboard is scenario-scoped:
- you select one simulation package at a time (`Highway Heavy`, `Highway Light`, `Highway`, `Square`, `Light`, or legacy mixed)
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
- `veins_qos/simulations/veins_inet_highway_heavy/results`
- `veins_qos/simulations/veins_inet_highway_light/results`
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
- use the `Baseline config` dropdown to select the comparison reference
- the header shows `Simulation: ...` so it is always clear which package is loaded

Open:
- `http://127.0.0.1:8050`

Custom results directory:

```bash
python app.py --results /absolute/path/to/results
```

Examples:

```bash
python app.py --results /home/goaguiar/master_veins/veins_qos/simulations/veins_inet_highway_heavy/results
python app.py --results /home/goaguiar/master_veins/veins_qos/simulations/veins_inet_highway_light/results
python app.py --results /home/goaguiar/master_veins/veins_qos/simulations/veins_inet_highway/results
python app.py --results /home/goaguiar/master_veins/veins_qos/simulations/veins_inet_square/results
python app.py --results /home/goaguiar/master_veins/veins_qos/simulations/veins_inet_light/results
```

## Typical Workflow

1. Run baseline config (for example `plain` or `highway_plain`).
2. Run adaptive config (for example `edca_v2x` or `highway_edca_v2x`) with the same seed.
3. Open dashboard and compare:
   - confirm the selected `Simulation` label matches the scenario you want
   - select `Baseline config` (usually `plain` or `edca_only`)
   - config summary table (single main table)
   - latency profile chart (`min`, `mean`, `P95`, `max`)
   - jitter chart
   - multicast reach chart (`RX per TX`)
   - BE/VO TX-RX count chart
   - MAC drop breakdown chart (`total`, `BE`, `VO`, `queue overflow`, `retry limit`)
   - normalized drop-rate chart (`overall`, `BE per BE_TX`, `VO per VO_TX`)
   - simulation timeline chart (throughput + active TX nodes over time)
   - protection-vs-cost scatter (`BE P95 delay` vs `VO P95 delay`, marker size = `VO RX per TX`)
   - comparison-vs-baseline table (absolute + percent deltas)
   - delta protection-vs-cost scatter (`BE P95 delta` vs `VO P95 delta`)
   - `Share With AI` section:
   - click the copy icon to copy a JSON snapshot
   - or click `Download Snapshot JSON`
   - paste the snapshot in chat for feedback

## Share Snapshot With AI

The dashboard includes a `Share With AI` panel that generates a JSON snapshot from the currently loaded data and selected baseline.

Snapshot includes:
- simulation label
- selected and effective baseline
- run/config counts
- run-level KPI rows
- config summary table values
- comparison vs baseline values

Workflow:
1. Load your scenario and baseline in the dashboard.
2. Open `Share With AI`.
3. Click copy icon (clipboard) or download JSON.
4. Paste the JSON in chat and ask for KPI interpretation.

## Comparison View

The `Comparison vs Baseline` table helps answer:

- Did VO improve? (`VO P95 Delta`, `VO Mean Delta`, `VO Jitter Delta` should be negative)
- What did BE pay? (`BE P95 Delta`, `BE Mean Delta`, `BE Jitter Delta` often become positive)
- Did multicast reach change? (`VO/BE RX per TX Delta`)

The delta scatter uses:

- `x = BE P95 Delta (ms)` where positive means higher BE delay cost
- `y = VO P95 Delta (ms)` where negative means better VO protection

The lower-right area indicates stronger VO protection with BE penalty, which is the expected adaptive-MAC trade-off region.

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
- `MAC drops (total)`: sum of `packetDrop:count` over `Scenario.node[*].wlan[*].mac`
- `MAC sum of all drops`: alias of `MAC drops (total)` for explicit JSON export
- `MAC drops (BE)`: sum of `droppedPacketsQueueOverflow:count` + `retryLimitReached:count` over `Scenario.node[*].wlan[*].mac.hcf.edca.edcaf[1]` (`AC_BE`)
- `MAC drops (VO)`: sum of `droppedPacketsQueueOverflow:count` + `retryLimitReached:count` over `Scenario.node[*].wlan[*].mac.hcf.edca.edcaf[3]` (`AC_VO`)
- `MAC drops (unclassified)`: `packetDropAcUnclassifiedCount` when available; otherwise falls back to total MAC drops when BE/VO attribution is absent (typical in plain DCF)
- `MAC drops (queue overflow)`: sum of `packetDropQueueOverflow:count` over `Scenario.node[*].wlan[*].mac`
- `MAC drops (retry limit)`: sum of `packetDropRetryLimitReached:count` over `Scenario.node[*].wlan[*].mac`
- `MAC drops per app TX`: `mac_drop_sum_count / (BE_TX + VO_TX)`
- `MAC BE drops per BE TX`: `mac_drop_be_count / BE_TX`
- `MAC VO drops per VO TX`: `mac_drop_vo_count / VO_TX`

For legacy runs without per-AC attribution scalars, the dashboard falls back to EDCAF queue-overflow/retry counters (or `NaN` when unavailable, for example plain DCF).
- If `V2xIeee80211Mac` instrumentation is enabled, BE/VO totals are read from
  top-level MAC scalars such as `packetDropAcBeCount` and `packetDropAcVoCount`.
- `Network throughput over time`: total aggregated from `app[*].packetSent:vector(packetBytes)` binned per second
- `BE throughput over time`: aggregated from `app[0].packetSent:vector(packetBytes)` binned per second
- `VO throughput over time`: aggregated from `app[1].packetSent:vector(packetBytes)` binned per second
- `Active TX nodes over time`: count of distinct nodes with at least one app TX in each 1s bin
- `V2X state occupancy`: count of nodes in each `V2xEdcaFsmController` state per 1s bin, from `v2xState:vector`

For multicast runs, the dashboard uses `RX per TX` instead of calling this a delivery ratio:
- `be_rx_per_tx = BE_RX / BE_TX`
- `vo_rx_per_tx = VO_RX / VO_TX`

This is more honest for the current experiment because one transmission may be received by multiple vehicles.

The drop breakdown and normalized drop-rate charts help check whether changes in delay/reach are coupled with stronger MAC-level losses.

The dashboard shows timeline data in two separate plots:
- a throughput timeline split into Total, BE, and VO subplots
- a state timeline (active TX nodes plus LISTENING/BLOCKING/SENDING occupancy)

This avoids large throughput values visually compressing state-count curves.

If `LISTENING/BLOCKING/SENDING` curves are missing, rerun simulations after rebuilding `veins_qos` so the new `v2xState` signal is recorded into `.vec`.
