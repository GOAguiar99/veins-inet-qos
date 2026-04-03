from __future__ import annotations

import math
import re
from pathlib import Path
from typing import Dict, List, Set, Tuple

import pandas as pd


SCALAR_RE = re.compile(r"^scalar\s+(\S+)\s+(\S+)\s+(\S+)\s*$")
VECTOR_HEADER_RE = re.compile(r"^vector\s+(\d+)\s+(\S+)\s+(\S+)\s+\S+\s*$")
APP0_RE = re.compile(r"^Scenario\.node\[\d+\]\.app\[0\]$")
APP1_RE = re.compile(r"^Scenario\.node\[\d+\]\.app\[1\]$")
APP_MODULE_RE = re.compile(r"^Scenario\.node\[(\d+)\]\.app\[([01])\]$")
FSM_CONTROLLER_RE = re.compile(r"^Scenario\.node\[(\d+)\]\.wlan\[\d+\]\.mac\.hcf\.FSMController$")
MAC_RE = re.compile(r"^Scenario\.node\[\d+\]\.wlan\[\d+\]\.mac$")
MAC_AC_DROP_SCALAR_RE = re.compile(
    r"^packetDropAc(Be|Vo|Unclassified)(?:Reason([A-Za-z0-9]+))?Count$"
)
EDCAF_PENDING_QUEUE_RE = re.compile(
    r"^Scenario\.node\[\d+\]\.wlan\[\d+\]\.mac\.hcf\.edca\.edcaf\[(\d+)\]\.pendingQueue$"
)
EDCAF_RECOVERY_RE = re.compile(
    r"^Scenario\.node\[\d+\]\.wlan\[\d+\]\.mac\.hcf\.edca\.edcaf\[(\d+)\]\.recoveryProcedure$"
)

AC_INDEX_BE = 1
AC_INDEX_VO = 3


def _to_float(raw: str) -> float:
    try:
        return float(raw)
    except ValueError:
        return math.nan


def _safe_weighted_mean(weighted_sum: float, count_sum: float) -> float:
    if count_sum <= 0:
        return math.nan
    return weighted_sum / count_sum


def _safe_min(values: List[float]) -> float:
    return min(values) if values else math.nan


def _safe_max(values: List[float]) -> float:
    return max(values) if values else math.nan


def _percentile(values: List[float], quantile: float) -> float:
    if not values:
        return math.nan
    if len(values) == 1:
        return values[0]

    ordered = sorted(values)
    position = (len(ordered) - 1) * quantile
    lower_index = int(math.floor(position))
    upper_index = int(math.ceil(position))
    if lower_index == upper_index:
        return ordered[lower_index]

    lower_value = ordered[lower_index]
    upper_value = ordered[upper_index]
    fraction = position - lower_index
    return lower_value + (upper_value - lower_value) * fraction


def _compute_jitter(values: List[float]) -> tuple[float, int]:
    if len(values) < 2:
        return 0.0, 0

    jitter_sum = 0.0
    for previous, current in zip(values, values[1:]):
        jitter_sum += abs(current - previous)
    return jitter_sum, len(values) - 1


def _extract_run_metadata(path: Path) -> Tuple[str, str]:
    configname = path.stem
    run_name = path.stem

    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("attr configname "):
                configname = line.split(" ", 2)[2].strip()
            elif line.startswith("run "):
                run_name = line.split(" ", 1)[1].strip()
            if configname != path.stem and run_name != path.stem:
                break

    return configname, run_name


