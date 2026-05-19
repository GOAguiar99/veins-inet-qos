# Publication Checklist — `master_veins`

Use before submitting the ETFA paper or archiving reproduction artifacts.

## Simulation

- [ ] `veins_launchd` running; TraCI port 9999 reachable
- [ ] `veins_qos` built (`make` in `veins_qos/`)
- [ ] Active results from `veins_inet_highway_light` and/or `veins_inet_highway_heavy` only
- [ ] `PROFILE=full` (or documented subset) with consistent `RUNS` and `--repeat`
- [ ] Old incompatible `results/` archived or removed
- [ ] Config names match paper tables (`plain_netload_high`, etc.)

## KPI Dashboard

- [ ] `cargo test` passes in `kpi_dashboard/`
- [ ] `PARSER_VERSION` documented if bumped (`rust-kpi-dashboard-0.2.0`)
- [ ] `cargo run --release -- --rebuild` against correct `--results` paths
- [ ] Baseline config noted (`plain_netload_high` default)

## Figures

- [ ] `cargo run --release --bin export_figures` with both result directories
- [ ] All expected `fig_01` … `fig_07` SVGs for each density (or omissions explained)
- [ ] LaTeX `\includegraphics` stems match exported filenames
- [ ] Axis labels and legends verified visually
- [ ] Paper text uses **logical vs physical VO TX** consistently

## Documentation

- [ ] [`README.md`](../README.md) matches active matrix
- [ ] [`veins_qos/AI_CONTEXT.md`](../veins_qos/AI_CONTEXT.md) matches `omnetpp.ini`
- [ ] Legacy scenarios marked deprecated
- [ ] [`AUDIT_REPORT.md`](../AUDIT_REPORT.md) reviewed for open Phase B items

## Code Hygiene

- [ ] No hardcoded debug log paths in `veins_qos/src`
- [ ] Git commit hash recorded in supplementary material (optional)
- [ ] Submodule pins (`inet`, `veins`) recorded

## Claims Sanity

- [ ] VO gains reported with BE cost (especially emergency)
- [ ] Multicast framed as reception reach, not guaranteed delivery
- [ ] P95/jitter described as `node[0]` observatory unless extended
- [ ] Adaptive modes described as protection mechanisms, not only EDCA tuning
