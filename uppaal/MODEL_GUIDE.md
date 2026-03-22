# How To Read `v2x_edca.xml`

This guide is for quickly understanding the model without reverse-engineering every transition.

## Reading Order

1. Open `v2x_edca.xml` and read the global `declaration` block first.
2. Read templates in this order:
   - `Medium`
   - `CrashWorkload`
   - `Controller`
   - `GrantObserver`
   - `Receiver`
   - `BeTraffic`
3. Open `v2x_edca.q` and map each query to the relevant template/state.

## Global Declarations

The global block defines:
- controller parameters: `blockDuration`, `maxContinuousBlock`, `sendingGuardTimeout`, `voQueueThreshold`
- workload parameters: `burstCount`, `sendInterval`, `repeatCount`, `repeatGapMin`, `repeatGapMax`
- environment parameters: `mediumFreeDuration`, `mediumBusyDuration`
- dedup abstraction: `voDedupWindow`
- channels/events used to synchronize automata:
  - `vo_demand`, `be_request`, `grant_be`, `grant_vo`, `vo_tx_done`
- observables/counters:
  - `voQueueDepth`, `beGrantCount`, `voGrantCount`, `logicalVoRx`, `duplicateVoRx`, `burstDone`

## Template Roles

### `Medium`

Alternates between `FREE` and `BUSY` with bounded dwell times.
It is the external contention environment abstraction.

### `CrashWorkload`

Generates `vo_demand!` bursts with intra-burst repeats.
It represents crash-triggered critical traffic generation.

### `Controller`

This is the core MAC policy abstraction:
- states: `LISTENING`, `BLOCKING`, `SENDING`
- BE can be granted only from `LISTENING` when VO pressure is below threshold.
- VO demand moves behavior toward `BLOCKING` and `SENDING`.
- `cycle` clock enforces the continuous blocking cap.

### `GrantObserver`

Simple observer automaton to expose discrete grant events as locations:
- `BE_GRANTED`
- `VO_GRANTED`

This makes safety queries easier to write/read.

### `Receiver`

Abstracts receiver-side logical counting under dedup:
- first `vo_tx_done?` increments logical reception
- repeated `vo_tx_done?` inside dedup window increments duplicate counter

### `BeTraffic`

Periodically emits `be_request!` requests to keep BE pressure present.

## Query Mapping (`v2x_edca.q`)

- Q1 checks BE suppression safety while controller is in `BLOCKING` or `SENDING`.
- Q2 checks boundedness of the blocking/sending cycle via `maxContinuousBlock`.
- Q3 checks VO progress when demand+medium conditions are favorable.
- Q4 checks controller recovery from `SENDING` to `LISTENING` once VO queue empties.
- Q5 checks eventual idle-ready posture after burst generation is complete.

## Model Scope

This is an offline verification model.
It is intentionally abstract and does not replace INET runtime behavior.
Use OMNeT++ runs for end-to-end performance KPIs; use this model for timing-logic sanity and parameter reasoning.
