# README Standard — `master_veins`

All project-owned README files should follow this structure and terminology for publication-grade reproducibility.

## Required Sections (in order)

1. **Title and status** — Active / Legacy / Supporting; one-sentence purpose.
2. **Research question** — What this scenario or tool answers.
3. **Prerequisites** — OMNeT++, Veins launchd, Rust/cargo, optional `rsvg-convert`.
4. **Quick start** — Copy-paste commands using **relative paths** from repo root.
5. **Experiment matrix** — Table of configs, loads, and runner profiles (`quick` / `core` / `full`).
6. **Simulation parameters** — Sim time, crash window, vehicle count, load profiles (must match `omnetpp.ini`).
7. **Packet and QoS flow** — BE vs VO, DSCP 46, classifier, MAC policy behavior.
8. **Metrics and outputs** — Scalars/vectors, cache paths, figure names.
9. **Reproducibility** — `RUNS`, `--repeat`, seed policy, results hygiene.
10. **Troubleshooting** — TraCI, missing P95, mixed results.
11. **See also** — Links to parent README, `AI_CONTEXT.md`, audit docs.
12. **Legacy / deprecated** — If applicable: do not use for current paper runs.

## Canonical Terminology

| Use this | Not these | Meaning |
|----------|-----------|---------|
| **plain** / **DCF** | `plain_netload_*` in speech | Non-QoS baseline config |
| **edca_only** / **EDCA** | default QoS | Standard 802.11e without V2X HCF |
| **stable** | `edca_v2x_vo_stable` | Mild adaptive VO protection |
| **guarded** | `edca_v2x_vo_guarded` | Shorter, stricter blocking windows |
| **emergency** | `edca_v2x_vo_emergency` | BE drop/suppression during VO protection |
| **network load** / **workload** | channel load (informal) | `_netload_low\|medium\|high` suffix |
| **density** | vehicle count | `highway_light` (10) vs `highway_heavy` (100) |
| **logical VO TX** | crash sequences | `voLogicalTxPackets` — one per burst |
| **physical VO TX** | VO repeats | `voTxPackets` — includes `repeatCount` |
| **BE** | best-effort traffic | DSCP 0 ordinary traffic |
| **VO** / **crash VO** | voice, accident | DSCP 46 crash-marked traffic |

## Legacy Terms (deprecated for paper)

| Legacy config | Replacement | Notes |
|---------------|-------------|-------|
| `edca_v2x_be_friendly` | `edca_v2x_vo_stable` | Old square/highway only |
| `edca_v2x_vo_protect` | `edca_v2x_vo_guarded` or `emergency` | Stronger blocking, no emergency drop |
| `edca_v2x` | `edca_v2x_vo_stable` | Umbrella name retired |

## Diagram Suggestions

Include or link these figures in thesis README / paper supplement:

1. **Packet path** — Apps → DSCP → Classifier → EDCA → multicast receivers.
2. **Crash timeline** — 70s sim: BE always; VO window 30s–60s on target node.
3. **MAC policy comparison** — DCF vs EDCA vs stable/guarded/emergency (BE suppression semantics).
4. **Reproduction pipeline** — SUMO → OMNeT++ → `results/` → `kpi_dashboard` → `publication_figures/`.

## Path Style

- Prefer: `cd master_veins/kpi_dashboard && cargo run --release`
- Avoid hard-coded `/home/...` paths in new text (legacy examples may remain in comments only if unavoidable).

## Review Checklist (before merge)

- [ ] Load table matches `omnetpp.ini` exactly
- [ ] All listed configs exist in `omnetpp.ini`
- [ ] `run` / `run_matrix.sh` commands tested or marked untested
- [ ] Legacy banner present where configs are not paper-active
- [ ] Metrics mention node[0] vector scope if discussing P95/jitter
