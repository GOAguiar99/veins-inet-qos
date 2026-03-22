# How To Read `v2x_edca.xml`

This guide is for quickly understanding the model without reverse-engineering every transition.

## Model Variants

- `v2x_edca.xml`: main model aligned with runtime semantics and timing defaults.
- `v2x_edca_fast.xml`: tiny finite-state variant for faster sanity checks.
  - Saturates VO queue depth to `0/1/2+`
  - Bounded request/demand budgets
  - Preserves core LISTENING/BLOCKING/SENDING suppression logic

## Reading Order

1. Open the model XML and read the global `declaration` block first.
2. Read templates in this order:
   - `Medium` / `Workload` (depends on model variant)
   - `Controller`
   - `VoTransmitter`
   - `GrantObserver`
   - `Receiver` / `BeTraffic` (main model only)
3. Open the `.q` mirror (optional) and map each query to the relevant template/state.

## Global Declarations

The global block defines:
- controller parameters: `blockDuration`, `maxContinuousBlock`, `sendingGuardTimeout`, `voQueueThreshold`
- workload parameters: `crashStart`, `crashDuration`, `sendInterval`, `repeatCount`, `repeatGapMin`, `repeatGapMax`
- environment parameters: `mediumFreeDuration`, `mediumBusyDuration`
- channels/events used to synchronize automata:
  - `vo_demand`, `be_request`, `grant_be`, `grant_vo`, `vo_tx_done_pending`, `vo_tx_done_clear`
- observables/counters:
  - `voQueueDepth`, `beGrantCount`, `voGrantCount`, `burstDone`

## Template Roles

### `Medium`

Alternates between `FREE` and `BUSY` with bounded dwell times.
It is the external contention environment abstraction.

### `CrashWorkload`

Generates `vo_demand!` bursts after crash time with repeats and an explicit crash activity window.

### `Workload` (fast model)

Emits a bounded number of VO and BE requests without timed loops.

### `Controller`

This is the core MAC policy abstraction aligned with `V2xEdcaFsmController` states:
- `LISTENING`
- `BLOCKING`
- `SENDING`

BE grants are emitted only while suppression is inactive.
VO demand moves behavior into suppression (`BLOCKING`) and medium grants move into `SENDING`.

### `VoTransmitter`

Represents VO transmission completion timing after `grant_vo` and emits:
- `vo_tx_done_pending` when more VO remains
- `vo_tx_done_clear` when VO queue is drained

### `GrantObserver`

Simple observer automaton to expose discrete grant events as locations:
- `BE_GRANTED`
- `VO_GRANTED`

This keeps safety queries compact and readable.

### `Receiver` (main model)

Tracks deduplicated logical VO receptions for analysis support.
The fast model omits this template to reduce interleavings.

### `BeTraffic`

Periodically emits `be_request!` requests to keep BE pressure present.

## Query Mapping

- Q1 checks BE suppression safety while controller is in `BLOCKING` or `SENDING`.
- Q2 checks boundedness of blocking/sending cycle.
- Q3 checks VO progress when demand+medium conditions are favorable.
- Q4 checks controller recovery from `SENDING` to `LISTENING` once VO queue empties.
- Q5 checks eventual return to idle posture after crash workload completes.

## Model Scope

This is an offline verification model.
It is intentionally abstract and does not replace INET runtime behavior.
Use OMNeT++ runs for end-to-end performance KPIs; use this model for timing-logic sanity and parameter reasoning.