def parse_vec_file(path: Path) -> Dict[str, float]:
    if not path.exists():
        return {}

    metric_by_vector_id: Dict[str, str] = {}
    values_by_metric: Dict[str, List[float]] = {"be": [], "vo": []}
    jitter_sum_by_metric: Dict[str, float] = {"be": 0.0, "vo": 0.0}
    jitter_count_by_metric: Dict[str, int] = {"be": 0, "vo": 0}
    values_by_vector_id: Dict[str, List[float]] = {}

    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            header_match = VECTOR_HEADER_RE.match(line)
            if header_match:
                vector_id, module, metric = header_match.groups()
                if APP0_RE.match(module):
                    if metric == "beEndToEndDelay:vector":
                        metric_by_vector_id[vector_id] = "be"
                        values_by_vector_id[vector_id] = []
                    elif metric == "voEndToEndDelay:vector":
                        metric_by_vector_id[vector_id] = "vo"
                        values_by_vector_id[vector_id] = []
                continue

            parts = line.split()
            if len(parts) < 4:
                continue

            vector_id = parts[0]
            metric_name = metric_by_vector_id.get(vector_id)
            if metric_name is None:
                continue

            value = _to_float(parts[3])
            if math.isnan(value):
                continue

            values_by_vector_id[vector_id].append(value)
            values_by_metric[metric_name].append(value)

    for vector_id, metric_name in metric_by_vector_id.items():
        jitter_sum, jitter_count = _compute_jitter(values_by_vector_id[vector_id])
        jitter_sum_by_metric[metric_name] += jitter_sum
        jitter_count_by_metric[metric_name] += jitter_count

    result: Dict[str, float] = {}
    for metric_name in ("be", "vo"):
        metric_values = values_by_metric[metric_name]
        jitter_count = jitter_count_by_metric[metric_name]
        jitter_s = (jitter_sum_by_metric[metric_name] / jitter_count) if jitter_count > 0 else math.nan

        result[f"{metric_name}_delay_p95_s"] = _percentile(metric_values, 0.95)
        result[f"{metric_name}_jitter_s"] = jitter_s

    return result


