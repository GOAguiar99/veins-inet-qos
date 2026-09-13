# Scientific Dashboard and Figure Strategy

This dashboard is intended to support paper-quality analysis of the Veins QoS experiments, not only run monitoring. The central comparison is ordinary BE traffic versus crash-triggered VO traffic under DCF, default EDCA, and tuned V2X EDCA variants.

## Interactive UI (implemented)

The Rust server at `http://127.0.0.1:8050` currently exposes **four auditable tables** (comparison, config summary, run details, V2X matrix) plus cache/rebuild status. Sections 1–6 below describe the **analytical lens** for interpretation and map to exported figures — they are not separate UI tabs.

## Analytical Lenses

1. **Research Summary**
   - High-level KPIs for the selected result set: run count, strategy coverage, workload coverage, and cache provenance.
   - Primary questions: whether VO gets lower delay, whether BE pays a measurable cost, and whether losses grow with contention.

2. **Priority Effectiveness**
   - BE versus VO delay, jitter, and reception under the same strategy/load.
   - Main lens: high-load tail behavior, because safety-critical traffic is sensitive to worst-case delay.

3. **Contention and Scalability**
   - Strategy x workload matrices for normalized packet drops, VO reception, and P95 delay.
   - Main lens: whether conclusions survive the low -> medium -> high network-load sweep and light/heavy density split.

4. **Tradeoff Analysis**
   - Joint delay/jitter views and BE cost versus VO benefit.
   - Main lens: tuned prioritization is useful only if VO gains are not achieved by uncontrolled instability or extreme BE starvation.

5. **Loss Attribution**
   - BE, VO, unclassified, queue-overflow, retry-limit, and congestion drop views when scalars are present.
   - Main lens: separating contention failures from application-level delivery ratios.

6. **Adaptive-Control Diagnostics**
   - V2X HCF counters: VO protection activations, BE grants suppressed while blocked, and BE dropped while blocked.
   - Main lens: explaining why a tuned mode improves or harms the observed network behavior.

7. **Publication Figures**
   - Deterministic SVG-first export pipeline for figures used directly in the paper.
   - Optional PNG/PDF conversion for conference templates and review systems.

## Scientifically Meaningful Comparisons

- **DCF vs EDCA:** isolates whether the 802.11e access category path improves crash traffic over the non-QoS baseline.
- **EDCA vs Stable/Guarded:** tests whether explicit VO protection improves beyond default EDCA.
- **VO vs BE within each strategy:** tests priority differentiation directly under identical channel conditions.
- **Low/medium/high network load:** exposes contention sensitivity and scalability.
- **Light/heavy highway density:** separates traffic generation load from vehicular density and propagation/mobility stress.
- **VO gain vs BE penalty:** prevents interpreting priority improvements without measuring the cost to ordinary traffic.
- **Delay percentiles vs mean delay:** checks safety-relevant tail latency rather than relying on averages.
- **Drops per TX rather than only raw drops:** normalizes across runs with different packet counts and repeat behavior.

## Recommended Figures

Default canvas is **720×480** (publication preset). The exporter prefers a valid `.kpi_cache_rs` when present, otherwise rebuilds from `.sca`/`.vec`. Heatmaps **02/03 are audit-only** (omitted from the default thesis export; pass `--figures 02 03` explicitly). Cross-regime figures **04/08/09/10** are written once with density stem `highway_heavy` (use `--summary-json` for hotspot archives without local `.sca`).

