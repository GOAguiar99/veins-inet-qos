#!/bin/sh
set -eu

cd "$(dirname "$0")"

CONFIGS="plain_netload_high edca_only_netload_high edca_v2x_vo_stable_netload_low edca_v2x_vo_stable_netload_medium edca_v2x_vo_stable_netload_high edca_v2x_vo_guarded_netload_low edca_v2x_vo_guarded_netload_medium edca_v2x_vo_guarded_netload_high"
RUNS="${RUNS:-0}"
UI="${UI:-Cmdenv}"
EXTRA_ARGS="${EXTRA_ARGS:-}"

echo "Highway heavy matrix runner"
echo "configs: $CONFIGS"
echo "runs:    $RUNS"
echo "ui:      $UI"
echo

for config in $CONFIGS; do
    echo "--- Running: $config / run $RUNS ---"
    ./run -u "$UI" -c "$config" -r "$RUNS" $EXTRA_ARGS
done