def parse_vec_timeseries(path: Path, bin_size_s: float = 1.0) -> List[Dict[str, float]]:
    if not path.exists():
        return []

    packet_sent_vector_to_app: Dict[str, Tuple[int, int]] = {}
    state_vector_to_node: Dict[str, int] = {}
    bits_by_bin_total: Dict[float, float] = {}
    bits_by_bin_be: Dict[float, float] = {}
    bits_by_bin_vo: Dict[float, float] = {}
    active_nodes_by_bin: Dict[float, Set[int]] = {}
    state_events_by_node: Dict[int, List[Tuple[float, int]]] = {}

    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            header_match = VECTOR_HEADER_RE.match(line)
            if header_match:
                vector_id, module, metric = header_match.groups()
                module_match = APP_MODULE_RE.match(module)
                if module_match and metric == "packetSent:vector(packetBytes)":
                    node_index = int(module_match.group(1))
                    app_index = int(module_match.group(2))
                    packet_sent_vector_to_app[vector_id] = (node_index, app_index)
                fsm_match = FSM_CONTROLLER_RE.match(module)
                if fsm_match and metric == "v2xState:vector":
                    node_index = int(fsm_match.group(1))
                    state_vector_to_node[vector_id] = node_index
                continue

            parts = line.split()
            if len(parts) < 4:
                continue

            vector_id = parts[0]
            time_s = _to_float(parts[2])
            if math.isnan(time_s):
                continue

            packet_source = packet_sent_vector_to_app.get(vector_id)
            if packet_source is not None:
                packet_bytes = _to_float(parts[3])
                if math.isnan(packet_bytes):
                    continue
                bin_index = int(math.floor(time_s / bin_size_s))
                bin_time_s = float(bin_index * bin_size_s)
                bits = packet_bytes * 8.0
                node_index, app_index = packet_source
                bits_by_bin_total[bin_time_s] = bits_by_bin_total.get(bin_time_s, 0.0) + bits
                # app[0] = BE sender app, app[1] = VO crash sender app in this project setup.
                if app_index == 0:
                    bits_by_bin_be[bin_time_s] = bits_by_bin_be.get(bin_time_s, 0.0) + bits
                elif app_index == 1:
                    bits_by_bin_vo[bin_time_s] = bits_by_bin_vo.get(bin_time_s, 0.0) + bits
                active_nodes_by_bin.setdefault(bin_time_s, set()).add(node_index)
                continue

            state_node_index = state_vector_to_node.get(vector_id)
            if state_node_index is not None:
                state_value = _to_float(parts[3])
                if math.isnan(state_value):
                    continue
                state_events_by_node.setdefault(state_node_index, []).append((time_s, int(state_value)))
                continue

    rows: List[Dict[str, float]] = []
    state_nodes = sorted(state_events_by_node.keys())
    all_bin_times = set(bits_by_bin_total.keys())
    all_bin_times.update(bits_by_bin_be.keys())
    all_bin_times.update(bits_by_bin_vo.keys())
    for events in state_events_by_node.values():
        for event_time_s, _ in events:
            all_bin_times.add(float(int(math.floor(event_time_s / bin_size_s)) * bin_size_s))

    if not all_bin_times:
        return rows

    max_bin_time_s = max(all_bin_times)
    num_bins = int(round(max_bin_time_s / bin_size_s)) + 1
    bin_times = [float(i * bin_size_s) for i in range(num_bins)]

    sorted_events_by_node: Dict[int, List[Tuple[float, int]]] = {}
    for node_index, events in state_events_by_node.items():
        sorted_events_by_node[node_index] = sorted(events, key=lambda item: item[0])

    current_state_by_node: Dict[int, int] = {node_index: 0 for node_index in state_nodes}
    event_pos_by_node: Dict[int, int] = {node_index: 0 for node_index in state_nodes}

    for bin_time_s in bin_times:
        bits_total = bits_by_bin_total.get(bin_time_s, 0.0)
        bits_be = bits_by_bin_be.get(bin_time_s, 0.0)
        bits_vo = bits_by_bin_vo.get(bin_time_s, 0.0)
        active_nodes = active_nodes_by_bin.get(bin_time_s, set())

        listening_nodes = math.nan
        blocking_nodes = math.nan
        sending_nodes = math.nan
        if state_nodes:
            listening = 0
            blocking = 0
            sending = 0
            for node_index in state_nodes:
                events = sorted_events_by_node[node_index]
                event_pos = event_pos_by_node[node_index]
                while event_pos < len(events) and events[event_pos][0] <= bin_time_s:
                    current_state_by_node[node_index] = events[event_pos][1]
                    event_pos += 1
                event_pos_by_node[node_index] = event_pos

                node_state = current_state_by_node[node_index]
                if node_state == 0:
                    listening += 1
                elif node_state == 1:
                    blocking += 1
                elif node_state == 2:
                    sending += 1

            listening_nodes = float(listening)
            blocking_nodes = float(blocking)
            sending_nodes = float(sending)

        rows.append(
            {
                "time_s": bin_time_s,
                "throughput_kbps": bits_total / bin_size_s / 1000.0,
                "throughput_be_kbps": bits_be / bin_size_s / 1000.0,
                "throughput_vo_kbps": bits_vo / bin_size_s / 1000.0,
                "active_tx_nodes": float(len(active_nodes)),
                "listening_nodes": listening_nodes,
                "blocking_nodes": blocking_nodes,
                "sending_nodes": sending_nodes,
            }
        )

    return rows