| Figure | Research question | Chart | Primary metrics | Notes |
| --- | --- | --- | --- | --- |
| `fig_01_p95_delay_priority_gap_<density>` | Does VO keep a low P95 while BE absorbs prioritization cost? | Dual-panel bars (VO P95 / BE P95) at netload high | VO/BE P95 delay | Per density |
| `fig_02_mac_drop_rate_by_strategy_load_<density>` | How quickly does contention translate into normalized packet loss? | Strategy × workload heatmap | MAC drops / app TX | Audit-only |
| `fig_03_vo_reception_by_strategy_load_<density>` | Which MAC strategy preserves crash-message reception as load grows? | Strategy × workload heatmap | VO RX / logical TX | Audit-only |
| `fig_04_vo_gain_vs_be_cost_highway_heavy` | Relative to DCF, how much VO P95 improves vs BE P95 cost? | Dual-panel scatter (ΔVO vs ΔBE), independent axes per regime; markers by policy | Δ VO/BE P95 vs plain | Cross-regime |
| `fig_05_mac_drop_attribution_high_load_<density>` | Are losses concentrated in BE, VO, or other? | Stacked bars; legend BE/VO/Other | BE/VO/unclassified MAC drops | Netload high |
| `fig_06_vo_delay_cdf_high_load_<density>` | Does prioritization improve the full VO delay distribution? | Empirical CDF | VO delay vector samples | Skipped on light under defaults |
| `fig_07_v2x_control_actions_by_load_<density>` | How do VO protection and BE blocking scale with load? | Triple panel × load sweep | VO protection, BE grants suppressed, BE dropped while blocked | Per density |
| `fig_08_hotspot_vo_delay_by_policy_highway_heavy` | Under hotspot high, how do VO mean and P95 compare by policy? | Grouped bars | VO mean, VO P95 | Hotspot |
| `fig_09_hotspot_vo_p95_by_load_highway_heavy` | How does Emergency VO P95 compare with DCF across hotspot load? | Grouped bars by load | VO P95 (plain vs emergency) | Hotspot |
| `fig_10_regime_vo_p95_contrast_highway_heavy` | How does high-load VO P95 differ between netload and hotspot? | Paired bars by policy | VO P95 | Cross-regime |

These figures are intentionally comparative. Raw run tables remain available for auditability, but paper figures should focus on differences across strategies, workloads, densities, and access categories.

## Metric Treatment

- Use **P95** delay as the default tail-latency figure. Add P99 only when enough vector samples exist to make it stable.
- P95 and jitter in the dashboard/exporter are computed from **`Scenario.node[0].app[0]`** vector streams; scalar delay means aggregate all nodes.
- Use **P50/P95/P99** in text or supplementary tables when discussing delay distributions.
- Use **mean absolute jitter** to summarize short-term delay variability; interpret it alongside delay, not as a standalone quality score.
- Use **RX per TX** for multicast reception because one transmission can produce multiple receptions.
- Use **VO logical TX** for application-level crash delivery ratios and **VO physical TX** for MAC-normalized drop rates.
- Use **drops per TX** for cross-load comparisons; raw drops are still useful for loss attribution.
- Keep BE and VO axes consistent within a figure so priority gaps are visually honest.

## Export Pipeline

Generate figures from raw collected simulation data:

```bash
cd /home/goaguiar/master/master_veins/kpi_dashboard
cargo run --release --bin export_figures -- \
  --results ../veins_qos/simulations/veins_inet_highway_light/results \
  --results ../veins_qos/simulations/veins_inet_highway_heavy/results \
  --summary-json ../../Masters/context/eval-kpis/highway_heavy_hotspot/config_summary.json \
  --summary-json ../../Masters/context/eval-kpis/highway_heavy/config_summary.json \
  --summary-json ../../Masters/context/eval-kpis/highway_light/config_summary.json \
  --output publication_figures \
  --formats svg,png,pdf \
  --dpi 300
```

The exporter writes SVG always. PNG and PDF are written when either `rsvg-convert` or `inkscape` is installed. If neither converter exists, the SVG files remain the canonical publication artifacts. A valid `.kpi_cache_rs` is reused when present; otherwise results are rebuilt from raw files.

## Naming Convention

```text
fig_<two-digit-order>_<question-slug>_<density>.<ext>
```

Examples:

- `fig_01_p95_delay_priority_gap_highway_light.svg`
- `fig_02_mac_drop_rate_by_strategy_load_highway_heavy.png`
- `fig_06_vo_delay_cdf_high_load_highway_heavy.pdf`

Use the same filename stem in LaTeX and change only the extension required by the build pipeline.

## Publication Defaults

- Canvas size: `720 x 480 px` (publication preset; tall variants for multi-panel figures)
- Raster export: `300 dpi`
- Font family: Times New Roman / serif
- Background: white
- Color encoding:
  - BE: blue (`#4c78a8`)
  - VO: red (`#e45756`)
  - neutral/unclassified: gray
- Layout: deterministic ordering by DCF, EDCA, Stable, Guarded, Emergency; workload ordering by low, medium, high.
- Preferred vector artifact: SVG for editing and PDF conversion.
- Preferred raster artifact: PNG at 300 dpi for review systems that reject vector formats.
