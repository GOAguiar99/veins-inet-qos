#!/bin/sh
set -eu

cd "$(dirname "$0")"

CONFIGS="plain_netload_high edca_only_netload_high edca_v2x_vo_stable_netload_low edca_v2x_vo_stable_netload_medium edca_v2x_vo_stable_netload_high edca_v2x_vo_guarded_netload_low edca_v2x_vo_guarded_netload_medium edca_v2x_vo_guarded_netload_high"
RUNS="${RUNS:-0}"
UI="${UI:-Cmdenv}"
EXTRA_ARGS="${EXTRA_ARGS:-}"
TRACI_HOST="${TRACI_HOST:-localhost}"
TRACI_PORT="${TRACI_PORT:-9999}"

echo "Highway heavy matrix runner"
echo "configs: $CONFIGS"
echo "runs:    $RUNS"
echo "ui:      $UI"
echo "traci:   $TRACI_HOST:$TRACI_PORT"
echo

if ! nc -z "$TRACI_HOST" "$TRACI_PORT" >/dev/null 2>&1; then
    echo "TraCI server is not reachable at $TRACI_HOST:$TRACI_PORT." >&2
    echo "Start veins_launchd first, for example:" >&2
    echo "  cd /home/goaguiar/master/master_veins/veins/bin" >&2
    echo "  ./veins_launchd -vv" >&2
    exit 1
fi

for config in $CONFIGS; do
    echo "--- Running: $config / run $RUNS ---"
    ./run -u "$UI" -c "$config" -r "$RUNS" $EXTRA_ARGS
done
