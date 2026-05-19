# Deprecated Simulation Modes

**Decision (2026-05-18):** Legacy `friendly` and `protect` configs remain in the repository for historical reference but are **not supported for paper reproduction**.

## Mapping

| Legacy config | Packages | Successor in active study |
|---------------|----------|---------------------------|
| `edca_v2x_be_friendly` | `veins_inet_square`, `veins_inet_highway` | `edca_v2x_vo_stable` |
| `edca_v2x_vo_protect` | same | `edca_v2x_vo_guarded` or `edca_v2x_vo_emergency` |
| `edca_v2x` | same | `edca_v2x_vo_stable` |

## Behavioral differences

Legacy configs extend a single `edca_v2x` base with longer default blocks (40 ms, 250 ms cap) and **no** `emergencyPreemption`. Active highway configs use separately tuned stable/guarded/emergency profiles with a 70 s horizon and `_netload_*` matrix.

## Runnable status

- **Configs:** Present in legacy `omnetpp.ini` files; OMNeT++ can run them after `make` in `veins_qos/`.
- **Run scripts:** Updated to invoke `../../src/veins_qos` (same as active scenarios).
- **KPI / figures:** Not included in `run_matrix.sh`, dashboard defaults, or `export_figures` strategy order.
- **Documentation:** Marked deprecated in scenario READMEs and [`AUDIT_REPORT.md`](../AUDIT_REPORT.md).

## Paper reproduction

Use only:

- `veins_inet_highway_light` / `veins_inet_highway_heavy`
- Configs `*_netload_{low,medium,high}` for plain, edca_only, stable, guarded, emergency
