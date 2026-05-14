# Minimal VO Guarantee Plan

## Summary
Keep the project as a BE-vs-VO crash-priority study. Do not redesign the MAC, mobility, propagation, or application model. The claim should be: **under stated simulation conditions, crash VO packets meet an empirical deadline/reach guarantee while BE is allowed to degrade**.

Do not claim hard deterministic delivery. With EDCA, contention, and multicast, that is not scientifically defensible.

## Minimal Implementation Changes
- Keep the existing DSCP 46 -> VO path and `V2xHcf` adaptive BE suppression.
- Add one stronger config: `edca_v2x_vo_emergency`.
  - Extend `edca_only`.
  - Use `veins_qos.mac.V2xHcf`.
  - Set `voQueueThreshold = 1`.
  - Set `blockDuration = 10ms`.
  - Set `maxContinuousBlock = 60ms`.
  - Set `sendingGuardTimeout = 5ms`.
  - Keep the same EDCA CW/AIFS/queue settings as `edca_only`.
- Add only minimal observability:
  - Logical VO TX count: one per crash sequence, not one per repeat.
  - Logical VO RX sequence vector/count after deduplication.
  - First successful VO RX delay per logical sequence.
  - `beGrantWhileBlockedCount` in `V2xHcf` to prove BE suppression is actually active.

## Experiment Plan
- Run only high-load configs first:
  - `plain_netload_high`
  - `edca_only_netload_high`
  - `edca_v2x_vo_guarded_netload_high`
  - `edca_v2x_vo_emergency_netload_high`
- Run both active scenarios:
  - highway light
  - highway heavy
- Use at least `RUNS=0..9` for thesis-level evidence.
- Analyze only the crash window for the main claim: `30s` to `60s`.
- Keep BE degradation as an explicit cost metric, not a failure.

## Acceptance Criteria
The selected final config must satisfy all of these in the crash window:

- `zeroReachVoSequences = 0`: every logical crash packet reaches at least one non-self receiver.
- `VO first-success P99 <= 50ms`.
- `VO first-success deadline misses = 0` for a `50ms` deadline.
- VO reach and VO P95/P99 delay are better than `plain_netload_high` and `edca_only_netload_high`.
- BE may have worse delay, jitter, reach, and drops, but the thesis reports this as the cost of emergency priority.

If `edca_v2x_vo_emergency` does not improve VO over `guarded`, use `guarded` as the final policy and state that more aggressive BE suppression only increases BE damage without further VO benefit.

## Assumptions
- “Guarantee” means empirical guarantee under the tested simulator assumptions, not absolute wireless delivery.
- The claim is about crash-message propagation priority, not fairness.
- The contribution is the bounded crash-triggered BE suppression policy, not DSCP marking or standard EDCA tuning alone.