def parse_sca_file(path: Path) -> Dict[str, float]:
    configname = path.stem
    run_name = path.stem

    be_tx_total = 0.0
    be_rx_total = 0.0
    vo_tx_total = 0.0
    vo_rx_total = 0.0

    be_delay_weighted_sum = 0.0
    be_delay_count_sum = 0.0
    vo_delay_weighted_sum = 0.0
    vo_delay_count_sum = 0.0

    be_delay_count_by_module: Dict[str, float] = {}
    be_delay_mean_by_module: Dict[str, float] = {}
    be_delay_min_values: List[float] = []
    be_delay_max_values: List[float] = []
    vo_delay_count_by_module: Dict[str, float] = {}
    vo_delay_mean_by_module: Dict[str, float] = {}
    vo_delay_min_values: List[float] = []
    vo_delay_max_values: List[float] = []

    mac_drop_total = 0.0
    mac_drop_queue_overflow_total = 0.0
    mac_drop_retry_limit_total = 0.0
    mac_drop_be_queue_overflow_total = 0.0
    mac_drop_be_retry_limit_total = 0.0
    mac_drop_vo_queue_overflow_total = 0.0
    mac_drop_vo_retry_limit_total = 0.0
    mac_drop_be_total_from_mac = 0.0
    mac_drop_vo_total_from_mac = 0.0
    mac_drop_unclassified_total = 0.0
    saw_be_ac_metrics_from_mac = False
    saw_vo_ac_metrics_from_mac = False
    saw_unclassified_ac_metrics = False
    saw_be_ac_metrics = False
    saw_vo_ac_metrics = False

    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("attr configname "):
                configname = line.split(" ", 2)[2].strip()
                continue
            if line.startswith("run "):
                run_name = line.split(" ", 1)[1].strip()
                continue

            match = SCALAR_RE.match(line)
            if not match:
                continue

            module, metric, value_raw = match.groups()
            value = _to_float(value_raw)
            if math.isnan(value):
                continue

            if APP0_RE.match(module):
                if metric == "beTxPackets:count":
                    be_tx_total += value
                elif metric == "beRxPackets:count":
                    be_rx_total += value
                elif metric == "voRxPackets:count":
                    vo_rx_total += value
                elif metric == "beEndToEndDelay:count":
                    be_delay_count_by_module[module] = value
                elif metric == "beEndToEndDelay:mean":
                    be_delay_mean_by_module[module] = value
                elif metric == "beEndToEndDelay:min":
                    be_delay_min_values.append(value)
                elif metric == "beEndToEndDelay:max":
                    be_delay_max_values.append(value)
                elif metric == "voEndToEndDelay:count":
                    vo_delay_count_by_module[module] = value
                elif metric == "voEndToEndDelay:mean":
                    vo_delay_mean_by_module[module] = value
                elif metric == "voEndToEndDelay:min":
                    vo_delay_min_values.append(value)
                elif metric == "voEndToEndDelay:max":
                    vo_delay_max_values.append(value)
            elif APP1_RE.match(module):
                if metric == "voTxPackets:count":
                    vo_tx_total += value
            elif MAC_RE.match(module):
                if metric == "packetDrop:count":
                    mac_drop_total += value
                elif metric == "packetDropQueueOverflow:count":
                    mac_drop_queue_overflow_total += value
                elif metric == "packetDropRetryLimitReached:count":
                    mac_drop_retry_limit_total += value
                else:
                    mac_ac_drop_match = MAC_AC_DROP_SCALAR_RE.match(metric)
                    if mac_ac_drop_match:
                        ac_name, reason_name = mac_ac_drop_match.groups()
                        # Only top-level per-AC totals are used for dashboard attribution.
                        if reason_name is None:
                            if ac_name == "Be":
                                saw_be_ac_metrics_from_mac = True
                                mac_drop_be_total_from_mac += value
                            elif ac_name == "Vo":
                                saw_vo_ac_metrics_from_mac = True
                                mac_drop_vo_total_from_mac += value
                            elif ac_name == "Unclassified":
                                saw_unclassified_ac_metrics = True
                                mac_drop_unclassified_total += value
            else:
                pending_queue_match = EDCAF_PENDING_QUEUE_RE.match(module)
                if pending_queue_match and metric == "droppedPacketsQueueOverflow:count":
                    ac_index = int(pending_queue_match.group(1))
                    if ac_index == AC_INDEX_BE:
                        saw_be_ac_metrics = True
                        mac_drop_be_queue_overflow_total += value
                    elif ac_index == AC_INDEX_VO:
                        saw_vo_ac_metrics = True
                        mac_drop_vo_queue_overflow_total += value
                    continue

                recovery_match = EDCAF_RECOVERY_RE.match(module)
                if recovery_match and metric == "retryLimitReached:count":
                    ac_index = int(recovery_match.group(1))
                    if ac_index == AC_INDEX_BE:
                        saw_be_ac_metrics = True
                        mac_drop_be_retry_limit_total += value
                    elif ac_index == AC_INDEX_VO:
                        saw_vo_ac_metrics = True
                        mac_drop_vo_retry_limit_total += value

    for module, count in be_delay_count_by_module.items():
        mean = be_delay_mean_by_module.get(module, math.nan)
        if count > 0 and not math.isnan(mean):
            be_delay_count_sum += count
            be_delay_weighted_sum += count * mean

    for module, count in vo_delay_count_by_module.items():
        mean = vo_delay_mean_by_module.get(module, math.nan)
        if count > 0 and not math.isnan(mean):
            vo_delay_count_sum += count
            vo_delay_weighted_sum += count * mean

    be_delay_s = _safe_weighted_mean(be_delay_weighted_sum, be_delay_count_sum)
    vo_delay_s = _safe_weighted_mean(vo_delay_weighted_sum, vo_delay_count_sum)
    be_delay_min_s = _safe_min(be_delay_min_values)
    be_delay_max_s = _safe_max(be_delay_max_values)
    vo_delay_min_s = _safe_min(vo_delay_min_values)
    vo_delay_max_s = _safe_max(vo_delay_max_values)

    vec_stats = parse_vec_file(path.with_suffix(".vec"))
    be_delay_p95_s = vec_stats.get("be_delay_p95_s", math.nan)
    be_jitter_s = vec_stats.get("be_jitter_s", math.nan)
    vo_delay_p95_s = vec_stats.get("vo_delay_p95_s", math.nan)
    vo_jitter_s = vec_stats.get("vo_jitter_s", math.nan)

    mac_drop_be_total_fallback = mac_drop_be_queue_overflow_total + mac_drop_be_retry_limit_total
    mac_drop_vo_total_fallback = mac_drop_vo_queue_overflow_total + mac_drop_vo_retry_limit_total

    if saw_be_ac_metrics_from_mac:
        mac_drop_be_count = int(round(mac_drop_be_total_from_mac))
    elif saw_be_ac_metrics:
        mac_drop_be_count = int(round(mac_drop_be_total_fallback))
    else:
        mac_drop_be_count = math.nan

    if saw_vo_ac_metrics_from_mac:
        mac_drop_vo_count = int(round(mac_drop_vo_total_from_mac))
    elif saw_vo_ac_metrics:
        mac_drop_vo_count = int(round(mac_drop_vo_total_fallback))
    else:
        mac_drop_vo_count = math.nan

    if saw_unclassified_ac_metrics:
        mac_drop_unclassified_count = int(round(mac_drop_unclassified_total))
        if mac_drop_total > 0 and mac_drop_unclassified_count > 0:
            ratio = mac_drop_unclassified_count / mac_drop_total
            # Some MAC instrumentation reports per-AC totals at roughly 2x packetDrop:count.
            if 1.9 <= ratio <= 2.1:
                mac_drop_unclassified_count = int(round(mac_drop_unclassified_count / 2.0))
    else:
        no_be_attribution = math.isnan(mac_drop_be_count) or mac_drop_be_count == 0
        no_vo_attribution = math.isnan(mac_drop_vo_count) or mac_drop_vo_count == 0
        # In plain DCF runs, total MAC drops can be present without BE/VO AC attribution.
        mac_drop_unclassified_count = int(round(mac_drop_total)) if mac_drop_total > 0 and no_be_attribution and no_vo_attribution else 0

    app_tx_total = be_tx_total + vo_tx_total

    return {
        "config": configname,
        "run": run_name,
        "source_file": path.name,
        "be_delay_ms": be_delay_s * 1000.0 if not math.isnan(be_delay_s) else math.nan,
        "be_delay_min_ms": be_delay_min_s * 1000.0 if not math.isnan(be_delay_min_s) else math.nan,
        "be_delay_max_ms": be_delay_max_s * 1000.0 if not math.isnan(be_delay_max_s) else math.nan,
        "be_delay_p95_ms": be_delay_p95_s * 1000.0 if not math.isnan(be_delay_p95_s) else math.nan,
        "be_jitter_ms": be_jitter_s * 1000.0 if not math.isnan(be_jitter_s) else math.nan,
        "vo_delay_ms": vo_delay_s * 1000.0 if not math.isnan(vo_delay_s) else math.nan,
        "vo_delay_min_ms": vo_delay_min_s * 1000.0 if not math.isnan(vo_delay_min_s) else math.nan,
        "vo_delay_max_ms": vo_delay_max_s * 1000.0 if not math.isnan(vo_delay_max_s) else math.nan,
        "vo_delay_p95_ms": vo_delay_p95_s * 1000.0 if not math.isnan(vo_delay_p95_s) else math.nan,
        "vo_jitter_ms": vo_jitter_s * 1000.0 if not math.isnan(vo_jitter_s) else math.nan,
        "be_tx_count": int(round(be_tx_total)),
        "be_rx_count": int(round(be_rx_total)),
        "vo_tx_count": int(round(vo_tx_total)),
        "vo_rx_count": int(round(vo_rx_total)),
        "be_rx_per_tx": (be_rx_total / be_tx_total) if be_tx_total > 0 else math.nan,
        "vo_rx_per_tx": (vo_rx_total / vo_tx_total) if vo_tx_total > 0 else math.nan,
        "mac_drop_sum_count": int(round(mac_drop_total)),
        "mac_drop_queue_overflow_count": int(round(mac_drop_queue_overflow_total)),
        "mac_drop_retry_limit_count": int(round(mac_drop_retry_limit_total)),
        "mac_drop_be_count": mac_drop_be_count,
        "mac_drop_vo_count": mac_drop_vo_count,
        "mac_drop_unclassified_count": mac_drop_unclassified_count,
        "mac_drop_vo_per_vo_tx": (mac_drop_vo_count / vo_tx_total) if vo_tx_total > 0 and not math.isnan(mac_drop_vo_count) else math.nan,
        "mac_drop_be_per_be_tx": (mac_drop_be_count / be_tx_total) if be_tx_total > 0 and not math.isnan(mac_drop_be_count) else math.nan,
        "mac_drop_per_tx": (mac_drop_total / app_tx_total) if app_tx_total > 0 else math.nan,
    }


