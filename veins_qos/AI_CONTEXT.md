# AI Context for `veins_qos`

## What this project is

`veins_qos` is the active codebase for a master's-project research line on crash-aware QoS in vehicular networks.

The current implementation is intentionally minimal:
- normal traffic is sent as Best Effort (BE)
- crash traffic is sent as Voice (VO) by using DSCP `46`
- a custom classifier maps DSCP to Wi-Fi user priority
- simulations compare plain DCF against EDCA behavior

The main question is simple and important:

Can crash-critical traffic actually obtain better channel access and better end-to-end performance than ordinary traffic in Veins/INET Wi-Fi simulations, especially under contention?

## What this project is not

This codebase is not trying to be a full ITS stack, a generic QoS framework, or a policy engine.

Do not assume the goal is to add:
- complex scheduling frameworks
- orchestration layers
- many traffic classes just because EDCA supports them
- infrastructure-heavy AP/RSU logic by default

The minimal design is deliberate because it keeps the thesis baseline easy to explain, reproduce, and measure.

## Repository boundaries

Use this mental model when working in the repo:

- `veins_qos/` is the active project and the normal target for edits.
- `veins_qos/src/traffic/` contains the custom application-layer traffic generators.
- `veins_qos/src/qos/` contains the DSCP to user-priority classifier.
- `veins_qos/src/veins_inet/` contains the Veins+INET integration base and car module used by this project.
- `veins_qos/simulations/veins_inet/` contains the active OMNeT++/SUMO scenarios and experiment configs.
- `ap_servers/` contains old simulations and should not guide current design decisions.
- `inet/`, `veins/`, and `omnetpp-6.1/` are dependency/framework trees and are usually read-only.

Supporting documentation and analysis tools:

- `/README.md` for top-level quick start
- `/kpi_dashboard/README.md` for KPI plotting and result comparison
- `/uppaal/README.md` for formal verification workflow

If there is any duplicate-looking project tree elsewhere, prefer the top-level `veins_qos/` directory as the authoritative source unless the user explicitly says otherwise.

## Thesis framing

Confirmed from the current code and configs, the master's-project idea is:

1. Model ordinary vehicular traffic as periodic BE packets.
2. Model an accident or crash event as a temporary state change in one vehicle.
3. Escalate the crash vehicle's traffic to VO by DSCP marking and EDCA classification.
4. Compare whether this prioritization survives contention and improves service for critical traffic.
5. Measure the trade-off between helping critical traffic and degrading ordinary traffic.

This means the project sits at the intersection of:
- vehicular networking
- QoS-aware wireless medium access
- event-driven priority escalation
- safety-oriented communication under congestion

## Working research hypotheses

The current master's-project direction strongly suggests these hypotheses:

- H1: crash-triggered VO traffic should achieve lower end-to-end delay than ordinary BE traffic under contention.
- H2: crash-triggered VO traffic should maintain better packet reception than BE traffic when the channel is congested.
- H3: default EDCA may help, but explicit CW/AIFS tuning may be necessary to obtain stronger or more stable prioritization.
- H4: any gain for crash traffic should be evaluated together with the penalty imposed on ordinary BE traffic.
- H5: conclusions should be checked in both simple and more realistic mobility/propagation settings, not only in the square baseline.

## Current experiment story

The current experiment logic is:

- Every vehicle runs `CritPacketSender` and emits periodic multicast traffic.
- That normal traffic is BE by default through DSCP `0`.
- Every vehicle also instantiates `CrashBurstApp`, but only the configured target node becomes active.
- By default, `CrashBurstApp` acts on node index `0`.
- At the configured crash time, that node changes behavior:
  - the vehicle is visually marked red
  - the vehicle is commanded to stop through TraCI
  - it starts sending periodic VO-marked traffic
- After the configured resume interval, the vehicle resumes movement and the crash VO stream stops.

In the default `omnetpp.ini`:
- simulation time is `100s`
- normal traffic uses `exponential(1s)` intervals and `100` byte payloads
- crash traffic starts at `30s`
- crash traffic continues for `30s`
- crash traffic uses `100ms` intervals and `120` byte payloads
- crash burst repeat settings:
  - `repeatCount = 3`
  - `repeatGap = 20ms`
  - `repeatJitter = 5ms`

