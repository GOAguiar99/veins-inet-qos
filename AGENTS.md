# AI Agent Guide

Start in `veins_qos/`.

This repository is a master's-project workspace centered on crash-aware QoS for vehicular communications using Veins, INET, OMNeT++, and SUMO.

Repository boundaries:
- `veins_qos/` is the active project and the default place for code, configs, docs, and experiments.
- `inet/`, `veins/`, and `omnetpp-6.1/` are dependency/framework trees. Treat them as read-only unless a task explicitly requires framework-level changes.

Before making non-trivial changes, read `veins_qos/AI_CONTEXT.md`.

Project guardrails:
- Preserve the minimal experiment design unless asked otherwise.
- The core study is about BE vs VO differentiation, crash-triggered priority escalation, and clean DCF vs EDCA comparison.
- Avoid adding large policy layers, unrelated abstractions, or "helpful" complexity that would blur the baseline experiment.
