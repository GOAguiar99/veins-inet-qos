# Crash-Aware EDCA V2X Evaluation And Improvement Plan

## Summary

The current model is a minimal BE-vs-VO crash-priority experiment: `CritPacketSender` emits BE multicast with DSCP 0, `CrashBurstApp` emits crash multicast with DSCP 46, `QosClassifier` maps DSCP 46 to VO, and EDCA or custom `V2xHcf` arbitrates channel access. Key files: [CritPacketSender.cc](/home/aguiar/master/veins-inet-qos/veins_qos/src/traffic/CritPacketSender.cc:82), [CrashBurstApp.cc](/home/aguiar/master/veins-inet-qos/veins_qos/src/traffic/CrashBurstApp.cc:84), [QosClassifier.cc](/home/aguiar/master/veins-inet-qos/veins_qos/src/qos/QosClassifier.cc:65), [V2xHcf.cc](/home/aguiar/master/veins-inet-qos/veins_qos/src/mac/V2xHcf.cc:124), and [omnetpp.ini](/home/aguiar/master/veins-inet-qos/veins_qos/simulations/veins_inet_highway_light/omnetpp.ini:107).

Critical verdict: this is not yet publishable as a strong QoS/scheduling contribution. DSCP-to-VO mapping and EDCA CW/AIFS differentiation are known mechanisms. The only potentially novel element is the adaptive `V2xHcf` behavior that suppresses BE channel requests when VO demand is locally queued or overheard, but that must be reframed and validated as a bounded, event-triggered contention-suppression mechanism. As implemented, it may improve VO partly by shifting congestion into BE queues and reducing BE offered contention, not by creating real reliability under saturation.

No active `.sca/.vec` result folders were present, so empirical claims cannot be audited from raw data. A root note references one-run results, but one seed/config is not statistically defensible.

## Critical Weaknesses

- Runtime MAC correctness is not proven. `V2xHcf` suppresses new BE requests while blocked, but it does not cancel or reject a BE EDCAF that already requested channel access before blocking. Add a runtime counter/assertion for “BE grant while FSM blocked”; otherwise the UPPAAL safety query is stronger than the actual simulator behavior.
- VO reliability metrics are currently ambiguous. `CrashBurstApp` counts every repeated physical VO packet as `voTx`, while `CritPacketSender` deduplicates VO receptions by sequence number. This mixes physical transmissions with logical receptions and can invalidate `VO RX per TX`, especially when `repeatCount` changes.
- Current traffic is multicast to `224.0.0.1`; 802.11 multicast/broadcast safety traffic normally has no per-receiver ACK/retry semantics. Therefore “delivery reliability” must be defined as spatial reach or receiver-opportunity success, not unicast packet delivery ratio.
- BE/VO comparisons are diluted by whole-run aggregation. The crash lasts 30s inside a 100s run, so BE metrics mix pre-crash, crash, and recovery periods. The thesis question requires crash-window KPIs.
- `exponential(...)` BE intervals are sampled once at app startup because `sendInterval` is stored once, making each node periodic with a random fixed period rather than true exponential inter-arrival.
- The heavy highway scenario is not necessarily a high-contention scenario: 100 vehicles over 5 km is spatially diluted. It needs measured channel busy ratio and contenders-in-carrier-sense-range before claiming saturation.
- `IdealObstacleLoss` is binary obstruction/no-obstruction, not realistic vehicle/building attenuation. It is useful as a control, but not sufficient for realism.
- Queue fairness is confounded: DCF has one 128-packet queue; EDCA has separate AC queues, including a 32-packet VO queue and 128-packet BE queue. Some observed gain may be queue isolation rather than contention priority.
- The adaptive MAC can starve BE in bursts. `maxContinuousBlock` caps one window, not long-term duty cycle; repeated VO demand can still create high BE head-of-line delay.

## Research Contribution Framing

Treat the baseline study as known EDCA behavior: INET documents EDCA as four access categories with separate queues, shorter contention for high priority, and lower delay/jitter for VO at BE/BK cost. That alone is not novel.