The applications communicate via UDP multicast to `224.0.0.1` on `wlan0`.

## Packet path and QoS path

The key packet flow is:

1. `CritPacketSender` creates ordinary packets and tags them with DSCP `0`.
2. `CrashBurstApp` creates crash packets and tags them with DSCP `46`.
3. In EDCA configurations, `QosClassifier` reads DSCP and maps:
   - DSCP `46` -> `UP_VO`
   - everything else -> default BE priority
4. The 802.11 MAC then places traffic into the proper access category behavior.
5. KPIs are collected from received packets, excluding local multicast loopback from the sender itself.

This separation is important:
- application modules decide semantic criticality by DSCP marking
- the classifier translates network-layer intent into Wi-Fi MAC priority
- EDCA settings decide how strong the prioritization is at the medium-access level

## Core modules and responsibilities

### `src/traffic/CritPacketSender.*`

Purpose:
- generate baseline BE traffic for all vehicles
- mark packets with configurable DSCP
- measure BE and VO reception and end-to-end delay at the application layer

Important details:
- current active config uses DSCP `0`
- packets carry `CreationTimeTag` so delay can be computed
- self-originated multicast echoes are ignored for KPIs

Main exported signals/statistics:
- `beTxPacketCount`
- `beRxPacketCount`
- `voRxPacketCount`
- `beE2eDelay`
- `voE2eDelay`

### `src/traffic/CrashBurstApp.*`

Purpose:
- represent a crash event as a temporary burst/window of critical traffic
- stop the target vehicle at crash time
- emit VO-marked packets during the crash window

Important details:
- DSCP is fixed to `46`
- only the `targetNodeIndex` becomes active
- default target is node `0`
- crash traffic starts immediately when the event triggers
- traffic stops again when the resume timer fires

Main exported signal/statistic:
- `voTxPacketCount`

### `src/qos/QosClassifier.*`

Purpose:
- provide a minimal, explicit DSCP to Wi-Fi user-priority mapping

Important details:
- prioritizes reliability of DSCP extraction by checking indication tags, then request tags, then safe packet-header parsing
- current policy is intentionally only two-class:
  - crash DSCP -> VO
  - all other traffic -> default BE

This is one of the most thesis-critical files because it connects application intent to EDCA behavior.

### `src/veins_inet/VeinsInetApplicationBase.*`

Purpose:
- provide the UDP/multicast base used by the project applications
- integrate with Veins mobility and TraCI

Important details:
- sends to multicast `224.0.0.1`
- binds UDP ports configured in NED/INI
- exposes access to the vehicle TraCI interface
- joins local multicast groups

### `src/veins_inet/VeinsInetCar.ned`

Purpose:
- define the vehicle host type used by the scenario

For this project, it mainly acts as the integration point where the applications, mobility, Wi-Fi, and INET host stack come together.

## Scenario and configuration matrix

The active simulation family is in `simulations/veins_inet/`.

### Baseline scenario family

Files:
- `Scenario.ned`
- `omnetpp.ini`
- `square/` (launch, SUMO config, network, routes, obstacles)
- `highway/` (launch, SUMO config, network, routes, obstacles)

### Configs in `omnetpp.ini`

`plain`
- non-QoS baseline
- finite DCF pending queue
- DSCP tags may still exist at the application level, but they should not translate into EDCA prioritization here
- use this as the reference case where EDCA prioritization is absent

`edca_only`
- enables `qosStation = true`
- uses finite per-access-category queues
- installs the custom `QosClassifier`
- this is the standard EDCA baseline on this branch

`edca_v2x`
- extends `edca_only`
- swaps in the custom `veins_qos.mac.V2xHcf`
- activates the local LISTENING/BLOCKING/SENDING FSM that suppresses BE requests while VO demand is active
- this is the branch's adaptive V2X EDCA experiment

`highway_edca_only`
- standard EDCA on the highway scenario
- uses the same propagation/environment choices as the highway baseline

