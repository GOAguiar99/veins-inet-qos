# Confirmed Hypothesis: Why VO Gains Little from Emergency BE Protection

**Status:** Confirmed by simulation evidence (2026-08-16, post Poisson/clock measurement fix)
**Scope:** highway scenario, 802.11p 6 Mb/s, DSCP-46 crash VO (1 or 4 sources) vs Poisson BE

## Hypothesis

> Emergency-style protection (suppress/drop neighboring BE around crash VO bursts)
> cannot measurably improve VO end-to-end delay or delivery in the current
> scenario, because VO's dominant impairment is **receiver-side collision loss
> inflicted by ~100 equiprobable low-duty-cycle senders**, not contention with
> any identifiable hot BE neighbors; per-copy VO airtime is small relative to
> burst self-queueing; and protection removes only a node's own BE for tens of
> milliseconds at a time, after a *decodable* copy — so the marginal reduction
> of aggregate collision probability is below measurement noise.

Protection is real (it fires, caps blocks, drops BE in emergency), but it
differentiates policies only on **BE cost**, not on VO benefit.

## Causal chain

1. Each vehicle contributes ~1/N of the near-crash collision opportunities.
   Blocking one node's BE for ~15–80 ms removes ~1% of ambient airtime briefly.
2. VO "drops" at the MAC are ~100% RX-side `incorrectlyReceived` (FCS corruption
   from mid-air collision): retry-limit = 0 and queue-overflow = 0 in *every*
   config and load (light high/stress; heavy low/medium/high; heavy stress).
   → VO packets are lost before any queueing or retry policy can act.
3. Reactive protection triggers only on **decoded** VO copies
   (`V2xHcf::processLowerFrame` → `isReceivedVoDataForUs`). The corrupted first
   copy — the event that matters — triggers nothing.
4. Alert-age KPI (burst creation → first received copy) is floored by burst
   **self-queueing** at the single VO source: even with an empty channel, copy
   k leaves after k×(airtime+AIFS+backoff). Removing all BE cannot eliminate
   this floor.
5. Delivery is range-limited and policy-invariant: ~9.4–9.95 received copies
   per logical alert (highway spread ⇒ ~10 neighbors in range), identical
   across DCF/EDCA/stable/guarded/emergency at every tested load.

## Evidence (all from dashboard-fed matrices, 3 seeds each)

| Experiment | Setup | VO mean / P95 by policy | Reading |
|---|---|---|---|
| Light high/stress | 10 veh | 0.78–1.03 ms / 2.08–2.19 ms, all policies tie | no VO headroom at small N |
| Heavy low→high | 100 veh | 0.99–1.69 ms / 2.2–4.9 ms, ties | flat across saturation onset |
| Heavy stress (BE exp(25ms), ~13 Mb/s offered) | 100 veh | 2.55–2.83 ms / 15.6–15.9 ms, ties; delivery 9.3–9.4 | flat even past collapse |
| Multi-crasher high (4 VO sources) | 100 veh | 1.40–1.44 ms / 5.7–5.9 ms, delivery 8.56 everywhere | 4× alert weight, still flat |
| Predictive pre-blocking (`_pb`) | burstgap profile | stable/emergency −1–4% (noise), guarded +11%/+3.7% (regression) | pre-clearing gaps does not help → blocking-window timing is not the bottleneck |
| Loss attribution | all configs | VO retry-limit = 0, queue-overflow = 0; VO incorrectRx = 3.6–3.8k (heavy high) | VO dies on the air interface, not in queues |

### Counterfactual confirmation (why old data *did* show gains)

The archived pre-fix figure (`fig_01_p95_delay_priority_gap`, 2026-08-15 18:58)
showed emergency cutting VO P95 ~in half vs DCF (1.34 → 0.70 ms). That effect
was produced by the frozen-`sendInterval` bug: each node drew its
`exponential(mean)` once, creating heterogeneous hot neighbors (BE TX swung
6× across seeds; BE P95 reached 60–182 ms). Protection = anti-hot-neighbor
mechanism; hot neighbors existed → protection "worked". With correct Poisson
arrivals (BE TX 4784/4715/4713 across seeds; BE P95 0.4–0.5 ms), contention is
uniform and there is no target to suppress. The pre-fix raw results no longer
exist on disk; numbers above were recovered from the exported SVG bar heights.
All thesis figures were re-exported from post-fix data at 21:01–21:02.

## Ruled-out alternatives

- **"Blocking windows are mistimed"** — predictive pre-blocking was implemented
  and tested (`predictiveBlocking`, commit 9a92dc1, reverted in 6189876): no VO
  gain, one regression. Window timing is not the issue.
- **"Load too low to differentiate"** — tested up to ~2× channel capacity
  (heavy stress): flat.
- **"VO load too small to matter"** — 4 concurrent crash sources: flat.
- **"Delivery loss dominates but is hidden by the delay metric"** — delivery
  (`vo_rx_per_logical_tx`) is also policy-invariant and range-limited.
- **"Sample size"** — n=3 seeds limits |Δ| detection to ~±3–5%, but the
  policy-invariance is structural (identical to 3 decimal places on delivery),
  not marginal.

## Falsifiable predictions (how to verify further, if needed)

1. Hot-spot scenario: 2–3 high-rate CBR nodes near the crash node → emergency
   VO gains should *reappear* (target exists again).
2. Capture effect on the radio: VO absolute delays fall for *all* policies
   alike; policy ranking unchanged.
3. Bigger VO frames (e.g., 1 kB): VO delays grow uniformly per policy; no new
   policy separation.
4. If a future MAC coordinates *network-wide* (not node-local) silences during
   bursts, VO P95 should drop where collisions were dominant — the first
   mechanism that would actually attack the bound above.

## Implication for the dissertation

The honest claim is not "emergency improves VO delay" but:

- Under uniform random access, VO over 802.11p is **collision-limited**, and
  queue/queue-suppression policies cannot help it;
- stable / guarded / emergency form a **BE-cost graduation** (BE-from-crash P95
  at heavy high: 13.8 / 3.9 / 0.44 ms vs plain 0.63; emergency additionally
  drops BE while protected) while preserving VO service;
- To demonstrate positive VO gains, the evaluation needs heterogeneous load
  (hot spots, hidden nodes) or coordinated silencing — a designed condition,
  not an RNG artifact.

## Data locations (this machine: 192.168.0.42)

- Post-fix raw results: `veins_inet_highway_{light,heavy}/results/`
- Heavy stress: `veins_inet_highway_heavy/results_stress_25ms/`
- Multi-crasher A/B (kept as negative-result evidence): `results/*multivo*`
- Pre-fix exports (only surviving pre-fix numbers): `Masters/Figs/*.svg` (18:58)
- Dashboards: `http://localhost:8050` (light+heavy), `http://localhost:8051` (stress)
