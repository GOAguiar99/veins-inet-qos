# UPPAAL Workflow for `feature/mac_changes`

This directory contains the offline formal-analysis assets for the adaptive V2X EDCA work in `veins_qos`.

Scope:
- keep OMNeT++/INET runtime behavior unchanged
- model-check the current `V2xHcf` + `V2xEdcaFsmController` logic offline
- use the results to validate and tune the existing heuristic parameters

Files:
- `v2x_edca.xml`: timed-automata model of the branch's controller and a small environment abstraction
- `v2x_edca.q`: symbolic queries for the safety and liveness properties we care about
- `parameter_matrix.csv`: checked-in experiment matrix tying OMNeT++ configs to the formal model parameters
- `run_verifyta.sh`: helper script that invokes `verifyta`

What the model captures:
- controller states `LISTENING`, `BLOCKING`, and `SENDING`
- local VO demand as the trigger for BE suppression
- medium availability as an environment process
- crash-burst repetition with `repeatCount`, `repeatGap`, and `repeatJitter`
- receiver-side logical delivery under a `voDedupWindow` abstraction

What the model does not try to do:
- replace the INET MAC event loop at runtime
- model packet headers, PHY details, or every INET queueing detail
- synthesize a controller for direct execution in OMNeT++ in this first iteration

Usage:
```bash
cd /home/goaguiar/master_veins
./uppaal/run_verifyta.sh
```

Notes:
- `verifyta` is not bundled in this workspace
- by default the formal model matches the `edca_v2x` profile from `veins_qos/simulations/veins_inet/omnetpp.ini`
- the `plain` and `edca_only` rows are included in the parameter matrix as runtime baselines for comparison, not as separate controller models