`highway_edca_v2x`
- adaptive V2X EDCA on the highway scenario
- use this to check whether the branch's MAC behavior survives more realistic propagation and mobility

No legacy alias configs are kept in the current `.ini`; only the six active configs above are available.

## Wireless and mobility assumptions

The current wireless profile in the active config is close to an 802.11p-style setup:
- op mode `p`
- `5.9 GHz`
- channel `3`
- `10 MHz` bandwidth
- `20 mW` transmit power

The scenario is Veins-driven:
- mobility is controlled through `VeinsInetMobility`
- the manager talks to `sumo-launchd`
- the project can switch between a square scenario and a highway corridor scenario

The highway family is the more realism-oriented branch of the study.

## Main experimental knobs

These are the parameters most likely to matter in future thesis iterations:

- traffic load from `CritPacketSender`
- crash traffic interval and duration from `CrashBurstApp`
- choice of crash source node or number of simultaneous crash sources
- queue capacities in DCF or per EDCA access category
- AIFSN and CW values in the tuned EDCA configuration
- vehicle density and route composition in the SUMO scenarios
- propagation/environment choice, especially the move from square to highway

## Metrics that matter

The code already exposes the thesis-relevant KPIs:
- BE transmitted packet count
- BE received packet count
- VO transmitted packet count
- VO received packet count
- BE end-to-end delay
- VO end-to-end delay

When interpreting results, the important questions are:
- Does VO traffic achieve lower delay than BE during congestion?
- Does crash traffic maintain better reception than BE?
- How much BE performance degrades when VO prioritization is enabled?
- Do the conclusions hold in both square and highway scenarios?
- Does EDCA tuning improve prioritization beyond default EDCA?

## What a future AI should preserve

Unless explicitly asked to change the research direction, preserve these choices:

- keep the baseline understandable and reproducible
- keep the crash logic event-driven and easy to explain
- keep the two-class BE vs VO comparison as the anchor experiment
- keep DCF vs EDCA comparisons possible
- keep square vs highway scenario comparisons possible
- keep KPI collection aligned with packet delivery and delay

Be careful not to accidentally break the research design by:
- adding hidden logic that changes traffic priority without documentation
- moving the classifier outside the EDCA path
- mixing framework experimentation with thesis experimentation
- treating legacy `ap_servers/` material as active project scope

## Good next steps for the master's project

The items below are partly confirmed by the existing code and partly inferred as the natural next thesis directions. They should be treated as the current project backlog unless the user redirects the study.

1. Compare plain DCF, default EDCA, and tuned EDCA under the same traffic conditions.
2. Quantify both benefit and cost:
   - benefit for VO crash traffic
   - cost to ordinary BE traffic
3. Stress the system by varying load:
   - vehicle density
   - send intervals
   - queue capacities
   - crash duration
4. Move from the square scenario to the highway corridor to see whether conclusions survive more realistic propagation and mobility.
5. Study whether one crash source and multiple simultaneous crash sources behave differently.
6. Check whether timing matters:
   - earlier crash event
   - later crash event
   - longer or shorter recovery window
7. Decide whether EDCA defaults are enough or whether explicit CW/AIFS tuning is needed for safety-critical traffic.
8. Keep the current minimal two-class experiment as the baseline before introducing extra traffic classes, RSUs, or more complex policies.

## Practical commands

Typical build flow:

```bash
cd /home/goaguiar/master_veins/veins_qos
make makefiles
make
```

Typical run flow:

```bash
cd /home/goaguiar/master_veins/veins_qos/simulations/veins_inet
./run -u Cmdenv -c edca_v2x
```

Notes:
- `sumo-launchd` must be running before the simulation connects
- the launch config in `omnetpp.ini` chooses the scenario family
- the most important place to inspect experiment behavior is `simulations/veins_inet/omnetpp.ini`

## Short version for future sessions

If you only remember one thing, remember this:

`veins_qos` is a deliberately minimal master's-project testbed for crash-aware vehicular QoS. The active experiment is about whether DSCP-marked crash traffic can gain meaningful EDCA prioritization over ordinary BE traffic in Veins/INET, first in a simple scenario and then in a more realistic highway scenario, without adding unnecessary framework complexity.
