# V2X MAC Notes

This folder contains the custom MAC pieces used by the `edca_v2x` experiments.

## Components

- `V2xHcf`: custom HCF wrapper that decides when to suppress BE access requests.
- `V2xEdcaFsmController`: small FSM that tracks V2X alert state and exposes whether BE must stay blocked.
- `V2xIeee80211Mac`: instrumentation wrapper over `Ieee80211Mac` that records per-AC drop counters
  (BK/BE/VI/VO/Unclassified) and per-AC per-reason drop scalars.

### MAC drop observability scalars

`V2xIeee80211Mac` records:

- Per AC totals:
  - `packetDropAcBkCount`
  - `packetDropAcBeCount`
  - `packetDropAcViCount`
  - `packetDropAcVoCount`
  - `packetDropAcUnclassifiedCount`
- Per AC and reason:
  - `packetDropAc<Ac>Reason<Reason>Count`
  - Example: `packetDropAcVoReasonRetryLimitReachedCount`

## FSM states

- `LISTENING` (`0`): normal operation.
- `BLOCKING` (`1`): BE should not request channel access.
- `SENDING` (`2`): VO transmission is in progress; BE remains blocked.

BE is considered blocked while FSM is `BLOCKING` or `SENDING`.

## Trigger logic (current behavior)

The alert/block window is extended by VO activity from both directions:

1. Local VO demand:
- when upper-layer VO traffic is enqueued, `V2xHcf` calls `onVoDemandDetected(...)`.

2. Received VO demand:
- when this node receives a VO data frame addressed to itself, `V2xHcf::processLowerFrame(...)`
  also calls `onVoDemandDetected(...)`.

This keeps nodes in alert mode while crash-related VO traffic is still active around them.

## BE suppression policy

- While FSM is blocking/sending, BE channel requests are suppressed.
- If BE queue has packets, retries are deferred to `blockingUntil`.
- Once alert ends, deferred BE requests are retried.

## Main tuning knobs

- `blockDuration`: base extension applied on each VO demand event.
- `maxContinuousBlock`: hard cap for one continuous alert period (`<=0` disables cap).
- `sendingGuardTimeout`: short grace period after VO transmit start/end transitions.
- `voQueueThreshold`: local VO queue threshold to trigger alert from upper traffic.

Use these knobs to balance VO protection and BE delay impact.