def load_results(results_dir: Path) -> pd.DataFrame:
    if not results_dir.exists():
        raise FileNotFoundError(f"Results directory not found: {results_dir}")

    sca_files = sorted(results_dir.glob("*.sca"))
    if not sca_files:
        raise FileNotFoundError(f"No .sca files found in: {results_dir}")

    rows: List[Dict[str, float]] = [parse_sca_file(path) for path in sca_files]
    frame = pd.DataFrame(rows)
    frame = frame.sort_values(["config", "run", "source_file"]).reset_index(drop=True)
    return frame


def load_timeseries(results_dir: Path, bin_size_s: float = 1.0) -> pd.DataFrame:
    if not results_dir.exists():
        raise FileNotFoundError(f"Results directory not found: {results_dir}")

    sca_files = sorted(results_dir.glob("*.sca"))
    if not sca_files:
        raise FileNotFoundError(f"No .sca files found in: {results_dir}")

    rows: List[Dict[str, float | str]] = []
    for sca_path in sca_files:
        config, run = _extract_run_metadata(sca_path)
        source_file = sca_path.name
        for entry in parse_vec_timeseries(sca_path.with_suffix(".vec"), bin_size_s=bin_size_s):
            rows.append(
                {
                    "config": config,
                    "run": run,
                    "source_file": source_file,
                    "time_s": entry["time_s"],
                    "throughput_kbps": entry["throughput_kbps"],
                    "throughput_be_kbps": entry.get("throughput_be_kbps", math.nan),
                    "throughput_vo_kbps": entry.get("throughput_vo_kbps", math.nan),
                    "active_tx_nodes": entry["active_tx_nodes"],
                    "listening_nodes": entry.get("listening_nodes", math.nan),
                    "blocking_nodes": entry.get("blocking_nodes", math.nan),
                    "sending_nodes": entry.get("sending_nodes", math.nan),
                }
            )

    if not rows:
        return pd.DataFrame(
            columns=[
                "config",
                "run",
                "source_file",
                "time_s",
                "throughput_kbps",
                "throughput_be_kbps",
                "throughput_vo_kbps",
                "active_tx_nodes",
                "listening_nodes",
                "blocking_nodes",
                "sending_nodes",
            ]
        )

    frame = pd.DataFrame(rows)
    return frame.sort_values(["config", "run", "time_s"]).reset_index(drop=True)
