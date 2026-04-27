# Crash-Aware Vehicular QoS Workspace

This repository is a master's-project workspace for evaluating crash-aware QoS behavior in vehicular Wi-Fi simulations.

The central research question is:

Can crash-critical traffic obtain better service than ordinary traffic under contention, and what is the performance cost imposed on ordinary traffic?

The active project is [`veins_qos/`](veins_qos/).

## Project Scope

- Ordinary periodic traffic is modeled as Best Effort (BE).
- Crash traffic is modeled as Voice (VO) using DSCP `46`.
- A custom classifier maps DSCP to Wi-Fi user priority in EDCA runs.
- Experiments compare non-QoS DCF baseline behavior with EDCA and adaptive V2X variants.
- Evaluation focuses on delay, reception/reach, and BE-vs-VO trade-off.

This workspace intentionally keeps a minimal two-class design (BE vs VO) to preserve thesis clarity and reproducibility.

## Repository Boundaries

- [`veins_qos/`](veins_qos/): active codebase (primary target for development and experiments).
- [`kpi_dashboard/`](kpi_dashboard/): KPI analysis dashboard for OMNeT++ outputs.
- [`uppaal/`](uppaal/): offline formal verification models and queries.
- `inet/`, `veins/`, `omnetpp-6.1/`: framework/dependency trees; treat as read-only unless framework-level changes are explicitly required.
- [`ap_servers/`](ap_servers/): legacy material, not the active study baseline.

## Active Simulation Packages

Simulation packages live under `veins_qos/simulations/`:

- Active density-study packages:
  - [`veins_qos/simulations/veins_inet_highway_light/`](veins_qos/simulations/veins_inet_highway_light/)
  - [`veins_qos/simulations/veins_inet_highway_heavy/`](veins_qos/simulations/veins_inet_highway_heavy/)
- Legacy or supporting packages still present in the tree:
  - [`veins_qos/simulations/veins_inet_square/`](veins_qos/simulations/veins_inet_square/)
  - [`veins_qos/simulations/veins_inet_highway/`](veins_qos/simulations/veins_inet_highway/)
  - [`veins_qos/simulations/veins_inet_light/`](veins_qos/simulations/veins_inet_light/)

Current active highway-density study structure:

- scenario package selects vehicle density:
  - `veins_inet_highway_light`
  - `veins_inet_highway_heavy`
- config name selects MAC policy and load profile:
  - `plain_netload_<low|medium|high>`
  - `edca_only_netload_<low|medium|high>`
  - `edca_v2x_vo_stable_netload_<low|medium|high>`
  - `edca_v2x_vo_guarded_netload_<low|medium|high>`

## Core Implementation Areas (`veins_qos/src`)

- `traffic/`: application-layer traffic generators for BE and crash-triggered VO flows.
- `qos/`: DSCP-to-user-priority classifier used by EDCA-enabled runs.
- `mac/`: adaptive EDCA/V2X MAC logic and FSM behavior.
- `veins_inet/`: integration layer between Veins mobility and INET networking stack.

## KPI Focus

Primary analysis dimensions:

- BE/VO transmitted and received packet counts
- BE/VO end-to-end delay (mean and distribution-sensitive views)
- multicast reach (`RX per TX`)
- MAC-level drops (including overflow/retry-limit categories)
- protection-vs-cost trade-off (VO improvement vs BE degradation)

## Documentation Index

- Project context and guardrails: [`veins_qos/AI_CONTEXT.md`](veins_qos/AI_CONTEXT.md)
- KPI dashboard details: [`kpi_dashboard/README.md`](kpi_dashboard/README.md)
- Formal model workflow: [`uppaal/README.md`](uppaal/README.md)
- Highway light notes: [`veins_qos/simulations/veins_inet_highway_light/README`](veins_qos/simulations/veins_inet_highway_light/README)
- Highway heavy notes: [`veins_qos/simulations/veins_inet_highway_heavy/README`](veins_qos/simulations/veins_inet_highway_heavy/README)
- Legacy square scenario notes: [`veins_qos/simulations/veins_inet_square/README`](veins_qos/simulations/veins_inet_square/README)
- Legacy highway scenario notes: [`veins_qos/simulations/veins_inet_highway/README`](veins_qos/simulations/veins_inet_highway/README)
- Legacy light scenario notes: [`veins_qos/simulations/veins_inet_light/README`](veins_qos/simulations/veins_inet_light/README)
- Repo-level agent instructions: [`AGENTS.md`](AGENTS.md)

## Dependency/Framework Documentation

Framework-specific documentation is maintained in their own trees:

- INET: [`inet/README.md`](inet/README.md), [`inet/INSTALL.md`](inet/INSTALL.md)
- Veins: [`veins/README.txt`](veins/README.txt)
- OMNeT++: [`omnetpp-6.1/README`](omnetpp-6.1/README), [`omnetpp-6.1/INSTALL.md`](omnetpp-6.1/INSTALL.md)
