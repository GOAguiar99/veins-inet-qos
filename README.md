# Crash-Aware EDCA Traffic for Veins

This project provides a **minimal setup to evaluate IEEE 802.11e EDCA behavior in Veins**.

It generates **normal Best Effort (BE)** traffic and switches to **Voice (VO)** traffic when a crash event occurs, allowing direct observation of EDCA prioritization under congestion.

Project context for future AI/code sessions: see [`veins_qos/AI_CONTEXT.md`](veins_qos/AI_CONTEXT.md).

The focus is **only** on:
- BE vs VO differentiation
- Crash-triggered priority escalation
- Clean, deterministic EDCA evaluation

No extra QoS logic, no policy frameworks, no side effects.

Intended use: **measure whether crash packets actually get VO treatment in Veins/INET Wi-Fi simulations**.
