# V2X MAC Notes

**Status:** Active — custom HCF and MAC instrumentation for the highway QoS study.

## Components

| Module | Role |
|--------|------|
| `V2xHcf` | Adaptive HCF: VO-driven BE suppression; optional emergency BE drop |
| `V2xEdcaFsmController` | FSM: `LISTENING`, `BLOCKING`, `SENDING` |
| `V2xIeee80211Mac` | Wraps `Ieee80211Mac`; per-AC drop scalars |

## FSM States

| State | Value | BE behavior |
|-------|-------|-------------|
| `LISTENING` | 0 | Normal |
| `BLOCKING` | 1 | BE channel requests suppressed |
| `SENDING` | 2 | VO TX in progress; BE remains blocked |

## Trigger Logic

Protection extends on:

1. **Local VO demand** — upper-layer VO enqueued (`onVoDemandDetected`)
2. **Overheard VO** — received VO-classified data frames (multicast-aware; not limited to unicast-to-self)
3. **Predicted VO bursts** — `predictiveBlocking` pre-blocks BE just before the next expected burst per source cadence (useful when bursts are sparse; corrupted/undecoded first copies can't trigger reactive protection)

## Mode Comparison (active configs)

| Mode | `emergencyPreemption` | BE while blocked | Typical use |
|------|----------------------|------------------|-------------|
| stable | false | Suppress + retry | Mild protection |
| guarded | false | Shorter block windows | Stricter timing |
| emergency | true | Drop + suppress grants | Maximum VO protection |

## Tuning Knobs (`V2xHcf`)

- `blockDuration` — extension per VO demand event
- `maxContinuousBlock` — cap on one continuous alert period
- `sendingGuardTimeout` — grace after VO TX transitions (FSM submodule)
- `voQueueThreshold` — local VO queue depth to trigger alert
- `emergencyPreemption` — drop new BE and suppress stale BE grants while blocking
- `predictiveBlocking` — learn burst cadence per overheard VO source and block BE `predictiveLead` before each predicted burst (window `predictiveWindow`)
- `predictiveMinGap` / `predictiveMinPeriod` / `predictiveMaxPeriod` — burst-start detection and accepted period range; prediction only engages when bursts are separated by silent gaps (overlapping burst trains keep reactive blocking active anyway)

Predictive activations are counted as `voPredictiveBlockCount` (in addition to `voProtectionActivationCount`, which they also increment).

## MAC Drop Scalars (`V2xIeee80211Mac`)

Per-AC totals: `packetDropAc{Bk,Be,Vi,Vo,Unclassified}Count`  
Per-AC per-reason: e.g. `packetDropAcVoReasonRetryLimitReachedCount`

## HCF Instrumentation (`V2xHcf`)

- `beDroppedWhileBlockedCount`
- `beGrantSuppressedWhileBlockedCount`
- `voProtectionActivationCount`

Exported to KPI dashboard as fig_07 when non-zero.

## See Also

- [`../../simulations/veins_inet_highway_light/README`](../../simulations/veins_inet_highway_light/README)
- [`../../../docs/DEPRECATED_MODES.md`](../../../docs/DEPRECATED_MODES.md) — legacy `be_friendly` / `vo_protect`