Potential publishable claim: “A bounded crash-triggered BE contention-suppression policy for 802.11p/OCB-style vehicular multicast that improves first-success latency and reach for emergency traffic while bounding BE starvation.” To make this defensible, prove and measure:
- no BE grants during suppression, or explicitly model residual BE access;
- bounded BE blocking duty cycle and maximum BE HOL delay;
- VO first-success latency improvement under controlled contention;
- cost to BE reach, delay, jitter, and drop probability;
- behavior under multiple simultaneous crash sources and hidden terminals.

Do not claim deterministic networking or TSN guarantees. EDCA plus local suppression is still stochastic contention. TSN/DetNet claims require admission control, allocated resources, timing bounds, and congestion-loss guarantees.

## Implementation And Experiment Plan

1. Instrument before changing policy:
   - Record per-AC queue occupancy, HOL delay, enqueue-to-dequeue delay, channel grants, backoff/collision events, retry attempts, drops by reason, airtime, and channel busy ratio.
   - Add `beGrantWhileBlocked`, `blockDutyCycle`, `maxContinuousBlockObserved`, and per-node FSM occupancy.
   - Split VO into `voLogicalTx`, `voPhysicalTx`, `voLogicalRx`, `voDuplicateRx`, and receiver-opportunity success.

2. Fix experimental controls:
   - Assign independent RNG streams for app traffic, MAC backoff, mobility, and jitter.
   - Add time-windowed summaries: pre-crash, crash, post-crash.
   - Add a no-repeat VO config to isolate EDCA from application-layer repetition.
   - Add equalized queue-capacity configs so DCF vs EDCA is not only queue isolation.
   - Add default EDCA, tuned EDCA, adaptive `stable`, adaptive `guarded`, and DCC-style rate-control baselines.

3. Expand scenarios:
   - Keep light/heavy highway, but add a controlled dense clique and a controlled hidden-terminal topology.
   - Test 1, 2, 5, and 10 crash sources.
   - Test broadcast/multicast separately from unicast emergency flows.
   - Replace/compare `IdealObstacleLoss` with a more graded path loss/shadowing setup.
   - Run at least 10 exploratory seeds and 30 publication seeds per final config.

4. Report KPIs:
   - VO/BE P50/P95/P99 delay, jitter, deadline-miss ratio, first-success latency, receiver-opportunity reach, MAC drop reason, collision probability, airtime share, CBR, queue occupancy, HOL delay, Jain fairness, and BE starvation duration.
   - Use confidence intervals and effect sizes, not only means. Treat p95/p99 as primary for safety traffic.

## Assumptions And Sources

Assumptions: preserve the minimal BE-vs-VO thesis anchor; avoid framework-level edits; frame adaptive `V2xHcf` as a research mechanism only after runtime validation.

Suggested baselines from standards/literature context: DCF, standard EDCA, tuned EDCA, application repetition only, adaptive BE suppression, ETSI DCC-style channel-load control, and WAVE/1609.4-style priority-access comparisons.

Sources used: INET QoS docs https://inet.omnetpp.org/docs/showcases/wireless/qos/doc/index.html, INET 802.11 model docs https://inet.omnetpp.org/docs/users-guide/ch-80211.html, IEEE 1609.4 https://standards.ieee.org/ieee/1609.4/6183/, IEEE 802.11bd/802.11p page https://standards.ieee.org/ieee/802.11bd/7451/, ETSI DCC TS 102 687 https://www.etsi.org/deliver/etsi_ts/102600_102699/102687/01.01.01_60/ts_102687v010101p.pdf, IETF DetNet bounded latency RFC 9320 https://www.ietf.org/rfc/rfc9320.html, IEEE TSN overview https://1.ieee802.org/tsn/, INET IdealObstacleLoss docs https://doc.omnetpp.org/inet/api-current/neddoc/inet.physicallayer.wireless.common.obstacleloss.IdealObstacleLoss.html.
